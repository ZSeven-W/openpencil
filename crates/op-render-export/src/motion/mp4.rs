//! Feed the native raster frames to the shared Rust MP4 encoder.

use std::io::{Seek, Write};

use op_motion_export::FrameClock;

use super::render::FrameRenderer;
use crate::ExportError;

pub(super) fn available() -> bool {
    cfg!(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    ))
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(super) fn encode<W: Write + Seek>(
    writer: W,
    renderer: &FrameRenderer,
    clock: FrameClock,
) -> Result<(), ExportError> {
    let size = renderer.size();
    let mut encoder = op_motion_export::Mp4Encoder::new(writer, size, clock)?;
    let mut rgba = vec![0; size.0 as usize * size.1 as usize * 4];
    for index in 0..clock.frame_count() {
        renderer.read_rgba(clock.time_at(index), &mut rgba)?;
        encoder.push_rgba(&rgba)?;
    }
    Ok(encoder.finish()?)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(super) fn encode<W: Write + Seek>(
    _: W,
    _: &FrameRenderer,
    _: FrameClock,
) -> Result<(), ExportError> {
    Err(ExportError::Mp4EncoderUnavailable)
}
