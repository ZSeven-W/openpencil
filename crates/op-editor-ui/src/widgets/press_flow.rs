//! Press-dispatch flow shared by the native and web widget hosts.
//!
//! The two `widget_host/press.rs` twins used to carry these blocks as
//! copy-pasted inline code. They now map their hit-test result through
//! the functions here and run only the platform tail each step asks for
//! (`mark_dirty`, `blur_text_inputs_on_blank_press`, viewport fits,
//! toolbar actions, …). Everything below is pure `EditorState` mutation
//! plus widget-layer types, so it stays wasm32-clean.

use op_editor_core::host_press_transitions as core_press;
use op_editor_core::ui_draft::LayerContextTarget;
use op_editor_core::{BooleanOp, EditorState, Tool};

use crate::widgets::layer_context_menu::LayerContextAction;
use crate::widgets::property_panel_fill;
use crate::widgets::{
    LayerPanelHit, LocalePicker, PropertyPanel, PropertyPanelAction, ShapeChoice, ShapePicker,
    TopBarHit, TOP_BAR_HEIGHT,
};
use crate::{Point2D, Rect};

/// Screen rect of the right-hand property rail. Both hosts derive it
/// from the same two inputs, so the walk lives here instead of being
/// re-spelled at every popover hit-test site.
pub fn property_panel_rect(state: &EditorState, viewport_width: f32, viewport_height: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            viewport_width - state.editor_ui.property_panel_width,
            TOP_BAR_HEIGHT,
        ),
        size: Point2D::new(
            state.editor_ui.property_panel_width,
            (viewport_height - TOP_BAR_HEIGHT).max(0.0),
        ),
    }
}

// ─── Property-panel popovers ───────────────────────────────────────────

/// Outcome of a press routed to an open property-panel popover. Every
/// variant consumes the press.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyOverlayPress {
    /// A row was picked — the host dispatches this property action.
    Action(PropertyPanelAction),
    /// Popup chrome (not a row) — swallow and keep the popup open.
    Swallow,
    /// Outside press — the popup closed itself; the host marks dirty.
    Dismissed,
}

/// Fill-type dropdown press (host block `0c0`). Caller must have
/// checked `editor_ui.fill_type_picker.open`.
pub fn press_fill_type_picker(
    state: &mut EditorState,
    viewport_width: f32,
    viewport_height: f32,
    point: Point2D,
) -> PropertyOverlayPress {
    let property_rect = property_panel_rect(state, viewport_width, viewport_height);
    if let Some(panel) = PropertyPanel::for_selection(state) {
        match panel.fill_type_picker_hit(property_rect, point) {
            property_panel_fill::SelectHit::Row(idx) => {
                if let Some(fill_type) = property_panel_fill::fill_type_at(idx) {
                    let index = state.editor_ui.fill_type_picker_index;
                    return PropertyOverlayPress::Action(PropertyPanelAction::SetFillType {
                        index,
                        fill_type,
                    });
                }
            }
            property_panel_fill::SelectHit::Inside => return PropertyOverlayPress::Swallow,
            property_panel_fill::SelectHit::Outside => {}
        }
    }
    state.editor_ui.close_fill_type_picker();
    PropertyOverlayPress::Dismissed
}

/// Layer / mask / fill-blend compositing picker press (host block
/// `0c0a`). The popup is painted over the inspector body, so it owns
/// both its rows and its padded chrome; the first outside press
/// dismisses and is swallowed.
pub fn press_compositing_picker(
    state: &mut EditorState,
    viewport_width: f32,
    viewport_height: f32,
    point: Point2D,
) -> PropertyOverlayPress {
    let property_rect = property_panel_rect(state, viewport_width, viewport_height);
    if let Some(panel) = PropertyPanel::for_selection(state) {
        if let Some(action) = panel.compositing_picker_action_at(property_rect, point) {
            return PropertyOverlayPress::Action(action);
        }
        if panel.compositing_picker_contains(property_rect, point) {
            return PropertyOverlayPress::Swallow;
        }
    }
    state.editor_ui.close_compositing_picker();
    PropertyOverlayPress::Dismissed
}

