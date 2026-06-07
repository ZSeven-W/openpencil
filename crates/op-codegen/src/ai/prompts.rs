//! Request builders for the AI codegen pipeline. Ported from the TS
//! `apps/web/src/services/ai/codegen-prompts.ts` USER-message assembly.
//!
//! Unlike the TS file, these builders do NOT embed skill text. The pipeline
//! is transport-agnostic: it emits skill NAMES + the pipeline-owned user
//! message, and the host/proxy expands those names into the final system
//! prompt later (via a shared helper). So each builder returns a
//! `PendingRequest { skills, user_message, ... }` rather than `{ system, user }`.

use crate::ai::types::{ChunkContract, CodegenInput, PendingRequest, RequestId, RequestKind};
use op_editor_core::codegen::Framework;

// Per-phase output token budgets. These mirror the TS skill `budget`
// frontmatter in `packages/pen-ai-skills/skills/` (planning ~2000,
// chunk ~3000, assembly default). The pipeline carries them on the
// `PendingRequest` so the transport can cap each model call per phase.
const PLANNING_MAX_OUTPUT_TOKENS: u32 = 2000;
const CHUNK_MAX_OUTPUT_TOKENS: u32 = 3000;
const ASSEMBLY_MAX_OUTPUT_TOKENS: u32 = 4000;

fn planning_skills() -> Vec<&'static str> {
    vec!["codegen-planning"]
}

fn chunk_skills(fw: Framework) -> Vec<&'static str> {
    vec!["codegen-chunk", fw.skill_name()]
}

fn assembly_skills(fw: Framework) -> Vec<&'static str> {
    vec!["codegen-assembly", fw.skill_name()]
}

/// Strip fields that don't influence code generation. Port of
/// `codegen-prompts.ts::stripNoise`: keeps request bodies small (so proxies
/// don't reject them with 403/413) and reduces input tokens.
///
/// Dropped: `id` (model picks its own names), `parentId` / `pageId` /
/// `_meta` (tree structure is implicit in nesting), and default-valued
/// `rotation` (0) / `opacity` (1) / `visible` (true). Recurses into nested
/// objects + arrays.
fn strip_noise(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                strip_noise(item);
            }
        }
        serde_json::Value::Object(map) => {
            map.retain(|k, v| {
                if matches!(k.as_str(), "id" | "parentId" | "pageId" | "_meta") {
                    return false;
                }
                if k == "rotation" && v.as_f64() == Some(0.0) {
                    return false;
                }
                if k == "opacity" && v.as_f64() == Some(1.0) {
                    return false;
                }
                if k == "visible" && v.as_bool() == Some(true) {
                    return false;
                }
                true
            });
            for v in map.values_mut() {
                strip_noise(v);
            }
        }
        _ => {}
    }
}

/// Apply `strip_noise` to an already-compacted node JSON string and
/// re-serialize compactly. Mirrors `codegen-prompts.ts::compactNodes`.
/// Falls back to the input verbatim if it isn't parseable JSON, so a caller
/// passing pre-formatted text still gets embedded.
fn compact_nodes(nodes_json: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(nodes_json) {
        Ok(mut value) => {
            strip_noise(&mut value);
            serde_json::to_string(&value).unwrap_or_else(|_| nodes_json.to_string())
        }
        Err(_) => nodes_json.to_string(),
    }
}

/// Render a planning node-tree summary that PRESERVES each node's `id`. Port
/// of `pen-core::nodeTreeToSummary` (used by the TS planning prompt). Unlike
/// `compact_nodes` / `strip_noise`, this keeps ids so the planner can emit
/// `nodeIds` that reference real nodes (without ids, hydration drops every
/// chunk). Each line is:
/// `  - [id] type "name" (WxH) [role] [N children]` with 2-space indent per
/// depth. Falls back to the input verbatim if it isn't parseable JSON.
fn summarize_nodes_with_ids(nodes_json: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(nodes_json) {
        Ok(serde_json::Value::Array(items)) => {
            let mut out = String::new();
            summarize_node_list(&items, 0, &mut out);
            out
        }
        // A single object (not wrapped in an array) is still summarizable.
        Ok(value @ serde_json::Value::Object(_)) => {
            let mut out = String::new();
            summarize_node_list(std::slice::from_ref(&value), 0, &mut out);
            out
        }
        _ => nodes_json.to_string(),
    }
}

fn summarize_node_list(nodes: &[serde_json::Value], depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    for node in nodes {
        let serde_json::Value::Object(map) = node else {
            continue;
        };
        let id = map
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let ty = map
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let name = map
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let width = json_dim(map.get("width"));
        let height = json_dim(map.get("height"));
        let role = map.get("role").and_then(serde_json::Value::as_str);
        let children = map.get("children").and_then(serde_json::Value::as_array);
        let child_count = children.map(|c| c.len()).unwrap_or(0);

        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!(
            "{indent}- [{id}] {ty} \"{name}\" ({width}x{height})"
        ));
        if let Some(role) = role.filter(|r| !r.is_empty()) {
            out.push_str(&format!(" [{role}]"));
        }
        if child_count > 0 {
            out.push_str(&format!(" [{child_count} children]"));
        }
        if let Some(children) = children.filter(|c| !c.is_empty()) {
            summarize_node_list(children, depth + 1, out);
        }
    }
}

