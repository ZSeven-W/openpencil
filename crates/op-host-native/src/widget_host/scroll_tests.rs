use super::WidgetHostNative;

fn nested_frame_doc(depth: usize) -> String {
    let mut src = String::from(r#"{"version":"0.8.0","children":["#);
    for i in 0..depth {
        src.push_str(&format!(
            r##"{{"type":"frame","id":"nest-{i:05}","name":"Nested Layer {i:05}","x":8,"y":6,"width":400,"height":220,"fill":[{{"type":"solid","color":"#ffffff20"}}],"stroke":{{"thickness":1,"fill":[{{"type":"solid","color":"#0088ff"}}]}},"children":["##
        ));
    }
    for _ in 0..depth {
        src.push_str("]}");
    }
    src.push_str("]}");
    src
}

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

fn run_deep_layer_fixture(test: impl FnOnce() + Send + 'static) {
    let handle = std::thread::Builder::new()
        .name("op-host-native-deep-layer-fixture".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(test)
        .expect("spawn deep layer fixture test");
    if let Err(payload) = handle.join() {
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn layer_panel_trackpad_pan_scrolls_horizontally() {
    run_deep_layer_fixture(|| {
        let mut host = WidgetHostNative::new();
        seed(&mut host, &nested_frame_doc(50));
        let viewport_w = 1200.0;
        let viewport_h = 800.0;
        let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
        let rect = op_editor_ui::Rect {
            origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
            size: op_editor_ui::Point2D::new(
                host.editor_state().editor_ui.layer_panel_width,
                viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT,
            ),
        };
        let regions = panel.regions(rect);
        assert!(regions.layers_max_h_scroll > 0.0);

        assert!(host.apply_pan_gesture(
            80.0,
            regions.layers_rows_top + 12.0,
            -180.0,
            0.0,
            viewport_w,
            viewport_h
        ));

        assert!(host.editor_state().editor_ui.layer_layers_h_scroll > 0.0);
    });
}
