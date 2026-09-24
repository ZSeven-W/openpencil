//! The banners that sit over the top of the workspace canvas: the
//! failed / stopped run's Retry + 返回修改, the one-click template
//! draft's connect-or-refine action, and a shared document's
//! Make-one-like-this.
//!
//! They paint AFTER the real canvas (the host calls
//! [`WorkspaceSurface::paint_canvas_banners`] once the canvas and the
//! pinned chat are down). Painting them with the chrome put them under
//! the canvas background, so the buttons the hit-test answered to were
//! invisible.

use super::paint::{fade_all, text, tr, SANS};
use super::{workspace_enter, StudioPalette, WorkspaceLayout, WorkspaceSurface};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::{WorkspaceHit, WorkspacePhase};

impl WorkspaceSurface<'_> {
    /// Paint whichever canvas banner the phase calls for, over the canvas.
    pub fn paint_canvas_banners(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        let layout = self.layout(rect.size.x, rect.size.y);
        let (_, alpha) = workspace_enter(self.state.shown_at_ms, self.now_ms);
        let palette = fade_all(
            StudioPalette::for_mode(self.ui.effective_theme_mode()),
            alpha,
        );
        paint_failed_banner(self, cx, &layout, palette);
        paint_draft_banner(self, cx, &layout, palette);
        paint_make_same_banner(self, cx, &layout, palette);
        super::variants_bar::paint_variant_bar(self, cx, &layout, palette);
    }
}

/// The strip behind a banner: from the canvas top edge to just below the
/// actions, opaque so board labels and art never show through, with the
/// note centred in the band above the buttons (they sit at the canvas top
/// + 28, see `banner_buttons` / `draft_banner_button`).
fn paint_banner_strip(
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    first_button: Rect,
    note: &str,
    palette: StudioPalette,
) {
    let top = layout.canvas.origin.y;
    let bottom = first_button.origin.y + first_button.size.y + 12.0;
    let banner = Rect::xywh(
        layout.canvas.origin.x,
        top,
        layout.canvas.size.x,
        bottom - top,
    );
    cx.backend.fill_rect(banner, palette.chip_bg);
    cx.backend.fill_rect(
        Rect::xywh(banner.origin.x, bottom - 1.0, banner.size.x, 1.0),
        palette.line,
    );
    let note_band = Rect::xywh(
        banner.origin.x,
        top,
        banner.size.x,
        first_button.origin.y - top,
    );
    let note_w = cx.backend.measure_text_family(note, 13.0, SANS);
    text(
        cx,
        note,
        Point2D::new(
            banner.origin.x + ((banner.size.x - note_w) / 2.0).max(18.0),
            jian_widgets::centered_text_baseline_y(note_band, 13.0) + 2.0,
        ),
        13.0,
        palette.sub,
    );
}

/// One banner button: filled primary or outlined secondary.
fn paint_banner_button(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    button: Rect,
    hit: WorkspaceHit,
    label: &str,
    filled: bool,
    palette: StudioPalette,
) {
    let hovered = surface.state.hover == Some(hit);
    cx.backend.fill_round_rect(
        button,
        8.0,
        if filled {
            if hovered {
                palette.blue_hover
            } else {
                palette.blue
            }
        } else if hovered {
            palette.button_hover
        } else {
            palette.panel
        },
    );
    if !filled {
        cx.backend.stroke_round_rect(button, 8.0, palette.line, 1.0);
    }
    let label_w = cx.backend.measure_text_family(label, 13.0, SANS);
    text(
        cx,
        label,
        Point2D::new(
            button.origin.x + (button.size.x - label_w) / 2.0,
            jian_widgets::centered_text_baseline_y(button, 13.0),
        ),
        13.0,
        if filled { palette.panel } else { palette.ink },
    );
}

fn paint_failed_banner(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let Some((retry, return_edit)) = surface.banner_buttons(layout) else {
        return;
    };
    let locale = surface.ui.locale;
    let note = if surface.state.phase == WorkspacePhase::Stopped {
        "workspace.phase.stopped"
    } else {
        "workspace.failed.note"
    };
    paint_banner_strip(cx, layout, retry, tr(locale, note), palette);
    paint_banner_button(
        surface,
        cx,
        retry,
        WorkspaceHit::Retry,
        tr(locale, "workspace.retry"),
        true,
        palette,
    );
    paint_banner_button(
        surface,
        cx,
        return_edit,
        WorkspaceHit::ReturnEdit,
        tr(locale, "workspace.returnEdit"),
        false,
        palette,
    );
}

/// The template draft's banner: no model yet → say what the draft is and
/// offer 接入模型; a model connected since → offer 让 AI 细化.
fn paint_draft_banner(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let Some(button) = surface.draft_banner_button(layout) else {
        return;
    };
    let locale = surface.ui.locale;
    let (note, label) = if surface.usable_agent {
        ("workspace.draft.readyNote", "workspace.draft.refine")
    } else {
        ("workspace.draft.note", "home.submit.connect")
    };
    paint_banner_strip(cx, layout, button, tr(locale, note), palette);
    paint_banner_button(
        surface,
        cx,
        button,
        WorkspaceHit::DraftAction,
        tr(locale, label),
        true,
        palette,
    );
}

/// The shared-document banner: say what the document is and offer to
/// make one like it (the recipe's task, options and style, the brief as
/// an editable start).
fn paint_make_same_banner(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let Some(button) = surface.make_same_button(layout) else {
        return;
    };
    let locale = surface.ui.locale;
    paint_banner_strip(
        cx,
        layout,
        button,
        tr(locale, "makeSame.bannerNote"),
        palette,
    );
    paint_banner_button(
        surface,
        cx,
        button,
        WorkspaceHit::MakeSame,
        tr(locale, "makeSame.action"),
        true,
        palette,
    );
}