/// Format a width/height field like the TS summary (`n.width ?? '?'`).
fn json_dim(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => "?".to_string(),
    }
}

/// Build the planning request. `strict` appends an "ONLY valid JSON"
/// instruction to the USER message (the TS appends it to the system prompt,
/// but since the proxy owns system-prompt assembly we carry it in the user
/// message so both transports behave identically).
pub fn plan_request(id: RequestId, input: &CodegenInput, strict: bool) -> PendingRequest {
    let mut user_message = String::new();
    user_message.push_str(&format!(
        "Target framework: {}\n",
        input.framework.as_wire()
    ));
    user_message.push('\n');
    // PLANNING ONLY: embed a node-tree SUMMARY that PRESERVES each node's id
    // (TS uses `nodeTreeToSummary`). The planner returns `nodeIds` referencing
    // these ids, so stripping them (as the chunk request does) would make
    // hydration drop every chunk. Mirrors `codegen-prompts.ts::buildPlanningPrompt`.
    user_message.push_str("Node tree:\n");
    user_message.push_str(&summarize_nodes_with_ids(&input.nodes_json));
    user_message.push('\n');
    user_message.push_str("\nAnalyze this node tree and output a JSON code generation plan.");
    if strict {
        user_message
            .push_str("\n\nCRITICAL: Respond with ONLY valid JSON. No markdown, no explanation.");
    }

    PendingRequest {
        id,
        kind: RequestKind::Planning,
        skills: planning_skills(),
        user_message,
        max_output_tokens: PLANNING_MAX_OUTPUT_TOKENS,
        thinking: input.thinking,
        effort: input.effort,
    }
}

