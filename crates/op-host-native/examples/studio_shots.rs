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

/// Home with the keyboard focus ring on the 网站设计 (Websites) chip.
fn home_focused() -> WidgetHostNative {
    let mut host = home_host(Locale::EnUs, HomeFamily::AppUi, HomeDevice::Mobile);
    host.editor_state_mut().editor_ui.home.key_focus =
        Some(op_editor_core::HomeHit::Tab(HomeFamily::Web));
    host
}

/// A finished deck workspace with the focus ring on the strip's Play tile.
fn workspace_focused() -> WidgetHostNative {
    let mut host = workspace_host(
        Locale::EnUs,
        "ppt-demo.op",
        HomeFamily::Presentation,
        1280.0,
        800.0,
    );
    host.editor_state_mut().editor_ui.workspace.key_focus =
        Some(op_editor_core::WorkspaceHit::Play);
    host
}

/// A finished deck workspace with its quality report open: two itemized
/// fixes, a prose fix, and two remaining findings.
fn workspace_quality(locale: Locale) -> WidgetHostNative {
    use op_editor_core::{QualityItem, QualityRepairRecord, QualityReport, QualityTopic};
    let mut host = workspace_host(
        locale,
        "ppt-demo.op",
        HomeFamily::Presentation,
        1280.0,
        800.0,
    );
    let mut report = QualityReport::default();
    let record = |pass: &str, family: &str, name: &str, detail: &str| QualityRepairRecord {
        pass: pass.into(),
        family: family.into(),
        node_id: String::new(),
        node_name: Some(name.into()),
        detail: detail.into(),
    };
    report.ingest_repairs(
        &["layout".into(), "overflow".into()],
        &[
            record("unify-section-margins", "layout", "Agenda", "gap 24 → 16"),
            record(
                "text-fit",
                "overflow",
                "Cover title",
                "fontSize 72 → 64, height (unset) → 180",
            ),
            record(
                "chrome-dedupe",
                "structure",
                "Footer",
                "removed frame (+3 descendant(s))",
            ),
        ],
        &[],
    );
    let remaining = |topic, source: &str, name: &str, reason: &str| QualityItem {
        topic,
        source: source.into(),
        node_id: None,
        node_name: Some(name.into()),
        board_id: None,
        detail: reason.into(),
    };
    report.ingest_audit(
        &[QualityTopic::Contrast, QualityTopic::Charts],
        vec![
            remaining(
                QualityTopic::Contrast,
                "text-bg-contrast",
                "Subtitle",
                "contrast 2.1:1 < 3:1 against #f5f7ff",
            ),
            remaining(
                QualityTopic::Charts,
                "no-baseline-bars",
                "Growth chart",
                "bars have no shared baseline",
            ),
        ],
    );
    let workspace = &mut host.editor_state_mut().editor_ui.workspace;
    workspace.quality = Some(report);
    workspace.quality_open = true;
    host
}

/// The one-click example draft whose AI refine then failed: Home's empty
/// Send on 演示文稿 loads the deck template and queues the refine; the
/// provider ends the turn as `error: …`; the idle edge settles it through
/// the same verdicts the desktop runner uses.
fn refine_failed() -> WidgetHostNative {
    let (w, h) = (1280.0, 800.0);
    let mut host = home_host(Locale::EnUs, HomeFamily::Presentation, HomeDevice::Mobile);
    let send = op_editor_ui::widgets::HomeSurface::for_editor(host.editor_state())
        .expect("home")
        .layout(w, h)
        .send;
    host.apply_press(
        send.origin.x + send.size.x / 2.0,
        send.origin.y + send.size.y / 2.0,
        w,
        h,
    );
    host.apply_release_with_viewport(w, h);
    {
        let state = host.editor_state_mut();
        state.chat.pending_send = None;
        // The launcher clears the hand-off selection, and the transport
        // ends the queued bubble the way it ends every dead turn.
        state.clear_selection();
        if let Some(reply) = state
            .chat
            .messages
            .iter_mut()
            .rev()
            .find(|message| message.role == op_editor_core::ChatRole::Assistant)
        {
            reply.content = "error: 401 invalid api key".into();
            reply.streaming = false;
        }
    }
    let state = host.editor_state();
    let failed = op_editor_core::workspace_run::last_assistant_failed(state);
    let boards = op_editor_core::workspace_run::produced_board_count(state);
    let epoch = state.editor_ui.workspace.run_epoch;
    host.settle_workspace_idle_edge(epoch, boards, failed, w, h);
    host
}

