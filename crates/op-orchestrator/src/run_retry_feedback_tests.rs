use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};

const PARTIAL_PLAN: &str = r##"{
  "rootFrame": {
    "id": "root", "name": "Weather", "width": 375, "height": 812,
    "layout": "vertical", "gap": 0,
    "fill": [{ "type": "solid", "color": "#0F172A" }]
  },
  "subtasks": [
    { "id": "sun_arc", "label": "Sunrise & Sunset Arc",
      "region": { "width": 375, "height": 180 } },
    { "id": "summary", "label": "Weather Summary",
      "region": { "width": 375, "height": 120 } }
  ]
}"##;

fn rejected_radial_script(prefix_nodes: usize) -> String {
    let mut script =
        r#"const sec=I(null,{type:"frame",name:"Sun Section",layout:"vertical"});"#.to_string();
    for _ in 0..prefix_nodes {
        script.push_str(r#"I(sec,{type:"text",content:"marker"});"#);
    }
    script.push_str(
        r##"const ring=I(sec,{type:"frame",name:"Sun Arc",width:120,height:120});
I(ring,{type:"ellipse",name:"Ring Track",width:120,height:120,innerRadius:0.82,fill:[{type:"solid",color:"#334155"}]});
I(ring,{type:"ellipse",name:"Ring Progress",width:60,height:60,innerRadius:0.82,startAngle:-90,sweepAngle:240,fill:[{type:"solid",color:"#FACC15"}]});
const centre=I(ring,{type:"frame",name:"Ring Center",width:80,height:44});
I(centre,{type:"text",content:"8h 24m"});"##,
    );
    script
}

fn request() -> DesignRequest {
    DesignRequest {
        prompt: "weather now screen".into(),
        model: Some("gemini-3.6-flash".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

fn providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

#[test]
fn self_check_feedback_survives_attempt_three_and_salvage() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PARTIAL_PLAN.into()),
        ScriptResponse::Text(rejected_radial_script(0)),
        ScriptResponse::Text(rejected_radial_script(1)),
        ScriptResponse::Text(rejected_radial_script(2)),
        ScriptResponse::Text(
            r#"const sec=I(null,{type:"frame",name:"Weather Summary",layout:"vertical"});I(sec,{type:"text",content:"Weather overview"});"#
                .into(),
        ),
        ScriptResponse::Text(rejected_radial_script(3)),
    ]);
    let mut sink = VecDocSink::new();
    let mut progress = |_progress: Progress| {};

    let summary = futures::executor::block_on(Orchestrator::new().run(
        request(),
        &mut sink,
        &llm,
        &mut progress,
        &AbortFlag::new(),
        &providers(),
    ))
    .expect("the successful summary section keeps the partial run");

    let prompts = llm.user_prompts();
    assert_eq!(
        prompts.len(),
        6,
        "planning, three attempts, summary, and salvage"
    );
    assert!(
        !prompts[1].contains("SELF-CHECK FIX REQUIRED"),
        "attempt 1 has no prior rejection"
    );
    for (attempt, previous_id) in [(2, "at n2:"), (3, "at n3:"), (5, "at n4:")] {
        let prompt = &prompts[attempt];
        assert!(
            prompt.contains("SELF-CHECK FIX REQUIRED")
                && prompt.contains("radial-stack-not-concentric")
                && prompt.contains(previous_id),
            "attempt {attempt} must keep the immediately previous self-check feedback: {prompt}"
        );
    }

    let failed = summary
        .subtasks
        .iter()
        .find(|outcome| outcome.id == "sun_arc")
        .expect("sun arc outcome");
    let feedback = failed
        .subtask
        .as_ref()
        .and_then(|subtask| subtask.retry_feedback.as_ref())
        .expect("manual retry keeps the terminal feedback");
    assert!(
        matches!(
            feedback,
            crate::plan::RetryFeedback::SelfCheck(message)
                if message.contains("at n5:")
        ),
        "manual retry must use the salvage failure, not an earlier attempt: {feedback:?}"
    );
}
