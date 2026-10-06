use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, DesignRequest, GeometryEchoBudget, Progress};
use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

fn text_node(id: &str, content: Value) -> PenNode {
    serde_json::from_value(json!({
        "type": "text",
        "id": id,
        "content": content,
    }))
    .expect("synthetic text node")
}

fn frame_node(id: &str, role: Option<&str>, children: Vec<PenNode>) -> PenNode {
    let mut value = json!({
        "type": "frame",
        "id": id,
        "children": children,
    });
    if let Some(role) = role {
        value["role"] = json!(role);
    }
    serde_json::from_value(value).expect("synthetic frame node")
}

fn refs(nodes: &[PenNode]) -> Vec<&PenNode> {
    nodes.iter().collect()
}

#[test]
fn brief_language_classifies_cjk_latin_and_mixed() {
    assert_eq!(brief_language("短视频 App 首页"), Some(Lang::Cjk));
    assert_eq!(
        brief_language("Design a short-video app home screen"),
        Some(Lang::Latin)
    );
    assert_eq!(
        brief_language("做一个 English learning App 首页"),
        Some(Lang::Cjk),
        "CJK ratio wins when a Chinese brief embeds English product words"
    );
    // Both scripts present, neither threshold met (CJK < 30% of letters and
    // not Latin-exclusive). Word-level mix looks ~50/50 even though Latin
    // letters outnumber CJK characters.
    assert_eq!(
        brief_language(
            "Please design a modern mobile home screen with search and profile tabs 短视频首页"
        ),
        None
    );
}

#[test]
fn copy_language_mismatch_app14_english_copy_on_cjk_brief() {
    let nodes = [
        text_node("t1", json!("Follow")),
        text_node("t2", json!("For You")),
        text_node("t3", json!("Live")),
        text_node("t4", json!("HOME")),
        text_node("t5", json!("DISCOVER")),
        text_node("t6", json!("INBOX")),
        text_node("t7", json!("PROFILE")),
        text_node("t8", json!("128.4K")),
        text_node("t9", json!("3,201")),
    ];
    let report = copy_language_mismatch(&refs(&nodes), Lang::Cjk).expect("mismatch");
    assert_eq!(report.checked, 7, "numbers must be skipped");
    assert_eq!(report.mismatched, 7);
    assert!(report.samples.iter().any(|sample| sample == "Follow"));
}

#[test]
fn copy_language_mismatch_allows_acronyms_in_cjk_copy() {
    let nodes = [
        text_node("t1", json!("关注好友")),
        text_node("t2", json!("为你推荐")),
        text_node("t3", json!("正在直播")),
        text_node("t4", json!("消息通知")),
        text_node("t5", json!("AI 助手")),
        text_node("t6", json!("VIP")),
    ];
    assert_eq!(copy_language_mismatch(&refs(&nodes), Lang::Cjk), None);
}

#[test]
fn copy_language_mismatch_ignores_subtrees_below_checked_floor() {
    let nodes = [
        text_node("t1", json!("Follow")),
        text_node("t2", json!("For You")),
        text_node("t3", json!("Live")),
    ];
    assert_eq!(copy_language_mismatch(&refs(&nodes), Lang::Cjk), None);
}

#[test]
fn copy_language_mismatch_skips_status_bar_subtrees() {
    let status = frame_node(
        "status",
        Some("status-bar"),
        vec![
            text_node("s1", json!("Follow")),
            text_node("s2", json!("For You")),
            text_node("s3", json!("Live")),
            text_node("s4", json!("HOME")),
            text_node("s5", json!("DISCOVER")),
        ],
    );
    let feed = frame_node(
        "feed",
        None,
        vec![
            text_node("t1", json!("关注好友")),
            text_node("t2", json!("为你推荐")),
            text_node("t3", json!("正在直播")),
            text_node("t4", json!("消息通知")),
        ],
    );
    let nodes = [status, feed];
    assert_eq!(copy_language_mismatch(&refs(&nodes), Lang::Cjk), None);
}

#[test]
fn language_feedback_names_chinese_brief_and_english_samples() {
    let report = MismatchReport {
        checked: 7,
        mismatched: 7,
        samples: vec!["Follow".into(), "For You".into()],
    };
    let feedback = language_feedback(Lang::Cjk, &report);
    assert_eq!(
        feedback,
        "The brief is written in Chinese, but 7 of 7 text nodes are in English (e.g. \"Follow\", \"For You\"). Rewrite all user-facing copy in Chinese; keep brand names and acronyms as they are."
    );
}

#[test]
fn expected_output_language_gates_off_when_brief_asks_for_english() {
    let request = DesignRequest {
        prompt: "做一个短视频首页，文案用英文界面".into(),
        ..DesignRequest::default()
    };
    assert_eq!(expected_output_language(&request), None);
}

