//! `batch_design` DSL executors for the clone / kit-instantiate /
//! replace / image operations (`C` / `K` / `R` / `G`), plus the
//! container-layout and resolved-size probes the image placement
//! contract depends on.
//!
//! Split out of `batch_program.rs` to stay under the 800-line cap.

use jian_ops_schema::node::{ContainerProps, LayoutMode, PenNode};
use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use serde_json::json;

use super::batch_design::{find_top_level_char, normalize_node_shape};
use super::batch_direct_ops::split_top_level_args;
use super::batch_program::{ProgramCtx, Result};
use super::batch_program_error::ProgramError;
use super::batch_program_parse::{parse_json_arg, parse_node_json, parse_string_arg};
use super::batch_program_resolve::{
    find_node_by_path, lookup_id, parent_node_id, resolve_kit_component_id, resolve_parent_ref,
    resolve_path_expr, resolve_ref,
};
use super::EditorCommand;

/// `binding=C(sourceId, parent[, overrides])` — clone with fresh ids.
pub(crate) fn execute_copy(binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<()> {
    let first = find_top_level_char(args, ',')
        .ok_or_else(|| ProgramError::Syntax("Copy requires sourceId, parent, and data".into()))?;
    let source_raw = args[..first].trim();
    let rest = args[first + 1..].trim();
    let (parent_raw, data_str) = match find_top_level_char(rest, ',') {
        None => (rest, "{}"),
        Some(second) => (rest[..second].trim(), rest[second + 1..].trim()),
    };
    let source_id = lookup_id(&resolve_ref(source_raw, &ctx.bindings), &ctx.alias);
    let Some(source) =
        op_editor_core::walkers::find_node(ctx.sim.active_children(), &NodeId::new(&source_id))
    else {
        return Err(ProgramError::NotFound(format!(
            "Copy source not found: {source_id}"
        )));
    };
    let mut cloned_value = serde_json::to_value(source)
        .map_err(|e| ProgramError::InvalidNode(format!("Copy source unserializable: {e}")))?;
    // TS `Object.assign(cloned, data)` minus `id` (never overridden)
    // and `descendants` (a TS no-op — see module docs).
    let overrides = parse_json_arg(data_str)?;
    let Some(overrides) = overrides.as_object() else {
        return Err(ProgramError::Syntax(
            "C() overrides JSON must be an object".into(),
        ));
    };
    if let Some(obj) = cloned_value.as_object_mut() {
        for (key, value) in overrides {
            if key == "id" || key == "descendants" {
                continue;
            }
            obj.insert(key.clone(), value.clone());
        }
    }
    normalize_node_shape(&mut cloned_value);
    let mut node: PenNode = serde_json::from_value(cloned_value).map_err(|e| {
        ProgramError::InvalidNode(format!("C() overrides produce an invalid node: {e}"))
    })?;
    let parent = resolve_parent_ref(parent_raw, &ctx.bindings);
    if ctx.post_process {
        if parent.is_none() {
            let _ = op_editor_core::command_refine::refine_subtree(&mut node);
        } else {
            let _ = op_editor_core::command_refine::refine_child_subtree(&mut node);
        }
    }

    let mut nodes = vec![node];
    // Cloning mints fresh ids for the whole subtree
    // (`cloneNodeWithNewIds` parity); the clone's old ids are the LIVE
    // source ids, so they must NOT enter the alias table — `remap`'s
    // first-wins entry never fires because the source ids already
    // resolve directly in the sim tree.
    let map = ctx.remap(&mut nodes)?;
    let clone_id = map
        .first()
        .map(|(_, new)| new.clone())
        .ok_or(ProgramError::ProducedNoNode("Copy"))?;
    ctx.emit(
        EditorCommand::InsertAuthoredSubtree {
            nodes,
            parent_id: parent_node_id(parent.as_deref()),
            page_id: ctx.page_id.clone(),
        },
        &format!(
            "Copy parent not found or not a container: {}",
            parent.as_deref().unwrap_or("null")
        ),
    )?;
    ctx.bind(binding, &clone_id);
    Ok(())
}

/// `binding=K("kit/component", parent[, overrides])` — instantiate a
/// built-in UI-kit component. Model-facing ids are compact aliases:
/// `starter/<component-id>` maps to kit `openpencil-starter`, and
/// `shadcn/<short-id>` maps to kit `shadcn-ui` with component
/// `shadcn-<short-id>` (for example `shadcn/btn-primary` →
/// `shadcn-ui` / `shadcn-btn-primary`). Exact `<kit-id>/<component-id>`
/// pairs are also accepted for imported/future kits.
pub(crate) fn execute_kit_instantiate(
    binding: &str,
    args: &str,
    ctx: &mut ProgramCtx,
) -> Result<()> {
    let parts = split_top_level_args(args);
    if !(2..=3).contains(&parts.len()) {
        return Err(ProgramError::Syntax(
            "K() requires kitComponentId, parent, and optional overrides".into(),
        ));
    }
    let kit_component_id = parse_string_arg(parts[0].trim(), "K() kitComponentId")?;
    let (kit_id, component_id) = resolve_kit_component_id(&kit_component_id, &ctx.sim)?;
    let parent_raw = parts[1].trim();
    let resolved_parent = resolve_parent_ref(parent_raw, &ctx.bindings);
    let parent = resolved_parent
        .as_deref()
        .map(|resolved| NodeId::new(lookup_id(resolved, &ctx.alias)))
        .unwrap_or(NodeId::NONE);
    let overrides_json = match parts.get(2) {
        None => None,
        Some(raw) => {
            let value = parse_json_arg(raw)?;
            if !value.is_object() {
                return Err(ProgramError::Syntax(
                    "K() overrides JSON must be an object".into(),
                ));
            }
            Some(value.to_string())
        }
    };
    ctx.emit(
        EditorCommand::InstantiateKitComponent {
            kit_id,
            component_id,
            doc_x: None,
            doc_y: None,
            target_parent: parent,
            page_id: ctx.page_id.clone(),
            overrides_json,
        },
        "K() kit/component not found, parent is not a container, or overrides are invalid",
    )?;
    let node_id = ctx.sim.selection.anchor.clone();
    if !node_id.is_real()
        || op_editor_core::walkers::find_node(ctx.sim.active_children(), &node_id).is_none()
    {
        return Err(ProgramError::Rejected(
            "K() did not produce a selected node".into(),
        ));
    }
    ctx.bind(binding, node_id.as_str());
    Ok(())
}

/// `binding=R(path, data)` — replace the node at `path` with a fresh
/// node built from `data` (same slot, fresh id, children dropped).
pub(crate) fn execute_replace(binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<()> {
    let comma = find_top_level_char(args, ',')
        .ok_or_else(|| ProgramError::Syntax("Replace requires path and node data".into()))?;
    let path_raw = args[..comma].trim();
    let path = resolve_path_expr(path_raw, &ctx.bindings);
    let Some(old) = find_node_by_path(ctx.sim.active_children(), &path, &ctx.alias) else {
        return Err(ProgramError::NotFound(format!(
            "Replace target not found: {path}"
        )));
    };
    let old_id = old.id_str().to_string();
    let replaces_document_root = ctx
        .sim
        .active_children()
        .iter()
        .any(|root| root.id_str() == old_id);
    let mut node = parse_node_json(&args[comma + 1..], ctx.post_process, replaces_document_root)?;
    // Drain node-level `state` BEFORE the probe clone; hold the merge
    // and emit it only after the replace below succeeds (emit applies
    // immediately — emitting the merge first would leak an orphan
    // `$app` state command into `ctx.commands` if the replace then
    // fails; state from a line that never landed must not ship).
    let merge = super::batch_design::hoist_generation_state(std::slice::from_mut(&mut node));

    // Predict the ids `cmd_replace_subtree` will assign: it remaps off
    // the live seed/taken BEFORE removing the old subtree — identical
    // inputs to this sim-side dry run, so the mapping is identical.
    let mut probe = vec![node.clone()];
    let map = ctx.remap(&mut probe)?;
    let new_id = map
        .first()
        .map(|(_, new)| new.clone())
        .ok_or(ProgramError::ProducedNoNode("Replace"))?;
    ctx.emit(
        EditorCommand::ReplaceSubtree {
            node_id: NodeId::new(&old_id),
            node: Box::new(node),
            drop_children: true,
            page_id: ctx.page_id.clone(),
        },
        &format!("Replace failed for: {path}"),
    )?;
    if let Some(merge) = merge {
        // Emit AFTER the replace succeeds (a failed line must not leak
        // orphan $app state), then swap the two recorded commands so
        // the batch still carries MergeAppState before its replace.
        // Sim-apply order between the two is immaterial: merge touches
        // only doc.state, the replace only the tree. (If this merge
        // emit itself ever failed post-replace, the line would error
        // after its replace already landed — state dropped but never
        // orphaned; an additive MergeAppState on the sim can't
        // realistically fail.)
        ctx.emit(merge, "merge generated app state")?;
        let n = ctx.commands.len();
        ctx.commands.swap(n - 1, n - 2);
    }
    ctx.bind(binding, &new_id);
    Ok(())
}

/// `binding=G(parent, mode, prompt[, placement])` — emit an image node. The
/// parent accepts a binding or quoted existing id (`null` is rejected because
/// both placements require a concrete target); `mode` must be `search` or
/// `generate`. Placement defaults to `slot`; the explicit `append` escape
/// hatch allows a new sibling only under a horizontal/vertical flow parent.
/// No fetcher at this layer — `src` stays empty (browser-caller parity);
/// the host's own image pipeline enriches later.
pub(crate) fn execute_image(binding: &str, args: &str, ctx: &mut ProgramCtx) -> Result<()> {
    let parts = split_top_level_args(args);
    if !matches!(parts.len(), 3 | 4) {
        return Err(ProgramError::Syntax(format!("Invalid G() syntax: {args}")));
    }
    let parent_raw = parts[0].trim();
    let parent = if matches!(parent_raw, "null" | "undefined" | "0" | "\"\"" | "\"0\"") {
        String::new()
    } else {
        resolve_path_expr(parent_raw, &ctx.bindings)
    };
    let mode = serde_json::from_str::<String>(parts[1].trim())
        .map_err(|_| ProgramError::Syntax(format!("Invalid G() syntax: {args}")))?;
    if !matches!(mode.as_str(), "search" | "generate") {
        return Err(ProgramError::Rejected(format!(
            "G() mode must be search or generate: {mode}"
        )));
    }
    let prompt = serde_json::from_str::<String>(parts[2].trim())
        .map_err(|_| ProgramError::Syntax(format!("Invalid G() syntax: {args}")))?;
    let placement = match parts.get(3) {
        None => "slot".to_string(),
        Some(raw) => serde_json::from_str::<String>(raw.trim())
            .map_err(|_| ProgramError::Syntax(format!("Invalid G() syntax: {args}")))?,
    };
    if !matches!(placement.as_str(), "slot" | "append") {
        return Err(ProgramError::Rejected(format!(
            "G() placement must be \"slot\" or \"append\", got {placement:?}"
        )));
    }
    let name: String = prompt.chars().take(40).collect();
    let mut value = json!({
        "type": "image",
        "id": "__op_tmp_image_1",
        "name": name,
        "src": "",
        "objectFit": "crop",
        "width": 400,
        "height": 300
    });
    if mode == "generate" {
        value["imagePrompt"] = json!(prompt);
    } else {
        value["imageSearchQuery"] = json!(prompt);
    }
    if parent.trim().is_empty() || parent.trim() == "0" {
        return Err(ProgramError::Rejected(format!(
            "G() placement {placement:?} requires an explicit frame/rectangle target id; create the target first instead of using null"
        )));
    }
    let target =
        find_node_by_path(ctx.sim.active_children(), &parent, &ctx.alias).ok_or_else(|| {
            ProgramError::NotFound(format!("G() parent not found or not a container: {parent}"))
        })?;
    // `parent` may be a slash path or an authored id that `find_node_by_path`
    // translated through `ctx.alias`. The emitted insert must target the live
    // resolved node id, never the caller's path/alias spelling.
    let target_id = target.id_str().to_string();
    let container = node_container(target).ok_or_else(|| {
        ProgramError::NotFound(format!("G() parent not found or not a container: {parent}"))
    })?;
    // Placement is an explicit structural contract. Slot-fill accepts only an
    // EMPTY target; append accepts only an explicitly-authored flow parent.
    // Never recover intent from names, dimensions, child kinds, or position.
    match placement.as_str() {
        "slot" => {
            let child_ids = target
                .children()
                .into_iter()
                .flatten()
                .map(PenNode::id_str)
                .collect::<Vec<_>>();
            if !child_ids.is_empty() {
                // Name the empty slot the caller almost certainly meant. The
                // authored shape is a painted card wrapping one empty media
                // frame, and a measured run aimed every G() at the wrapper,
                // burned its whole batch on the rejection, and never tried the
                // child it had just built. Suggesting is not redirecting: a
                // lone empty child can also be a deliberate scrim, so the
                // caller still chooses.
                let suggestion = empty_container_children(target);
                let hint = match suggestion.as_slice() {
                    [only] => format!(" The only empty container child is {only} — target that instead if it is the media slot."),
                    [_, ..] => format!(
                        " Empty container children that could be the slot: [{}].",
                        suggestion.join(", ")
                    ),
                    [] => String::new(),
                };
                return Err(ProgramError::Rejected(format!(
                    "G() slot target {} must be empty, but it has children [{}].{hint} Pass the exact empty frame/rectangle slot id; use \"append\" only for an intentional child of an explicit horizontal/vertical flow parent",
                    target.id_str(),
                    child_ids.join(", ")
                )));
            }
        }
        "append" => {
            if explicit_flow_layout(container).is_none() {
                return Err(ProgramError::Rejected(format!(
                    "G() append target {} must declare layout \"horizontal\" or \"vertical\"; got {}. Append means a new flow sibling and is never an absolute overlay",
                    target.id_str(),
                    layout_label(container)
                )));
            }
            if !ctx
                .explicitly_sized_append_lines
                .contains(&ctx.current_line)
            {
                return Err(ProgramError::Rejected(
                    "G() append requires a result binding followed in the same batch by U(binding, {\"width\": <positive number>, \"height\": <positive number>}); refusing an unsized flow child with default fill_container width and height"
                        .into(),
                ));
            }
        }
        _ => unreachable!("placement validated above"),
    }
    if matches!(container.layout, Some(LayoutMode::None)) {
        let has_declared_size = container.width.is_some() && container.height.is_some();
        let resolved = has_declared_size
            .then(|| resolved_node_size(&ctx.sim, &target_id))
            .flatten();
        let width = target.width_px().or_else(|| resolved.map(|size| size.0));
        let height = target.height_px().or_else(|| resolved.map(|size| size.1));
        let (Some(width), Some(height)) = (width, height) else {
            return Err(ProgramError::Rejected(format!(
                    "G() target {target_id} uses layout none, so it needs declared width and height that resolve above zero before an image can fill it"
                )));
        };
        if width <= 0.0 || height <= 0.0 {
            return Err(ProgramError::Rejected(format!(
                    "G() target {target_id} uses layout none, so it needs declared width and height that resolve above zero before an image can fill it"
                )));
        }
        value["x"] = json!(0);
        value["y"] = json!(0);
        value["width"] = json!(width);
        value["height"] = json!(height);
    } else {
        // A G() image serves its target slot; its intrinsic/search size is
        // not evidence for card geometry. Flex sizing keeps the image
        // inside the slot and lets crop/object-fit do the visual work.
        value["width"] = json!("fill_container");
        value["height"] = json!("fill_container");
    }
    let node: PenNode = serde_json::from_value(value)
        .map_err(|e| ProgramError::InvalidNode(format!("invalid G() image node: {e}")))?;
    let mut nodes = vec![node];
    let map = ctx.remap(&mut nodes)?;
    let image_id = map
        .first()
        .map(|(_, new)| new.clone())
        .ok_or(ProgramError::ProducedNoNode("G()"))?;
    ctx.emit(
        EditorCommand::InsertAuthoredSubtree {
            nodes,
            parent_id: NodeId::new(&target_id),
            page_id: ctx.page_id.clone(),
        },
        &format!("G() parent not found or not a container: {parent}"),
    )?;
    ctx.bind(binding, &image_id);
    Ok(())
}

/// Ids of `node`'s direct children that are themselves empty containers —
/// the shape an authored media slot takes when the model wraps it in a
/// painted card. Used only to name a candidate in a rejection message.
fn empty_container_children(node: &PenNode) -> Vec<String> {
    node.children()
        .into_iter()
        .flatten()
        .filter(|child| {
            node_container(child).is_some() && child.children().is_some_and(|kids| kids.is_empty())
        })
        .map(|child| child.id_str().to_string())
        .collect()
}

fn explicit_flow_layout(container: &ContainerProps) -> Option<&'static str> {
    match container.layout {
        Some(LayoutMode::Horizontal) => Some("horizontal"),
        Some(LayoutMode::Vertical) => Some("vertical"),
        _ => None,
    }
}

fn layout_label(container: &ContainerProps) -> &'static str {
    match container.layout {
        Some(LayoutMode::Horizontal) => "horizontal",
        Some(LayoutMode::Vertical) => "vertical",
        Some(LayoutMode::None) => "none",
        None => "omitted",
    }
}

fn node_container(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(node) => Some(&node.container),
        PenNode::Group(node) => Some(&node.container),
        PenNode::Rectangle(node) => Some(&node.container),
        _ => None,
    }
}

fn resolved_node_size(state: &EditorState, node_id: &str) -> Option<(f64, f64)> {
    fn find(nodes: &[SceneNode], node_id: &str) -> Option<(f64, f64)> {
        for node in nodes {
            if node.id == node_id {
                let bounds = node.aggregate_bounds();
                return Some((f64::from(bounds.size.x), f64::from(bounds.size.y)));
            }
            if let Some(size) = find(&node.children, node_id) {
                return Some(size);
            }
        }
        None
    }

    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    scene
        .active_page()
        .and_then(|page| find(&page.children, node_id))
}
