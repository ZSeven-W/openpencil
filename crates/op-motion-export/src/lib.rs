//! Shared, renderer-independent motion export. Hosts paint each sampled
//! frame on their own surface and give the opaque RGBA bytes to the encoder.

mod scene;

#[cfg(any(
    target_arch = "wasm32",
    target_os = "macos",
    target_os = "linux",
    target_os = "windows"
))]
mod mp4;

#[cfg(any(
    target_arch = "wasm32",
    target_os = "macos",
    target_os = "linux",
    target_os = "windows"
))]
pub use mp4::Mp4Encoder;
pub use scene::{collect_raster_bounds, MotionScene};

use op_editor_core::motion_export::{motion_frame_count, MAX_MP4_DURATION_MS};
use op_editor_ui::Rect;

pub const MAX_RASTER_SIDE_PX: i64 = 16_384;
pub const MAX_RASTER_TOTAL_PX: i64 = 64_000_000;

#[derive(Debug)]
pub enum MotionError {
    NoActivePage,
    RootMissing(String),
    RootHidden(String),
    RootPaintsNothing(String),
    NonFiniteSize,
    OutputTooLarge { width_px: i64, height_px: i64 },
    InvalidFps(u32),
    EmptyClip,
    DurationRequired,
    DurationTooLong { duration_ms: u64, limit_ms: u64 },
    Encoding(String),
}

impl std::fmt::Display for MotionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoActivePage => f.write_str("the document has no active page"),
            Self::RootMissing(id) => write!(f, "motion root {id} is missing on the active page"),
            Self::RootHidden(id) => write!(f, "motion root {id} is hidden"),
            Self::RootPaintsNothing(id) => write!(f, "motion root {id} paints nothing"),
            Self::NonFiniteSize => f.write_str("non-finite export dimensions"),
            Self::OutputTooLarge {
                width_px,
                height_px,
            } => write!(
                f,
                "export dimensions {width_px}×{height_px} exceed the raster limit"
            ),
            Self::InvalidFps(fps) => write!(f, "motion frame rate {fps} is outside 1–60 fps"),
            Self::EmptyClip => f.write_str("there is no matching motion clip"),
            Self::DurationRequired => f.write_str("a looping clip needs an export duration"),
            Self::DurationTooLong {
                duration_ms,
                limit_ms,
            } => write!(f, "motion duration {duration_ms} ms exceeds {limit_ms} ms"),
            Self::Encoding(error) => write!(f, "MP4 encoding failed: {error}"),
        }
    }
}

impl std::error::Error for MotionError {}

/// Clamp the shared export scale to @0.5x–@8x, using @2x for a non-finite input.
pub fn clamp_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.clamp(0.5, 8.0)
    } else {
        2.0
    }
}

pub fn raster_size(bounds: Rect, scale: f32, margin: f32) -> Result<(i32, i32), MotionError> {
    let width = ((bounds.size.x + margin * 2.0) * scale).round();
    let height = ((bounds.size.y + margin * 2.0) * scale).round();
    if !width.is_finite() || !height.is_finite() {
        return Err(MotionError::NonFiniteSize);
    }
    let width_px = (width as i64).max(1);
    let height_px = (height as i64).max(1);
    check_raster_size(width_px, height_px)?;
    Ok((width_px as i32, height_px as i32))
}

fn check_raster_size(width_px: i64, height_px: i64) -> Result<(), MotionError> {
    if width_px > MAX_RASTER_SIDE_PX
        || height_px > MAX_RASTER_SIDE_PX
        || width_px.saturating_mul(height_px) > MAX_RASTER_TOTAL_PX
    {
        Err(MotionError::OutputTooLarge {
            width_px,
            height_px,
        })
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct FrameClock {
    duration_ms: u64,
    fps: u32,
}

impl FrameClock {
    pub fn validate_fps(fps: u32) -> Result<(), MotionError> {
        if (1..=60).contains(&fps) {
            Ok(())
        } else {
            Err(MotionError::InvalidFps(fps))
        }
    }

    pub fn new(fps: u32, duration_ms: Option<u64>) -> Result<Self, MotionError> {
        Self::validate_fps(fps)?;
        Ok(Self {
            fps,
            duration_ms: duration_ms.ok_or(MotionError::DurationRequired)?,
        })
    }

    pub fn validate_mp4(self) -> Result<(), MotionError> {
        if self.duration_ms > MAX_MP4_DURATION_MS {
            Err(MotionError::DurationTooLong {
                duration_ms: self.duration_ms,
                limit_ms: MAX_MP4_DURATION_MS,
            })
        } else {
            Ok(())
        }
    }

    pub fn frame_count(self) -> u64 {
        motion_frame_count(self.duration_ms, self.fps)
    }

    pub fn time_at(self, index: u64) -> u64 {
        (index.saturating_mul(1_000) / u64::from(self.fps)).min(self.duration_ms)
    }
}