/// A finished deck workspace on a host that can write the share page, so
/// the header carries 分享 beside 导出.
fn workspace_share(locale: Locale) -> WidgetHostNative {
    let mut host = workspace_host(
        locale,
        "ppt-demo.op",
        HomeFamily::Presentation,
        1280.0,
        800.0,
    );
    host.editor_state_mut().editor_ui.deck_html_export_supported = true;
    host
}

/// The sample deck opened as a SHARED document: its file carries a share
/// recipe, so it lands in the workspace with the Make-one-like-this
/// banner.
fn shared_view(locale: Locale) -> WidgetHostNative {
    let (w, h) = (1280.0, 800.0);
    let src = std::fs::read_to_string(resource("ppt-demo.op")).expect("read sample document");
    let loaded = op_pen_loader::load_canonical(&src).expect("parse sample document");
    let meta = op_pen_loader::EditorMeta {
        share_recipe: Some(op_editor_core::ShareRecipe {
            brief: "A 6-slide product intro deck for OpenPencil".into(),
            family: HomeFamily::Presentation,
            device: HomeDevice::Mobile,
            ratio: op_editor_core::SlideRatio::Wide169,
            info_kind: op_editor_core::InfoKind::Data,
            style_guide: Some("editorial-dark".into()),
        }),
        ..op_pen_loader::EditorMeta::default()
    };
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = locale;
    host.editor_state_mut().editor_ui.theme_mode = op_editor_core::ThemeMode::Light;
    host.editor_state_mut().editor_ui.deck_html_export_supported = true;
    host.set_now_ms(1_000);
    host.install_open_document(loaded.value, Some(meta), Some("ppt-demo.op".into()))
        .expect("install shared document");
    host.fit_content_to_viewport(w, h);
    assert!(host.adopt_shared_recipe_view(w, h));
    // The sample's CJK face is not installed on every shot machine; its
    // one-shot prompt is not what these shots are about.
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.missing_fonts_modal_open = false;
    ui.missing_fonts_pending_open_modal = false;
    host
}

/// Home right after the shared view's Make-one-like-this: the recipe's
/// task and options selected, the brief pre-filled and editable.
fn make_same_home(locale: Locale) -> WidgetHostNative {
    let mut host = shared_view(locale);
    assert!(host.editor_state_mut().editor_ui.begin_make_same(1_000));
    host.editor_state_mut().editor_ui.agent_settings.connected[0] = true;
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
        ("home-app-mobile-ru", 1440.0, 900.0, || {
            home_host(Locale::Ru, HomeFamily::AppUi, HomeDevice::Mobile)
        }),
        // A narrower window squeezes the explore cards' copy column —
        // the long-text locales must wrap left of the art.
        ("home-explore-1180-en", 1180.0, 900.0, || {
            home_host(Locale::EnUs, HomeFamily::AppUi, HomeDevice::Mobile)
        }),
        ("home-explore-1180-de", 1180.0, 900.0, || {
            home_host(Locale::De, HomeFamily::AppUi, HomeDevice::Mobile)
        }),
        ("home-explore-1180-ru", 1180.0, 900.0, || {
            home_host(Locale::Ru, HomeFamily::AppUi, HomeDevice::Mobile)
        }),
        ("home-deck-43-zh", 1440.0, 900.0, || {
            let mut host = home_host(Locale::ZhCn, HomeFamily::Presentation, HomeDevice::Mobile);
            host.editor_state_mut()
                .editor_ui
                .home
                .set_ratio(op_editor_core::SlideRatio::Classic43);
            host
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
        ("home-focus-ring-en", 1440.0, 900.0, home_focused),
        ("ws-focus-ring-en", 1280.0, 800.0, workspace_focused),
        ("ws-quality-en", 1280.0, 800.0, || {
            workspace_quality(Locale::EnUs)
        }),
        ("ws-quality-ja", 1280.0, 800.0, || {
            workspace_quality(Locale::Ja)
        }),
        ("ws-refine-failed-en", 1280.0, 800.0, refine_failed),
        ("ws-share-header-en", 1280.0, 800.0, || {
            workspace_share(Locale::EnUs)
        }),
        ("ws-share-header-ru", 1280.0, 800.0, || {
            workspace_share(Locale::Ru)
        }),
        ("ws-shared-view-en", 1280.0, 800.0, || {
            shared_view(Locale::EnUs)
        }),
        ("ws-shared-view-zh", 1280.0, 800.0, || {
            shared_view(Locale::ZhCn)
        }),
        ("ws-shared-view-ja", 1280.0, 800.0, || {
            shared_view(Locale::Ja)
        }),
        ("home-make-same-zh", 1440.0, 900.0, || {
            make_same_home(Locale::ZhCn)
        }),
        ("home-make-same-en", 1440.0, 900.0, || {
            make_same_home(Locale::EnUs)
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
