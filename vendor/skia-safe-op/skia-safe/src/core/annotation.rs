use crate::{Canvas, Data, Point, Rect};

pub mod annotate {
    use crate::prelude::*;
    use crate::{Canvas, Data, Point, Rect};
    use skia_bindings::{
        SkAnnotateLinkToDestination, SkAnnotateNamedDestination, SkAnnotateRectWithURL,
    };

    pub fn rect_with_url(canvas: &Canvas, rect: impl AsRef<Rect>, data: &Data) {
        unsafe {
            SkAnnotateRectWithURL(
                canvas.native_mut(),
                rect.as_ref().native(),
                data.native_mut_force(),
            )
        }
    }

    pub fn named_destination(canvas: &Canvas, point: impl Into<Point>, data: &Data) {
        unsafe {
            SkAnnotateNamedDestination(
                canvas.native_mut(),
                point.into().native(),
                data.native_mut_force(),
            )
        }
    }

    pub fn link_to_destination(canvas: &Canvas, rect: impl AsRef<Rect>, data: &Data) {
        unsafe {
            SkAnnotateLinkToDestination(
                canvas.native_mut(),
                rect.as_ref().native(),
                data.native_mut_force(),
            )
        }
    }
}

impl Canvas {
    // TODO: accept str or the Url type from the url crate?
    pub fn annotate_rect_with_url(&self, rect: impl AsRef<Rect>, data: &Data) -> &Self {
        annotate::rect_with_url(self, rect, data);
        self
    }

    // TODO: is data a string here, and if so, of what encoding?
    pub fn annotate_named_destination(&self, point: impl Into<Point>, data: &Data) -> &Self {
        annotate::named_destination(self, point, data);
        self
    }

    // TODO: use str?
    pub fn annotate_link_to_destination(&self, rect: impl AsRef<Rect>, data: &Data) -> &Self {
        annotate::link_to_destination(self, rect, data);
        self
    }
}