/// Effects "+" add-menu press (host block `0c1`).
pub fn press_effect_add_menu(
    state: &mut EditorState,
    viewport_width: f32,
    viewport_height: f32,
    point: Point2D,
) -> PropertyOverlayPress {
    let property_rect = property_panel_rect(state, viewport_width, viewport_height);
    if let Some(panel) = PropertyPanel::for_selection(state) {
        match panel.effect_add_menu_hit(property_rect, point) {
            crate::widgets::EffectAddMenuHit::Row(action) => {
                return PropertyOverlayPress::Action(action)
            }
            crate::widgets::EffectAddMenuHit::Inside => return PropertyOverlayPress::Swallow,
            crate::widgets::EffectAddMenuHit::Outside => {}
        }
    }
    state.editor_ui.close_effect_add_picker();
    PropertyOverlayPress::Dismissed
}

/// Fill / stroke colour-variable picker press (host block `0c0a0`).
pub fn press_color_variable_picker(
    state: &mut EditorState,
    viewport_width: f32,
    viewport_height: f32,
    point: Point2D,
) -> PropertyOverlayPress {
    let property_rect = property_panel_rect(state, viewport_width, viewport_height);
    if let Some(panel) = PropertyPanel::for_selection(state) {
        if let Some(action) = panel.hit_test_action(property_rect, point) {
            if matches!(
                action,
                PropertyPanelAction::ToggleColorVariablePicker(_)
                    | PropertyPanelAction::BindColorVariable { .. }
                    | PropertyPanelAction::UnbindColorVariable(_)
            ) {
                return PropertyOverlayPress::Action(action);
            }
        }
        // Popup chrome (padding / scrollbar gutter) swallows the press —
        // only a press OUTSIDE the popup dismisses it.
        if panel.color_variable_picker_contains(property_rect, point) {
            return PropertyOverlayPress::Swallow;
        }
    }
    state.editor_ui.property_color_variable_picker_open = None;
    PropertyOverlayPress::Dismissed
}

// ─── Locale picker ─────────────────────────────────────────────────────

/// Outcome of a press routed to the open TopBar locale dropdown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocalePickerPress {
    /// A row was picked: the locale is applied and the popup closed.
    Selected,
    /// Popup chrome — swallow, keep it open.
    Swallow,
    /// Outside press — the host blurs its text inputs (a blank press),
    /// then closes the picker.
    Outside,
}

/// Locale-dropdown press (host block `0a`).
pub fn press_locale_picker(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
) -> LocalePickerPress {
    let picker = LocalePicker::for_editor_ui(&state.editor_ui);
    match picker.hit_popup(panel_rect, point) {
        crate::widgets::locale_picker::SelectHit::Row(idx) => {
            if let Some(locale) = LocalePicker::locale_at(idx) {
                state.editor_ui.locale = locale;
            }
            core_press::close_locale_picker(&mut state.editor_ui);
            LocalePickerPress::Selected
        }
        crate::widgets::locale_picker::SelectHit::Inside => LocalePickerPress::Swallow,
        crate::widgets::locale_picker::SelectHit::Outside => LocalePickerPress::Outside,
    }
}

// ─── Shape picker ──────────────────────────────────────────────────────

/// Outcome of a press routed to the open toolbar shape dropdown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapePickerPress {
    /// A tool row was picked — the host applies the tool, then closes
    /// the picker.
    SetTool(Tool),
    /// A non-tool row applied its state change — the host closes the
    /// picker.
    Close,
    /// Row resolved to no choice, or the press hit popup chrome —
    /// swallow and keep the picker open.
    Swallow,
    /// Outside press — the host blurs its text inputs (a blank press),
    /// then closes the picker.
    Outside,
}

