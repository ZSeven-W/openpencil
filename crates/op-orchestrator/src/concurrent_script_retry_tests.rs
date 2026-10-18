//! The retained failure sequence must repair the protocol without losing its grammar.

use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::LlmError;
use futures::executor::block_on;
use serde_json::json;

const STATIC_SCRIPT: &str = r#"const section=I(null,{type:'frame',name:'Options',layout:'vertical',width:'fill_container',height:'fit_content',padding:20,gap:12});
I(section,{type:'text',name:'Heading',content:'Options',fontSize:20});"#;

const INTERACTIVE_SCRIPT: &str = r##"const section=I(null,{type:'frame',name:'Options',layout:'vertical',width:'fill_container',height:'fit_content',padding:20,gap:12,state:{choice:{type:'string',default:'A'}}});
I(section,{type:'text',name:'Heading',content:'Options',fontSize:20});
const button=I(section,{type:'frame',name:'Choose B',layout:'horizontal',width:180,height:44,fill:'#202020',events:{onTap:[{set:{'$app.choice':"'B'"}}]}});
I(button,{type:'text',content:'Choose B',fill:'#FFFFFF'});
I(section,{type:'text',name:'Selection',content:'Selected: A',bindings:{content:"'Selected: ' + $app.choice"}});"##;

fn fixture(interactive: bool) -> (Subtask, OrchestratorPlan, DesignRequest) {
    let task: Subtask = serde_json::from_value(json!({
        "id":"options", "label":"Options", "region":{"width":375,"height":220},
        "idPrefix":"options",
        "elements": if interactive { "Selection control: onTap changes choice, with a selected-value label and reactive state bindings." } else { "A heading reading Options." }
    }))
    .unwrap();
    let plan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"Options","width":375,"height":812},
        "sharedContext":{}, "subtasks":[task]
    }))
    .unwrap();
    let request = DesignRequest {
        prompt: if interactive {
            "An interactive selection control. onTap changes choice and updates the selected-value label."
        } else {
            "A static page heading reading Options."
        }
        .into(),
        model: Some("glm-5.3-flash".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    (task, plan, request)
}

fn run_sequence(
    interactive: bool,
    responses: Vec<ScriptResponse>,
) -> (SubtaskOutcome, ScriptedLlm, VecDocSink) {
    run_sequence_with_feedback(interactive, responses, None)
}

fn run_sequence_with_feedback(
    interactive: bool,
    responses: Vec<ScriptResponse>,
    feedback: Option<crate::plan::RetryFeedback>,
) -> (SubtaskOutcome, ScriptedLlm, VecDocSink) {
    let (mut task, plan, request) = fixture(interactive);
    task.retry_feedback = feedback;
    let llm = ScriptedLlm::new(responses);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask_retry_ladder_with_outcomes(
        &task,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        crate::model_profile::resolve_model_profile(
            request.model.as_deref().expect("fixture model"),
        )
        .tier,
        None,
        &GeometryEchoBudget::new(0),
        &mut |_| {},
        &[],
    ));
    (outcome, llm, sink)
}

fn ambiguous_response() -> ScriptResponse {
    ScriptResponse::Text(format!(
        "Option A:\n```javascript\n{STATIC_SCRIPT}\n```\nOption B:\n```javascript\n{INTERACTIVE_SCRIPT}\n```"
    ))
}

#[test]
fn expression_error_then_multiple_programs_reaches_both_retries_with_interaction_grammar() {
    // The retained product-info attempt evaluated the concatenation's scope
    // in QuickJS, then emitted two alternative programs on its second attempt.
    let broken = INTERACTIVE_SCRIPT.replace(
        "content:\"'Selected: ' + $app.choice\"",
        "content:'Selected: ' + $state.choice",
    );
    let (outcome, llm, sink) = run_sequence(
        true,
        vec![
            ScriptResponse::Text(broken),
            ambiguous_response(),
            ScriptResponse::Text(INTERACTIVE_SCRIPT.into()),
        ],
    );
    assert!(outcome.error.is_none(), "{outcome:?}");
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 3);
    assert!(prompts[1].contains("$state is not defined"));
    assert!(prompts[1].contains("Quote the ENTIRE runtime expression"));
    assert!(!prompts[1].contains("not I($state,"));
    assert!(prompts[2].contains("script response contains 2 code blocks"));
    for prompt in &prompts[1..] {
        assert!(prompt.contains("one complete JavaScript program"));
        assert!(prompt.contains("do not replace required events/state/bindings"));
        assert!(prompt.contains("onTap changes choice"));
    }
    let systems = llm.system_prompts();
    assert!(systems[2].len() < systems[0].len());
    for required in [
        "STATE — declare reactive values",
        "BINDINGS — declarative reads",
        "EVENTS — action handlers",
        "OUTPUT PROTOCOL: JAVASCRIPT PROGRAM",
    ] {
        assert!(systems[2].contains(required), "missing {required}");
    }
    let state = serde_json::to_value(sink.state.doc.state.as_ref().unwrap()).unwrap();
    assert_eq!(state["choice"]["default"], "A");
    let document = serde_json::to_string(&sink.state.doc).unwrap();
    assert!(document.contains("onTap"));
    assert!(document.contains("'Selected: ' + $app.choice"));
}

