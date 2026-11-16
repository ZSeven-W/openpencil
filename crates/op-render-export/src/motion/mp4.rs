//! Platform availability for the in-process MP4 encoder.

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
use {super::render::FrameRenderer, crate::ExportError, std::io::Write};

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[path = "mp4_rust.rs"]
mod encoder;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(super) use encoder::encode;

pub(super) fn available() -> bool {
    cfg!(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(super) fn encode<W: Write>(
    _: W,
    _: &FrameRenderer,
    _: u32,
    _: u64,
    _: impl Fn(u64) -> u64,
) -> Result<(), ExportError> {
    Err(ExportError::Mp4EncoderUnavailable)
}
