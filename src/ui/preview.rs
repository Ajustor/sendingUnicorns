//! The response preview: the OS web view (`WebView2`, `WKWebView`, `WebKitGTK`),
//! as in browsingUnicorns, laid over the Preview tab as a native child view.
//!
//! A native view covers whatever egui draws under it, so it is hidden while
//! a window, a dialog or a menu is open. The body is written to a temporary
//! file and loaded from there: images, PDFs and SVGs show as a browser shows
//! them; an HTML page gets a `<base>` so that its relative links resolve
//! against the address it came from.

use std::path::{Path, PathBuf};

use domain::Response;
use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};
use wry::dpi::{PhysicalPosition, PhysicalSize};
use wry::{Rect, WebView, WebViewBuilder};

/// The window the preview is embedded in, kept past the borrow eframe
/// hands out at start-up.
#[derive(Debug, Clone, Copy)]
struct ParentWindow(RawWindowHandle);

// R-RS-22: the only unsafe code, two lines behind a safe constructor that
// only `Preview::new` calls, with eframe's creation context.
#[allow(unsafe_code)]
impl HasWindowHandle for ParentWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: the handle comes from eframe's window, which outlives the
        // app (eframe drops the app, and with it this view, first).
        Ok(unsafe { WindowHandle::borrow_raw(self.0) })
    }
}

/// What a response's preview shows, by the file it is written to.
pub fn previewable(response: &Response) -> Option<&'static str> {
    let mime = response
        .header("content-type")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let extension = match mime.split(';').next().unwrap_or_default().trim() {
        "text/html" | "application/xhtml+xml" => "html",
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
        "image/bmp" => "bmp",
        "application/pdf" => "pdf",
        "text/xml" | "application/xml" => "xml",
        _ => return None,
    };
    Some(extension)
}

/// `html` with `<base href="url">`, so that relative links and assets load.
pub fn with_base(html: &str, url: &str) -> String {
    let base = format!("<base href=\"{}\">", url.replace('"', "%22"));
    let lower = html.to_ascii_lowercase();
    if lower.contains("<base") {
        return html.to_owned();
    }
    match lower
        .find("<head")
        .and_then(|i| lower[i..].find('>').map(|j| i + j + 1))
    {
        Some(at) => format!("{}{base}{}", &html[..at], &html[at..]),
        None => format!("{base}{html}"),
    }
}

pub struct Preview {
    parent: Option<ParentWindow>,
    view: Option<WebView>,
    /// Which response is loaded: its exchange's number.
    loaded: Option<u64>,
    dir: PathBuf,
    /// Shown this frame (else hidden at its end).
    shown: bool,
    visible: bool,
    bounds: Option<egui::Rect>,
    pub error: Option<String>,
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Preview")
            .field("loaded", &self.loaded)
            .field("visible", &self.visible)
            .finish_non_exhaustive()
    }
}