#[test]
fn static_retry_does_not_load_interactivity_or_create_state_and_events() {
    let (outcome, llm, sink) = run_sequence(
        false,
        vec![
            ScriptResponse::Text(String::new()),
            ambiguous_response(),
            ScriptResponse::Text(STATIC_SCRIPT.into()),
        ],
    );
    assert!(outcome.error.is_none(), "{outcome:?}");
    let systems = llm.system_prompts();
    assert_eq!(systems.len(), 3);
    assert!(!systems[2].contains("STATE — declare reactive values"));
    assert!(!systems[2].contains("EVENTS — action handlers"));
    assert!(systems[2].contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"));
    assert!(llm.user_prompts()[2].contains("Do not add interactions absent from the brief"));
    let document = serde_json::to_value(&sink.state.doc).unwrap();
    assert!(document.get("state").is_none_or(serde_json::Value::is_null));
    assert!(!document.to_string().contains("onTap"));
}

#[test]
fn transport_failure_never_becomes_model_diagnostic_text() {
    let (outcome, llm, _) = run_sequence(
        false,
        vec![
            ScriptResponse::Fail(LlmError {
                message: "connection closed: PRIVATE_PROVIDER_RESPONSE".into(),
                aborted: false,
            }),
            ScriptResponse::Text(STATIC_SCRIPT.into()),
        ],
    );
    assert!(outcome.error.is_none(), "{outcome:?}");
    assert_eq!(llm.user_prompts().len(), 2);
    assert!(!llm.user_prompts()[1].contains("PRIVATE_PROVIDER_RESPONSE"));
    assert!(!llm.user_prompts()[1].contains("SCRIPT/PROTOCOL FIX REQUIRED"));
}

#[test]
fn radial_quality_rejection_survives_an_empty_second_attempt() {
    // Same unrepairable radial geometry as rejected_radial_script(0) in
    // run_retry_feedback_tests; only the retry between rejection and fix differs.
    let rejected = r##"const sec=I(null,{type:"frame",name:"Sun Section",layout:"vertical"});
const ring=I(sec,{type:"frame",name:"Sun Arc",width:120,height:120});
I(ring,{type:"ellipse",name:"Ring Track",width:120,height:120,innerRadius:0.82,fill:[{type:"solid",color:"#334155"}]});
I(ring,{type:"ellipse",name:"Ring Progress",width:60,height:60,innerRadius:0.82,startAngle:-90,sweepAngle:240,fill:[{type:"solid",color:"#FACC15"}]});
const centre=I(ring,{type:"frame",name:"Ring Center",width:80,height:44});
I(centre,{type:"text",content:"8h 24m"});"##;
    let corrected = rejected.replace("width:60,height:60", "width:120,height:120");
    let (outcome, llm, _) = run_sequence(
        false,
        vec![
            ScriptResponse::Text(rejected.into()),
            ScriptResponse::Text(String::new()),
            ScriptResponse::Text(corrected),
        ],
    );
    assert!(outcome.error.is_none(), "{outcome:?}");
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 3);
    for prompt in &prompts[1..] {
        assert!(prompt.contains("SELF-CHECK FIX REQUIRED"));
        assert!(prompt.contains("radial-stack-not-concentric"));
        assert!(prompt.contains("at n2:"));
    }
    assert!(prompts[2].contains("script is empty after stripping fences"));
    assert_eq!(
        prompts[2].matches("SCRIPT/PROTOCOL FIX REQUIRED:").count(),
        1
    );
}

