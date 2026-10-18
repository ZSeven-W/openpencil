//! Read-only, opt-in initial-state and authored-click checks on a saved .op.
//! OPENPENCIL_QA_INITIAL_STATE_CONFIG names a JSON file with document, output,
//! root, initial {app, texts, paints}, optional clicks [{node_id, expect}], and
//! button_labels [{button, label}]. Expectations may also specify root.
//! Paint expectations are {fill: "#rgba", stroke: "#rrggbb"}; omitted fields
//! are not checked. All requested visual nodes must be mounted and visible.
//! Optional initial_scroll / clicks[].before_scroll are {node_id, dy}: actual
//! wheel input, never a state write or scene edit. Initial app values are also
//! checked BEFORE this scroll. OUTPUT must be new. Failed checks retain live
//! PNGs and a receipt. This is native engine evidence, not OS/device evidence.

use super::*;
use op_editor_ui::layout_scene::{NodeKind, SceneFillLayer, SceneFillType};
use op_editor_ui::{Color, ImageBlendMode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InitialCase {
    document: PathBuf,
    output: PathBuf,
    root: String,
    initial: Expected,
    #[serde(default)]
    initial_scroll: Option<Wheel>,
    #[serde(default)]
    clicks: Vec<Click>,
    #[serde(default)]
    button_labels: Vec<ButtonLabel>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    app: BTreeMap<String, Value>,
    #[serde(default)]
    texts: BTreeMap<String, String>,
    #[serde(default)]
    paints: BTreeMap<String, Paint>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Paint {
    #[serde(default)]
    fill: Option<String>,
    #[serde(default)]
    stroke: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Click {
    node_id: String,
    expect: Expected,
    #[serde(default)]
    before_scroll: Option<Wheel>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Wheel {
    node_id: String,
    dy: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ButtonLabel {
    button: String,
    label: String,
}

fn hex(value: &str) -> Option<Color> {
    // Guard UTF-8 before the production parser's byte slicing.
    if !value.is_ascii() {
        return None;
    }
    let c = jian_core::scene::Color::from_hex(value)?;
    Some(Color::rgba_u8(c.r(), c.g(), c.b(), c.a() as f32 / 255.0))
}

fn rgba(c: Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

fn same_color(actual: Option<Color>, expected: &str) -> bool {
    actual.zip(hex(expected)).is_some_and(|(a, e)| {
        rgba(a)
            .into_iter()
            .zip(rgba(e))
            .all(|(a, e)| a.is_finite() && (a - e).abs() <= 0.5 / 255.0)
    })
}

fn solid_fill(node: &SceneNode) -> Option<Color> {
    if node.fill_type != SceneFillType::Solid || node.blend_mode != ImageBlendMode::Normal {
        return None;
    }
    match node.fill_layers.as_slice() {
        [] => node.fill,
        [SceneFillLayer::Solid {
            color,
            blend_mode: ImageBlendMode::Normal,
        }] => Some(Color {
            a: color.a * node.opacity,
            ..*color
        }),
        _ => None,
    }
}

fn issue(errors: &mut Vec<String>, pass: bool, message: impl Into<String>) {
    if !pass {
        errors.push(message.into());
    }
}

fn same_value(actual: &Value, expected: &Value) -> bool {
    actual == expected
        || (actual.is_number()
            && expected.is_number()
            && (actual.is_f64() || expected.is_f64())
            && actual
                .as_f64()
                .is_some_and(|n| n.abs() < 9_007_199_254_740_992.0)
            && actual.as_f64() == expected.as_f64())
}

fn check_config(case: &InitialCase) {
    assert!(
        case.document.is_absolute() && case.output.is_absolute(),
        "use absolute paths"
    );
    assert!(!case.root.trim().is_empty(), "root is required");
    for expected in std::iter::once(&case.initial).chain(case.clicks.iter().map(|c| &c.expect)) {
        assert!(
            expected.root.is_some()
                || !expected.app.is_empty()
                || !expected.texts.is_empty()
                || !expected.paints.is_empty(),
            "empty expectation"
        );
        for key in expected.app.keys() {
            assert!(
                !key.is_empty() && !key.starts_with('$'),
                "app keys use bare names, not $app.*"
            );
        }
        for (id, paint) in &expected.paints {
            assert!(
                !id.is_empty() && (paint.fill.is_some() || paint.stroke.is_some()),
                "empty paint expectation"
            );
            for value in [&paint.fill, &paint.stroke].into_iter().flatten() {
                assert!(
                    hex(value).is_some(),
                    "paint must be a standard hex color: {value}"
                );
            }
        }
    }
    for wheel in case
        .initial_scroll
        .iter()
        .chain(case.clicks.iter().filter_map(|c| c.before_scroll.as_ref()))
    {
        assert!(
            !wheel.node_id.is_empty() && wheel.dy.is_finite() && wheel.dy != 0.0,
            "invalid wheel request"
        );
    }
    assert!(
        case.clicks.iter().all(|c| !c.node_id.is_empty()),
        "empty click node id"
    );
    assert!(
        case.button_labels
            .iter()
            .all(|p| !p.button.is_empty() && !p.label.is_empty() && p.button != p.label),
        "invalid button label pair"
    );
}

fn observed_app(
    session: &PreviewSession,
    expected: &Expected,
    raw: &Value,
    errors: &mut Vec<String>,
) -> Value {
    let mut observations = serde_json::Map::new();
    for (key, want) in &expected.app {
        let actual = session.app_state_value_for_test(key).map(|v| v.0);
        issue(
            errors,
            actual.as_ref().is_some_and(|value| same_value(value, want)),
            format!("$app.{key}: expected {want}, got {actual:?}"),
        );
        observations.insert(
            key.clone(),
            json!({"expected":want,"present":actual.is_some(),
            "actual":actual,"saved_declaration":raw["state"].get(key)}),
        );
    }
    Value::Object(observations)
}

fn node_observation(root: &SceneNode, target: &SceneNode) -> Value {
    let visible = visible_rect(root, &target.id, root.bounds);
    json!({"bounds":rect_value(target.bounds),"visible_rect":visible.map(rect_value),
        "fully_visible":visible.is_some_and(|v| inside(target.bounds,v)),
        "text":target.text,"fill_rgba":target.fill.map(rgba),
        "solid_fill_rgba":solid_fill(target).map(rgba),"fill_type":format!("{:?}",target.fill_type),
        "fill_layers":format!("{:?}",target.fill_layers),
        "stroke_rgba":target.stroke.map(|s|rgba(s.color)),
        "stroke_width":target.stroke.map(|s|s.width),"opacity":target.opacity,
        "composite_opacity":target.composite_opacity})
}

fn visual<'a>(
    root: &'a SceneNode,
    raw: &Value,
    id: &str,
    errors: &mut Vec<String>,
) -> Option<&'a SceneNode> {
    issue(
        errors,
        node(raw, id).is_some(),
        format!("{id}: no original saved node"),
    );
    let target = root.find(id);
    issue(
        errors,
        target.is_some(),
        format!("{id}: absent from mounted root {}", root.id),
    );
    if let Some(target) = target {
        issue(
            errors,
            visible_rect(root, id, root.bounds).is_some_and(|r| inside(target.bounds, r)),
            format!("{id}: not fully visible through root/ancestor clips"),
        );
    }
    target
}

fn observations(
    session: &PreviewSession,
    raw: &Value,
    expected: &Expected,
    labels: &[ButtonLabel],
    errors: &mut Vec<String>,
) -> Value {
    let mut out = json!({"mounted":mounted(session),"expected":expected,
        "app":observed_app(session,expected,raw,errors),"texts":{},"paints":{},"button_labels":[]});
    let scene = session.preview_scene_for_test();
    let root_id = session.framed_root().map(|(id, _)| id);
    let root = root_id
        .as_deref()
        .and_then(|id| scene.active_page()?.children.iter().find(|n| n.id == id));
    issue(
        errors,
        root.is_some(),
        "framed root is absent from the mounted page",
    );
    let Some(root) = root else {
        return out;
    };
    issue(
        errors,
        visible_rect(root, &root.id, root.bounds).is_some_and(|v| inside(root.bounds, v)),
        "mounted root is hidden or has invalid bounds",
    );
    if let Some(want) = &expected.root {
        issue(
            errors,
            root.id == *want,
            format!("expected mounted root {want}, got {}", root.id),
        );
    }
    for (id, want) in &expected.texts {
        if let Some(target) = visual(root, raw, id, errors) {
            out["texts"][id] = node_observation(root, target);
            issue(
                errors,
                target.kind == NodeKind::Text && target.text.as_deref() == Some(want),
                format!("{id}: expected live text {want:?}, got {:?}", target.text),
            );
        } else {
            out["texts"][id] = json!({"missing":true});
        }
    }
    for (id, want) in &expected.paints {
        if let Some(target) = visual(root, raw, id, errors) {
            out["paints"][id] = node_observation(root, target);
            if let Some(color) = &want.fill {
                issue(
                    errors,
                    same_color(solid_fill(target), color),
                    format!("{id}: solid fill does not match {color}"),
                );
            }
            if let Some(color) = &want.stroke {
                let stroke = target.stroke.filter(|s| {
                    s.width.is_finite()
                        && s.width > 0.0
                        && s.sides
                            .is_none_or(|sides| sides.into_iter().all(|v| v.is_finite() && v > 0.0))
                });
                issue(
                    errors,
                    same_color(stroke.map(|s| s.color), color),
                    format!("{id}: visible stroke does not match {color}"),
                );
            }
        } else {
            out["paints"][id] = json!({"missing":true});
        }
    }
    // Check each pair on its saved screen; never mark an off-screen pair passed.
    for pair in labels {
        let belongs = node(raw, &root.id)
            .and_then(|r| node(r, &pair.button))
            .is_some();
        if !belongs {
            continue;
        }
        let descendant = node(raw, &pair.button)
            .and_then(|b| node(b, &pair.label))
            .is_some();
        let button = visual(root, raw, &pair.button, errors);
        let label = visual(root, raw, &pair.label, errors);
        let contained = button.zip(label).is_some_and(|(b, l)| {
            l.kind == NodeKind::Text
                && l.text.as_deref().is_some_and(|t| !t.trim().is_empty())
                && b.find(&pair.label).is_some()
                && inside(l.bounds, b.bounds)
        });
        issue(
            errors,
            descendant && contained,
            format!(
                "{}: label {} must be a real text descendant inside its button bounds",
                pair.button, pair.label
            ),
        );
        out["button_labels"].as_array_mut().unwrap().push(json!({"button":pair.button,"label":pair.label,
            "saved_descendant":descendant,"live_contained":contained,
            "button_bounds":button.map(|n|rect_value(n.bounds)),"label_bounds":label.map(|n|rect_value(n.bounds))}));
    }
    out
}

fn center_for_input(session: &PreviewSession, raw: &Value, id: &str) -> (f32, f32) {
    let scene = session.preview_scene_for_test();
    let (root_id, _) = session.framed_root().expect("mounted root for input");
    let root = live(&scene, &root_id);
    assert!(
        node(raw, &root_id).and_then(|r| node(r, id)).is_some(),
        "input must belong to saved mounted root"
    );
    let target = root.find(id).expect("input target belongs to mounted root");
    assert!(
        visible_rect(root, id, root.bounds).is_some_and(|r| inside(target.bounds, r)),
        "input target {id} is not fully visible"
    );
    (
        target.bounds.origin.x + target.bounds.size.x / 2.0,
        target.bounds.origin.y + target.bounds.size.y / 2.0,
    )
}

fn wheel(session: &mut PreviewSession, raw: &Value, request: &Wheel, now: u64, out: &mut Value) {
    *out = json!({"request":request,"before":mounted(session)});
    let point = center_for_input(session, raw, &request.node_id);
    out["point"] = json!([point.0, point.1]);
    // Require the saved scroll surface; a random ancestor's wheel is not proof.
    let saved = node(raw, &request.node_id).unwrap();
    assert!(
        saved["role"] == "scroll-area"
            || saved["events"]["onScroll"]
                .as_array()
                .is_some_and(|a| !a.is_empty()),
        "wheel target has no original scroll role or onScroll"
    );
    session.set_now_ms(now);
    let consumed = session.dispatch_wheel(point.0, point.1, 0.0, request.dy);
    settle(session, now + 500);
    out["consumed"] = json!(consumed);
    out["after"] = mounted(session);
    out["trace"] = json!(session.trace_entries());
    assert!(
        consumed,
        "wheel input not consumed; no synthetic scrolling allowed"
    );
}

fn authored_tap_ran(trace: &Value, start: u64, id: &str, actions: &Value) -> bool {
    let kinds: Vec<_> = actions
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .flat_map(|a| a.keys())
        .collect();
    let results: Vec<_> = trace
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| {
            e["sequence"].as_u64().is_some_and(|s| s > start)
                && e["kind"] == "action"
                && e["node_id"] == id
                && e["event"] == "onTap"
                && e["detail"]["phase"] == "result"
        })
        .collect();
    !results.is_empty()
        && results.iter().all(|e| e["detail"]["ok"] == true)
        && results.iter().any(|e| {
            e["action"]
                .as_str()
                .is_some_and(|a| kinds.iter().any(|k| k.as_str() == a))
        })
}

fn perform_click(
    case: &InitialCase,
    raw: &Value,
    session: &mut PreviewSession,
    index: usize,
    receipt: &mut Value,
) {
    let request = &case.clicks[index];
    let name = format!("click-{:02}", index + 1);
    stage(receipt, &name);
    let now = 5000 + index as u64 * 4000;
    receipt["clicks"]
        .as_array_mut()
        .unwrap()
        .push(json!({"request":request}));
    if let Some(scroll) = &request.before_scroll {
        wheel(
            session,
            raw,
            scroll,
            now,
            &mut receipt["clicks"][index]["before_scroll"],
        );
    }
    let saved = node(raw, &request.node_id).expect("original saved click node");
    let actions = &saved["events"]["onTap"];
    receipt["clicks"][index]["saved_on_tap"] = actions.clone();
    assert!(
        actions.as_array().is_some_and(|a| !a.is_empty()),
        "click target has no original saved onTap"
    );
    let point = center_for_input(session, raw, &request.node_id);
    receipt["clicks"][index]["point"] = json!([point.0, point.1]);
    let start = session.trace_entries().last().map_or(0, |e| e.sequence);
    session.set_now_ms(now + 1600);
    let consumed = session.dispatch_tap(point.0, point.1);
    // Snapshot before pump to exclude later lifecycle/inView actions as proof.
    let dispatch_trace = json!(session.trace_entries());
    let ran = authored_tap_ran(&dispatch_trace, start, &request.node_id, actions);
    let switched = settle(session, now + 2000);
    let mut errors = Vec::new();
    issue(&mut errors, consumed, "tap was not consumed");
    issue(
        &mut errors,
        ran,
        "configured saved node's onTap did not execute successfully during dispatch",
    );
    let after = observations(
        session,
        raw,
        &request.expect,
        &case.button_labels,
        &mut errors,
    );
    let entry = &mut receipt["clicks"][index];
    entry["consumed"] = json!(consumed);
    entry["authored_tap_ran"] = json!(ran);
    entry["reconcile_switched"] = json!(switched);
    entry["dispatch_trace"] = dispatch_trace;
    entry["trace"] = json!(session.trace_entries());
    entry["after"] = after;
    entry["errors"] = json!(errors);
    capture(session, &case.output, &format!("02-{name}.png"), receipt);
    assert!(errors.is_empty(), "{}", errors.join("; "));
    receipt["stages"][name] = json!("passed");
}

fn run_initial(
    case: &InitialCase,
    bytes: &[u8],
    session: &mut Option<PreviewSession>,
    receipt: &mut Value,
) {
    stage(receipt, "load");
    let raw: Value = serde_json::from_slice(bytes).expect("saved JSON document");
    let loaded =
        crate::doc_io::load_editor_state_with_report(&case.document, op_i18n::Locale::EnUs)
            .expect("production document load");
    assert!(
        !loaded.report.rewrite_blocked_by_schema_warning,
        "schema warning blocks faithful validation"
    );
    let original = loaded.state.doc.clone();
    receipt["stages"]["load"] = json!("passed");
    stage(receipt, "setup");
    *session = Some(enter(&loaded.state));
    let preview = session.as_mut().unwrap();
    preview.begin_lifecycle(0);
    settle(preview, 0);
    receipt["setup"] = setup_root(preview, &raw, &case.root, 1500);
    assert_mounted(preview, &case.root);
    receipt["stages"]["setup"] = json!("passed");
    stage(receipt, "initial");
    let mut errors = Vec::new();
    receipt["pristine_app"] = observed_app(preview, &case.initial, &raw, &mut errors);
    receipt["pristine_mounted"] = mounted(preview);
    capture(preview, &case.output, "00-pristine.png", receipt);
    if let Some(request) = &case.initial_scroll {
        wheel(preview, &raw, request, 3000, &mut receipt["initial_scroll"]);
    }
    receipt["initial"] = observations(
        preview,
        &raw,
        &case.initial,
        &case.button_labels,
        &mut errors,
    );
    receipt["initial"]["errors"] = json!(errors);
    receipt["initial"]["trace"] = json!(preview.trace_entries());
    capture(preview, &case.output, "01-initial.png", receipt);
    assert_mounted(preview, &case.root);
    assert!(errors.is_empty(), "{}", errors.join("; "));
    receipt["stages"]["initial"] = json!("passed");
    for index in 0..case.clicks.len() {
        perform_click(case, &raw, preview, index, receipt);
    }
    // Pairs on unvisited pages cannot silently count as validated.
    for pair in &case.button_labels {
        let checked = std::iter::once(&receipt["initial"])
            .chain(
                receipt["clicks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| &c["after"]),
            )
            .any(|snapshot| {
                snapshot["button_labels"].as_array().is_some_and(|pairs| {
                    pairs
                        .iter()
                        .any(|p| p["button"] == pair.button && p["label"] == pair.label)
                })
            });
        assert!(
            checked,
            "button label pair {:?} was never mounted and checked",
            pair
        );
    }
    assert_eq!(
        loaded.state.doc, original,
        "Preview must not mutate loaded author data"
    );
}

fn panic_message(error: &(dyn std::any::Any + Send)) -> &str {
    error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap_or("non-string panic")
}

#[test]
#[ignore = "requires OPENPENCIL_QA_INITIAL_STATE_CONFIG; untouched saved artifact; native engine only"]
fn retained_mobile_initial_state_and_clicks() {
    let config = PathBuf::from(
        std::env::var("OPENPENCIL_QA_INITIAL_STATE_CONFIG").expect("config JSON path"),
    );
    let config_bytes = std::fs::read(&config).expect("read config");
    let input: Value = serde_json::from_slice(&config_bytes).expect("config must be JSON");
    let output = PathBuf::from(input["output"].as_str().expect("output directory"));
    std::fs::create_dir(&output).expect("output must be new under an existing parent");
    let document = PathBuf::from(input["document"].as_str().unwrap_or_default());
    let mut receipt = json!({"status":"running","config":input,"config_path":config,"config_sha256":sha(&config_bytes),
        "scope":"native Preview engine, real dispatch_tap/wheel; no OS input or device validation",
        "capture_scope":"native raster export of live Preview scene; not OS/device-frame screenshots",
        "fonts":"test-process/platform fonts; not initialized desktop font-catalog parity",
        "limits":["not visual quality acceptance","not backend/cart/payment persistence","visual bounds do not prove pixel-level occlusion; tap trace verifies the actual handler"],
        "settle_frames_ms":[0,500,1000],"stages":{},"clicks":[],"captures":[]});
    let mut source = None;
    let mut session = None;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        stage(&mut receipt, "config");
        let case: InitialCase =
            serde_json::from_slice(&config_bytes).expect("strict initial-state config");
        check_config(&case);
        receipt["stages"]["config"] = json!("passed");
        let bytes = std::fs::read(&case.document).expect("original artwork bytes");
        receipt["input_sha256"] = json!(sha(&bytes));
        source = Some(bytes);
        run_initial(&case, source.as_ref().unwrap(), &mut session, &mut receipt);
    }));
    if let Some(preview) = session.as_ref() {
        receipt["final_mounted"] = mounted(preview);
        receipt["final_trace"] = json!(preview.trace_entries());
        receipt["final_runtime"] = json!(preview.debug_snapshot());
        receipt["effect_diagnostics"] = json!(preview.effect_diagnostics());
        receipt["warnings"] = json!(preview.warnings());
        if outcome.is_err() {
            let screenshot = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                capture(preview, &output, "99-failure.png", &mut receipt);
            }));
            if let Err(error) = screenshot {
                receipt["failure_capture_error"] = json!(panic_message(error.as_ref()));
            }
        }
    } else {
        receipt["failure_capture_unavailable"] = json!("Preview could not be entered");
    }
    let current = std::fs::read(&document);
    let unchanged = source
        .as_ref()
        .map(|original| current.as_ref().is_ok_and(|bytes| bytes == original));
    receipt["source_unchanged"] = json!(unchanged);
    receipt["input_sha256_after"] = json!(current.as_ref().ok().map(|bytes| sha(bytes)));
    if let Err(error) = &outcome {
        receipt["failure"] = json!(panic_message(error.as_ref()));
        let name = receipt["current_stage"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned();
        receipt["stages"][name] = json!("failed");
    }
    receipt["status"] = json!(if outcome.is_ok() && unchanged == Some(true) {
        "passed-requested-engine-checks"
    } else {
        "failed"
    });
    std::fs::write(
        output.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .expect("failure-safe receipt");
    assert_ne!(unchanged, Some(false), "original artwork bytes changed");
    if let Err(error) = outcome {
        std::panic::resume_unwind(error);
    }
}

#[test]
fn initial_state_harness_hex_and_trace_reject_false_evidence() {
    assert!(same_color(hex("#ABC"), "#aabbccff"));
    assert!(!same_color(hex("#ABC8"), "#aabbccff"));
    assert!(hex("#红色").is_none());
    let actions = json!([{"set":{"$app.qty":"$app.qty + 1"}}]);
    let entry = json!({"sequence":2,"kind":"action","node_id":"plus","event":"onTap",
        "action":"set","detail":{"phase":"result","ok":true}});
    assert!(authored_tap_ran(
        &json!([entry.clone()]),
        1,
        "plus",
        &actions
    ));
    assert!(!authored_tap_ran(
        &json!([entry.clone()]),
        2,
        "plus",
        &actions
    ));
    assert!(!authored_tap_ran(
        &json!([entry.clone()]),
        1,
        "other",
        &actions
    ));
    let mut lifecycle = entry.clone();
    lifecycle["event"] = json!("onInView");
    assert!(!authored_tap_ran(&json!([lifecycle]), 1, "plus", &actions));
    let mut failed = entry.clone();
    failed["detail"]["ok"] = json!(false);
    assert!(!authored_tap_ran(
        &json!([entry, failed]),
        1,
        "plus",
        &actions
    ));
}

#[test]
fn initial_state_harness_rejects_unknown_or_empty_expectations() {
    assert!(serde_json::from_value::<Expected>(json!({"state":{"qty":1}})).is_err());
    assert!(serde_json::from_value::<Paint>(json!({"fill":"#fff","width":4})).is_err());
    let case: InitialCase = serde_json::from_value(
        json!({"document":"/tmp/source.op","output":"/tmp/new-result",
        "root":"root","initial":{}}),
    )
    .unwrap();
    assert!(std::panic::catch_unwind(|| check_config(&case)).is_err());
}

#[test]
fn initial_state_harness_reads_live_binding_and_requires_real_target_event() {
    let source = r##"{"version":"1.1","formatVersion":"1.1","id":"initial-qa",
      "app":{"name":"QA","version":"1","id":"qa"},
      "state":{"qty":{"type":"int","default":1}},
      "children":[{"type":"frame","id":"screen","name":"Screen","screen":"/",
        "width":200,"height":200,"children":[
        {"type":"frame","id":"plus","name":"Plus","role":"button","x":10,"y":10,
          "width":40,"height":40,"fill":"#ffffff","events":{"onTap":[{"set":{"$app.qty":"$app.qty + 1"}}]}},
        {"type":"text","id":"qty","name":"Quantity","x":70,"y":10,"width":80,"height":40,
          "content":"stale static text","bindings":{"content":"'' + $app.qty"},"fill":"#111111"}]}]}"##;
    let raw: Value = serde_json::from_str(source).unwrap();
    let doc = op_pen_loader::payload::load_canonical(source)
        .unwrap()
        .value;
    let mut session = PreviewSession::enter(
        &doc,
        (800.0, 1000.0),
        &Default::default(),
        0,
        false,
        false,
        std::rc::Rc::new(jian_skia::SkiaMeasure::new()),
        0,
    )
    .unwrap();
    session.begin_lifecycle(0);
    settle(&mut session, 0);
    let mut expected: Expected =
        serde_json::from_value(json!({"app":{"qty":1},"texts":{"qty":"1"},
        "paints":{"plus":{"fill":"#fff"}}}))
        .unwrap();
    let mut errors = Vec::new();
    observations(&session, &raw, &expected, &[], &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    let invalid_pair = [ButtonLabel {
        button: "plus".into(),
        label: "qty".into(),
    }];
    let mut invalid_errors = Vec::new();
    observations(
        &session,
        &raw,
        &expected,
        &invalid_pair,
        &mut invalid_errors,
    );
    assert!(invalid_errors
        .iter()
        .any(|e| e.contains("real text descendant")));
    let start = session.trace_entries().last().map_or(0, |e| e.sequence);
    let point = center_for_input(&session, &raw, "plus");
    session.set_now_ms(2000);
    assert!(session.dispatch_tap(point.0, point.1));
    let trace = json!(session.trace_entries());
    let actions = &node(&raw, "plus").unwrap()["events"]["onTap"];
    assert!(authored_tap_ran(&trace, start, "plus", actions));
    assert!(!authored_tap_ran(&trace, start, "qty", actions));
    settle(&mut session, 2500);
    expected.app.insert("qty".into(), json!(2));
    expected.texts.insert("qty".into(), "2".into());
    observations(&session, &raw, &expected, &[], &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    expected
        .texts
        .insert("qty".into(), "stale static text".into());
    observations(&session, &raw, &expected, &[], &mut errors);
    assert!(errors.iter().any(|e| e.contains("expected live text")));
    let mut gradient = SceneNode::leaf("gradient", NodeKind::Frame);
    gradient.fill = hex("#fff");
    gradient.fill_type = SceneFillType::LinearGradient;
    assert!(
        solid_fill(&gradient).is_none(),
        "gradient first stop is not solid selection paint"
    );
    assert!(!same_value(
        &json!(9007199254740992u64),
        &json!(9007199254740993u64)
    ));
}
