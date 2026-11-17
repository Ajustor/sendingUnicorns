//! The dependency rule (R-ARCH-01, R-VER-04): each layer depends only on the
//! layers inside it, and the domain on nothing at all.

// A test file: a failed read is the failure report.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

/// The names under `[dependencies]` of a crate's manifest.
fn dependencies(krate: &str) -> Vec<String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates")
        .join(krate)
        .join("Cargo.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    let mut in_dependencies = false;
    let mut names = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
        } else if in_dependencies
            && !line.is_empty()
            && !line.starts_with('#')
            && let Some((name, _)) = line.split_once('=')
        {
            names.push(name.trim().trim_end_matches(".workspace").to_owned());
        }
    }
    names
}

#[test]
fn the_domain_depends_on_nothing() {
    assert_eq!(dependencies("domain"), Vec::<String>::new());
}

#[test]
fn the_application_depends_only_on_the_domain() {
    assert_eq!(dependencies("application"), ["domain"]);
}

#[test]
fn the_infrastructure_does_not_know_the_ui() {
    let deps = dependencies("infrastructure");
    assert!(deps.contains(&"domain".to_owned()) && deps.contains(&"application".to_owned()));
    for ui in ["egui", "eframe", "rfd"] {
        assert!(
            !deps.contains(&ui.to_owned()),
            "infrastructure depends on {ui}"
        );
    }
}

#[test]
fn the_inner_crates_do_not_reach_outward() {
    for (krate, forbidden) in [
        ("domain", ["application", "infrastructure"]),
        ("application", ["infrastructure", "updater"]),
    ] {
        let sources = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("crates")
            .join(krate)
            .join("src");
        for entry in fs::read_dir(&sources).unwrap() {
            let path = entry.unwrap().path();
            let text = fs::read_to_string(&path).unwrap();
            for outer in forbidden {
                assert!(
                    !text.contains(&format!("{outer}::")),
                    "{} uses {outer}",
                    path.display()
                );
            }
        }
    }
}