/// Shape-dropdown press. Caller must have checked
/// `editor_ui.shape_picker.open` and resolved `panel_rect`.
pub fn press_shape_picker(
    state: &mut EditorState,
    panel_rect: Rect,
    point: Point2D,
) -> ShapePickerPress {
    let picker = ShapePicker::for_editor_ui(&state.editor_ui);
    match picker.hit_popup(panel_rect, point) {
        crate::widgets::shape_picker::SelectHit::Row(idx) => match picker.choice_at(idx) {
            Some(ShapeChoice::Tool(tool)) => ShapePickerPress::SetTool(tool),
            Some(ShapeChoice::OpenIconPicker) => {
                state.editor_ui.open_icon_picker(false);
                ShapePickerPress::Close
            }
            Some(ShapeChoice::ImportImageOrSvg) => {
                // Neither host owns a file-picker service here — both
                // raise the same pending flag for their host loop.
                state.editor_ui.pending_file_action =
                    Some(op_editor_core::editor_ui_state::FileAction::ImportImageOrSvg);
                ShapePickerPress::Close
            }
            None => ShapePickerPress::Swallow,
        },
        crate::widgets::shape_picker::SelectHit::Inside => ShapePickerPress::Swallow,
        crate::widgets::shape_picker::SelectHit::Outside => ShapePickerPress::Outside,
    }
}

// ─── TopBar ────────────────────────────────────────────────────────────

/// Outcome of a TopBar hit routed through the shared arms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopBarPress {
    /// Handled — the host marks dirty and consumes the press.
    Handled,
    /// The file menu was toggled: the host also clears the layer-panel
    /// hover wash, then marks dirty and consumes the press.
    FileMenuToggled,
    /// Platform-specific hit (fullscreen / preview / git panel /
    /// account) — the host handles it itself.
    Platform,
}

/// Apply the TopBar hits whose behaviour is identical on both hosts.
/// The platform arms stay in each host's `press.rs`.
pub fn apply_shared_top_bar_hit(
    state: &mut EditorState,
    hit: TopBarHit,
    now_ms: u64,
) -> TopBarPress {
    match hit {
        TopBarHit::ToggleSidebar => {
            let v = &mut state.editor_ui.sidebar_open;
            *v = !*v;
            TopBarPress::Handled
        }
        TopBarHit::ToggleTheme => {
            state.editor_ui.theme_mode = state.editor_ui.theme_mode.flipped();
            TopBarPress::Handled
        }
        TopBarHit::ToggleLocale => {
            core_press::toggle_locale_picker(&mut state.editor_ui);
            TopBarPress::Handled
        }
        TopBarHit::OpenAgentSettings => {
            state.editor_ui.agent_settings_open = true;
            state.chat.blur_input(now_ms);
            TopBarPress::Handled
        }
        TopBarHit::Collaboration => {
            let phase = state.editor_ui.collab.phase;
            let panel = &mut state.editor_ui.collab.panel;
            panel.open = !panel.open;
            panel.hover = None;
            if panel.open {
                panel.view = if phase.is_authenticated() {
                    op_editor_core::CollabPanelView::Session
                } else if phase == op_editor_core::CollabConnectionPhase::Discovering {
                    op_editor_core::CollabPanelView::Join
                } else {
                    op_editor_core::CollabPanelView::Home
                };
                state.editor_ui.file_menu_open = false;
                state.editor_ui.import_menu_open = false;
                state.editor_ui.import_menu.open = false;
                core_press::close_locale_picker(&mut state.editor_ui);
                state.editor_ui.account_menu_open = false;
            }
            state.chat.blur_input(now_ms);
            TopBarPress::Handled
        }
        TopBarHit::OpenImportMenu => {
            core_press::toggle_import_menu(&mut state.editor_ui);
            TopBarPress::Handled
        }
        TopBarHit::ToggleFileMenu => {
            core_press::toggle_file_menu(&mut state.editor_ui);
            TopBarPress::FileMenuToggled
        }
        TopBarHit::ToggleFullscreen
        | TopBarHit::TogglePreview
        | TopBarHit::ToggleGitPanel
        | TopBarHit::Account => TopBarPress::Platform,
    }
}

// ─── LayerPanel click ──────────────────────────────────────────────────

