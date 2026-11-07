//! Opt-in acceptance of an unedited AI-generated document through native input and paint.
#![cfg(all(feature = "gl-host", feature = "testing", target_os = "macos"))]

use jian_core::gesture::pointer::{Modifiers, PointerKind, PointerPhase};
use op_editor_core::EditorState;
use op_editor_ui::{Point2D, Rect};
use op_host_native::{NativeBackend, NativeFrameBackend, PreviewSession, WidgetHostNative};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf, rc::Rc};

const W: f32 = 1440.0;
const H: f32 = 1000.0;

struct Replay {
    host: WidgetHostNative,
    session: PreviewSession,
    names: BTreeMap<String, String>,
    source: String,
    input: PathBuf,
    out: PathBuf,
    now: u64,
    receipts: Vec<Value>,
}

impl Replay {
    fn new(case: &str) -> Self {
        let input = PathBuf::from(std::env::var("OPENPENCIL_NATIVE_QA_DOCUMENT").unwrap());
        let out = PathBuf::from(std::env::var("OPENPENCIL_NATIVE_QA_OUTPUT").unwrap()).join(case);
        std::fs::create_dir_all(&out).unwrap();
        let source = std::fs::read_to_string(&input).unwrap();
        let value: Value = serde_json::from_str(&source).unwrap();
        fn collect(v: &Value, names: &mut BTreeMap<String, String>) {
            if let (Some(name), Some(id)) = (v["name"].as_str(), v["id"].as_str()) {
                names.insert(name.into(), id.into());
            }
            match v {
                Value::Object(map) => map.values().for_each(|v| collect(v, names)),
                Value::Array(items) => items.iter().for_each(|v| collect(v, names)),
                _ => {}
            }
        }
        let mut names = BTreeMap::new();
        collect(&value, &mut names);
        let doc = jian_ops_schema::load_str(&source).unwrap().value;
        let mut session = PreviewSession::enter(
            &doc,
            (W, H),
            &BTreeMap::new(),
            0,
            false,
            false,
            Rc::new(jian_skia::SkiaMeasure::new()),
            0,
        )
        .unwrap();
        session.begin_lifecycle(0);
        let mut host = WidgetHostNative::new();
        host.install_imported_state(EditorState::from_document(doc));
        host.editor_state_mut().editor_ui.home.visible = false;
        if std::env::var("OPENPENCIL_NATIVE_QA_MODE").as_deref() != Ok("professional") {
            // Match the desktop's open_loaded_normal_workspace entry path.
            let state = host.editor_state_mut();
            op_editor_core::work_identity::open_for_reading(state, 0);
            state.editor_ui.entry_surface = op_editor_core::EntrySurface::Home;
            state.editor_ui.workspace.view =
                op_editor_core::WorkspaceView::default_for(state.editor_ui.workspace.family);
            state.editor_ui.workspace.previous_tool = Some(state.tool);
            state.editor_ui.enter_chat_tab();
            state.tool = op_editor_core::Tool::Hand;
        }
        host.preview_resize(W, H);
        assert!(host.enter_preview((W, H)));
        let mut replay = Self {
            host,
            session,
            names,
            source,
            input,
            out,
            now: 1000,
            receipts: vec![],
        };
        replay.settle();
        replay.capture("initial");
        replay
    }

    fn id(&self, name: &str) -> String {
        self.names
            .get(name)
            .unwrap_or_else(|| panic!("missing named control {name}"))
            .clone()
    }

    fn settle(&mut self) {
        // Sampling a changed binding starts its authored transition at this frame.
        let _ = self.session.preview_scene_for_test();
        self.now += 600;
        self.session.set_now_ms(self.now);
        self.session.pump(self.now);
        self.session.reconcile(self.now);
        self.host.set_now_ms(self.now);
        self.host.pump_preview();
        // The native paint pass settles mode transitions and reconciles routing.
        self.paint();
        // A native paint can start a property tween; render a later frame too.
        self.now += 600;
        self.session.set_now_ms(self.now);
        self.session.pump(self.now);
        self.host.set_now_ms(self.now);
        self.host.pump_preview();
        self.paint();
    }

    fn paint(&mut self) -> Vec<u8> {
        let mut surface = skia_safe::surfaces::raster_n32_premul((W as i32, H as i32)).unwrap();
        let mut backend = NativeBackend::with_dpi(1.0);
        let mut frame = NativeFrameBackend::new(&mut backend, surface.canvas());
        self.host.paint(&mut frame, W, H);
        surface
            .image_snapshot()
            .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
            .unwrap()
            .as_bytes()
            .to_vec()
    }

    fn capture(&mut self, label: &str) -> Vec<u8> {
        let bytes = self.paint();
        std::fs::write(self.out.join(format!("{label}.png")), &bytes).unwrap();
        bytes
    }

