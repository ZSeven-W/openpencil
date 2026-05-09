//! Skottie - Lottie Animation Support
//!
//! This module provides support for rendering Lottie animations via Skia's Skottie module.
//!
//! # Example
//!
//! ```no_run
//! use skia_safe::{skottie::Animation, surfaces};
//!
//! let json = r#"{"v":"5.5.7","fr":30,"ip":0,"op":60,"w":200,"h":200,"layers":[]}"#;
//! if let Some(animation) = Animation::from_str(json) {
//!     let mut surface = surfaces::raster_n32_premul((200, 200)).unwrap();
//!     animation.render(surface.canvas(), None);
//! }
//! ```

use std::{ffi::CString, fmt, path::Path};

use crate::{interop, prelude::*, Canvas, FontMgr, Rect, Size};
use skia_bindings::{self as sb, SkNVRefCnt};

use super::resources::NativeResourceProvider;

/// A Lottie animation that can be rendered to a canvas.
///
/// Animations are reference-counted and can be cloned cheaply.
pub type Animation = RCHandle<sb::skottie_Animation>;
unsafe_send_sync!(Animation);
require_base_type!(sb::skottie_Animation, SkNVRefCnt);

impl NativeRefCounted for sb::skottie_Animation {
    fn _ref(&self) {
        unsafe { sb::C_skottie_Animation_ref(self) }
    }

    fn _unref(&self) {
        unsafe { sb::C_skottie_Animation_unref(self) }
    }

    fn unique(&self) -> bool {
        unsafe { sb::C_skottie_Animation_unique(self) }
    }
}

impl fmt::Debug for Animation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Animation")
            .field("version", &self.version())
            .field("duration", &self.duration())
            .field("fps", &self.fps())
            .field("size", &self.size())
            .finish()
    }
}

bitflags::bitflags! {
    /// Flags for rendering control.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RenderFlags: u32 {
        /// When rendering into a transparent canvas, disables the implicit
        /// top-level isolation layer.
        const SKIP_TOP_LEVEL_ISOLATION = 0x01;
        /// Disables the top-level clipping to the animation bounds.
        const DISABLE_TOP_LEVEL_CLIPPING = 0x02;
    }
}

bitflags::bitflags! {
    /// Flags for configuring the animation builder.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct BuilderFlags: u32 {
        /// Defer image loading until the image asset is actually used.
        const DEFER_IMAGE_LOADING = 0x01;
        /// Prefer embedded fonts over system fonts.
        const PREFER_EMBEDDED_FONTS = 0x02;
    }
}

/// A builder for creating [`Animation`] instances with custom configuration.
///
/// The builder allows setting a resource provider for loading external assets,
/// a font manager for text rendering, and various flags to control animation loading.
///
/// # Example
///
/// ```no_run
/// use skia_safe::skottie::Builder;
///
/// let json = r#"{"v":"5.5.7","fr":30,"ip":0,"op":60,"w":200,"h":200,"layers":[]}"#;
/// let animation = Builder::new().make(json);
/// ```
pub type Builder = RefHandle<sb::skottie_Animation_Builder>;

impl NativeDrop for sb::skottie_Animation_Builder {
    fn drop(&mut self) {
        unsafe { sb::C_skottie_Builder_delete(self) }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Builder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder").finish()
    }
}

impl Builder {
    /// Create a new animation builder with default settings.
    pub fn new() -> Self {
        Self::from_ptr(unsafe { sb::C_skottie_Builder_new(0) }).unwrap()
    }

    /// Create a new animation builder with the specified flags.
    pub fn with_flags(flags: BuilderFlags) -> Self {
        Self::from_ptr(unsafe { sb::C_skottie_Builder_new(flags.bits()) }).unwrap()
    }

    /// Set the font manager to use for text rendering.
    ///
    /// Consumes and returns self for method chaining.
    pub fn set_font_manager(mut self, font_mgr: FontMgr) -> Self {
        unsafe { sb::C_skottie_Builder_setFontManager(self.native_mut(), font_mgr.into_ptr()) }
        self
    }

    /// Set the resource provider for loading external assets.
    ///
    /// The resource provider is used to load images, fonts, and other external
    /// resources referenced by the animation.
    ///
    /// Consumes and returns self for method chaining.
    pub fn set_resource_provider(mut self, provider: impl Into<NativeResourceProvider>) -> Self {
        let provider = provider.into();
        unsafe { sb::C_skottie_Builder_setResourceProvider(self.native_mut(), provider.into_ptr()) }
        self
    }

