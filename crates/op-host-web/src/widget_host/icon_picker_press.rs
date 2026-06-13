//! Icon-picker panel press dispatch — mirror of the native host's
//! `widget_host/icon_picker_press.rs`.
//!
//! Local-catalog search + selection work everywhere. `LoadMore`
//! raises the same `icon_picker_load_more_request` flag the native
//! host does; on the `codegen` build `iconify_web.rs` drains it
//! against `api.iconify.design` directly (the TS web app fetches the
//! same CORS-open API from the browser). Transport-less builds leave
//! the request undrained — the remote section keeps its loading row
//! (documented divergence, same posture as the other web IO seams).

use op_editor_core::IconifyLoadMoreRequest;
use op_editor_ui::widgets::{IconPickerHit, IconPickerPanel};
use op_editor_ui::Point2D;

use super::{PanelDragState, WidgetHost};

impl WidgetHost {
    pub(in crate::widget_host) fn dispatch_icon_picker_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.icon_picker_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        let point = Point2D::new(x, y);
        let Some((hit, pressed)) = IconPickerPanel::for_editor(&self.editor_state).and_then(|p| {
            Some((
                p.hit_test(panel_rect, point)?,
                p.hover_at(panel_rect, point),
            ))
        }) else {
            return false;
        };
        self.editor_state.editor_ui.icon_picker.pressed = pressed;
        match hit {
            IconPickerHit::Close => {
                self.editor_state.editor_ui.close_icon_picker();
            }
            IconPickerHit::DragHeader => {
                self.icon_picker_drag = Some(PanelDragState {
                    grab_dx: x - panel_rect.origin.x,
                    grab_dy: y - panel_rect.origin.y,
                });
            }
            IconPickerHit::SelectIcon { collection, name } => {
                let replace_selection = self.editor_state.editor_ui.icon_picker_replace_selection;
                if replace_selection {
                    let svg_path = self
                        .editor_state
                        .editor_ui
                        .icon_picker_remote
                        .icons
                        .iter()
                        .find(|i| i.collection == collection && i.name == name)
                        .map(|i| i.d.clone())
                        .or_else(|| {
                            op_editor_ui::widgets::icon_catalog::lookup_icon(&collection, &name)
                                .map(|icon| icon.d.clone())
                        });
                    if self.editor_state.replace_selected_icon(
                        &name,
                        &collection,
                        svg_path.as_deref(),
                    ) {
                        self.mark_dirty();
                    }
                } else {
                    let (_cx0, _cy0, cw, ch) = self.canvas_region(viewport_width, viewport_height);
                    let doc = self
                        .editor_state
                        .viewport
                        .to_document(Point2D::new(cw / 2.0, ch / 2.0));
                    // Bake the fetched SVG `d` like the REPLACE path
                    // above so a remote icon inserts its real glyph
                    // (Path node + iconId, TS toolbar.tsx:107-122),
                    // not the fallback dot — mirrors the native host.
                    // Local lucide catalog glyphs stay `icon_font`
                    // inserts.
                    let svg_path = self
                        .editor_state
                        .editor_ui
                        .icon_picker_remote
                        .icons
                        .iter()
                        .find(|i| i.collection == collection && i.name == name)
                        .map(|i| i.d.clone());
                    let inserted = self.editor_state.insert_icon_node_at(
                        &name,
                        &collection,
                        svg_path.as_deref(),
                        doc.x as f64,
                        doc.y as f64,
                    );
                    self.editor_state.editor_ui.close_icon_picker();
                    if inserted.is_some() {
                        self.mark_dirty();
                    }
                }
            }
            IconPickerHit::LoadMore => {
                // Raise the same request flag the native host does; the
                // `codegen` build's `iconify_web.rs` drains it against
                // api.iconify.design after this press handler returns.
                // Transport-less builds leave the loading row painted
                // (module-doc divergence).
                let ui = &mut self.editor_state.editor_ui;
                let query = ui.icon_picker_search.trim().to_lowercase();
                if !query.is_empty() && !ui.icon_picker_remote.loading {
                    let start = if ui.icon_picker_remote.query == query {
                        ui.icon_picker_remote.next_start
                    } else {
                        ui.icon_picker_remote = Default::default();
                        0
                    };
                    ui.icon_picker_remote.query = query.clone();
                    ui.icon_picker_remote.loading = true;
                    ui.icon_picker_remote.error = None;
                    ui.icon_picker_load_more_request = Some(IconifyLoadMoreRequest {
                        query,
                        start,
                        limit: op_editor_ui::widgets::ICONIFY_LOAD_MORE_LIMIT,
                    });
                }
            }
            IconPickerHit::Inside => {
                // Panel chrome that hit no control — blank press; blur
                // the chrome text inputs (the picker's own search box
                // has no focus flag and stays live while open).
                self.blur_text_inputs_on_blank_press();
            }
        }
        self.mark_dirty();
        true
    }
}