/// Residual host work after [`apply_layer_panel_click`]. Every variant
/// consumes the click.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerPanelClick {
    /// Nothing left to do.
    Consumed,
    /// Host marks dirty.
    Dirty,
    /// Host re-fits the viewport on the (new) active page.
    Refit,
    /// The row selection changed. Native repaints off the consumed
    /// press; the web host additionally marks dirty (see its wrapper).
    SelectionChanged,
}

/// LayerPanel click dispatch — double-click rename detection, page
/// activation, row selection, and the history-guarded row toggles.
pub fn apply_layer_panel_click(
    state: &mut EditorState,
    hit: LayerPanelHit,
    now_ms: u64,
    shift_held: bool,
) -> LayerPanelClick {
    // Build the op-editor-core context target for the double-click
    // rename detection.
    let target_for_double_click = match &hit {
        LayerPanelHit::Layer(id) => Some(LayerContextTarget::Layer(id.clone())),
        LayerPanelHit::Page(idx) => Some(LayerContextTarget::Page(*idx)),
        _ => None,
    };
    if let Some(target) = target_for_double_click {
        if let Some((prev, prev_ms)) = state.editor_ui.last_layer_click.clone() {
            if prev == target && now_ms.saturating_sub(prev_ms) < 400 {
                let started = match &target {
                    LayerContextTarget::Layer(id) => state.start_rename_layer(id.clone()),
                    LayerContextTarget::Page(idx) => state.start_rename_page(*idx),
                };
                if started {
                    if let Some(rename) = state.ui.layer_rename.as_mut() {
                        rename.input.touch(now_ms);
                    }
                }
                state.editor_ui.last_layer_click = None;
                return LayerPanelClick::Dirty;
            }
        }
        state.editor_ui.last_layer_click = Some((target, now_ms));
    }
    match hit {
        LayerPanelHit::Page(idx) => {
            let page_changed = idx != state.ui.active_page_index;
            let _ = state.set_active_page(idx);
            state.clear_selection();
            if page_changed {
                // Land centered on the new page's content instead of
                // keeping the previous page's pan/zoom.
                LayerPanelClick::Refit
            } else {
                // Clicking the already-active page may clear selection,
                // but must preserve the user's current canvas view.
                LayerPanelClick::Dirty
            }
        }
        LayerPanelHit::Layer(node_id) => {
            if shift_held {
                state.toggle_selection(node_id);
            } else {
                state.set_single_selection(node_id);
            }
            LayerPanelClick::SelectionChanged
        }
        LayerPanelHit::ToggleHidden(node_id) => {
            // TS toggleVisibility → mutateWithHistory
            // (document-store-node-actions.ts:162-174).
            core_press::with_doc_history(state, |s| s.toggle_node_hidden(&node_id));
            LayerPanelClick::Dirty
        }
        LayerPanelHit::ToggleLocked(node_id) => {
            // TS toggleLock → mutateWithHistory
            // (document-store-node-actions.ts:176-188).
            core_press::with_doc_history(state, |s| s.toggle_node_locked(&node_id));
            LayerPanelClick::Dirty
        }
        LayerPanelHit::ToggleCollapsed(node_id) => {
            // Collapse is a tree-view-only concern (not document state),
            // so it stays OUT of history.
            state.toggle_node_collapsed(&node_id);
            LayerPanelClick::Dirty
        }
        LayerPanelHit::AddPage => {
            // TS addPage pushes history before the insert
            // (document-store-pages.ts:19-49).
            if core_press::with_doc_history(state, |s| s.add_page().is_some()) {
                LayerPanelClick::Refit
            } else {
                LayerPanelClick::Consumed
            }
        }
        LayerPanelHit::DeletePage(idx) => {
            // TS removePage pushes history after its last-page guard
            // (document-store-pages.ts:51-63) — `with_doc_history` skips
            // the push when the guard rejects the delete.
            let deleting_active = idx == state.ui.active_page_index;
            if core_press::with_doc_history(state, |s| s.remove_page(idx)) {
                if deleting_active {
                    LayerPanelClick::Refit
                } else {
                    LayerPanelClick::Dirty
                }
            } else {
                LayerPanelClick::Consumed
            }
        }
    }
}

