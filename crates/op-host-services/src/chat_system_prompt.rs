//! Per-turn chat system-prompt assembly + transcript→history mapping.
//!
//! Ports the TS chat context plumbing:
//! - `buildChatSystemPrompt` (`apps/web/src/services/ai/ai-prompts.ts`)
//!   — `CHAT_CORE_PROMPT` + generation-phase skills resolved with the
//!   `hasDesignMd` / `hasVariables` flags and the condensed design.md
//!   style policy as `{{designMdContent}}` dynamic content.
//! - `AGENT_TOOL_INSTRUCTIONS_CRUD` + `buildContextString`
//!   (`apps/web/src/components/panels/ai-chat-handlers.ts` +
//!   `ai-chat-context-builder.ts`) — the system prompt for the
//!   tool-executing builtin agent loop.
//! - the `chatHistory` mapping (`ai-chat-handlers.ts:684`) — the chat
//!   transcript minus the in-flight turn, as role + text pairs.

use op_ai::chat_provider::ChatHistoryRole;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::{ChatMessage, EditorState};
use op_orchestrator::build_design_md_style_policy;

/// TS `CHAT_CORE_PROMPT` — verbatim port (BLOCK = ``` expanded).
const CHAT_CORE_PROMPT: &str = r#"You are a design assistant for OpenPencil, a vector design tool that renders PenNode JSON on a canvas.