impl Preview {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(target_os = "linux")]
        if let Err(e) = gtk::init() {
            log::warn!("preview: GTK initialisation failed: {e}");
        }
        let parent = match cc.window_handle() {
            Ok(handle) => Some(ParentWindow(handle.as_raw())),
            Err(e) => {
                log::warn!("preview: no native window: {e}");
                None
            }
        };
        let dir =
            std::env::temp_dir().join(format!("sending-unicorns-preview-{}", std::process::id()));
        Self {
            parent,
            view: None,
            loaded: None,
            dir,
            shown: false,
            visible: false,
            bounds: None,
            error: None,
        }
    }

    /// Call at the start of every frame.
    pub fn begin_frame(&mut self, ctx: &egui::Context) {
        self.shown = false;
        // WebKitGTK only advances when its loop is pumped.
        #[cfg(target_os = "linux")]
        if self.view.is_some() {
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }
            if self.visible {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = ctx;
    }

    /// Shows `response` (exchange number `key`, from `url`) over `rect`.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        rect: egui::Rect,
        key: u64,
        url: &str,
        response: &Response,
    ) {
        self.shown = true;
        if self.view.is_none() {
            self.create(ctx, rect);
        }
        if self.loaded != Some(key) {
            self.loaded = Some(key);
            self.load(url, response);
        }
        let Some(view) = &self.view else {
            return;
        };
        if self.bounds != Some(rect)
            && let Err(e) = view.set_bounds(physical(ctx, rect))
        {
            log::warn!("preview: {e}");
        }
        self.bounds = Some(rect);
    }

    /// Call at the end of every frame: the view shows only if [`Self::show`]
    /// was called and nothing of egui's is over it.
    pub fn end_frame(&mut self, covered: bool) {
        let visible = self.shown && !covered;
        if visible != self.visible
            && let Some(view) = &self.view
        {
            if let Err(e) = view.set_visible(visible) {
                log::warn!("preview: {e}");
            }
            self.visible = visible;
        }
    }

    fn create(&mut self, ctx: &egui::Context, rect: egui::Rect) {
        let Some(parent) = &self.parent else {
            self.error = Some("No native window to show the preview in.".into());
            return;
        };
        match WebViewBuilder::new()
            .with_bounds(physical(ctx, rect))
            .with_background_color((255, 255, 255, 255))
            .with_focused(false)
            .build_as_child(parent)
        {
            Ok(view) => {
                self.view = Some(view);
                self.visible = true;
            }
            Err(e) => {
                log::warn!("preview: {e}");
                self.error = Some(format!("The system's web view is not available: {e}"));
            }
        }
    }

    fn load(&mut self, url: &str, response: &Response) {
        let Some(view) = &self.view else {
            return;
        };
        let Some(extension) = previewable(response) else {
            return;
        };
        let result = write_body(&self.dir, extension, url, response)
            .map_err(|e| e.to_string())
            .and_then(|path| view.load_url(&file_url(&path)).map_err(|e| e.to_string()));
        self.error = result.err();
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        // The temporary files go with the app.
        let _ = std::fs::remove_dir_all(&self.dir); // nothing to do if it can't
    }
}

/// egui's points to the window's physical pixels.
fn physical(ctx: &egui::Context, rect: egui::Rect) -> Rect {
    let scale = ctx.pixels_per_point();
    // Window coordinates: small, and sizes made positive by `max(1.0)`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        let (x, y) = (
            (rect.left() * scale).round() as i32,
            (rect.top() * scale).round() as i32,
        );
        let (w, h) = (
            (rect.width() * scale).round().max(1.0) as u32,
            (rect.height() * scale).round().max(1.0) as u32,
        );
        Rect {
            position: PhysicalPosition::new(x, y).into(),
            size: PhysicalSize::new(w, h).into(),
        }
    }
}

/// A new file per response, so that the web view never shows a cached one.
fn write_body(
    dir: &Path,
    extension: &str,
    url: &str,
    response: &Response,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = dir.join(format!("response-{nanos:x}.{extension}"));
    if extension == "html" {
        std::fs::write(&path, with_base(&response.text(), url))?;
    } else {
        std::fs::write(&path, &response.body)?;
    }
    Ok(path)
}

fn file_url(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    let path = path.replace(' ', "%20").replace('#', "%23");
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn response(content_type: &str) -> Response {
        Response {
            status: 200,
            reason: "OK".into(),
            headers: vec![("Content-Type".into(), content_type.into())],
            body: Vec::new(),
            elapsed: Duration::ZERO,
        }
    }

    #[test]
    fn what_can_be_previewed() {
        assert_eq!(
            previewable(&response("text/html; charset=utf-8")),
            Some("html")
        );
        assert_eq!(previewable(&response("image/PNG")), Some("png"));
        assert_eq!(previewable(&response("application/pdf")), Some("pdf"));
        assert_eq!(previewable(&response("application/json")), None);
    }

    #[test]
    fn html_gets_a_base() {
        assert_eq!(
            with_base("<html><HEAD><title>t</title></head></html>", "https://h/a/"),
            "<html><HEAD><base href=\"https://h/a/\"><title>t</title></head></html>"
        );
        assert_eq!(
            with_base("<p>hi</p>", "https://h/"),
            "<base href=\"https://h/\"><p>hi</p>"
        );
        assert_eq!(
            with_base("<base href=\"/x\"><p/>", "https://h/"),
            "<base href=\"/x\"><p/>"
        );
    }

    #[test]
    fn file_urls() {
        assert_eq!(
            file_url(Path::new(r"C:\Temp\a b.html")),
            "file:///C:/Temp/a%20b.html"
        );
        assert_eq!(file_url(Path::new("/tmp/x.png")), "file:///tmp/x.png");
    }
}