#[test]
fn externally_supplied_language_feedback_survives_repeated_empty_scripts() {
    let reason = "Preserve the requested copy language.";
    let (outcome, llm, _) = run_sequence_with_feedback(
        false,
        vec![
            ScriptResponse::Text(String::new()),
            ScriptResponse::Text(String::new()),
            ScriptResponse::Text(STATIC_SCRIPT.into()),
        ],
        Some(crate::plan::RetryFeedback::Language(reason.into())),
    );
    assert!(outcome.error.is_none(), "{outcome:?}");
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 3);
    for prompt in &prompts {
        assert!(prompt.contains("LANGUAGE FIX REQUIRED"));
        assert!(prompt.contains(reason));
    }
    for prompt in &prompts[1..] {
        assert!(prompt.contains("script is empty after stripping fences"));
        assert_eq!(prompt.matches("SCRIPT/PROTOCOL FIX REQUIRED:").count(), 1);
    }
}

#[test]
fn minimal_interaction_prompt_fits_existing_budgets_and_keeps_script_protocol() {
    for (model, tier) in [
        ("glm-4-plus", ModelTier::Basic),
        ("gpt-4o", ModelTier::Standard),
        ("glm-5.3-flash", ModelTier::Full),
    ] {
        assert_eq!(
            crate::model_profile::resolve_model_profile(model).tier,
            tier
        );
        for width in [375.0, 1440.0] {
            let (mut task, mut plan, mut request) = fixture(true);
            task.region.width = width;
            plan.root_frame.width = width;
            request.model = Some(model.into());
            let (call, report) = crate::prompt::build_subagent_prompt(
                &task,
                &plan,
                &request,
                AbortFlag::new(),
                true,
                true,
                &op_editor_core::ComponentLibrary::default(),
            );
            assert!(report.budget_used <= report.budget_max);
            assert!(report.budget_used <= 5_000, "{model}: {report:?}");
            assert_eq!(report.included.len(), 2, "{model}: {report:?}");
            assert!(report.included.iter().any(|s| s.name == "schema"));
            assert!(report.included.iter().any(|s| s.name == "interactivity"));
            assert!(call.system_prompt.contains("EVENTS — action handlers"));
            assert!(call
                .system_prompt
                .contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"));
            assert!(!call.system_prompt.contains("Output ONLY the JSON lines."));
        }
    }
}

#[test]
fn retained_builtin_reference_failures_reach_retry_without_losing_interactions_or_inserting_prefixes(
) {
    // The retained first attempts and salvage failed before a program existed.
    // This only verifies feedback delivery and a controlled valid response;
    // it does not claim the retained full design passes downstream QA.
    let corrected = INTERACTIVE_SCRIPT.replace(
        "'Selected: ' + $app.choice",
        "'Selected: ' + to_str($app.choice)",
    );
    for broken in [
        include_str!("../../op-mcp/tests/fixtures/script_runner/product-spec-unquoted-1018.js"),
        include_str!(
            "../../op-mcp/tests/fixtures/script_runner/product-spec-argument-quoted-1018.js"
        ),
        include_str!(
            "../../op-mcp/tests/fixtures/script_runner/product-spec-actions-batch-unquoted-1018.js"
        ),
    ] {
        // The action gate cannot conceal the original runner failure or turn
        // a recorded prefix into a successful section.
        let error = crate::script_gen::parse_script(broken).unwrap_err();
        assert!(
            matches!(&error, crate::script_gen::ScriptGenError::ScriptRun(message)
            if message.contains("registered Jian Preview builtin"))
        );
        let (outcome, llm, sink) = run_sequence(
            true,
            vec![
                ScriptResponse::Text(broken.into()),
                ScriptResponse::Text(corrected.clone()),
            ],
        );
        assert!(outcome.error.is_none(), "{outcome:?}");
        let prompts = llm.user_prompts();
        assert_eq!(prompts.len(), 2);
        assert!(prompts[1].contains("registered Jian Preview builtin"));
        assert!(prompts[1].contains("quote the ENTIRE expression"));
        assert!(prompts[1].contains("do not replace required events/state/bindings"));
        assert!(prompts[1].contains("onTap changes choice"));
        assert!(!prompts[1].contains("not I(to_str,"));
        let state = serde_json::to_value(sink.state.doc.state.as_ref().unwrap()).unwrap();
        assert_eq!(state["choice"]["default"], "A");
        assert!(state.get("qty").is_none());
        let document = serde_json::to_string(&sink.state.doc).unwrap();
        assert!(document.contains("onTap"));
        assert!(document.contains("'Selected: ' + to_str($app.choice)"));
        assert!(!document.contains("qty-stepper"));
    }
}
