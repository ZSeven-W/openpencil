//! Tablet scenes for `studio_shots`: the touch Studio (Home, the works
//! page and the works reader) at iPad / Android tablet sizes in both
//! orientations.

use super::{resource, Scenario};
use op_editor_core::size_class::size_class;
use op_editor_core::{HomeDevice, HomeFamily, TaskDraft, Tool, WorkspacePhase};
use op_host_native::widget_host::WidgetHostNative;
use op_i18n::Locale;

/// The tablet viewports the scenes cover: iPad Pro 12.9" portrait and
/// landscape, iPad 10.9" portrait, and an Android tablet both ways.
const IPAD_PORTRAIT: (f32, f32) = (1024.0, 1366.0);
const IPAD_LANDSCAPE: (f32, f32) = (1366.0, 1024.0);
const IPAD_AIR_PORTRAIT: (f32, f32) = (820.0, 1180.0);
const ANDROID_PORTRAIT: (f32, f32) = (800.0, 1280.0);
const ANDROID_LANDSCAPE: (f32, f32) = (1280.0, 800.0);

/// A touch host at `w × h` with the size class the mobile shell would
/// compute, on the Home `task` with a usable model selected.
fn tablet_home(locale: Locale, task: HomeFamily, w: f32, h: f32) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.touch = true;
    state.editor_ui.size_class = size_class(w, h);
    state.editor_ui.sidebar_open = state.editor_ui.size_class.is_rail_layout();
    state.editor_ui.locale = locale;
    state.editor_ui.home.visible = true;
    state.editor_ui.home.set_task(task, 1);
    state.editor_ui.home.set_device(HomeDevice::Mobile);
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

/// Home's 作品 page with a few recent documents recorded.
fn tablet_works(locale: Locale, w: f32, h: f32) -> WidgetHostNative {
    let mut host = tablet_reader(locale, "ppt-demo.op", HomeFamily::Presentation, w, h);
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.home.visible = true;
    ui.home.works_open = true;
    ui.recent_files = [
        "季度复盘.op",
        "新品发布会.op",
        "Onboarding flow.op",
        "Coffee shop poster.op",
        "Team offsite deck.op",
    ]
    .iter()
    .enumerate()
    .map(|(i, name)| op_editor_core::RecentFile {
        path: format!("/Documents/{name}"),
        modified_at: 1_700_000_000 - i as u64 * 3_600,
    })
    .collect();
    host
}

/// A tablet reading a finished `family` work over the sample `file`.
fn tablet_reader(
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
    state.editor_ui.touch = true;
    state.editor_ui.size_class = size_class(w, h);
    state.editor_ui.sidebar_open = state.editor_ui.size_class.is_rail_layout();
    state.editor_ui.locale = locale;
    state.editor_ui.theme_mode = op_editor_core::ThemeMode::Light;
    assert!(host.replace_editor_state(state));
    host.set_now_ms(1_000);
    let brief = "为 OpenPencil 做一份产品介绍 PPT";
    host.editor_state_mut()
        .editor_ui
        .open_workspace_for_generation(
            family,
            brief,
            TaskDraft::default(),
            0,
            1_000,
            Some(Tool::Select),
        );
    {
        let chat = &mut host.editor_state_mut().chat;
        chat.messages.push(op_editor_core::ChatMessage::user(brief));
        chat.messages.push(op_editor_core::ChatMessage::assistant(
            "已完成 5 页：封面、目录和三页功能介绍。想改哪里直接告诉我。",
        ));
    }
    host.editor_state_mut().editor_ui.workspace.phase = WorkspacePhase::Done;
    host.frame_reader_board(w, h);
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.missing_fonts_modal_open = false;
    ui.missing_fonts_pending_open_modal = false;
    host
}

/// Press and release at the centre of `rect`.
fn tap(host: &mut WidgetHostNative, rect: op_editor_ui::Rect, w: f32, h: f32) {
    let (x, y) = (
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    );
    host.apply_press(x, y, w, h);
    host.apply_release_with_viewport(w, h);
}

fn reader_layout(host: &WidgetHostNative, w: f32, h: f32) -> op_editor_ui::widgets::ReaderLayout {
    op_editor_ui::widgets::WorksReader::for_editor(host.editor_state())
        .expect("reader visible")
        .layout(w, h)
}

/// The reader on page 3 with 改这一页 tapped: the chat opens bound to it.
fn reader_editing(locale: Locale, w: f32, h: f32) -> WidgetHostNative {
    let mut host = tablet_reader(locale, "ppt-demo.op", HomeFamily::Presentation, w, h);
    let third = reader_layout(&host, w, h).thumbs[2].1;
    tap(&mut host, third, w, h);
    let edit = reader_layout(&host, w, h).edit_page;
    tap(&mut host, edit, w, h);
    host
}