    /// Build an animation from a JSON string.
    ///
    /// Returns `None` if the JSON cannot be parsed as a valid Lottie animation.
    pub fn make(mut self, json: impl AsRef<str>) -> Option<Animation> {
        let json = json.as_ref();
        Animation::from_ptr(unsafe {
            sb::C_skottie_Builder_make(self.native_mut(), json.as_ptr() as _, json.len())
        })
    }

    /// Build an animation from a file path.
    ///
    /// Returns `None` if the file cannot be loaded or parsed.
    /// Note: This will return `None` for non-UTF8 paths or paths containing null bytes.
    pub fn make_from_file(mut self, path: impl AsRef<Path>) -> Option<Animation> {
        let path = CString::new(path.as_ref().to_str()?).ok()?;
        Animation::from_ptr(unsafe {
            sb::C_skottie_Builder_makeFromFile(self.native_mut(), path.as_ptr())
        })
    }
}

impl Animation {
    /// Parse a Lottie animation from a JSON string.
    ///
    /// Returns `None` if the string cannot be parsed.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(json: impl AsRef<str>) -> Option<Self> {
        Self::from_bytes(json.as_ref().as_bytes())
    }

    /// Parse a Lottie animation from JSON bytes.
    ///
    /// Returns `None` if the data cannot be parsed.
    pub fn from_bytes(json: &[u8]) -> Option<Self> {
        Self::from_ptr(unsafe {
            sb::C_skottie_Animation_Make(json.as_ptr() as *const _, json.len())
        })
    }

    /// Load a Lottie animation from a file path.
    ///
    /// Returns `None` if the file cannot be loaded or parsed.
    /// Note: This will return `None` for non-UTF8 paths or paths containing null bytes.
    pub fn from_file(path: impl AsRef<Path>) -> Option<Self> {
        let path_str = path.as_ref().to_str()?;
        let c_path = CString::new(path_str).ok()?;
        Self::from_ptr(unsafe { sb::C_skottie_Animation_MakeFromFile(c_path.as_ptr()) })
    }

    /// Returns the Lottie format version string from the animation file.
    pub fn version(&self) -> interop::String {
        let mut version = interop::String::default();
        unsafe { sb::C_skottie_Animation_version(self.native(), version.native_mut()) };
        version
    }

    /// Returns the animation duration in seconds.
    pub fn duration(&self) -> f32 {
        unsafe { sb::C_skottie_Animation_duration(self.native()) }
    }

    /// Returns the animation frame rate (frames per second).
    pub fn fps(&self) -> f32 {
        unsafe { sb::C_skottie_Animation_fps(self.native()) }
    }

    /// Returns the first frame index (usually 0).
    pub fn in_point(&self) -> f32 {
        unsafe { sb::C_skottie_Animation_inPoint(self.native()) }
    }

    /// Returns the last frame index.
    pub fn out_point(&self) -> f32 {
        unsafe { sb::C_skottie_Animation_outPoint(self.native()) }
    }

    /// Returns the intrinsic animation size.
    pub fn size(&self) -> Size {
        let mut size = Size::default();
        unsafe { sb::C_skottie_Animation_size(self.native(), size.native_mut()) };
        size
    }

    /// Seek to a normalized position in the animation.
    ///
    /// `t` is in the range `[0, 1]` where 0 is the first frame and 1 is the last frame.
    ///
    /// Note: This method uses interior mutability as the underlying Skia animation
    /// state is updated internally.
    pub fn seek(&self, t: f32) {
        unsafe { sb::C_skottie_Animation_seek(self.native() as *const _ as *mut _, t) }
    }

    /// Seek to a specific frame number.
    ///
    /// Frame numbers are in the range `[in_point(), out_point()]`.
    /// Fractional frame values are supported for sub-frame accuracy.
    ///
    /// Note: This method uses interior mutability as the underlying Skia animation
    /// state is updated internally.
    pub fn seek_frame(&self, frame: f64) {
        unsafe { sb::C_skottie_Animation_seekFrame(self.native() as *const _ as *mut _, frame) }
    }

    /// Seek to a specific time in seconds.
    ///
    /// Time is in the range `[0, duration()]`.
    ///
    /// Note: This method uses interior mutability as the underlying Skia animation
    /// state is updated internally.
    pub fn seek_frame_time(&self, time: f64) {
        unsafe { sb::C_skottie_Animation_seekFrameTime(self.native() as *const _ as *mut _, time) }
    }

    /// Render the current frame to the canvas.
    ///
    /// If `dst` is provided, the animation is scaled/positioned to fit the rectangle.
    /// If `dst` is `None`, the animation is rendered at its intrinsic size at the origin.
    pub fn render(&self, canvas: &Canvas, dst: impl Into<Option<Rect>>) {
        let dst = dst.into();
        unsafe {
            sb::C_skottie_Animation_render(
                self.native(),
                canvas.native_mut(),
                dst.as_ref()
                    .map(|r| r.native() as *const _)
                    .unwrap_or(std::ptr::null()),
            )
        }
    }

    /// Render the current frame to the canvas with additional flags.
    ///
    /// If `dst` is provided, the animation is scaled/positioned to fit the rectangle.
    /// If `dst` is `None`, the animation is rendered at its intrinsic size at the origin.
    pub fn render_with_flags(
        &self,
        canvas: &Canvas,
        dst: impl Into<Option<Rect>>,
        flags: RenderFlags,
    ) {
        let dst = dst.into();
        unsafe {
            sb::C_skottie_Animation_render_with_flags(
                self.native(),
                canvas.native_mut(),
                dst.as_ref()
                    .map(|r| r.native() as *const _)
                    .unwrap_or(std::ptr::null()),
                flags.bits(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surfaces;

    #[test]
    fn parse_minimal_animation() {
        let json = r#"{"v":"5.5.7","fr":30,"ip":0,"op":60,"w":200,"h":200,"layers":[]}"#;
        let anim = Animation::from_str(json).expect("Failed to parse animation");

        assert_eq!(anim.version().as_str(), "5.5.7");
        assert_eq!(anim.fps(), 30.0);
        assert_eq!(anim.in_point(), 0.0);
        assert_eq!(anim.out_point(), 60.0);
        // duration = (out_point - in_point) / fps = 60 / 30 = 2.0
        assert!((anim.duration() - 2.0).abs() < 0.001);
        assert_eq!(anim.size(), Size::new(200.0, 200.0));
    }

    #[test]
    fn render_animation() {
        let json = r#"{"v":"5.5.7","fr":30,"ip":0,"op":60,"w":200,"h":200,"layers":[]}"#;
        let anim = Animation::from_str(json).expect("Failed to parse animation");

        let mut surface = surfaces::raster_n32_premul((200, 200)).unwrap();
        anim.seek(0.0);
        anim.render(surface.canvas(), None);
    }

    #[test]
    fn invalid_json_returns_none() {
        assert!(Animation::from_str("not valid json").is_none());
        assert!(Animation::from_str("{}").is_none());
    }

    #[test]
    fn builder_basic() {
        let json = r#"{"v":"5.5.7","fr":30,"ip":0,"op":60,"w":200,"h":200,"layers":[]}"#;
        let anim = Builder::new().make(json).expect("build failed");
        assert_eq!(anim.fps(), 30.0);
    }

    #[test]
    fn builder_with_flags() {
        let json = r#"{"v":"5.5.7","fr":30,"ip":0,"op":60,"w":200,"h":200,"layers":[]}"#;
        let anim = Builder::with_flags(BuilderFlags::DEFER_IMAGE_LOADING)
            .make(json)
            .expect("build failed");
        assert_eq!(anim.fps(), 30.0);
    }

    #[test]
    fn builder_with_font_manager() {
        let json = r#"{"v":"5.5.7","fr":30,"ip":0,"op":60,"w":200,"h":200,"layers":[]}"#;
        let font_mgr = FontMgr::default();
        let anim = Builder::new()
            .set_font_manager(font_mgr)
            .make(json)
            .expect("build failed");
        assert_eq!(anim.fps(), 30.0);
    }

    #[test]
    fn builder_default() {
        // Test that Default is implemented
        let builder: Builder = Default::default();
        let json = r#"{"v":"5.5.7","fr":30,"ip":0,"op":60,"w":200,"h":200,"layers":[]}"#;
        let _anim = builder.make(json).expect("build failed");
    }
}