#[test]
fn expected_output_language_keeps_english_learning_app_as_cjk() {
    let request = DesignRequest {
        prompt: "做一个 English learning App 首页".into(),
        ..DesignRequest::default()
    };
    assert_eq!(expected_output_language(&request), Some(Lang::Cjk));
}

fn subtask() -> Subtask {
    Subtask {
        id: "home".into(),
        label: "首页内容".into(),
        region: Region {
            width: 375.0,
            height: 400.0,
        },
        bleed_hero: false,
        id_prefix: "home".into(),
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn plan(task: &Subtask) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![task.clone()],
        style_guide_name: None,
    }
}

fn request(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        validation_enabled: false,
        ..DesignRequest::default()
    }
}

const ENGLISH_COPY: &str = r#"I(null,{"type":"frame","name":"Feed","children":[{"type":"text","content":"Follow"},{"type":"text","content":"For You"},{"type":"text","content":"Live"},{"type":"text","content":"HOME"},{"type":"text","content":"DISCOVER"},{"type":"text","content":"INBOX"},{"type":"text","content":"PROFILE"},{"type":"text","content":"128.4K"},{"type":"text","content":"3,201"}]});"#;

const CHINESE_COPY: &str = r#"I(null,{"type":"frame","name":"Feed","children":[{"type":"text","content":"关注好友"},{"type":"text","content":"为你推荐"},{"type":"text","content":"正在直播"},{"type":"text","content":"发现页"},{"type":"text","content":"消息通知"},{"type":"text","content":"个人主页"},{"type":"text","content":"附近的人"}]});"#;

fn run_ladder(
    prompt: &str,
    responses: Vec<&str>,
) -> (
    crate::types::SubtaskOutcome,
    Vec<Progress>,
    Vec<String>,
    Vec<PenNode>,
) {
    let task = subtask();
    let plan = plan(&task);
    let llm = ScriptedLlm::new(
        responses
            .into_iter()
            .map(|response| ScriptResponse::Text(response.into()))
            .collect(),
    );
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |progress| events.push(progress);
    let outcome =
        futures::executor::block_on(crate::concurrent::run_subtask_retry_ladder_with_outcomes(
            &task,
            &plan,
            &request(prompt),
            &llm,
            &mut sink,
            &AbortFlag::new(),
            crate::model_profile::ModelTier::Full,
            None,
            &GeometryEchoBudget::new(0),
            &mut on_progress,
            &[],
        ));
    (
        outcome,
        events,
        llm.user_prompts(),
        sink.state.active_children().to_vec(),
    )
}

#[test]
fn english_copy_on_cjk_brief_is_rolled_back_and_retried_once() {
    let (outcome, events, prompts, roots) =
        run_ladder("短视频 App 首页", vec![ENGLISH_COPY, CHINESE_COPY]);
    assert!(outcome.node_count > 0);
    assert_eq!(roots.len(), 1, "the English first subtree must be removed");
    let expected_feedback = language_feedback(
        Lang::Cjk,
        &MismatchReport {
            checked: 7,
            mismatched: 7,
            samples: vec!["Follow".into(), "For You".into()],
        },
    );
    assert!(events.iter().any(|event| matches!(event,
        Progress::SubtaskRetry { attempt: 2, reason, .. } if reason == &expected_feedback
    )));
    assert!(prompts[1].contains(&expected_feedback));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskLanguageMismatch { .. })));
    let report = copy_language_mismatch(&refs(&roots), Lang::Cjk);
    assert_eq!(report, None, "retry must land Chinese copy");
}

#[test]
fn latin_brief_with_english_copy_does_not_retry() {
    let (outcome, events, prompts, roots) =
        run_ladder("Design a short-video app home screen", vec![ENGLISH_COPY]);
    assert!(outcome.node_count > 0);
    assert_eq!(roots.len(), 1);
    assert_eq!(
        prompts.len(),
        1,
        "Latin brief must not trigger a language retry"
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskRetry { .. })));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskLanguageMismatch { .. })));
}

#[test]
fn second_language_mismatch_is_kept_and_emitted_once() {
    let (outcome, events, prompts, roots) =
        run_ladder("短视频 App 首页", vec![ENGLISH_COPY, ENGLISH_COPY]);
    assert!(outcome.node_count > 0);
    assert_eq!(
        roots.len(),
        1,
        "only the last mismatched result is retained"
    );
    assert_eq!(prompts.len(), 2, "language gate retries once, never loops");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Progress::SubtaskLanguageMismatch { .. }))
            .count(),
        1
    );
    assert!(matches!(
        events
            .iter()
            .find(|event| matches!(event, Progress::SubtaskLanguageMismatch { .. })),
        Some(Progress::SubtaskLanguageMismatch {
            id,
            checked: 7,
            mismatched: 7
        }) if id == "home"
    ));
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::SubtaskRetry { attempt: 3, .. })));
}