// ─── LayerPanel context menu ───────────────────────────────────────────

/// Outcome of a right-press routed to the LayerPanel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerContextMenuPress {
    /// A context menu opened on the pressed row.
    Opened,
    /// No row was hit but an open menu was dismissed.
    Dismissed,
    /// Nothing was hit — the host treats it as a blank press.
    Missed,
}

/// Open the layer / page context menu for a right-press hit, or
/// dismiss an already-open menu.
pub fn open_layer_context_menu(
    state: &mut EditorState,
    hit: Option<LayerPanelHit>,
    anchor_x: f32,
    anchor_y: f32,
) -> LayerContextMenuPress {
    use op_editor_core::editor_ui_state::LayerContextMenuState;
    match hit {
        Some(LayerPanelHit::Layer(id)) => {
            // Right-clicking a row that's part of a multi-selection keeps
            // the whole selection (so context-menu Delete / Duplicate act
            // on every selected layer); right-clicking outside the
            // selection retargets to just that row.
            if !(state.is_selected(&id) && state.selection_count() > 1) {
                state.set_single_selection(id.clone());
            }
            state.editor_ui.layer_context_menu = Some(LayerContextMenuState {
                target: LayerContextTarget::Layer(id),
                anchor_x,
                anchor_y,
                menu: Default::default(),
            });
            LayerContextMenuPress::Opened
        }
        Some(LayerPanelHit::Page(idx)) => {
            state.editor_ui.layer_context_menu = Some(LayerContextMenuState {
                target: LayerContextTarget::Page(idx),
                anchor_x,
                anchor_y,
                menu: Default::default(),
            });
            LayerContextMenuPress::Opened
        }
        _ => {
            if state.editor_ui.layer_context_menu.take().is_some() {
                LayerContextMenuPress::Dismissed
            } else {
                LayerContextMenuPress::Missed
            }
        }
    }
}

/// Outcome of a press routed to the OPEN layer/page context menu.
#[derive(Debug, Clone, PartialEq)]
pub enum OpenLayerMenuPress {
    /// A row was picked — the host dispatches the action, then closes
    /// the menu and marks dirty.
    Action {
        action: LayerContextAction,
        target: LayerContextTarget,
    },
    /// Menu chrome, or a row that mapped to no action — swallow the
    /// press and keep the menu open.
    Swallow,
    /// Outside press — the host blurs its text inputs (a blank press),
    /// then closes the menu and marks dirty.
    Outside,
}

/// Route a press against the open layer/page context menu. `None` when
/// no menu is open (the press falls through to lower layers).
pub fn press_open_layer_context_menu(
    state: &EditorState,
    point: Point2D,
) -> Option<OpenLayerMenuPress> {
    use crate::widgets::layer_context_menu::{LayerContextMenu, MenuHit};
    let menu_state = state.editor_ui.layer_context_menu.clone()?;
    let menu = LayerContextMenu::for_state(state, menu_state.clone());
    Some(match menu.hit(point) {
        MenuHit::Row(_) => match menu.hit_test(point) {
            Some(action) => OpenLayerMenuPress::Action {
                action,
                target: menu_state.target,
            },
            None => OpenLayerMenuPress::Swallow,
        },
        MenuHit::Inside => OpenLayerMenuPress::Swallow,
        MenuHit::Outside => OpenLayerMenuPress::Outside,
    })
}

/// Residual host work after [`apply_layer_context_action`]. Both hosts
/// mark dirty afterwards regardless of the variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerContextStep {
    /// Fully applied by the shared dispatch.
    Done,
    /// Host runs its `apply_group()`.
    Group,
    /// Host runs its `apply_boolean_op(op)`.
    Boolean(BooleanOp),
    /// Host re-fits the viewport on the active page.
    Refit,
}

