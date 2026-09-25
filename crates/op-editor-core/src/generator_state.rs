//! `EditorState` surface for generator nodes.
//!
//! Two layers:
//!
//! * `generator_*_command` — PURE: validate, run the program against the
//!   installed (or given) runtime, and return the one [`EditorCommand`]
//!   that writes the result. The MCP tools use these on their snapshot so
//!   the host applies exactly what the editor UI would.
//! * `regenerate_generator` / `toggle_generator_bool_param` /
//!   `detach_generator` / `insert_generator_starter` — the UI mutators:
//!   apply that command and record exactly ONE undo step.
//!   `set_generator_param_input` records none; its caller (the property
//!   panel commit path) owns the snapshot-and-compare choke point.
//!
//! Every failure leaves the document untouched and, for the UI mutators,
//! parks a [`GeneratorPanelError`] on `ui.generator_error` for the panel.

use std::collections::HashSet;

use jian_ops_schema::node::PenNode;
use serde_json::{json, Map, Value};

use super::{
    generator_spec_of, materialize_children, output_hash, GeneratorError, GeneratorRequest,
    GeneratorRunner, GeneratorSpec, GeneratorStarter, GENERATOR_EXPLAIN_PREFIX,
};
use crate::command::EditorCommand;
use crate::id_allocator::{DocumentIdAllocator, IdAllocator};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;

/// The last generator failure, shown inline under that generator's params.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorPanelError {
    pub node_id: NodeId,
    pub message: String,
}

impl EditorState {
    /// The node `id` wherever it lives, plus the page id commands must
    /// target (`None` for a page-less legacy document).
    fn locate_generator_node(&self, id: &NodeId) -> Option<(Option<String>, &PenNode)> {
        match self.doc.pages.as_ref() {
            Some(pages) if !pages.is_empty() => pages.iter().find_map(|page| {
                walkers::find_node(&page.children, id).map(|node| (Some(page.id.clone()), node))
            }),
            _ => walkers::find_node(&self.doc.children, id).map(|node| (None, node)),
        }
    }

    /// The spec stored on generator `id`.
    pub fn generator_spec(&self, id: &NodeId) -> Result<GeneratorSpec, GeneratorError> {
        let (_, node) = self
            .locate_generator_node(id)
            .ok_or_else(|| GeneratorError::NodeNotFound { id: id.to_string() })?;
        generator_spec_of(node)
            .ok_or_else(|| GeneratorError::NotAGenerator { id: id.to_string() })?
    }

    /// Run `spec` for the generator frame `node` (whose id is `id`) and
    /// return its owned children.
    fn run_generator_program(
        id: &str,
        node: &PenNode,
        spec: &GeneratorSpec,
        runner: Option<GeneratorRunner>,
    ) -> Result<Vec<PenNode>, GeneratorError> {
        spec.validate()?;
        let runner = runner.ok_or(GeneratorError::RuntimeUnavailable)?;
        let mut frame = node.clone();
        if let Some(children) = frame.children_mut() {
            children.clear();
        }
        let raw = runner(&GeneratorRequest {
            generator_id: id,
            spec,
            frame: &frame,
        })?;
        materialize_children(id, raw)
    }

    /// Refuse output whose derived ids already belong to a node outside
    /// generator `id`'s own subtree (which the write replaces).
    fn check_generated_ids(
        &self,
        generator: Option<&PenNode>,
        children: &[PenNode],
    ) -> Result<(), GeneratorError> {
        let mut taken = self.collect_node_ids();
        if let Some(generator) = generator {
            let mut own = HashSet::new();
            walkers::collect_ids(
                generator.children().map(Vec::as_slice).unwrap_or(&[]),
                &mut own,
            );
            taken.retain(|id| !own.contains(id));
        }
        let mut incoming = HashSet::new();
        walkers::collect_ids(children, &mut incoming);
        match incoming.into_iter().find(|id| taken.contains(id)) {
            Some(id) => Err(GeneratorError::IdCollision { id: id.to_string() }),
            None => Ok(()),
        }
    }

    /// Build the command that re-runs generator `id`, optionally with new
    /// parameter `values` first. Pure — the document is not touched.
    /// `Ok(None)` means the run reproduced exactly what is stored (same
    /// spec, byte-identical children): there is nothing to write.
    pub fn generator_regenerate_command(
        &self,
        id: &NodeId,
        values: Option<&Map<String, Value>>,
        runner: Option<GeneratorRunner>,
    ) -> Result<Option<EditorCommand>, GeneratorError> {
        self.generator_rewrite_command(
            id,
            &|spec| match values {
                Some(values) => spec.apply_param_values(values),
                None => Ok(()),
            },
            runner,
        )
    }

