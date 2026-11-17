//! The operating system's interface font, read from its usual place: the
//! app then looks like the system's other apps.

use std::path::{Path, PathBuf};

/// Where the system's UI font usually is, best first.
pub fn candidates() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let windir =
            std::env::var_os("WINDIR").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        vec![windir.join("Fonts").join("segoeui.ttf")]
    }
    #[cfg(target_os = "macos")]
    {
        vec![
            PathBuf::from("/System/Library/Fonts/SFNS.ttf"),
            PathBuf::from("/System/Library/Fonts/Helvetica.ttc"),
        ]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // fontconfig's choice for the default sans-serif.
        std::process::Command::new("fc-match")
            .args(["-f", "%{file}", "sans-serif"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|p| vec![PathBuf::from(p.trim())])
            .unwrap_or_default()
    }
}

/// The bytes of the first font of `paths` that can be read.
pub fn first_readable(paths: &[PathBuf]) -> Option<(PathBuf, Vec<u8>)> {
    paths
        .iter()
        .find_map(|p| read(p).map(|bytes| (p.clone(), bytes)))
}

fn read(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok().filter(|b| !b.is_empty())
}

/// The system's UI font, if found.
pub fn load() -> Option<(PathBuf, Vec<u8>)> {
    first_readable(&candidates())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_first_readable_font() {
        let dir = std::env::temp_dir().join(format!("bu-font-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let empty = dir.join("empty.ttf");
        let font = dir.join("font.ttf");
        std::fs::write(&empty, b"").unwrap();
        std::fs::write(&font, b"font bytes").unwrap();
        let missing = dir.join("missing.ttf");

        let (path, bytes) = first_readable(&[missing.clone(), empty, font.clone()]).unwrap();
        assert_eq!((path, bytes), (font, b"font bytes".to_vec()));
        assert!(first_readable(&[missing]).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn segoe_ui_on_windows() {
        assert!(candidates()[0].ends_with("Fonts\\segoeui.ttf"));
    }
}