    fn tap(&mut self, name: &str) {
        let id = self.id(name);
        let scene = self.session.preview_scene_for_test();
        let rect = scene.active_page().unwrap().find(&id).unwrap().bounds;
        let point = Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        );
        // Click through the frame the host really paints (fitted below
        // the Preview switcher rows), not a recomputation of it.
        let geometry = self
            .host
            .preview_device_frame_for_test()
            .expect("device frame while previewing");
        let screen = Point2D::new(
            geometry.content_origin.x + point.x * geometry.fit,
            geometry.content_origin.y + point.y * geometry.fit,
        );
        self.now += 100;
        self.session.set_now_ms(self.now);
        self.host.set_now_ms(self.now);
        assert!(
            self.host.apply_press_at(screen.x, screen.y, W, H, self.now),
            "native down: {name}"
        );
        self.session.dispatch_pointer_for_id_at(
            1,
            PointerKind::Mouse,
            point.x,
            point.y,
            PointerPhase::Down,
            self.now,
        );
        assert!(
            self.host
                .apply_release_with_viewport_at(W, H, self.now + 20),
            "native up: {name}"
        );
        self.session.dispatch_pointer_for_id_at(
            1,
            PointerKind::Mouse,
            point.x,
            point.y,
            PointerPhase::Up,
            self.now + 20,
        );
        self.settle();
        self.check_host_state(name);
        self.capture(&format!("step-{}", self.receipts.len()));
    }

    fn check_host_state(&mut self, label: &str) {
        let snapshot = self.session.debug_snapshot();
        for row in &snapshot.state {
            if matches!(
                row.scope,
                op_preview_core::PreviewStateScope::App | op_preview_core::PreviewStateScope::State
            ) {
                let native = self.host.preview_app_state_value_for_test(&row.key);
                assert_eq!(
                    native.map(|v| v.0),
                    Some(row.value.clone()),
                    "native state diverged: {label} / {}",
                    row.key
                );
            }
        }
        self.receipts
            .push(json!({"step":label,"snapshot":snapshot,"trace":self.session.trace_entries()}));
        std::fs::write(
            self.out.join("receipt.json"),
            serde_json::to_vec_pretty(&self.receipts).unwrap(),
        )
        .unwrap();
        assert_eq!(self.source, std::fs::read_to_string(&self.input).unwrap());
    }

    fn text(&self, name: &str) -> String {
        self.session
            .preview_scene_for_test()
            .active_page()
            .unwrap()
            .find(&self.id(name))
            .unwrap()
            .text
            .clone()
            .unwrap_or_default()
    }

    fn fill(&self, name: &str) -> String {
        format!(
            "{:?}",
            self.session
                .preview_scene_for_test()
                .active_page()
                .unwrap()
                .find(&self.id(name))
                .unwrap()
                .fill
        )
    }

    fn assert_money(&self, qty: u32) {
        assert_eq!(self.text("Quantity value").trim(), qty.to_string());
        let amount: String = self
            .text("Total price")
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        assert_eq!(
            amount.parse::<f64>().unwrap(),
            f64::from(qty * 24),
            "total must track quantity"
        );
    }
}

#[test]
#[ignore = "requires OPENPENCIL_NATIVE_QA_DOCUMENT and OPENPENCIL_NATIVE_QA_OUTPUT"]
fn generated_initial_state_matches_authored_quantity_and_total() {
    let r = Replay::new("defaults");
    r.assert_money(1);
    let snapshot = r.session.debug_snapshot();
    assert!(snapshot
        .state
        .iter()
        .any(|s| s.key == "finish" && s.value == "cream"));
}

#[test]
#[ignore = "requires OPENPENCIL_NATIVE_QA_DOCUMENT and OPENPENCIL_NATIVE_QA_OUTPUT"]
fn generated_quantity_and_total_follow_native_clicks() {
    let mut r = Replay::new("quantity");
    // The clamp also lets this case test behavior independently of missing defaults.
    r.tap("Quantity minus");
    r.assert_money(1);
    for (control, qty) in [
        ("Quantity minus", 1),
        ("Quantity plus", 2),
        ("Quantity plus", 3),
        ("Quantity minus", 2),
        ("Quantity minus", 1),
        ("Quantity minus", 1),
    ] {
        r.tap(control);
        r.assert_money(qty);
        r.capture(&format!("step-{}-qty-{qty}", r.receipts.len()));
    }
}

