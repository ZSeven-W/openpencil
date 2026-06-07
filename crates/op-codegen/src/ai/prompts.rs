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
    user_message.push_str("Nodes (JSON):\n");
    user_message.push_str(&compact_nodes(&input.nodes_json));
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
            user_message.push_str(&format!(
                "- `{}` (chunk: {})\n",
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
