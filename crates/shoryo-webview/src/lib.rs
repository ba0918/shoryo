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
