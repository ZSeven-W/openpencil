//! Offscreen Studio (Home + generation workspace) screenshots through the
//! production paint path, for the states the Home-only `home_shots`
//! harness does not reach: other locales, the App task's desktop art,
//! and the workspace chrome.
//!
//! Run with:
//! ```text
//! cargo run -p op-host-native --features gl-host --example studio_shots -- <out_dir> [filter]
//! ```
//! `filter` keeps only the shots whose name contains it.

use op_editor_core::{HomeDevice, HomeFamily, TaskDraft};
use op_host_native::backend::{NativeBackend, NativeFrameBackend};
use op_host_native::widget_host::WidgetHostNative;
use op_i18n::Locale;

/// A host at `locale` on the given desktop Home task, with a usable
/// model selected (the state the approved prototype captures show).
fn home_host(locale: Locale, task: HomeFamily, device: HomeDevice) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.locale = locale;
    state.editor_ui.home.visible = true;
    state.editor_ui.home.set_task(task, 1);
    state.editor_ui.home.set_device(device);
    state.editor_ui.agent_settings.connected[0] = true;
    state.chat.available_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "glm-5.3-flash",
        "GLM 5.3 Flash",
    )];
    state.chat.selected_model = 0;
    host.set_now_ms(1_000);
    host
}

fn resource(name: &str) -> String {
    format!(
        "{}/../../packaging/ios/Resources/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// A desktop workspace over the sample `file`, settled Done as `family`
/// with a short conversation in the dock, at a `w`-wide window.
fn workspace_host(
    locale: Locale,
    file: &str,
    family: HomeFamily,
    w: f32,
    h: f32,
) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let src = std::fs::read_to_string(resource(file)).expect("read sample document");
    let loaded = op_pen_loader::load_canonical(&src).expect("parse sample document");
    let mut state = op_editor_core::EditorState::from_document(loaded.value);
    state.editor_ui.locale = locale;
    state.editor_ui.theme_mode = op_editor_core::ThemeMode::Light;
    assert!(host.replace_editor_state(state));
    host.set_now_ms(1_000);
    let brief = "A product intro deck for OpenPencil";
    host.editor_state_mut()
        .editor_ui
        .open_workspace_for_generation(family, brief, TaskDraft::default(), 0, 1_000, None);
    {
        let chat = &mut host.editor_state_mut().chat;
        chat.messages.push(op_editor_core::ChatMessage::user(brief));
        chat.messages.push(op_editor_core::ChatMessage::assistant(
            "Drafted 5 slides: cover, agenda, three feature pages. Tell me what to change.",
        ));
    }
    host.editor_state_mut()
        .editor_ui
        .workspace
        .sync_drawer_mode(w);
    let boards = op_editor_core::preview_slideshow::active_page_boards(host.editor_state()).len();
    host.settle_workspace_idle_edge(0, boards, false, w, h);
    host
}

/// [`workspace_host`] with the narrow window's drawer slid open.
fn open_drawer(mut host: WidgetHostNative) -> WidgetHostNative {
    host.editor_state_mut()
        .editor_ui
        .workspace
        .set_drawer_open(true, 0);
    host
}

/// Paint a few frames (letting lazy image decodes land in between) and
/// write the last one.
fn shoot(host: &mut WidgetHostNative, w: f32, h: f32, out_dir: &str, name: &str) {
    let mut backend = NativeBackend::with_dpi(2.0);
    let mut surface =
        skia_safe::surfaces::raster_n32_premul((w as i32, h as i32)).expect("raster surface");
    for frame in 0..6 {
        host.set_now_ms(10_000 + frame * 400);
        {
            let mut frame_backend = NativeFrameBackend::new(&mut backend, surface.canvas());
            host.paint(&mut frame_backend, w, h);
        }
        for pending in
            op_editor_ui::widgets::canvas_viewport_image::take_pending_decodes(usize::MAX)
        {
            match op_editor_ui::widgets::canvas_viewport_image::cached_bytes_for(pending.id)
                .and_then(|bytes| op_host_native::decode_raster_capped(&bytes, pending.max_edge_px))
            {
                Some((image, covers)) => {
                    backend.install_raster_image(pending.id, image, covers);
                    op_editor_ui::widgets::canvas_viewport_image::mark_decode_done(pending.id);
                }
                None => {
                    op_editor_ui::widgets::canvas_viewport_image::mark_decode_failed(pending.id)
                }
            }
        }
    }
    let data = surface
        .image_snapshot()
        .encode(None, skia_safe::EncodedImageFormat::PNG, 100)
        .expect("encode png");
    let target = format!("{out_dir}/{name}.png");
    std::fs::write(&target, data.as_bytes()).expect("write png");
    println!("studio-shots: wrote {target}");
}

type Scenario = (&'static str, f32, f32, fn() -> WidgetHostNative);

fn scenarios() -> Vec<Scenario> {
    vec![
        ("home-app-mobile-en", 1440.0, 900.0, || {
            home_host(Locale::EnUs, HomeFamily::AppUi, HomeDevice::Mobile)
        }),
        ("home-app-desktop-en", 1440.0, 900.0, || {
            home_host(Locale::EnUs, HomeFamily::AppUi, HomeDevice::Desktop)
        }),
        ("home-app-mobile-de", 1440.0, 900.0, || {
            home_host(Locale::De, HomeFamily::AppUi, HomeDevice::Mobile)
        }),
        ("home-app-desktop-de", 1440.0, 900.0, || {
            home_host(Locale::De, HomeFamily::AppUi, HomeDevice::Desktop)
        }),
        ("home-app-mobile-zh", 1440.0, 900.0, || {
            home_host(Locale::ZhCn, HomeFamily::AppUi, HomeDevice::Mobile)
        }),
        ("home-app-desktop-zh", 1440.0, 900.0, || {
            home_host(Locale::ZhCn, HomeFamily::AppUi, HomeDevice::Desktop)
        }),
        ("ws-wide-1280-en", 1280.0, 800.0, || {
            workspace_host(
                Locale::EnUs,
                "ppt-demo.op",
                HomeFamily::Presentation,
                1280.0,
                800.0,
            )
        }),
        ("ws-narrow-820-closed-en", 820.0, 760.0, || {
            workspace_host(
                Locale::EnUs,
                "ppt-demo.op",
                HomeFamily::Presentation,
                820.0,
                760.0,
            )
        }),
        ("ws-narrow-820-open-en", 820.0, 760.0, || {
            open_drawer(workspace_host(
                Locale::EnUs,
                "ppt-demo.op",
                HomeFamily::Presentation,
                820.0,
                760.0,
            ))
        }),
    ]
}

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/studio-shots".into());
    let filter = std::env::args().nth(2).unwrap_or_default();
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    for (name, w, h, build) in scenarios() {
        if !name.contains(filter.as_str()) {
            continue;
        }
        let mut host = build();
        shoot(&mut host, w, h, &out_dir, name);
    }
}
