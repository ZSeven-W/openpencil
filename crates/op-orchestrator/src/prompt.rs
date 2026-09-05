//! Prompt 构造 —— 规划 prompt 与 sub-agent prompt。
//!
//! skill 正文经 `op_ai_skills::resolve_skills` 解析:规划用
//! `Phase::Planning`,sub-agent 用 `Phase::Generation`。两段格式
//! 指令(产 OrchestratorPlan JSON / 产 PenNode JSON 数组)由本
//! 模块附加在 skill 正文之后。
//!
//! 注:格式指令文本是功能性的最小版;逐条对齐 TS
//! `orchestrator-prompt-optimizer.ts` / `orchestrator-sub-agent.ts`
//! 的细则(style-guide 注入、移动端禁 phone-wrapper 等)是后续
//! 细化项。

use crate::compact_prompt::build_compact_planning_prompt;
use crate::compact_skills::{apply_skill_filter, SkillNamed};
use crate::design_md_policy::build_design_md_style_policy;
use crate::design_type::{detect_design_type, DesignType};
use crate::model_profile::{resolve_model_profile, ModelTier};
use crate::plan::{OrchestratorPlan, Subtask};
use crate::resolved_style_prompt::build_resolved_style_instruction_for_plan;
use crate::style_guide_context::build_planning_style_guide_context;
use crate::timeouts::{
    apply_profile_to_timeouts, builtin_planning_timeouts, orchestrator_timeouts, sub_agent_timeouts,
};
use crate::types::{AbortFlag, CallRequest, DesignRequest, PlanningMode, PlanningPrompt};
use op_ai_skills::resolve_style::{resolve_style, ResolveOutcome};
use op_ai_skills::style_guide::extract_style_guide_values;
use op_ai_skills::{
    budget::trim_by_budget_pinned,
    get_skills_by_phase,
    resolver::{filter_by_intent, inject_dynamic_content},
    DropReason, DroppedSkill, Phase, ResolveOptions, ResolvedSkill, SkillLoadEntry,
    SkillLoadReport,
};
use op_editor_core::ComponentLibrary;
use std::collections::HashMap;

// Cluster submodules: this file keeps the shared format constants and the
// planning entry points; the token / skill / component / sub-agent builders
// live in their own files and are re-exported here so every existing
// `crate::prompt::…` path (and the test module mounted below) still resolves.
#[path = "prompt_components.rs"]
mod prompt_components;
#[path = "prompt_design_tokens.rs"]
mod prompt_design_tokens;
#[path = "prompt_style_skills.rs"]
mod prompt_style_skills;
#[path = "prompt_subagent.rs"]
mod prompt_subagent;

use prompt_components::*;
use prompt_design_tokens::*;
pub use prompt_style_skills::*;
pub use prompt_subagent::*;