#[test]
#[ignore = "requires OPENPENCIL_NATIVE_QA_DOCUMENT and OPENPENCIL_NATIVE_QA_OUTPUT"]
fn generated_selection_updates_both_options_through_native_clicks() {
    let mut r = Replay::new("selection");
    r.tap("Finish cream");
    let initial = (r.fill("Finish cream"), r.fill("Finish green"));
    r.tap("Finish green");
    let green = (r.fill("Finish cream"), r.fill("Finish green"));
    assert_ne!(initial.0, green.0);
    assert_ne!(initial.1, green.1);
    r.capture("green-selected");
    r.tap("Finish green");
    assert_eq!(green, (r.fill("Finish cream"), r.fill("Finish green")));
    r.tap("Finish cream");
    assert_eq!(initial, (r.fill("Finish cream"), r.fill("Finish green")));
    r.capture("cream-restored");
}

#[test]
#[ignore = "requires OPENPENCIL_NATIVE_QA_DOCUMENT and OPENPENCIL_NATIVE_QA_OUTPUT"]
fn generated_input_updates_live_text_with_native_keyboard_input() {
    let mut r = Replay::new("input");
    r.tap("Name input");
    for ch in "Alice测试".chars() {
        let text = ch.to_string();
        assert!(r.session.dispatch_text(&text));
        assert!(r.host.preview_dispatch_text(&text));
        r.settle();
        r.check_host_state(&text);
    }
    r.capture("typed");
    let scene = r.session.preview_scene_for_test();
    let input = scene
        .active_page()
        .unwrap()
        .find(&r.id("Name input"))
        .unwrap();
    assert!(input
        .widget
        .as_ref()
        .unwrap()
        .value_str
        .as_deref()
        .unwrap()
        .contains("Alice测试"));
    assert!(r.text("Name value").contains("Alice测试"));
    r.session.dispatch_key("Backspace", Modifiers::default());
    r.host.preview_dispatch_key("Backspace", false);
    r.settle();
    r.check_host_state("backspace");
    assert!(r.text("Name value").contains("Alice测"));
    assert!(!r.text("Name value").contains("Alice测试"));
    r.capture("backspace");
}

#[test]
#[ignore = "requires OPENPENCIL_NATIVE_QA_DOCUMENT and OPENPENCIL_NATIVE_QA_OUTPUT"]
fn generated_navigation_and_back_change_native_painted_page() {
    let mut r = Replay::new("navigation");
    let initial = r.capture("home");
    assert_eq!(r.session.current_path_for_test(), "/");
    r.tap("Open detail");
    assert_eq!(r.session.current_path_for_test(), "/detail");
    assert_ne!(initial, r.capture("detail"));
    r.tap("Go back");
    assert_eq!(r.session.current_path_for_test(), "/");
    r.capture("returned-home");
}

#[test]
#[ignore = "requires OPENPENCIL_NATIVE_QA_DOCUMENT and OPENPENCIL_NATIVE_QA_OUTPUT"]
fn bound_input_value_is_visible_immediately_after_navigation_back() {
    let mut r = Replay::new("input-return");
    r.tap("Name input");
    assert!(r.session.dispatch_text("Alice测试"));
    assert!(r.host.preview_dispatch_text("Alice测试"));
    r.settle();
    assert!(r.text("Name value").contains("Alice测试"));
    r.tap("Open detail");
    r.tap("Go back");
    assert!(r.text("Name value").contains("Alice测试"));
    let scene = r.session.preview_scene_for_test();
    let input = scene
        .active_page()
        .unwrap()
        .find(&r.id("Name input"))
        .unwrap();
    assert_eq!(
        input.widget.as_ref().unwrap().value_str.as_deref(),
        Some("Alice测试"),
        "bound input must display retained app state before another keystroke"
    );
}

#[test]
#[ignore = "requires OPENPENCIL_NATIVE_QA_DOCUMENT and OPENPENCIL_NATIVE_QA_OUTPUT"]
fn preview_switchers_do_not_overlap_the_device_frame() {
    let r = Replay::new("preview-chrome");
    let (x, y, w, h) =
        op_editor_ui::widgets::host_canvas_geometry::canvas_region(r.host.editor_state(), W, H);
    let canvas = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, h),
    };
    let geometry = r
        .host
        .preview_device_frame_for_test()
        .expect("device frame while previewing");
    let mut controls = vec![op_editor_ui::widgets::PreviewDeviceSwitcher::pill_rect(
        canvas,
    )];
    if r.session.is_app_mode() {
        controls.push(op_editor_ui::widgets::ScreenSwitcherPills::row_rect(
            canvas,
            r.session.screen_switcher_entries().len(),
        ));
    }
    for control in controls {
        let frame = geometry.frame;
        let disjoint = control.origin.x + control.size.x <= frame.origin.x
            || frame.origin.x + frame.size.x <= control.origin.x
            || control.origin.y + control.size.y <= frame.origin.y
            || frame.origin.y + frame.size.y <= control.origin.y;
        assert!(
            disjoint,
            "preview chrome {control:?} obscures device content {frame:?}"
        );
    }
}
