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
    fn embedded_screen_assets_are_available_with_their_media_types() {
        assert!(
            Screen::iter().next().is_some(),
            "the binary must contain the screen"
        );
        for path in Screen::iter() {
            let asset = file(&path).expect("every embedded asset is retrievable");
            let expected = match path.rsplit_once('.').map(|(_, extension)| extension) {
                Some("html") => "text/html; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("js") => "text/javascript; charset=utf-8",
                Some("svg") => "image/svg+xml",
                Some("json") => "application/json",
                _ => "application/octet-stream",
            };
            assert_eq!(asset.content_type, expected, "wrong media type: {path}");
        }
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
