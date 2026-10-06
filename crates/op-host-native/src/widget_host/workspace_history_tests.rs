use super::WidgetHostNative;
use op_editor_core::{
    EditorCommand, EditorState, HomeFamily, NodeId, WorkspaceHit, WorkspacePhase,
};
use op_editor_ui::widgets::{MobileMoreEntry, WorkspaceSurface};
use op_editor_ui::{Point2D, Rect};

const SOURCE: &str = r##"{"version":"1.0.0","children":[
 {"type":"frame","id":"work","name":"主海报","width":540,"height":720,"fill":[{"type":"solid","color":"#142B3D"}],"children":[
   {"type":"text","id":"title","content":"夜航咖啡节","x":48,"y":96,"width":440,"fontSize":52,"fill":[{"type":"solid","color":"#EAE4D8"}]},
   {"type":"text","id":"date","content":"11月14日19:00","x":48,"y":250,"fontSize":28,"fill":[{"type":"solid","color":"#EAE4D8"}]}
 ]},
 {"type":"frame","id":"other","name":"其他页","x":650,"width":540,"height":720,"children":[]}
]}"##;

fn host(touch: bool) -> WidgetHostNative {
    let state = EditorState::from_document(jian_ops_schema::load_str(SOURCE).unwrap().value);
    let mut host = WidgetHostNative::new();
    host.install_imported_state(state);
    host.set_now_ms(1_000);
    let state = host.editor_state_mut();
    state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    state.editor_ui.touch = touch;
    if touch {
        state.editor_ui.size_class = op_editor_core::size_class::EditorSizeClass::Compact;
    }
    state.editor_ui.home.hide();
    state
        .editor_ui
        .workspace
        .open_for_reading(HomeFamily::EventPoster, 1_000);
    state.editor_ui.sidebar_open = true;
    state.editor_ui.layer_panel_width = 320.0;
    state.editor_ui.file_name_display = Some("夜航咖啡节.op".into());
    host.set_now_ms(2_000);
    host
}

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn click_history(host: &mut WidgetHostNative, undo: bool, w: f32, h: f32) {
    let point = if host.editor_state().editor_ui.touch_chrome() {
        host.editor_state_mut().editor_ui.mobile_sheet =
            Some(op_editor_core::size_class::MobileSheetKind::More);
        let panel = host.mobile_sheet_rect(w, h, op_editor_core::size_class::MobileSheetKind::More);
        let entry = if undo {
            MobileMoreEntry::Undo
        } else {
            MobileMoreEntry::Redo
        };
        let index = MobileMoreEntry::visible(host.editor_state())
            .iter()
            .position(|item| *item == entry)
            .unwrap();
        center(op_editor_ui::widgets::mobile_chrome::more_entry_rect(
            host.editor_state(),
            panel,
            index,
        ))
    } else {
        let surface = WorkspaceSurface::for_editor(host.editor_state()).unwrap();
        let hit = if undo {
            WorkspaceHit::Undo
        } else {
            WorkspaceHit::Redo
        };
        let (_, rect, enabled) = surface
            .history_buttons(&surface.layout(w, h))
            .into_iter()
            .find(|(item, _, _)| *item == hit)
            .unwrap();
        assert!(enabled);
        center(rect)
    };
    assert!(host.apply_press(point.x, point.y, w, h));
    host.apply_release_with_viewport(w, h);
}

fn capture(host: &mut WidgetHostNative, w: f32, h: f32, name: &str) {
    let Some(folder) = std::env::var_os("OPENPENCIL_HISTORY_QA_DIR") else {
        return;
    };
    let mut backend = crate::backend::NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((w as i32, h as i32)).unwrap();
    {
        let mut frame = crate::backend::NativeFrameBackend::new(&mut backend, surface.canvas());
        host.paint(&mut frame, w, h);
    }
    let image = surface
        .image_snapshot()
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .unwrap();
    let folder = std::path::PathBuf::from(folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join(format!("{name}.png")), image.as_bytes()).unwrap();
}

#[test]
fn normal_history_click_restores_whole_work_and_keeps_the_unsent_draft() {
    for (touch, w, h, prefix) in [
        (false, 1440.0, 900.0, "desktop"),
        (true, 390.0, 844.0, "phone"),
    ] {
        let mut host = host(touch);
        let original = host.editor_state().doc.clone();
        host.editor_state_mut().with_history_group(|state| {
            assert!(state.apply(EditorCommand::SetNodeText {
                node_id: NodeId::new("title"),
                text: "月下咖啡节".into()
            }));
            assert!(state.apply(EditorCommand::SetNodeText {
                node_id: NodeId::new("date"),
                text: "11月15日20:00".into()
            }));
        });
        let edited = host.editor_state().doc.clone();
        host.refresh_layout_scene();
        if touch {
            host.frame_reader_board(w, h);
        } else {
            host.apply_workspace_fit(w, h);
        }
        let camera = host.editor_state().viewport;
        host.editor_state_mut().chat.focused = true;
        host.editor_state_mut()
            .chat
            .set_input_text("还没发出去的修改想法");
        capture(&mut host, w, h, &format!("{prefix}-edited"));
        click_history(&mut host, true, w, h);
        assert_eq!(host.editor_state().doc, original);
        assert_eq!(
            host.editor_state().viewport,
            camera,
            "copy edits preserve reading zoom"
        );
        assert_eq!(
            host.editor_state().chat.input.text(),
            "还没发出去的修改想法"
        );
        assert!(host.editor_state().editor_ui.workspace.visible);
        capture(&mut host, w, h, &format!("{prefix}-undo"));
        click_history(&mut host, false, w, h);
        assert_eq!(host.editor_state().doc, edited);
        capture(&mut host, w, h, &format!("{prefix}-redo"));
    }
}

#[test]
fn normal_history_shortcuts_cannot_race_a_live_turn() {
    let mut host = host(false);
    host.editor_state_mut().with_history_group(|state| {
        assert!(state.apply(EditorCommand::SetNodeText {
            node_id: NodeId::new("title"),
            text: "Edited".into()
        }));
    });
    let edited = host.editor_state().doc.clone();
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Generating;
    assert!(!host.apply_undo());
    assert_eq!(host.editor_state().doc, edited);
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Done;
    assert!(host.apply_undo());
    let original = host.editor_state().doc.clone();
    host.editor_state_mut().chat.pending_send = Some("next edit".into());
    assert!(!host.apply_redo());
    assert_eq!(host.editor_state().doc, original);
}

#[test]
fn undoing_board_resize_refits_the_restored_work_on_desktop_and_phone() {
    for (touch, w, h) in [(false, 1440.0, 900.0), (true, 390.0, 844.0)] {
        let mut host = host(touch);
        host.last_viewport_w = w;
        host.last_viewport_h = h;
        host.editor_state_mut().with_history_group(|state| {
            assert!(state.apply(EditorCommand::UpdateNode {
                node_id: NodeId::new("work"),
                x: None,
                y: None,
                width: Some(2000),
                height: None,
                name: None,
                fill_hex: None,
                page_id: None,
            }));
        });
        host.refresh_layout_scene();
        if touch {
            host.frame_reader_board(w, h);
        } else {
            host.apply_workspace_fit(w, h);
        }
        let resized_zoom = host.editor_state().viewport.zoom;
        assert!(host.apply_undo());
        assert!(host.editor_state().viewport.zoom > resized_zoom);
        assert!(host.apply_redo());
        assert!((host.editor_state().viewport.zoom - resized_zoom).abs() < 0.001);
    }
}
