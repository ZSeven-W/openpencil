use std::{
    error::Error,
    fmt,
    io::{self},
};

use crate::{
    interop::{MemoryStream, NativeStreamBase, RustStream},
    prelude::*,
    resources::NativeResourceProvider,
    Canvas, Size,
};
use skia_bindings::{self as sb, SkRefCntBase};

use super::Svg;

pub type Dom = RCHandle<sb::SkSVGDOM>;
require_base_type!(sb::SkSVGDOM, sb::SkRefCnt);

impl NativeRefCountedBase for sb::SkSVGDOM {
    type Base = SkRefCntBase;
}

impl fmt::Debug for Dom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dom").finish()
    }
}

/// This type represents an SVG as a node-based data structure.
///
/// To convert an SVG to a `Dom`, a [`NativeResourceProvider`] is required.
///
/// ### Creating a Resource Provider
///
/// To create a resource provider, a [`crate::FontMgr`] is required at a minimum.
///
/// - If you don't need font support, pass [`crate::FontMgr::new_empty()`] as the resource provider.
/// - To use the installed fonts on your system, pass [`crate::FontMgr::default()`] as the resource provider.
///
/// When you pass a [`crate::FontMgr`] as the resource provider, a
/// [`crate::resources::LocalResourceProvider`] is created behind the scenes. This provider, in
/// addition to supporting typefaces, also adds support for `data:` URLs.
///
/// ### Supporting External Resources
///
/// To support `http://` or `https://` external resources, enable the `ureq` feature and create a
/// [`crate::resources::UReqResourceProvider`].
///
/// ### Custom Resource Providers
///
/// If you need more customization, you can implement the trait [`crate::resources::ResourceProvider`].
impl Dom {
    pub fn read<R: io::Read>(
        mut reader: R,
        resource_provider: impl Into<NativeResourceProvider>,
    ) -> Result<Self, LoadError> {
        let mut reader = RustStream::new(&mut reader);
        let stream = reader.stream_mut();
        let resource_provider = resource_provider.into();

        let out = unsafe { sb::C_SkSVGDOM_MakeFromStream(stream, resource_provider.into_ptr()) };

        Self::from_ptr(out).ok_or(LoadError)
    }

    pub fn from_str(
        svg: impl AsRef<str>,
        resource_provider: impl Into<NativeResourceProvider>,
    ) -> Result<Self, LoadError> {
        Self::from_bytes(svg.as_ref().as_bytes(), resource_provider)
    }

    pub fn from_bytes(
        svg: &[u8],
        resource_provider: impl Into<NativeResourceProvider>,
    ) -> Result<Self, LoadError> {
        let mut ms = MemoryStream::from_bytes(svg);
        let resource_provider = resource_provider.into();

        let out = unsafe {
            sb::C_SkSVGDOM_MakeFromStream(
                ms.native_mut().as_stream_mut(),
                resource_provider.into_ptr(),
            )
        };
        Self::from_ptr(out).ok_or(LoadError)
    }

    pub fn root(&self) -> Svg {
        unsafe {
            Svg::from_unshared_ptr(sb::C_SkSVGDOM_getRoot(self.native()) as *mut _)
                .unwrap_unchecked()
        }
    }

    pub fn render(&self, canvas: &Canvas) {
        // TODO: may be we should init ICU whenever we expose a Canvas?
        #[cfg(all(feature = "embed-icudtl", feature = "textlayout"))]
        crate::icu::init();

        unsafe { sb::SkSVGDOM::render(self.native() as &_, canvas.native_mut()) }
    }

    pub fn set_container_size(&mut self, size: impl Into<Size>) {
        let size = size.into();
        unsafe { sb::C_SkSVGDOM_setContainerSize(self.native_mut(), size.native()) }
    }
}

/// Error when something goes wrong when loading an SVG file. Sadly, Skia doesn't give further
/// details so we can't return a more expressive error type, but we still use this instead of
/// `Option` to express the intent and allow for `Try`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoadError;

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed to load svg (reason unknown)")
    }
}

impl Error for LoadError {
    fn description(&self) -> &str {
        "Failed to load svg (reason unknown)"
    }
}

impl From<LoadError> for io::Error {
    fn from(other: LoadError) -> Self {
        io::Error::other(other)
    }
}