/// sub-agent 阶段要求模型产出的 JSON 形状说明。
const NODE_FORMAT: &str = r#"
Respond with THIS section's canonical PenNode objects in the FLAT _parent format:
output ONE JSON object per line (NO enclosing [ ] array), each tagged by "type"
(frame/group/rectangle/ellipse/line/polygon/path/text/text_input/text_area/select/
switch/checkbox/slider/radio_group/number_input/progress/tabs/image/icon_font)
and carrying "_parent" — null for the section root, else the id of its parent
node (which MUST appear on an earlier line).
EVERY non-root node MUST set "_parent". Do NOT emit a flat list of siblings with
no _parent links, and do NOT rely on a "children" array — a flat list renders
BROKEN: a horizontal row whose items are not _parent-linked to it collapses into
a vertical stack. Express the WHOLE tree through _parent (row -> its cards -> each
card's texts/icons).
Interactive controls MUST be first-class nodes. Emit text_input/text_area with value;
select/radio_group with options:[{value,label}] and value; switch/checkbox with checked;
slider/number_input with min/max/step/value; progress with max/value; tabs with
tabs:[{value,label}] and value. Never generate a frame/rectangle mockup with a role marker.
Every interactive node MUST explicitly carry design-system fill, stroke, and cornerRadius.
fill is the active/accent paint (or field surface).
stroke.fill is the inactive track/border paint. Do not rely on renderer defaults.
Example (a horizontal row of two cards inside a section):
{"_parent":null,"id":"<prefix>-root","type":"frame","name":"Section","width":"fill_container","height":"fit_content","layout":"vertical","gap":16}
{"_parent":"<prefix>-root","id":"<prefix>-row","type":"frame","name":"Row","width":"fill_container","height":"fit_content","layout":"horizontal","gap":16}
{"_parent":"<prefix>-row","id":"<prefix>-card1","type":"frame","name":"Card","width":"fill_container","height":"fit_content","layout":"vertical","cornerRadius":12}
{"_parent":"<prefix>-card1","id":"<prefix>-card1-title","type":"text","name":"Title","content":"Revenue","fontSize":14}
{"_parent":"<prefix>-row","id":"<prefix>-card2","type":"frame","name":"Card","width":"fill_container","height":"fit_content","layout":"vertical","cornerRadius":12}
ALL field names are camelCase: cornerRadius, fontSize, fontWeight, justifyContent,
alignItems, clipContent. Geometry fields are x, y, width, height. Never snake_case.
Output ONLY the JSON lines."#;

/// Script-gen 模式的输出协议——OUTPUT PROTOCOL matches Pencil's `batch_design`
/// (a sandboxed JS script DSL). Honesty note (2026-07-04 audit): this aligns
/// the PROTOCOL only; it runs inside the ORCHESTRATOR single-shot path (the
/// fallback for CLI-agent providers), which has NO per-batch model feedback.
/// Pencil's defining feedback loop lives in the sonar design-agent loop
/// (`chat_agent_loop` + `design_agent_tools`), the builtin-provider default —
/// not here.
/// 模型写一段真 JavaScript（循环/数组/变量）调用全局 `I(parent, obj)`。引擎
/// (rquickjs) 执行、`JSON.stringify` 序列化每个对象（=完美 JSON,无手写括号/引号
/// 笔误),循环展开重复结构。`I` 返回新节点 id 字符串。
const SCRIPT_FORMAT: &str = r#"
OUTPUT PROTOCOL: JAVASCRIPT PROGRAM. Write a JavaScript program (no prose, no markdown
fences) that builds this section by calling the global function I(parent, node):
  const id = I(parent, { ...node... });   // inserts node, RETURNS its id (a string)
`parent` is null for THIS section's single root frame, otherwise an id returned by an
EARLIER I(...) call. A node is a child of X only if you call I(X, {...}).
I(...) is the ONLY function available — there is no console, and no other builder. Do
not call console.log or any helper; just call I(...).
USE REAL JAVASCRIPT — const/let, arrays of data, and for...of / .forEach loops — to
generate repeated structure (table rows, nav items, cards, list items) by looping over a
data array. PREFER a loop over copy-pasting near-identical I(...) calls.
Each node object starts with type ("frame"/"text"/"rectangle"/"ellipse"/"path"/
"icon_font"/"text_input"/"text_area"/"select"/"switch"/"checkbox"/"slider"/
"radio_group"/"number_input"/"progress"/"tabs") and uses camelCase props
(cornerRadius, fontSize, fontWeight, justifyContent, alignItems, clipContent). Do NOT set
x/y on children inside layout frames.
INTERACTIVE CONTROLS are native nodes: emit the first-class type directly with I(...),
never a frame/rectangle mockup with a role marker. text_input/text_area require value;
select/radio_group require options:[{value,label}] plus value; switch/checkbox require
checked; slider/number_input require min/max/step/value; progress requires max/value;
tabs requires tabs:[{value,label}] plus value. Every native control MUST explicitly carry
the design system's fill, stroke, and cornerRadius. fill is the active/accent paint (or
field surface); stroke.fill is the inactive track/border paint. Do not rely on defaults.
Each script runs in a FRESH sandbox: variables from an EARLIER batch do not exist. To attach
to a node an earlier batch created, pass its id STRING — I("n12", {...}) — never a `const` from
that batch. Ids come back in the batch result.
EVERY frame with children MUST declare layout ("vertical" or "horizontal"; "none" for an
absolute stack). A section that holds a title and a card rail is layout:"vertical". Omitting
layout is ambiguous and the current engine may place flow children in a row, so always choose it
explicitly.
Example:
  const sec = I(null, {type:"frame", name:"Clients", layout:"vertical", width:"fill_container", gap:0});
  const tbl = I(sec, {type:"frame", layout:"vertical", width:"fill_container"});
  const rows = [{name:"Alice Chen", status:"Active"}, {name:"Bob Ito", status:"VIP"}];
  for (const r of rows) {
    const row = I(tbl, {type:"frame", layout:"horizontal", width:"fill_container", padding:[12,16]});
    const c1 = I(row, {type:"frame", width:"fill_container"}); I(c1, {type:"text", content:r.name});
    const c2 = I(row, {type:"frame", width:"fill_container"}); I(c2, {type:"text", content:r.status});
  }
