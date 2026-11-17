//! The window without the system's frame (Windows, Linux): resized from its
//! edges by hand. macOS keeps its frame, the content drawn under its title
//! bar.

use egui::{CursorIcon, ResizeDirection, ViewportCommand};

/// The border, in points, where the pointer resizes the window.
pub const BORDER: f32 = 4.0;

/// Corners grab a bit more, to be easy to catch.
const CORNER: f32 = 12.0;

/// Whether this window is resized by hand: no system frame, not maximized
/// nor full screen.
pub fn resizable(ctx: &egui::Context) -> bool {
    !cfg!(target_os = "macos")
        && ctx.input(|i| {
            let v = i.viewport();
            v.maximized != Some(true) && v.fullscreen != Some(true)
        })
}

/// Over an edge, the resize cursor; pressed there, the system resizes the
/// window.
pub fn resize_edges(ctx: &egui::Context) {
    if !resizable(ctx) {
        return;
    }
    let Some(pointer) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let Some(direction) = edge(ctx.screen_rect(), pointer) else {
        return;
    };
    ctx.set_cursor_icon(cursor(direction));
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}

/// The edge or corner of `window` under `pointer`, if any.
fn edge(window: egui::Rect, pointer: egui::Pos2) -> Option<ResizeDirection> {
    let near = |distance: f32, reach: f32| (0.0..reach).contains(&distance);
    let left = pointer.x - window.left();
    let right = window.right() - pointer.x;
    let top = pointer.y - window.top();
    let bottom = window.bottom() - pointer.y;
    let (w, e) = (near(left, CORNER), near(right, CORNER));
    let (n, s) = (near(top, CORNER), near(bottom, CORNER));
    Some(match () {
        () if n && w => ResizeDirection::NorthWest,
        () if n && e => ResizeDirection::NorthEast,
        () if s && w => ResizeDirection::SouthWest,
        () if s && e => ResizeDirection::SouthEast,
        () if near(top, BORDER) => ResizeDirection::North,
        () if near(bottom, BORDER) => ResizeDirection::South,
        () if near(left, BORDER) => ResizeDirection::West,
        () if near(right, BORDER) => ResizeDirection::East,
        () => return None,
    })
}

fn cursor(direction: ResizeDirection) -> CursorIcon {
    match direction {
        ResizeDirection::North => CursorIcon::ResizeNorth,
        ResizeDirection::South => CursorIcon::ResizeSouth,
        ResizeDirection::East => CursorIcon::ResizeEast,
        ResizeDirection::West => CursorIcon::ResizeWest,
        ResizeDirection::NorthEast => CursorIcon::ResizeNorthEast,
        ResizeDirection::NorthWest => CursorIcon::ResizeNorthWest,
        ResizeDirection::SouthEast => CursorIcon::ResizeSouthEast,
        ResizeDirection::SouthWest => CursorIcon::ResizeSouthWest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_and_corners_are_found() {
        let w = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let at = |x, y| edge(w, egui::pos2(x, y));
        assert_eq!(at(400.0, 300.0), None);
        assert_eq!(at(400.0, 1.0), Some(ResizeDirection::North));
        assert_eq!(at(400.0, 598.0), Some(ResizeDirection::South));
        assert_eq!(at(1.0, 300.0), Some(ResizeDirection::West));
        assert_eq!(at(799.0, 300.0), Some(ResizeDirection::East));
        assert_eq!(at(5.0, 595.0), Some(ResizeDirection::SouthWest));
        assert_eq!(at(795.0, 8.0), Some(ResizeDirection::NorthEast));
        assert_eq!(at(400.0, 6.0), None, "corners only grab more");
    }
}