    /// [`generator_regenerate_command`](Self::generator_regenerate_command)
    /// with an arbitrary `edit` to the stored spec first (e.g. a new
    /// program). The edited spec and its output land in ONE command.
    pub fn generator_rewrite_command(
        &self,
        id: &NodeId,
        edit: &dyn Fn(&mut GeneratorSpec) -> Result<(), GeneratorError>,
        runner: Option<GeneratorRunner>,
    ) -> Result<Option<EditorCommand>, GeneratorError> {
        let (page_id, node) = self
            .locate_generator_node(id)
            .ok_or_else(|| GeneratorError::NodeNotFound { id: id.to_string() })?;
        let mut spec = generator_spec_of(node)
            .ok_or_else(|| GeneratorError::NotAGenerator { id: id.to_string() })??;
        if !matches!(node, PenNode::Frame(_)) {
            return Err(GeneratorError::NotAFrame { id: id.to_string() });
        }
        edit(&mut spec)?;
        let children = Self::run_generator_program(id.as_str(), node, &spec, runner)?;
        self.check_generated_ids(Some(node), &children)?;
        spec.output_hash = Some(output_hash(&children));
        let explain = spec.to_explain();
        let current = node.children().map(Vec::as_slice).unwrap_or(&[]);
        if node.base().explain.as_deref() == Some(explain.as_str()) && current == children {
            return Ok(None);
        }
        let patch = json!({ "explain": explain, "children": children });
        Ok(Some(EditorCommand::PatchNodeData {
            node_id: id.clone(),
            patch_json: patch.to_string(),
            page_id,
        }))
    }

    /// Build the command that inserts a NEW generator frame under
    /// `parent_id` (`NONE` = page root) with its first materialized
    /// output. `frame` must be a Frame; its id is replaced by a fresh
    /// editor id, which is returned alongside the command.
    pub fn generator_create_command(
        &self,
        parent_id: &NodeId,
        page_id: Option<&str>,
        frame: PenNode,
        mut spec: GeneratorSpec,
        runner: Option<GeneratorRunner>,
    ) -> Result<(EditorCommand, NodeId), GeneratorError> {
        let PenNode::Frame(_) = &frame else {
            return Err(GeneratorError::NotAFrame {
                id: frame.id_str().to_string(),
            });
        };
        let mut frame = frame;
        let mut taken = self.collect_node_ids();
        let id = DocumentIdAllocator::sequential_for_document(&self.doc)
            .and_then(|mut allocator| allocator.allocate(&mut taken))
            .map_err(|_| GeneratorError::ApplyRejected)?;
        let base = frame.base_mut();
        base.id = id.as_str().to_string();
        let human_explain = base
            .explain
            .take()
            .filter(|explain| !explain.starts_with(GENERATOR_EXPLAIN_PREFIX));
        spec.original_explain = spec.original_explain.or(human_explain);
        let children = Self::run_generator_program(id.as_str(), &frame, &spec, runner)?;
        self.check_generated_ids(None, &children)?;
        spec.output_hash = Some(output_hash(&children));
        frame.base_mut().explain = Some(spec.to_explain());
        if let Some(slot) = frame.children_mut() {
            *slot = children;
        }
        Ok((
            EditorCommand::InsertAuthoredSubtreePreservingRoots {
                nodes: vec![frame],
                parent_id: parent_id.clone(),
                page_id: page_id.map(str::to_string),
            },
            id,
        ))
    }

    /// Build the command that detaches generator `id`: the program and
    /// params are dropped, the children stay as plain nodes, and the
    /// node's pre-generator `explain` comes back.
    pub fn generator_detach_command(&self, id: &NodeId) -> Result<EditorCommand, GeneratorError> {
        let (page_id, node) = self
            .locate_generator_node(id)
            .ok_or_else(|| GeneratorError::NodeNotFound { id: id.to_string() })?;
        if !super::is_generator(node) {
            return Err(GeneratorError::NotAGenerator { id: id.to_string() });
        }
        // A marker that no longer parses still detaches cleanly — there is
        // just no original explain to restore.
        let original = generator_spec_of(node)
            .and_then(Result::ok)
            .and_then(|spec| spec.original_explain);
        Ok(EditorCommand::PatchNodeData {
            node_id: id.clone(),
            patch_json: json!({ "explain": original }).to_string(),
            page_id,
        })
    }

    /// True when generator `id`'s live children differ from what it last
    /// wrote (someone edited an owned child by hand).
    pub fn generator_children_edited(&self, id: &NodeId) -> bool {
        let Some((_, node)) = self.locate_generator_node(id) else {
            return false;
        };
        let Some(Ok(spec)) = generator_spec_of(node) else {
            return false;
        };
        let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
        spec.output_hash
            .is_some_and(|stored| stored != output_hash(children))
    }

