//! Replay cached, real model boards through native normal-mode controls.
//! This deliberately uses no provider and does not stand in for OS window QA.

use super::{centre, WidgetHostNative, H, W};
use op_editor_core::{
    EditorState, HomeFamily, PenNodeExt, WorkspaceHit, WorkspacePhase, WorkspaceVariant,
};
use op_editor_ui::widgets::WorkspaceSurface;
use std::path::Path;

fn capture(host: &mut WidgetHostNative, folder: &Path, name: &str) {
    host.set_now_ms(2_000);
    let mut backend = crate::backend::NativeBackend::with_dpi(1.0);
    let mut surface = skia_safe::surfaces::raster_n32_premul((W as i32, H as i32)).unwrap();
    {
        let mut frame = crate::backend::NativeFrameBackend::new(&mut backend, surface.canvas());
        host.paint(&mut frame, W, H);
    }
    let image = surface
        .image_snapshot()
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .unwrap();
    std::fs::write(folder.join(format!("{name}.png")), image.as_bytes()).unwrap();
}

fn pick(host: &mut WidgetHostNative, index: usize) {
    let surface = WorkspaceSurface::for_editor(host.editor_state()).unwrap();
    let layout = surface.layout(W, H);
    let (x, y) = centre(surface.variant_bar(&layout)[index].button);
    assert!(host.apply_press(x, y, W, H));
    host.apply_release_with_viewport(W, H);
}

fn history(host: &mut WidgetHostNative, hit: WorkspaceHit) {
    let surface = WorkspaceSurface::for_editor(host.editor_state()).unwrap();
    let layout = surface.layout(W, H);
    let (_, rect, enabled) = surface
        .history_buttons(&layout)
        .into_iter()
        .find(|(item, _, _)| *item == hit)
        .unwrap();
    assert!(enabled);
    let (x, y) = centre(rect);
    assert!(host.apply_press(x, y, W, H));
    host.apply_release_with_viewport(W, H);
}

#[cfg(feature = "bundled-design-fonts")]
#[test]
#[ignore = "requires an explicitly supplied cached GLM output; no provider calls"]
fn cached_directions_survive_pick_history_save_and_reopen() {
    let source = std::env::var("OPENPENCIL_VARIANTS_QA_SOURCE").expect("cached source");
    let folder = std::env::var("OPENPENCIL_VARIANTS_QA_DIR").expect("QA output directory");
    let folder = Path::new(&folder);
    std::fs::create_dir_all(folder).unwrap();
    crate::bundled_design_fonts::register();
    let text = std::fs::read_to_string(&source).unwrap();
    let doc = jian_ops_schema::load_str(&text).unwrap().value;
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.home.visible = true;
    host.install_imported_state(EditorState::from_document(doc));
    assert!(!host.editor_state().editor_ui.missing_fonts_modal_open);
    let roots = host.editor_state().active_children().to_vec();
    assert_eq!(roots.len(), 3, "one board per real model direction");
    let state = host.editor_state_mut();
    state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    state.editor_ui.home.hide();
    state.editor_ui.open_workspace_for_generation(
        HomeFamily::KnowledgeCards,
        "咖啡活动卡",
        Default::default(),
        0,
        1_000,
        None,
    );
    state.editor_ui.workspace.begin_variants(3);
    // Smoke files carry the landed, resolved boards but no transient chooser.
    // Rebuild their grouping from the runner's stamped names. Palette restoration
    // is separately covered by the core tests using the real variables tables.
    for index in 0..3 {
        let name = format!("Direction {}", op_editor_core::variant_letter(index));
        let root = roots
            .iter()
            .find(|root| {
                root.base()
                    .name
                    .as_deref()
                    .is_some_and(|label| label.starts_with(&format!("{name} · ")))
            })
            .unwrap();
        let label = root.base().name.as_ref().unwrap();
        let (_, remainder) = label.split_once(" · ").unwrap();
        let (style, _) = remainder.split_once(" · ").unwrap();
        state.editor_ui.workspace.record_variant(WorkspaceVariant {
            index,
            name: format!("方案 {}", op_editor_core::variant_letter(index)),
            style_guide: style.to_lowercase().replace(' ', "-"),
            style_label: style.into(),
            name_prefix: format!("{name} · {style} · "),
            root_ids: vec![root.id_str().into()],
            variables: None,
            themes: None,
        });
    }
    state.editor_ui.workspace.phase = WorkspacePhase::Done;
    let original = state.doc.clone();
    host.apply_workspace_fit(W, H);
    capture(&mut host, folder, "01-compare");
    pick(&mut host, 1);
    let picked = host.editor_state().doc.clone();
    capture(&mut host, folder, "02-picked-b");
    history(&mut host, WorkspaceHit::Undo);
    assert_eq!(host.editor_state().doc, original);
    capture(&mut host, folder, "03-undo-compare");
    history(&mut host, WorkspaceHit::Redo);
    assert_eq!(host.editor_state().doc, picked);
    capture(&mut host, folder, "04-redo-b");
    history(&mut host, WorkspaceHit::Undo);
    pick(&mut host, 2);
    let chosen = host.editor_state().doc.clone();
    capture(&mut host, folder, "05-picked-c");
    assert_eq!(host.editor_state().active_children().len(), 1);
    assert_eq!(chosen.pages.as_ref().unwrap()[1].children.len(), 2);
    let saved = serde_json::to_string_pretty(&chosen).unwrap();
    std::fs::write(folder.join("picked-c.op"), &saved).unwrap();
    let loaded = jian_ops_schema::load_str(&saved).unwrap().value;
    assert_eq!(
        loaded, chosen,
        "every chosen/alternative node survives serialization"
    );
    host.install_imported_state(EditorState::from_document(loaded));
    let state = host.editor_state_mut();
    state.editor_ui.home.hide();
    state.editor_ui.locale = op_editor_core::Locale::ZhCn;
    state
        .editor_ui
        .workspace
        .open_for_reading(HomeFamily::KnowledgeCards, 1_000);
    host.apply_workspace_fit(W, H);
    capture(&mut host, folder, "06-reopened-c");
    assert_eq!(
        std::fs::read_to_string(source).unwrap(),
        text,
        "source stays untouched"
    );
}