Generate EVERY row/card/item with realistic values. Output ONLY the JavaScript program."#;

/// Rich 模式 system prompt 末尾后缀 —— verbatim,`orchestrator.ts:1382-1383`。
const RICH_SUFFIX: &str = "\n\n---\nCRITICAL OUTPUT FORMAT ENFORCEMENT:\n\
You MUST output ONLY a single JSON object. Start your response with { and end with }.\n\
Do NOT output any text, explanation, analysis, markdown, or tool calls before or after the JSON.\n\
Do NOT \"explore\" or \"think out loud\". Do NOT use <tool_call> or function calls.\n\
Any pre-design analysis (concept extraction, superfan simulation, etc.) must happen internally — include results as JSON fields, never as prose.\n\
Violating this format will cause a system error.";

/// Minimal 模式后缀 —— verbatim,`orchestrator.ts:1384`。
const MINIMAL_SUFFIX: &str =
    "\n\nOUTPUT ONLY ONE JSON OBJECT. No prose. No markdown. No tool calls.";

const PLANNING_QUALITY_GUARDRAILS: &str = r#"

PLANNING QUALITY GUARDRAILS:
- Do not plan the same predictable mobile stack of search + categories + orange promo + two cards unless the request explicitly asks for that exact convention.
- Mobile top rhythm: planned header/title/search/primary-content sections should be compact; avoid allocating a huge empty band between the title and first useful module.
- Plan one signature moment in the first viewport: a crafted hero/product composition, editorial crop, distinctive category rail, refined data module, or other domain-specific focal idea.
- Bottom navigation is optional. If planned, it should be integrated with the page flow, not a detached floating pill, nested rounded capsule, or extra footer band.
- Product-card favorite/heart controls must be inside their card/image; never plan them as protruding decorative badges."#;

fn planning_suffix(mode: PlanningMode) -> &'static str {
    match mode {
        PlanningMode::Rich => RICH_SUFFIX,
        PlanningMode::Minimal => MINIMAL_SUFFIX,
        PlanningMode::Compact => "",
    }
}

/// 丢掉 `landing-page-predesign` skill(除非设计类型是 landing-page)。
/// 见 spec §5.10。
fn filter_planning_skills_for_prompt(
    skills: Vec<op_ai_skills::ResolvedSkill>,
    prompt: &str,
) -> Vec<op_ai_skills::ResolvedSkill> {
    // Each pre-design step belongs to exactly one design type: the keyword
    // triggers are deliberately loose ("app", "homepage") so the detector,
    // not the keyword, has the final say.
    let design_type = detect_design_type(prompt).type_;
    skills
        .into_iter()
        .filter(|s| match s.meta.name.as_str() {
            "landing-page-predesign" => design_type == DesignType::LandingPage,
            "mobile-app-predesign" => design_type == DesignType::MobileScreen,
            _ => true,
        })
        .collect()
}