/// Layer / page context-menu row dispatch.
pub fn apply_layer_context_action(
    state: &mut EditorState,
    next_node_id: &mut u64,
    action: LayerContextAction,
    target: LayerContextTarget,
    now_ms: u64,
) -> LayerContextStep {
    use LayerContextAction as A;
    use LayerContextTarget as T;
    match (action, target) {
        (A::Duplicate, T::Layer(id)) => {
            // Act on the whole multi-selection when the right-clicked
            // row is part of it; otherwise retarget to just this row.
            if !state.is_selected(&id) {
                state.set_single_selection(id);
            }
            state.commit_history();
            let _ = state.duplicate_selected(next_node_id, 10.0);
            LayerContextStep::Done
        }
        (A::Delete, T::Layer(id)) => {
            // Keep the multi-selection so Delete removes every selected
            // layer, not just the right-clicked one.
            if !state.is_selected(&id) {
                state.set_single_selection(id);
            }
            state.commit_history();
            let _ = state.delete_selected();
            LayerContextStep::Done
        }
        (A::GroupSelection, T::Layer(_)) => LayerContextStep::Group,
        // TS boolean rows act on the current selection and push history
        // explicitly (`layer-panel.tsx:389-407`); the host's
        // `apply_boolean_op` does both.
        (
            A::BooleanUnion | A::BooleanSubtract | A::BooleanIntersect | A::BooleanExclude,
            T::Layer(_),
        ) => {
            let op = match action {
                A::BooleanSubtract => BooleanOp::Subtract,
                A::BooleanIntersect => BooleanOp::Intersect,
                A::BooleanExclude => BooleanOp::Exclude,
                _ => BooleanOp::Union,
            };
            LayerContextStep::Boolean(op)
        }
        (A::ToggleLock, T::Layer(id)) => {
            // TS toggleLock runs through mutateWithHistory
            // (document-store-node-actions.ts:176-188).
            core_press::with_doc_history(state, |s| s.toggle_node_locked(&id));
            LayerContextStep::Done
        }
        (A::ToggleVisibility, T::Layer(id)) => {
            // TS toggleVisibility runs through mutateWithHistory
            // (document-store-node-actions.ts:162-174).
            core_press::with_doc_history(state, |s| s.toggle_node_hidden(&id));
            LayerContextStep::Done
        }
        (A::CreateComponent, T::Layer(id)) => {
            let _ = state.create_component_from_node_name(&id);
            LayerContextStep::Done
        }
        (A::DetachComponent | A::DetachInstance, T::Layer(id)) => {
            // Reusable component sheds its flag; a Ref instance
            // materializes into an independent subtree (#22).
            let _ = state.detach_component(&id);
            LayerContextStep::Done
        }
        // Page CRUD pushes history in TS (document-store-pages.ts:19-121)
        // — snapshot-before-mutate, skipped when the guard rejects the op.
        (A::DuplicatePage, T::Page(idx)) => {
            if core_press::with_doc_history(state, |s| s.duplicate_page(idx).is_some()) {
                LayerContextStep::Refit
            } else {
                LayerContextStep::Done
            }
        }
        (A::MovePageUp, T::Page(idx)) => {
            core_press::with_doc_history(state, |s| s.move_page_up(idx));
            LayerContextStep::Done
        }
        (A::MovePageDown, T::Page(idx)) => {
            core_press::with_doc_history(state, |s| s.move_page_down(idx));
            LayerContextStep::Done
        }
        (A::DeletePage, T::Page(idx)) => {
            let deleting_active = idx == state.ui.active_page_index;
            if core_press::with_doc_history(state, |s| s.remove_page(idx)) && deleting_active {
                LayerContextStep::Refit
            } else {
                LayerContextStep::Done
            }
        }
        (A::RenamePage, T::Page(idx)) => {
            if state.start_rename_page(idx) {
                if let Some(rename) = state.ui.layer_rename.as_mut() {
                    rename.input.touch(now_ms);
                }
            }
            LayerContextStep::Done
        }
        (A::RenameLayer, T::Layer(id)) => {
            if state.start_rename_layer(id) {
                if let Some(rename) = state.ui.layer_rename.as_mut() {
                    rename.input.touch(now_ms);
                }
            }
            LayerContextStep::Done
        }
        // Mismatched action/target — no-op.
        _ => LayerContextStep::Done,
    }
}
