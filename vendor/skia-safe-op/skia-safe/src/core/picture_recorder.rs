use crate::{prelude::*, Canvas, Drawable, Picture, Rect};
use skia_bindings::{self as sb, SkPictureRecorder, SkRect};
use std::{fmt, ptr};

pub type PictureRecorder = Handle<SkPictureRecorder>;

impl NativeDrop for SkPictureRecorder {
    fn drop(&mut self) {
        unsafe {
            sb::C_SkPictureRecorder_destruct(self);
        }
    }
}

impl fmt::Debug for PictureRecorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PictureRecorder").finish()
    }
}

impl PictureRecorder {
    pub fn new() -> Self {
        Self::construct(|pr| unsafe { sb::C_SkPictureRecorder_Construct(pr) })
    }

    pub fn begin_recording(&mut self, bounds: impl AsRef<Rect>, use_bbh: bool) -> &Canvas {
        let canvas_ref = unsafe {
            &*sb::C_SkPictureRecorder_beginRecording(
                self.native_mut(),
                bounds.as_ref().native(),
                use_bbh,
            )
        };

        Canvas::borrow_from_native(canvas_ref)
    }

    pub fn recording_canvas(&mut self) -> Option<&Canvas> {
        let canvas = unsafe { self.native_mut().getRecordingCanvas() };
        if canvas.is_null() {
            return None;
        }
        Some(Canvas::borrow_from_native(unsafe { &*canvas }))
    }

    pub fn finish_recording_as_picture(&mut self, cull_rect: Option<&Rect>) -> Option<Picture> {
        self.recording_canvas()?;
        let cull_rect_ptr: *const SkRect =
            cull_rect.map(|r| r.native() as _).unwrap_or(ptr::null());

        let picture_ptr = unsafe {
            sb::C_SkPictureRecorder_finishRecordingAsPicture(self.native_mut(), cull_rect_ptr)
        };

        Picture::from_ptr(picture_ptr)
    }

    pub fn finish_recording_as_drawable(&mut self) -> Option<Drawable> {
        self.recording_canvas()?;
        Drawable::from_ptr(unsafe {
            sb::C_SkPictureRecorder_finishRecordingAsDrawable(self.native_mut())
        })
    }
}

#[test]
fn good_case() {
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(Rect::new(0.0, 0.0, 100.0, 100.0), false);
    canvas.clear(crate::Color::WHITE);
    let _picture = recorder.finish_recording_as_picture(None).unwrap();
}

#[test]
fn begin_recording_two_times() {
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(Rect::new(0.0, 0.0, 100.0, 100.0), false);
    canvas.clear(crate::Color::WHITE);
    assert!(recorder.recording_canvas().is_some());
    let canvas = recorder.begin_recording(Rect::new(0.0, 0.0, 100.0, 100.0), false);
    canvas.clear(crate::Color::WHITE);
    assert!(recorder.recording_canvas().is_some());
}

#[test]
fn finishing_recording_two_times() {
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(Rect::new(0.0, 0.0, 100.0, 100.0), false);
    canvas.clear(crate::Color::WHITE);
    assert!(recorder.finish_recording_as_picture(None).is_some());
    assert!(recorder.recording_canvas().is_none());
    assert!(recorder.finish_recording_as_picture(None).is_none());
}

#[test]
fn not_recording_no_canvas() {
    let mut recorder = PictureRecorder::new();
    assert!(recorder.recording_canvas().is_none());
}

#[test]
fn record_with_bbox_hierarchy() {
    let mut paint = crate::Paint::new(crate::Color4f::new(0.0, 0.0, 0.0, 1.0), None);
    paint.set_style(crate::PaintStyle::Fill);

    let frame_rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let crop_rect = Rect::new(50.0, 50.0, 100.0, 100.0);
    let drawn_rect = Rect::new(70.0, 70.0, 80.0, 80.0);

    // with bbh disabled, cull rects reflect the arg passed to begin_recording
    let mut src_rec = PictureRecorder::new();
    src_rec
        .begin_recording(frame_rect, false)
        .draw_rect(drawn_rect, &paint);
    let picture = src_rec.finish_recording_as_picture(None).unwrap();
    assert!(picture.cull_rect() == frame_rect);

    let mut no_bbh = PictureRecorder::new();
    no_bbh
        .begin_recording(crop_rect, false)
        .draw_picture(&picture, None, None);
    let no_bbh_pict = no_bbh.finish_recording_as_picture(None).unwrap();
    assert!(no_bbh_pict.cull_rect() == crop_rect);

    // with bbh enabled, cull rect contracts to just the content drawn
    let mut with_bbh = PictureRecorder::new();
    with_bbh
        .begin_recording(frame_rect, true)
        .draw_picture(&picture, None, None);
    let bbh_pict = with_bbh.finish_recording_as_picture(None).unwrap();
    assert!(bbh_pict.cull_rect() == drawn_rect);
}
