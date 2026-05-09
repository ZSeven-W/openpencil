use std::{fmt, ptr};

use skia_bindings::{self as sb, SkDocument, SkRefCntBase};

use crate::{interop::RustWStream, prelude::*, Canvas, Rect, Size};

pub struct Document<'a, State = state::Open> {
    // Order matters here, first the document must be dropped _and then_ the stream.
    document: RCHandle<SkDocument>,
    stream: RustWStream<'a>,

    state: State,
}

require_type_equality!(sb::SkDocument_INHERITED, sb::SkRefCnt);

impl NativeRefCountedBase for SkDocument {
    type Base = SkRefCntBase;
}

impl<State: fmt::Debug> fmt::Debug for Document<'_, State> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("state", &self.state)
            .finish()
    }
}

pub mod state {
    use std::{fmt, ptr};

    use skia_bindings::SkCanvas;

    use crate::Canvas;

    /// Document is currently open. May contain several pages.
    #[derive(Debug)]
    pub struct Open {
        pub(crate) pages: usize,
    }

    /// Document is currently on a page and can be drawn onto.
    pub struct OnPage {
        pub(crate) page: usize,
        pub(crate) canvas: ptr::NonNull<SkCanvas>,
    }

    impl fmt::Debug for OnPage {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("OnPage")
                .field("page", &self.page)
                .field(
                    "canvas",
                    Canvas::borrow_from_native(unsafe { self.canvas.as_ref() }),
                )
                .finish()
        }
    }
}

impl<State> Document<'_, State> {
    pub fn abort(mut self) {
        unsafe { self.document.native_mut().abort() }
        drop(self)
    }
}

impl<'a> Document<'a, state::Open> {
    #[allow(unused)]
    pub(crate) fn new(stream: RustWStream<'a>, document: RCHandle<SkDocument>) -> Self {
        Document {
            document,
            stream,
            state: state::Open { pages: 0 },
        }
    }

    /// The number of pages in this document.
    pub fn pages(&self) -> usize {
        self.state.pages
    }

    // This function consumes the document and returns a document containing a
    // canvas that represents the page it's currently drawing on.
    pub fn begin_page(
        mut self,
        size: impl Into<Size>,
        content: Option<&Rect>,
    ) -> Document<'a, state::OnPage> {
        let size = size.into();
        let canvas = unsafe {
            self.document.native_mut().beginPage(
                size.width,
                size.height,
                content.native_ptr_or_null(),
            )
        };

        Document {
            stream: self.stream,
            document: self.document,
            state: state::OnPage {
                canvas: ptr::NonNull::new(canvas).unwrap(),
                page: self.state.pages + 1,
            },
        }
    }

    /// Close the document and return the encoded representation.
    ///
    /// This function consumes and drops the document.
    pub fn close(mut self) {
        unsafe {
            self.document.native_mut().close();
        };
    }
}

impl<'a> Document<'a, state::OnPage> {
    /// The current page we are currently drawing on.
    pub fn page(&self) -> usize {
        self.state.page
    }

    /// Borrows the canvas for the current page on the document.
    pub fn canvas(&mut self) -> &Canvas {
        Canvas::borrow_from_native(unsafe { self.state.canvas.as_ref() })
    }

    /// Ends the page.
    ///
    /// This function consumes the document and returns a new open document that
    /// contains the pages drawn so far.
    pub fn end_page(mut self) -> Document<'a> {
        unsafe {
            self.document.native_mut().endPage();
        }

        Document {
            stream: self.stream,
            document: self.document,
            state: state::Open {
                pages: self.state.page,
            },
        }

        // TODO: Think about providing a close() function that implicitly ends the page
        //       and calls close() on the Open document.
        // TODO: Think about providing a begin_page() function that implicitly ends the
        //       current page.
    }
}
