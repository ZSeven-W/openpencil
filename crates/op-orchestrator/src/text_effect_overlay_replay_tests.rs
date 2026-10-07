use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use crate::types::{AbortFlag, DesignRequest, Progress, ValidationProviders};

#[test]
#[ignore = "requires explicit retained GLM response directory; runs no provider calls"]
fn replay_retained_glm_overprint_through_the_complete_generation_pipeline() {
    let root = std::path::PathBuf::from(std::env::var("OPENPENCIL_OVERPRINT_REPLAY_DIR").unwrap());
    let fonts =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/shared/fonts");
    jian_skia::register_bundled_fonts(
        [
            "Archivo-VF.ttf",
            "LibreCaslonText-VF.ttf",
            "NotoSerifSC-VF.ttf",
        ]
        .iter()
        .map(|f| std::fs::read(fonts.join(f)).unwrap())
        .collect(),
    );
    let responses = (0..3)
        .filter(|i| root.join(format!("model-response-{i}.txt")).exists())
        .map(|i| {
            ScriptResponse::Text(
                std::fs::read_to_string(root.join(format!("model-response-{i}.txt"))).unwrap(),
            )
        })
        .collect();
    let llm = ScriptedLlm::new(responses);
    let (mut parsed, _) = crate::script_gen::parse_script(
        &std::fs::read_to_string(root.join("model-response-1.txt")).unwrap(),
    )
    .unwrap();
    println!("PARSED");
    title_layers(&parsed);
    crate::role_infer::resolve_forest_roles(
        &mut parsed,
        1080.0,
        crate::role_defaults::Theme::Light,
    );
    crate::text_effect_overlay::normalize_generated_order(&mut parsed);
    println!("ROLES");
    title_layers(&parsed);
    crate::role_post_pass::post_pass_forest(&mut parsed, 1080.0);
    println!("POSTPASS");
    title_layers(&parsed);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let req = DesignRequest {
        prompt: std::fs::read_to_string(root.join("input.txt")).unwrap(),
        model: Some("glm-5.3-flash".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: Some("leadprint-vermilion-light".into()),
        reference_skeleton: None,
    };
    let providers = ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    };
    let result = futures::executor::block_on(crate::run::Orchestrator::new().run(
        req,
        &mut sink,
        &llm,
        &mut |e| events.push(e),
        &AbortFlag::new(),
        &providers,
    ))
    .unwrap();
    assert!(result.subtasks.iter().all(|s| s.error.is_none()));
    fn title_layers(nodes: &[jian_ops_schema::node::PenNode]) {
        use op_editor_core::PenNodeExt;
        for node in nodes {
            if let jian_ops_schema::node::PenNode::Text(text) = node {
                if text.content == jian_ops_schema::node::TextContent::Plain("咖啡小聚".into())
                {
                    println!("INSERTED_TITLE {}", serde_json::to_string(node).unwrap());
                }
            }
            if let Some(kids) = node.children() {
                title_layers(kids);
            }
        }
    }
    for command in &sink.applied {
        if let op_editor_core::EditorCommand::InsertSubtree { nodes, .. } = command {
            title_layers(nodes);
        }
        if let op_editor_core::EditorCommand::PatchNodeData { patch_json, .. } = command {
            if patch_json.contains("fill") {
                println!("FILL_PATCH {command:?}");
            }
        }
    }
    let issues = crate::geometry_validation::geometry_diagnostics(&sink.state);
    assert!(
        !issues.iter().any(|i| i.contains("TEXT leaves")),
        "{issues:?}"
    );
    let out = root.join("cached-fixed.op");
    std::fs::write(&out, serde_json::to_string_pretty(&sink.state.doc).unwrap()).unwrap();
    println!(
        "REPLAY calls={} geometry_echoes={} diagnostics={issues:?} output={}",
        llm.user_prompts().len(),
        events
            .iter()
            .filter(|e| matches!(e, Progress::GeometryEcho { .. }))
            .count(),
        out.display()
    );
}
