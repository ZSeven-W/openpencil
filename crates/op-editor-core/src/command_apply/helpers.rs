//! Free helpers behind [`EditorState::apply`]: enum-string parsers,
//! the dirty-marking classifier, page-index resolution and the two
//! active-page insert shims.

use super::*;

/// Resolve an `align` action string into an [`AlignAction`].
pub(super) fn parse_align_action(s: &str) -> Option<AlignAction> {
    match s {
        "left" => Some(AlignAction::Left),
        "center_h" => Some(AlignAction::CenterH),
        "right" => Some(AlignAction::Right),
        "top" => Some(AlignAction::Top),
        "center_v" => Some(AlignAction::CenterV),
        "bottom" => Some(AlignAction::Bottom),
        "distribute_h" => Some(AlignAction::DistributeH),
        "distribute_v" => Some(AlignAction::DistributeV),
        _ => None,
    }
}

/// Resolve a `tool` string into a [`Tool`]. Accepts each tool's
/// stable [`Tool::ident`] token, so the form-widget tools select via
/// their `snake_case` kind string (`text_input`, `slider`, …; the
/// dropdown select widget uses `select_widget` to disambiguate from
/// the `select` pointer tool).
pub(super) fn parse_tool(s: &str) -> Option<Tool> {
    match s {
        "select" => Some(Tool::Select),
        "rect" => Some(Tool::Rect),
        "ellipse" => Some(Tool::Ellipse),
        "polygon" => Some(Tool::Polygon),
        "line" => Some(Tool::Line),
        "pen" => Some(Tool::Pen),
        "text" => Some(Tool::Text),
        "frame" => Some(Tool::Frame),
        "hand" => Some(Tool::Hand),
        "text_input" => Some(Tool::TextInput),
        "text_area" => Some(Tool::TextArea),
        "number_input" => Some(Tool::NumberInput),
        "select_widget" => Some(Tool::Select_),
        "radio_group" => Some(Tool::RadioGroup),
        "switch" => Some(Tool::Switch),
        "checkbox" => Some(Tool::Checkbox),
        "slider" => Some(Tool::Slider),
        "progress" => Some(Tool::Progress),
        "tabs" => Some(Tool::Tabs),
        _ => None,
    }
}

/// Resolve a variable `kind` string into a [`VariableKind`].
pub(super) fn parse_variable_kind(s: &str) -> Option<VariableKind> {
    match s {
        "color" => Some(VariableKind::Color),
        "number" => Some(VariableKind::Number),
        "boolean" => Some(VariableKind::Boolean),
        "string" => Some(VariableKind::String),
        _ => None,
    }
}

pub(super) fn command_page_index(state: &EditorState, page_id: Option<&str>) -> Option<usize> {
    let Some(raw) = page_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return Some(
            state
                .ui
                .active_page_index
                .min(state.page_count().saturating_sub(1)),
        );
    };
    match state.doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => pages
            .iter()
            .position(|page| page.id == raw)
            .or_else(|| raw.parse::<usize>().ok().filter(|idx| *idx < pages.len())),
        _ => raw.parse::<usize>().ok().filter(|idx| *idx == 0),
    }
}

pub(crate) fn command_marks_document_dirty(cmd: &EditorCommand) -> bool {
    use EditorCommand as C;
    if let C::Batch { commands } = cmd {
        return commands.iter().any(command_marks_document_dirty);
    }
    !matches!(
        cmd,
        C::SetActiveTool { .. }
            | C::SetViewport { .. }
            | C::Undo
            | C::Redo
            | C::CopySelected
            | C::ClearSelection
            | C::SetSelection { .. }
            | C::SetSelectionSet { .. }
            | C::ToggleNodeSelection { .. }
            | C::SetActivePage { .. }
            | C::SetActiveAxisValue { .. }
            | C::CycleActiveAxisValue { .. }
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_insert_node_on_active_page(
    state: &mut EditorState,
    kind: &str,
    name: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    fill_hex: &Option<String>,
    target_parent: &NodeId,
    allocator: &mut dyn crate::IdAllocator,
) -> Result<bool, crate::IdAllocError> {
    state.cmd_insert_node_with_allocator(
        kind,
        name,
        x,
        y,
        width,
        height,
        fill_hex,
        target_parent,
        allocator,
    )
}

pub(super) fn apply_import_svg_on_active_page(
    state: &mut EditorState,
    svg: &str,
    x: i32,
    y: i32,
    target_parent: &NodeId,
    allocator: &mut dyn crate::IdAllocator,
) -> Result<bool, crate::IdAllocError> {
    if target_parent.is_real() {
        match find_node(state.active_children(), target_parent) {
            Some(parent) if parent.is_container() => {}
            _ => return Ok(false),
        }
    }
    // `import_svg` pushes its own history snapshot when it inserts ≥ 1
    // node.
    let count = state.import_svg_with_allocator(allocator, svg, (x as f64, y as f64))?;
    if count == 0 {
        return Ok(false);
    }
    if target_parent.is_real() {
        let Some(imported_root) = state
            .active_children()
            .last()
            .map(|node| NodeId::new(node.id_str()))
        else {
            return Ok(false);
        };
        Ok(imported_root.is_real() && state.cmd_move_node(&imported_root, target_parent, None))
    } else {
        Ok(true)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_kit_component_on_page(
    state: &mut EditorState,
    kit_id: &str,
    component_id: &str,
    target_parent: &NodeId,
    doc_x: f64,
    doc_y: f64,
    overrides_json: Option<&str>,
    page_id: Option<&str>,
    allocator: &mut dyn crate::IdAllocator,
) -> Result<bool, crate::IdAllocError> {
    let Some(target_page_index) = command_page_index(state, page_id) else {
        return Ok(false);
    };
    let original_page_index = state.ui.active_page_index;
    let original_selection = state.selection.clone();
    let cross_page = page_id.is_some() && target_page_index != original_page_index;
    if page_id.is_some() {
        state.ui.active_page_index = target_page_index;
    }
    let changed = state
        .instantiate_kit_component_under_parent_with_allocator(
            kit_id,
            component_id,
            target_parent,
            doc_x,
            doc_y,
            overrides_json,
            allocator,
        )
        .map(|id| id.is_some());
    if cross_page {
        state.ui.active_page_index = original_page_index;
        state.selection = original_selection.clone();
        if matches!(&changed, Ok(true)) {
            if let Some(snapshot) = state.history.past.back_mut() {
                snapshot.active_page_index = original_page_index;
                snapshot.selection = original_selection;
            }
        }
    }
    changed
}