ABSOLUTE REQUIREMENT — When a user asks to create/generate/design/make ANY visual element or UI:
You MUST output a ```json code block containing a valid PenNode JSON array. This is NON-NEGOTIABLE.
Add a 1-2 sentence description AFTER the JSON block, not before.
NEVER describe what you "would" create — ALWAYS output the actual JSON immediately.
NEVER output HTML, CSS, or React code — ONLY PenNode JSON.
NEVER say "I will create..." — START DIRECTLY WITH <step>.
NEVER use "OpenPencil", "Pencil", or the tool name as brand/app name in designs. Use generic placeholders like "AppName", "Acme", or contextually relevant names.

You may include 1-2 brief <step> tags before the JSON (optional, keep them SHORT).
When a user asks non-design questions (explain, suggest colors, give advice), respond in text."#;

const AI_CHAT_DESIGN_QUALITY: &str = r#"AI CHAT DESIGN QUALITY:
- Mobile top rhythm: keep the header/title group close to the first useful control or content module. On 375-430px screens, the gap from title/header to search, primary action, chart, or first card should usually be 20-32px.
- Search bars should be a single compact control row. Avoid a large tinted rounded shell containing another rounded input unless the user explicitly asks for that visual.
- Product card favorite/heart controls are functional icon-buttons. Keep every favorite/heart fully inside its card or image with an 8-12px inset; do not straddle card borders or float into section headings.
- Do not repeat the same predictable mobile stack of search + categories + orange promo + two cards. Pick a distinct visual concept for the domain before composing the screen.
- Give the first viewport one signature moment: a crafted hero/product composition, editorial crop, distinctive category rail, refined data module, or other domain-specific focal idea.
- Bottom navigation is optional and should be integrated with the page flow, not a detached floating pill or extra footer band."#;

/// TS `AGENT_TOOL_INSTRUCTIONS_CRUD` — verbatim port. The system
/// prompt for tool-executing chat turns (builtin agent loop).
const AGENT_TOOL_INSTRUCTIONS_CRUD: &str = r##"You are a design editor. Use tools to inspect, modify, insert, and delete elements on the canvas.

WORKFLOW:
1. Use snapshot_layout or batch_get FIRST to see the tree structure and find node IDs.
2. Use the appropriate tool: insert_node to add, update_node to modify, delete_node to remove, move_node to reparent.
3. When inserting, use "after" parameter with a sibling ID to place the new node in the correct position.
4. After each operation, write 1-2 sentences summarizing what changed.

DIAGNOSING OVERLAP / STACKING BUGS — read this before "fixing" any visual overlap:
- When snapshot_layout.overlaps is non-empty, two or more siblings share screen area. Do NOT blindly enlarge heights, shrink fonts, or tweak padding — those are surface patches.
- Inspect the overlapping nodes' shared PARENT via batch_get. Look at its `layout` field:
  • `layout: "none"` → a deliberate absolute stack. Overlap can be intentional; verify every child has explicit numeric x/y/width/height and the intended z-order before changing the layout.
  • missing `layout` → ambiguous. Set `vertical`/`horizontal` for flow content, or explicit `none` only for a deliberate absolute stack.
  • `layout: "vertical"` with gap=0 and children using textGrowth:"fit_content" → text can visually touch; bump `gap` or add padding on the children.
- Preferred fix for an ACCIDENTAL `layout: "none"` parent whose children should flow (ordinary titles, rows, form fields):
  update_node(parent, { layout: "vertical", gap: 8, alignItems: "flex-start" })
  and strip the children's absolute x/y (the flex engine positions them).
- For a simple filled circular badge/avatar with centered initials or an icon: use a fixed square frame with cornerRadius = width/2, layout:"horizontal", alignItems:"center", justifyContent:"center", children:[ the text/icon ].
- For a progress ring/donut with centered content: use a fixed square `layout:"none"` wrapper. Give every child explicit numeric x/y/width/height and order children front-to-back as center content, progress arc, then track; lower child indexes paint on top. Never put track, progress, and center content as siblings in horizontal/vertical flow.

INSERT_NODE GUIDE — always include complete node data with children:
- Button example: {"type":"frame","name":"My Button","width":"fill_container","height":50,"cornerRadius":8,"fill":[{"type":"solid","color":"#1877F2"}],"layout":"horizontal","gap":8,"alignItems":"center","justifyContent":"center","children":[{"type":"text","name":"Label","text":"Continue","fontSize":15,"fontWeight":600,"fill":[{"type":"solid","color":"#FFFFFF"}]}]}
- Text example: {"type":"text","name":"Title","text":"Hello","fontSize":24,"fontWeight":700,"fill":[{"type":"solid","color":"#1A1A2E"}]}
- When adding next to a similar element, use batch_get to read that element's full data first, then create matching structure.

Focus on the specific operation the user requested."##;

/// Build the chat-mode system prompt for a plain (no-tools) turn.
/// TS `buildChatSystemPrompt` port: core prompt + generation-phase
/// skills resolved against the live document's design.md / variables.
pub fn build_chat_system_prompt(state: &EditorState, user_message: &str) -> String {
    let design_md = state.doc.design_md.as_ref();
    let has_variables = state
        .doc
        .variables
        .as_ref()
        .is_some_and(|vars| !vars.is_empty());

    let mut options = op_ai_skills::ResolveOptions::default();
    options
        .flags
        .insert("hasDesignMd".to_string(), design_md.is_some());
    options
        .flags
        .insert("hasVariables".to_string(), has_variables);
    if let Some(spec) = design_md {
        options.dynamic_content.insert(
            "designMdContent".to_string(),
            build_design_md_style_policy(spec),
        );
    }
    let ctx = op_ai_skills::resolve_skills(op_ai_skills::Phase::Generation, user_message, &options);
    let knowledge = ctx
        .skills
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("{CHAT_CORE_PROMPT}\n\n{AI_CHAT_DESIGN_QUALITY}\n\n{knowledge}")
}

/// Build the agent-mode system prompt for a tool-executing builtin
/// turn. TS parity: `buildAgentSystemPrompt(msg, …)` resolves to
/// `AGENT_TOOL_INSTRUCTIONS_CRUD` for CRUD intent, then appends the
/// canvas-context string. (Design intent routes to the orchestrator
/// pipeline in this shell, so the CRUD instructions are the only
/// agent prompt the chat tool loop needs.)
pub fn build_agent_system_prompt(state: &EditorState) -> String {
    format!(
        "{AGENT_TOOL_INSTRUCTIONS_CRUD}{}",
        canvas_context_string(state)
    )
}

/// TS `buildContextString` port — document node summary, selection
/// summary, and variable names. Empty string when the canvas carries
/// no usable context.
fn canvas_context_string(state: &EditorState) -> String {
    let mut parts: Vec<String> = Vec::new();

    let mut flat: Vec<&jian_ops_schema::node::PenNode> = Vec::new();
    collect_nodes(state.active_children(), &mut flat);
    if !flat.is_empty() {
        let summary = flat
            .iter()
            .take(20)
            .map(|n| format!("{}:{}", node_type_label(n), node_display_name(n)))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("Document has {} nodes: {summary}", flat.len()));
    }

    if !state.selection.set.is_empty() {
        let selected: Vec<String> = state
            .selection
            .set
            .iter()
            .filter_map(|id| op_editor_core::walkers::find_node(state.active_children(), id))
            .map(|n| {
                let dims = match (n.width_px(), n.height_px()) {
                    (Some(w), Some(h)) => format!(" ({w}x{h})"),
                    _ => String::new(),
                };
                format!("{}:{}{dims}", node_type_label(n), node_display_name(n))
            })
            .collect();
        if !selected.is_empty() {
            parts.push(format!("Selected: {}", selected.join(", ")));
        }
    }

    if let Some(vars) = state.doc.variables.as_ref() {
        if !vars.is_empty() {
            let names = vars
                .iter()
                .map(|(name, def)| format!("${name}({})", variable_kind_label(&def.kind)))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("Variables: {names}"));
        }
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("\n\n[Canvas context: {}]", parts.join(". "))
    }
}

fn collect_nodes<'a>(
    children: &'a [jian_ops_schema::node::PenNode],
    out: &mut Vec<&'a jian_ops_schema::node::PenNode>,
) {
    for node in children {
        out.push(node);
        if let Some(kids) = node.children() {
            collect_nodes(kids, out);
        }
    }
}

fn node_display_name(node: &jian_ops_schema::node::PenNode) -> &str {
    node.base()
        .name
        .as_deref()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| node.id_str())
}

fn node_type_label(node: &jian_ops_schema::node::PenNode) -> &'static str {
    use jian_ops_schema::node::PenNode;
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::TextArea(_) => "text_area",
        PenNode::Select(_) => "select",
        PenNode::Switch(_) => "switch",
        PenNode::Checkbox(_) => "checkbox",
        PenNode::Slider(_) => "slider",
        PenNode::RadioGroup(_) => "radio_group",
        PenNode::NumberInput(_) => "number_input",
        PenNode::Progress(_) => "progress",
        PenNode::Tabs(_) => "tabs",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
    }
}

fn variable_kind_label(kind: &jian_ops_schema::variable::VariableKind) -> &'static str {
    use jian_ops_schema::variable::VariableKind;
    match kind {
        VariableKind::Color => "color",
        VariableKind::Number => "number",
        VariableKind::String => "string",
        VariableKind::Boolean => "boolean",
    }
}

/// Map the chat transcript into `(role, text)` history pairs for the
/// in-flight turn, excluding the turn itself: the trailing streaming
/// assistant bubble (`begin_send` pushed it empty) and the trailing
/// user message (it rides `ChatRequest::user_message`). TS parity:
/// `ai-chat-handlers.ts:684` maps prior messages, then pushes the
/// current user message separately.
pub fn chat_history_from_transcript(messages: &[ChatMessage]) -> Vec<(ChatHistoryRole, String)> {
    op_editor_host_core::chat::chat_history_from_transcript(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_system_prompt_carries_core_prompt_and_skills() {
        let state = EditorState::new();
        let prompt = build_chat_system_prompt(&state, "design a login form");
        assert!(prompt.starts_with("You are a design assistant for OpenPencil"));
        // Generation-phase skills resolve for a design message, so the
        // prompt must be longer than the bare core prompt.
        assert!(prompt.len() > CHAT_CORE_PROMPT.len() + 100);
    }

    #[test]
    fn chat_system_prompt_carries_ai_chat_quality_guardrails() {
        let state = EditorState::new();
        let prompt = build_chat_system_prompt(&state, "设计一个精致的美食应用移动端首页");
        assert!(prompt.contains("AI CHAT DESIGN QUALITY"));
        assert!(prompt.contains("Mobile top rhythm"));
        assert!(prompt.contains("favorite/heart"));
        assert!(prompt.contains("Do not repeat the same predictable mobile stack"));
        assert!(prompt.contains("signature moment"));
    }

    #[test]
    fn chat_system_prompt_includes_design_md_policy_when_present() {
        let mut state = EditorState::new();
        state.doc.design_md = Some(jian_ops_schema::DesignMdSpec {
            raw: "# Acme".into(),
            project_name: Some("Acme".into()),
            visual_theme: Some("Calm minimal twilight".into()),
            color_palette: None,
            typography: None,
            component_styles: None,
            layout_principles: None,
            generation_notes: None,
        });
        let prompt = build_chat_system_prompt(&state, "design a login form");
        assert!(
            prompt.contains("Calm minimal twilight"),
            "design.md style policy must flow into the system prompt"
        );
    }

    #[test]
    fn agent_system_prompt_carries_crud_instructions_and_context() {
        let mut state = EditorState::new();
        state.doc.children.push(op_mcp_free_rect("n1", "Hero"));
        let prompt = build_agent_system_prompt(&state);
        assert!(prompt.starts_with("You are a design editor."));
        assert!(prompt.contains("[Canvas context: Document has 1 nodes: rectangle:Hero]"));
    }

    #[test]
    fn agent_system_prompt_lists_selection_and_variables() {
        let mut state = EditorState::new();
        state.doc.children.push(op_mcp_free_rect("n1", "Hero"));
        state.selection.set = vec![op_editor_core::NodeId::new("n1")];
        state.selection.anchor = op_editor_core::NodeId::new("n1");
        let mut vars = std::collections::BTreeMap::new();
        vars.insert(
            "color-1".to_string(),
            jian_ops_schema::variable::VariableDefinition {
                kind: jian_ops_schema::variable::VariableKind::Color,
                value: jian_ops_schema::variable::VariableValue::Scalar(
                    jian_ops_schema::variable::VariableScalar::Str("#112233".into()),
                ),
            },
        );
        state.doc.variables = Some(vars);
        let prompt = build_agent_system_prompt(&state);
        assert!(prompt.contains("Selected: rectangle:Hero (120x40)"));
        assert!(prompt.contains("Variables: $color-1(color)"));
    }

    #[test]
    fn transcript_history_drops_in_flight_turn() {
        let mut messages = vec![
            ChatMessage::user("first question"),
            ChatMessage::assistant("first answer"),
            ChatMessage::user("second question"),
        ];
        messages.push(ChatMessage::assistant_streaming());
        let history = chat_history_from_transcript(&messages);
        assert_eq!(
            history,
            vec![
                (ChatHistoryRole::User, "first question".to_string()),
                (ChatHistoryRole::Assistant, "first answer".to_string()),
            ],
            "trailing streaming bubble + current user message are this turn, not history"
        );
    }

    #[test]
    fn transcript_history_skips_blank_messages() {
        let messages = vec![
            ChatMessage::user("q1"),
            ChatMessage::assistant("   "),
            ChatMessage::user("q2"),
            ChatMessage::assistant_streaming(),
        ];
        let history = chat_history_from_transcript(&messages);
        assert_eq!(history, vec![(ChatHistoryRole::User, "q1".to_string())]);
    }

    /// Build a 120×40 rectangle leaf for the context-string tests.
    fn op_mcp_free_rect(id: &str, name: &str) -> jian_ops_schema::node::PenNode {
        use jian_ops_schema::node::{ContainerProps, PenNode, PenNodeBase, RectangleNode};
        use jian_ops_schema::sizing::SizingBehavior;
        PenNode::Rectangle(RectangleNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: Some(name.to_string()),
                x: Some(0.0),
                y: Some(0.0),
                ..Default::default()
            },
            container: ContainerProps {
                width: Some(SizingBehavior::Number(120.0)),
                height: Some(SizingBehavior::Number(40.0)),
                ..Default::default()
            },
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }
}