/// Build a chunk request: the chunk's node JSON + suggested name + dep
/// contracts + asset hints, as the user message.
pub fn chunk_request(
    id: RequestId,
    chunk_id: &str,
    chunk_nodes_json: &str,
    suggested_name: &str,
    dep_contracts: &[ChunkContract],
    asset_hints: &[String],
    input: &CodegenInput,
) -> PendingRequest {
    let framework = input.framework.as_wire();
    let mut user_message = String::new();
    user_message.push_str(&format!(
        "Generate a {framework} component named \"{suggested_name}\".\n"
    ));

    if !dep_contracts.is_empty() {
        user_message.push_str("\n## Dependency Contracts\n");
        user_message.push_str(
            "The following components are available from upstream chunks. Import and use them:\n\n",
        );
        for c in dep_contracts {
            // Mirror codegen-prompts.ts:88-90: include exported props
            // (name: type), slot names, and import sources so the dependent
            // chunk knows the upstream component's surface AND what it pulls in.
            let props = c
                .exported_props
                .iter()
                .map(|p| format!("{}: {}", p.name, p.ty))
                .collect::<Vec<_>>()
                .join(", ");
            let slots = c
                .slots
                .iter()
                .map(|s| s.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            let imports = c
                .imports
                .iter()
                .map(|i| {
                    if i.specifiers.is_empty() {
                        i.source.clone()
                    } else {
                        format!("{} from {}", i.specifiers.join(", "), i.source)
                    }
                })
                .collect::<Vec<_>>()
                .join("; ");
            user_message.push_str(&format!(
                "- `{}` (chunk: {}): props=[{props}], slots=[{slots}], imports=[{imports}]\n",
                c.component_name, c.chunk_id
            ));
        }
    }

    if !asset_hints.is_empty() {
        user_message.push_str("\n## Exported Image Assets\n");
        user_message.push_str(
            "The following image assets were exported from the design. Use these relative paths \
             directly as src/background-image URLs. Do NOT inline base64.\n\n",
        );
        for hint in asset_hints {
            user_message.push_str(&format!("- {hint}\n"));
        }
    }

    user_message.push_str("\nNodes (JSON):\n");
    user_message.push_str(&compact_nodes(chunk_nodes_json));
    user_message.push('\n');
    user_message.push_str("\nOutput the code followed by ---CONTRACT--- and the JSON contract.");

    PendingRequest {
        id,
        kind: RequestKind::Chunk {
            chunk_id: chunk_id.to_string(),
        },
        skills: chunk_skills(input.framework),
        user_message,
        max_output_tokens: CHUNK_MAX_OUTPUT_TOKENS,
        thinking: input.thinking,
        effort: input.effort,
    }
}

/// Build the assembly request: all chunk codes+contracts (pre-formatted into
/// `chunk_blocks`) + plan summary + variables + exported asset paths.
pub fn assembly_request(
    id: RequestId,
    chunk_blocks: &str,
    plan_summary: &str,
    input: &CodegenInput,
    asset_paths: &[String],
) -> PendingRequest {
    let framework = input.framework.as_wire();
    let mut user_message = String::new();
    user_message.push_str(&format!(
        "Assemble the following {framework} code chunks into a single production-ready file.\n"
    ));
    user_message.push('\n');
    user_message.push_str(plan_summary);
    user_message.push('\n');

    if let Some(variables_json) = &input.variables_json {
        user_message.push_str(&format!("Design variables: {variables_json}\n"));
    }

    if !asset_paths.is_empty() {
        user_message.push('\n');
        user_message.push_str("Exported image assets are available under ./assets/.\n");
        user_message
            .push_str("Keep any existing ./assets/... references unchanged in the final code.\n");
        user_message.push_str(&format!("Assets: {}\n", asset_paths.join(", ")));
    }

    user_message.push_str("\n## Chunks\n\n");
    user_message.push_str(chunk_blocks);
    user_message.push('\n');
    user_message.push_str("\nOutput ONLY the final assembled source code.");

    PendingRequest {
        id,
        kind: RequestKind::Assembly,
        skills: assembly_skills(input.framework),
        user_message,
        max_output_tokens: ASSEMBLY_MAX_OUTPUT_TOKENS,
        thinking: input.thinking,
        effort: input.effort,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::chat_provider::{EffortLevel, ThinkingMode};

    fn input() -> CodegenInput {
        CodegenInput {
            nodes_json: "[{\"type\":\"frame\",\"children\":[]}]".into(),
            framework: Framework::React,
            variables_json: None,
            max_output_tokens: 3000,
            thinking: ThinkingMode::Adaptive,
            effort: EffortLevel::Low,
        }
    }

    #[test]
    fn planning_request_names_planning_skill_and_carries_nodes() {
        let r = plan_request(RequestId(1), &input(), false);
        assert_eq!(r.kind, RequestKind::Planning);
        assert_eq!(r.skills, vec!["codegen-planning"]);
        assert!(r.user_message.contains("frame"));
        assert!(!r.user_message.contains("ONLY valid JSON"));
    }

    #[test]
    fn planning_request_preserves_node_ids() {
        // FIX 1: the planning prompt must include each node's id so the
        // planner can return `nodeIds` that hydrate against real nodes.
        let mut input = input();
        input.nodes_json = "[{\"type\":\"frame\",\"id\":\"n1\",\"children\":[]}]".into();
        let r = plan_request(RequestId(1), &input, false);
        assert!(r.user_message.contains("n1"));
        assert!(r.user_message.contains("[n1]"));
    }

    #[test]
    fn planning_summary_preserves_nested_ids() {
        let mut input = input();
        input.nodes_json =
            "[{\"type\":\"frame\",\"id\":\"root\",\"children\":[{\"type\":\"text\",\"id\":\"child\"}]}]"
                .into();
        let r = plan_request(RequestId(1), &input, false);
        assert!(r.user_message.contains("[root]"));
        assert!(r.user_message.contains("[child]"));
    }

    #[test]
    fn chunk_request_includes_dep_props_and_slots() {
        // FIX 2: dependency contract summaries carry exported props, slots,
        // and import sources.
        let dep = ChunkContract {
            chunk_id: "d1".into(),
            component_name: "Card".into(),
            exported_props: vec![crate::ai::types::PropDef {
                name: "title".into(),
                ty: "string".into(),
                required: true,
            }],
            slots: vec![crate::ai::types::SlotDef {
                name: "footer".into(),
                description: "card footer".into(),
            }],
            imports: vec![crate::ai::types::ImportDef {
                source: "lucide-react".into(),
                specifiers: vec!["Star".into()],
            }],
            ..Default::default()
        };
        let r = chunk_request(RequestId(2), "c1", "[]", "Hero", &[dep], &[], &input());
        assert!(r.user_message.contains("props=[title: string]"));
        assert!(r.user_message.contains("slots=[footer]"));
        assert!(r.user_message.contains("imports=[Star from lucide-react]"));
    }

    #[test]
    fn strict_planning_adds_json_only_instruction() {
        let r = plan_request(RequestId(1), &input(), true);
        assert!(r.user_message.contains("ONLY valid JSON"));
    }

    #[test]
    fn chunk_request_names_framework_skill() {
        let r = chunk_request(RequestId(2), "c1", "[]", "Navbar", &[], &[], &input());
        assert_eq!(
            r.kind,
            RequestKind::Chunk {
                chunk_id: "c1".into()
            }
        );
        assert_eq!(r.skills, vec!["codegen-chunk", "codegen-react"]);
        assert!(r.user_message.contains("Navbar"));
    }

    #[test]
    fn assembly_request_names_assembly_and_framework_skills() {
        let r = assembly_request(RequestId(3), "blocks", "summary", &input(), &[]);
        assert_eq!(r.kind, RequestKind::Assembly);
        assert_eq!(r.skills, vec!["codegen-assembly", "codegen-react"]);
    }
}
