//! Icon-picker panel press dispatch.

use op_editor_core::IconifyLoadMoreRequest;
use op_editor_ui::widgets::{IconPickerHit, IconPickerPanel};
use op_editor_ui::Point2D;

use super::{IconPickerDragState, WidgetHostNative};

impl WidgetHostNative {
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
        let hit = IconPickerPanel::for_editor(&self.editor_state)
            .and_then(|p| p.hit_test(panel_rect, Point2D::new(x, y)));
        let Some(hit) = hit else {
            return false;
        };
        match hit {
            IconPickerHit::Close => {
                self.editor_state.editor_ui.icon_picker_open = false;
                self.editor_state.editor_ui.icon_picker_replace_selection = false;
                self.editor_state.editor_ui.icon_picker_search.clear();
                self.editor_state.editor_ui.icon_picker_hover = None;
            }
            IconPickerHit::DragHeader => {
                self.icon_picker_drag = Some(IconPickerDragState {
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
                    // not the fallback dot. Local lucide catalog
                    // glyphs stay `icon_font` inserts — their
                    // stroke-art `d` must not be filled as a Path.
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
                    self.editor_state.editor_ui.icon_picker_open = false;
                    self.editor_state.editor_ui.icon_picker_search.clear();
                    self.editor_state.editor_ui.icon_picker_replace_selection = false;
                    self.editor_state.editor_ui.icon_picker_hover = None;
                    if inserted.is_some() {
                        self.mark_dirty();
                    }
                }
            }
            IconPickerHit::LoadMore => {
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
