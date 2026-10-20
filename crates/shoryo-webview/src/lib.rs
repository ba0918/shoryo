//! The screen's files, embedded into the binary so it runs without any file beside it.
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::borrow::Cow;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/"]
#[exclude = "tests/*"]
struct Screen;

/// One file of the screen.
pub struct File {
    pub bytes: Cow<'static, [u8]>,
    pub content_type: &'static str,
}

/// The screen's file at `path` (relative to `web/`), if there is one.
pub fn file(path: &str) -> Option<File> {
    let embedded = Screen::get(path)?;
    Some(File {
        bytes: embedded.data,
        content_type: content_type(path),
    })
}

// The screen has only a handful of file kinds, so a short table stands in for a MIME crate.
fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_types_follow_the_file_extension() {
        for (path, expected) in [
            ("index.html", "text/html; charset=utf-8"),
            ("style.css", "text/css; charset=utf-8"),
            ("components/card.js", "text/javascript; charset=utf-8"),
            ("icon.svg", "image/svg+xml"),
            ("data.json", "application/json"),
            ("archive.tar.gz", "application/octet-stream"),
            ("README", "application/octet-stream"),
        ] {
            assert_eq!(content_type(path), expected, "{path}");
        }
    }

    #[test]
    fn every_embedded_asset_is_served_with_a_known_media_type() {
        assert!(
            Screen::iter().next().is_some(),
            "the binary must contain the screen"
        );
        // Browsers refuse module scripts and style sheets sent as octet-stream.
        for path in Screen::iter() {
            let asset = file(&path).expect("every embedded asset is retrievable");
            assert_ne!(
                asset.content_type, "application/octet-stream",
                "unknown media type: {path}"
            );
        }
        assert_eq!(
            file("index.html").map(|asset| asset.content_type),
            Some("text/html; charset=utf-8")
        );
    }

    #[test]
    fn test_fixtures_and_paths_outside_the_screen_are_not_served() {
        assert!(Screen::iter().all(|path| !path.starts_with("tests/")));
        for path in [
            "tests/fixtures.js",
            "../Cargo.toml",
            "/etc/passwd",
            "missing-asset.js",
        ] {
            assert!(file(path).is_none(), "unexpected asset: {path}");
        }
    }
}
