//! MCP authoring tools backed by the editor's validated motion commands.

use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::motion::{MotionPreference, MotionTrigger, NodeAnimation, Transition};
use jian_ops_schema::node::PenNode;
use op_editor_core::motion_edit::MotionCommand;
use op_editor_core::motion_presets::{MotionPreset, STAGGER_STEP_MS};
use op_editor_core::motion_timeline_projection::strip;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::walkers::find_node;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::{McpTool, ToolErrorCode, ToolOutcome};

#[derive(Clone, Copy)]
pub enum MotionTool {
    Get,
    SetAnimation,
    RemoveAnimation,
    ApplyPreset,
    SetTransition,
    SetPreference,
}

impl MotionTool {
    pub const ALL: [Self; 6] = [
        Self::Get,
        Self::SetAnimation,
        Self::RemoveAnimation,
        Self::ApplyPreset,
        Self::SetTransition,
        Self::SetPreference,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Get => "get_motion",
            Self::SetAnimation => "set_node_animation",
            Self::RemoveAnimation => "remove_node_animation",
            Self::ApplyPreset => "apply_motion_preset",
            Self::SetTransition => "set_node_transition",
            Self::SetPreference => "set_motion_preference",
        }
    }

    pub fn snapshot(self, state: &EditorState) -> impl McpTool {
        MotionSnapshot {
            tool: self,
            state: state.clone(),
        }
    }
}

struct MotionSnapshot {
    tool: MotionTool,
    state: EditorState,
}

type Args = BTreeMap<String, String>;
type ToolError = (ToolErrorCode, String);

impl McpTool for MotionSnapshot {
    fn name(&self) -> &str {
        self.tool.name()
    }

    fn call(&self, args: &Args) -> ToolOutcome {
        self.execute(args)
            .unwrap_or_else(|(code, message)| ToolOutcome::Err(code, message))
    }
}

impl MotionSnapshot {
    fn execute(&self, args: &Args) -> Result<ToolOutcome, ToolError> {
        let command = match self.tool {
            MotionTool::Get => {
                let node_id = node_id(required(args, "nodeId")?)?;
                let deep = optional(args, "deep")?.unwrap_or(false);
                let node = find_node(self.state.active_children(), &node_id).ok_or_else(|| {
                    invalid(format!("node {node_id} not found on the active page"))
                })?;
                let mut nodes = Vec::new();
                collect(node, deep, &mut nodes);
                return Ok(ToolOutcome::OkJson(
                    json!({
                        "nodeId": node_id.as_str(),
                        "preference": self.state.doc.motion.unwrap_or(MotionPreference::Full),
                        "nodes": nodes,
                    })
                    .to_string(),
                ));
            }
            MotionTool::SetAnimation => {
                let node_id = node_id(required(args, "nodeId")?)?;
                let animation: NodeAnimation = structured(args, "animation")?;
                match optional(args, "index")? {
                    Some(index) => MotionCommand::UpdateAnimation {
                        node_id,
                        index,
                        animation,
                    },
                    None => MotionCommand::AddAnimation {
                        node_id,
                        animation,
                        index: None,
                    },
                }
            }
            MotionTool::RemoveAnimation => MotionCommand::RemoveAnimation {
                node_id: node_id(required(args, "nodeId")?)?,
                index: parse(required(args, "index")?, "index")?,
            },
            MotionTool::ApplyPreset => preset_command(args)?,
            MotionTool::SetTransition => MotionCommand::SetTransition {
                node_id: node_id(required(args, "nodeId")?)?,
                transition: structured::<Option<Transition>>(args, "transition")?,
            },
            MotionTool::SetPreference => {
                MotionCommand::SetPreference(choice(required(args, "value")?, "value")?)
            }
        };
        let changed = self
            .state
            .motion_command_changes(&command)
            .map_err(|error| invalid(error.to_string()))?;
        let value = json!({"changed": changed}).to_string();
        if changed {
            Ok(ToolOutcome::OkJsonWithCommand(
                value,
                EditorCommand::Motion(command),
            ))
        } else {
            Ok(ToolOutcome::OkJson(value))
        }
    }
}

fn collect(node: &PenNode, deep: bool, out: &mut Vec<Value>) {
    let (transition, animations) = node.motion_declarations();
    let animations: Vec<_> = animations
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, animation)| {
            let timing = strip(node.id_str(), index, animation);
            json!({
                "index": index,
                "animation": animation,
                "startMs": timing.start_ms,
                "endMs": timing.end_ms,
                "keyframeTimesMs": timing.keyframe_ms,
            })
        })
        .collect();
    if !deep || out.is_empty() || transition.is_some() || !animations.is_empty() {
        out.push(json!({
            "nodeId": node.id_str(), "transition": transition, "animations": animations,
        }));
    }
    if deep {
        for child in node.children().into_iter().flatten() {
            collect(child, true, out);
        }
    }
}

fn preset_command(args: &Args) -> Result<MotionCommand, ToolError> {
    let names: Vec<String> = structured(args, "nodeIds")?;
    let node_ids = names
        .iter()
        .map(|id| node_id(id))
        .collect::<Result<Vec<_>, _>>()?;
    if node_ids.is_empty() || names.iter().collect::<BTreeSet<_>>().len() != names.len() {
        return Err(invalid("nodeIds must contain distinct node ids"));
    }
    let preset: MotionPreset = choice(required(args, "preset")?, "preset")?;
    let id = args.get("clipId").cloned();
    let trigger = match args.get("trigger") {
        Some(value) => choice(value, "trigger")?,
        None if id.is_some() => MotionTrigger::Manual,
        None => MotionTrigger::Mount,
    };
    let stagger_ms = match args.get("stagger").map(String::as_str) {
        Some("true") => STAGGER_STEP_MS,
        None | Some("false") => 0,
        Some(value) => parse(value, "stagger")?,
    };
    Ok(MotionCommand::ApplyPreset {
        node_ids,
        preset,
        trigger,
        id,
        stagger_ms,
    })
}

fn node_id(id: &str) -> Result<NodeId, ToolError> {
    if id.trim().is_empty() {
        return Err(invalid("node id must not be empty"));
    }
    Ok(NodeId::new(id))
}

fn required<'a>(args: &'a Args, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .map(String::as_str)
        .ok_or_else(|| (ToolErrorCode::MissingArgument, format!("{key} is required")))
}

fn parse<T: std::str::FromStr>(raw: &str, key: &str) -> Result<T, ToolError> {
    raw.parse()
        .map_err(|_| invalid(format!("invalid {key}: {raw}")))
}

fn optional<T: std::str::FromStr>(args: &Args, key: &str) -> Result<Option<T>, ToolError> {
    args.get(key).map(|raw| parse(raw, key)).transpose()
}

fn structured<T: DeserializeOwned>(args: &Args, key: &str) -> Result<T, ToolError> {
    serde_json::from_str(required(args, key)?).map_err(|error| invalid(format!("{key}: {error}")))
}

fn choice<T: DeserializeOwned>(raw: &str, key: &str) -> Result<T, ToolError> {
    serde_json::from_value(json!(raw)).map_err(|error| invalid(format!("{key}: {error}")))
}

fn invalid(message: impl Into<String>) -> ToolError {
    (ToolErrorCode::InvalidArgument, message.into())
}

#[cfg(test)]
mod tests;