    /// Apply `command` (if any) and park the outcome for the panel. With
    /// `record_history`, exactly one undo step is recorded: the command's
    /// own push when it makes one, else ours.
    fn record_generator_result(
        &mut self,
        id: &NodeId,
        result: Result<Option<EditorCommand>, GeneratorError>,
        record_history: bool,
    ) -> Result<bool, GeneratorError> {
        let outcome = result.and_then(|command| {
            let Some(command) = command else {
                return Ok(false);
            };
            let before = self.snapshot_for_history();
            let pushes_before = self.history_push_count;
            if !self.apply(command) {
                return Err(GeneratorError::ApplyRejected);
            }
            if record_history && self.history_push_count == pushes_before {
                self.history_push_past(before);
            }
            Ok(true)
        });
        self.ui.generator_error = match &outcome {
            Ok(_) => None,
            Err(error) => Some(GeneratorPanelError {
                node_id: id.clone(),
                message: error.to_string(),
            }),
        };
        outcome
    }

    /// Re-run generator `id` with its current params (one undo step).
    pub fn regenerate_generator(
        &mut self,
        id: &NodeId,
        runner: Option<GeneratorRunner>,
    ) -> Result<bool, GeneratorError> {
        let result = self.generator_regenerate_command(id, None, runner);
        self.record_generator_result(id, result, true)
    }

    /// Commit a property-panel draft into parameter `index` and regenerate.
    /// Records no history: the panel's commit path snapshots around it, so
    /// the param write + new children land as ONE undo step.
    pub fn set_generator_param_input(
        &mut self,
        id: &NodeId,
        index: usize,
        raw: &str,
        runner: Option<GeneratorRunner>,
    ) -> Result<bool, GeneratorError> {
        let result = self.generator_spec(id).and_then(|spec| {
            let param = spec
                .params
                .get(index)
                .ok_or(GeneratorError::ParamIndexOutOfRange {
                    index,
                    count: spec.params.len(),
                })?;
            let mut values = Map::new();
            values.insert(param.name.clone(), param.parse_input(raw)?);
            self.generator_regenerate_command(id, Some(&values), runner)
        });
        self.record_generator_result(id, result, false)
    }

    /// Flip boolean parameter `index` and regenerate (one undo step).
    pub fn toggle_generator_bool_param(
        &mut self,
        id: &NodeId,
        index: usize,
        runner: Option<GeneratorRunner>,
    ) -> Result<bool, GeneratorError> {
        let result = self.generator_spec(id).and_then(|spec| {
            let param = spec
                .params
                .get(index)
                .ok_or(GeneratorError::ParamIndexOutOfRange {
                    index,
                    count: spec.params.len(),
                })?;
            let current =
                param
                    .value
                    .as_bool()
                    .ok_or_else(|| GeneratorError::InvalidParamValue {
                        name: param.name.clone(),
                        reason: "not a boolean parameter".into(),
                    })?;
            let mut values = Map::new();
            values.insert(param.name.clone(), Value::Bool(!current));
            self.generator_regenerate_command(id, Some(&values), runner)
        });
        self.record_generator_result(id, result, true)
    }

    /// Detach generator `id` (one undo step). Needs no runtime, so it
    /// works on every host.
    pub fn detach_generator(&mut self, id: &NodeId) -> bool {
        let result = self.generator_detach_command(id).map(Some);
        self.record_generator_result(id, result, true)
            .unwrap_or(false)
    }

    /// Insert a built-in starter near the top-left of the visible canvas,
    /// select it, and record one undo step.
    pub fn insert_generator_starter(
        &mut self,
        starter: &GeneratorStarter,
        runner: Option<GeneratorRunner>,
    ) -> Result<NodeId, GeneratorError> {
        // Generators write whole subtrees the collaboration protocol cannot
        // carry yet; a live session refuses them like other unsupported
        // node properties.
        let policy = crate::collab_gate::CollabGatePolicy::from(&self.editor_ui.collab);
        if policy
            .check(
                crate::collab_gate::CollabGateAction::Document(
                    crate::collab_gate::CollabDocumentMutation::Unsupported(
                        crate::collab_gate::CollabUnsupportedFeature::UnsupportedNodeProperty,
                    ),
                ),
                crate::collab_gate::CollabEditSource::User,
            )
            .is_err()
        {
            return Err(GeneratorError::CollaborationUnsupported);
        }
        let zoom = if self.viewport.zoom > 0.0 {
            self.viewport.zoom
        } else {
            1.0
        };
        let x = f64::from(((80.0 - self.viewport.pan_x) / zoom).round());
        let y = f64::from(((80.0 - self.viewport.pan_y) / zoom).round());
        let before = self.snapshot_for_history();
        let pushes_before = self.history_push_count;
        let (command, id) = self.generator_create_command(
            &NodeId::NONE,
            None,
            starter.frame(x, y),
            starter.spec(),
            runner,
        )?;
        if !self.apply(command) {
            return Err(GeneratorError::ApplyRejected);
        }
        let _ = self.apply(EditorCommand::SetSelection {
            node_id: id.clone(),
        });
        if self.history_push_count == pushes_before {
            self.history_push_past(before);
        }
        self.ui.generator_error = None;
        Ok(id)
    }
}