/// 规划阶段的 LLM 调用输入。`mode` 决定 prompt 构造方式;返回
/// `PlanningPrompt`(带 compact 的 forced style-guide 名,供 S3b-1b)。
///
/// 超时来源(对齐 TS `callOrchestrator` 中的 `timeouts` 分支):
/// - `Compact` 模式对应 TS `fastTimeout=true`(仅 builtin / Basic 兜底路径),
///   使用 `builtin_planning_timeouts` —— 较短,让规划快速失败。
/// - `Rich` / `Minimal` 使用 `orchestrator_timeouts(prompt_len)`,按
///   prompt 长度分桶后再乘以模型 tier 的 `timeout_multiplier`。
pub fn build_orchestrator_prompt(
    req: &DesignRequest,
    mode: PlanningMode,
    abort: AbortFlag,
) -> PlanningPrompt {
    let model_id = req.model.as_deref().unwrap_or("");
    let profile = resolve_model_profile(model_id);
    let multiplier = profile.timeout_multiplier;

    match mode {
        PlanningMode::Compact => {
            // TS fastTimeout=true path: getBuiltinPlanningTimeouts(model)
            let t = apply_profile_to_timeouts(builtin_planning_timeouts(profile.tier), multiplier);
            let cp = build_compact_planning_prompt(
                &req.prompt,
                req.design_md.as_ref(),
                req.pinned_style_guide.as_deref(),
            );
            let mut user_prompt = cp.user_prompt;
            append_reference_skeleton(&mut user_prompt, req);
            PlanningPrompt {
                call_request: CallRequest {
                    system_prompt: cp.system,
                    user_prompt,
                    model: req.model.clone(),
                    provider: req.provider.clone(),
                    timeout: t.hard,
                    abort,
                    no_text_timeout: Some(t.no_text),
                    first_text_timeout: Some(t.first_text),
                },
                forced_style_guide_name: Some(cp.selected_style_guide_name),
                mode,
            }
        }
        PlanningMode::Rich | PlanningMode::Minimal => {
            // TS fastTimeout=false path: getOrchestratorTimeouts(originalLength, model)
            let t = apply_profile_to_timeouts(orchestrator_timeouts(req.prompt.len()), multiplier);
            let ctx = build_planning_style_guide_context(
                &req.prompt,
                req.model.as_deref(),
                mode,
                req.design_md.as_ref(),
                req.pinned_style_guide.as_deref(),
            );
            let opts = op_ai_skills::ResolveOptions {
                dynamic_content: HashMap::from([(
                    "availableStyleGuides".to_string(),
                    ctx.available_style_guides,
                )]),
                ..Default::default()
            };
            let agent_ctx =
                op_ai_skills::resolve_skills(op_ai_skills::Phase::Planning, &req.prompt, &opts);
            let skills = filter_planning_skills_for_prompt(agent_ctx.skills, &req.prompt);
            let mut system_prompt = skills
                .iter()
                .map(|s| s.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            system_prompt.push_str(PLANNING_QUALITY_GUARDRAILS);
            system_prompt.push_str(planning_suffix(mode));
            let mut user_prompt = req.prompt.clone();
            append_reference_skeleton(&mut user_prompt, req);
            PlanningPrompt {
                call_request: CallRequest {
                    system_prompt,
                    user_prompt,
                    model: req.model.clone(),
                    provider: req.provider.clone(),
                    timeout: t.hard,
                    abort,
                    no_text_timeout: Some(t.no_text),
                    first_text_timeout: Some(t.first_text),
                },
                forced_style_guide_name: None,
                mode,
            }
        }
    }
}

fn append_reference_skeleton(user_prompt: &mut String, req: &DesignRequest) {
    if let Some(skeleton) = req.reference_skeleton.as_ref() {
        user_prompt.push_str("\n\n");
        user_prompt.push_str(&skeleton.render());
    }
}

/// Compose the intent string for per-subtask skill matching. Component 1:
/// keyword triggers must see the ORIGINAL request, not just the short subtask
/// label, or domain/knowledge skills (mobile-app, cjk-typography, food cues)
/// never fire. Order: original prompt, then label, then screen/element hints.
pub(crate) fn subtask_intent(req: &DesignRequest, subtask: &Subtask) -> String {
    let mut s = String::new();
    s.push_str(&req.prompt);
    s.push('\n');
    s.push_str(&subtask.label);
    if let Some(screen) = subtask.screen.as_ref() {
        s.push_str("\nscreen: ");
        s.push_str(screen);
    }
    if let Some(elements) = subtask.elements.as_ref() {
        s.push('\n');
        s.push_str(elements);
    }
    s
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "prompt_reference_tests.rs"]
mod reference_skeleton_tests;