/// A page edit in flight on page 2: status names the page, Stop offered.
fn reader_generating(locale: Locale, w: f32, h: f32) -> WidgetHostNative {
    let mut host = tablet_reader(locale, "ppt-demo.op", HomeFamily::Presentation, w, h);
    let boards = op_editor_core::preview_slideshow::active_page_boards(host.editor_state());
    let workspace = &mut host.editor_state_mut().editor_ui.workspace;
    workspace.select_board(1, boards.len());
    workspace.stage_page_edit(boards[1].clone(), 1);
    workspace.begin_page_edit_turn();
    workspace.phase = WorkspacePhase::Generating;
    host.frame_reader_board(w, h);
    host
}

/// The works grid scrolled into view.
fn works_scrolled(locale: Locale, w: f32, h: f32) -> WidgetHostNative {
    let mut host = tablet_works(locale, w, h);
    let max = op_editor_ui::widgets::HomeSurface::for_editor(host.editor_state())
        .expect("home")
        .tablet_max_scroll(w, h);
    host.editor_state_mut().editor_ui.home.scroll_y = max;
    host
}

const IP: (f32, f32) = IPAD_PORTRAIT;
const IL: (f32, f32) = IPAD_LANDSCAPE;
const AP: (f32, f32) = ANDROID_PORTRAIT;
const AL: (f32, f32) = ANDROID_LANDSCAPE;
const AIR: (f32, f32) = IPAD_AIR_PORTRAIT;

pub(super) fn scenarios() -> Vec<Scenario> {
    vec![
        ("tablet-home-ipad-portrait-zh", IP.0, IP.1, || {
            tablet_home(Locale::ZhCn, HomeFamily::Presentation, IP.0, IP.1)
        }),
        ("tablet-home-ipad-landscape-zh", IL.0, IL.1, || {
            tablet_home(Locale::ZhCn, HomeFamily::Presentation, IL.0, IL.1)
        }),
        ("tablet-home-ipad-air-portrait-en", AIR.0, AIR.1, || {
            tablet_home(Locale::EnUs, HomeFamily::AppUi, AIR.0, AIR.1)
        }),
        ("tablet-home-android-portrait-zh", AP.0, AP.1, || {
            tablet_home(Locale::ZhCn, HomeFamily::AppUi, AP.0, AP.1)
        }),
        ("tablet-home-android-landscape-en", AL.0, AL.1, || {
            tablet_home(Locale::EnUs, HomeFamily::AppUi, AL.0, AL.1)
        }),
        ("tablet-works-ipad-portrait-zh", IP.0, IP.1, || {
            tablet_works(Locale::ZhCn, IP.0, IP.1)
        }),
        ("tablet-works-ipad-landscape-zh", IL.0, IL.1, || {
            works_scrolled(Locale::ZhCn, IL.0, IL.1)
        }),
        ("tablet-works-android-portrait-en", AP.0, AP.1, || {
            works_scrolled(Locale::EnUs, AP.0, AP.1)
        }),
        ("tablet-reader-ipad-landscape-zh", IL.0, IL.1, || {
            tablet_reader(
                Locale::ZhCn,
                "ppt-demo.op",
                HomeFamily::Presentation,
                IL.0,
                IL.1,
            )
        }),
        ("tablet-reader-ipad-portrait-zh", IP.0, IP.1, || {
            tablet_reader(
                Locale::ZhCn,
                "ppt-demo.op",
                HomeFamily::Presentation,
                IP.0,
                IP.1,
            )
        }),
        ("tablet-reader-android-landscape-en", AL.0, AL.1, || {
            tablet_reader(
                Locale::EnUs,
                "ppt-demo.op",
                HomeFamily::Presentation,
                AL.0,
                AL.1,
            )
        }),
        ("tablet-reader-android-portrait-en", AP.0, AP.1, || {
            tablet_reader(
                Locale::EnUs,
                "ppt-demo.op",
                HomeFamily::Presentation,
                AP.0,
                AP.1,
            )
        }),
        ("tablet-reader-app-ipad-landscape-zh", IL.0, IL.1, || {
            tablet_reader(Locale::ZhCn, "sample.op", HomeFamily::AppUi, IL.0, IL.1)
        }),
        ("tablet-reader-edit-ipad-landscape-zh", IL.0, IL.1, || {
            reader_editing(Locale::ZhCn, IL.0, IL.1)
        }),
        ("tablet-reader-edit-ipad-portrait-zh", IP.0, IP.1, || {
            reader_editing(Locale::ZhCn, IP.0, IP.1)
        }),
        (
            "tablet-reader-generating-ipad-landscape-zh",
            IL.0,
            IL.1,
            || reader_generating(Locale::ZhCn, IL.0, IL.1),
        ),
        (
            "tablet-reader-generating-android-portrait-en",
            AP.0,
            AP.1,
            || reader_generating(Locale::EnUs, AP.0, AP.1),
        ),
    ]
}
