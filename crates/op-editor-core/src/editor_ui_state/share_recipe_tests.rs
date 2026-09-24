//! Tests for the share recipe: sanitization, capture, and the
//! Make-one-like-this prefill.

use super::*;
use crate::editor_ui_state::{EditorUiState, WorkspacePhase};

fn recipe(brief: &str) -> ShareRecipe {
    ShareRecipe {
        brief: brief.to_string(),
        family: HomeFamily::Presentation,
        device: HomeDevice::Mobile,
        ratio: SlideRatio::Classic43,
        info_kind: InfoKind::Data,
        style_guide: Some("editorial-dark".to_string()),
    }
}

#[test]
fn option_ids_round_trip() {
    for device in [HomeDevice::Mobile, HomeDevice::Desktop] {
        assert_eq!(HomeDevice::from_id(device.id()), Some(device));
    }
    for ratio in [SlideRatio::Wide169, SlideRatio::Classic43] {
        assert_eq!(SlideRatio::from_id(ratio.id()), Some(ratio));
    }
    for kind in [InfoKind::Data, InfoKind::Flow, InfoKind::Comparison] {
        assert_eq!(InfoKind::from_id(kind.id()), Some(kind));
    }
    assert_eq!(HomeDevice::from_id("watch"), None);
}

#[test]
fn local_paths_are_cut_out_of_cjk_prose_without_touching_it() {
    let text = sanitize_share_text("参考/Users/alice/Desktop/mood.png的配色做一份年终汇报", &[]);
    assert_eq!(text, format!("参考{SHARE_REDACTED}的配色做一份年终汇报"));
    for path in [
        "~/Pictures/logo.png",
        "C:\\Users\\bob\\brief.docx",
        "file:///tmp/a.png",
        "/opt/brand/kit/logo.svg",
        "/home/carol/notes.md",
    ] {
        let text = sanitize_share_text(&format!("use {path} please"), &[]);
        assert_eq!(text, format!("use {SHARE_REDACTED} please"), "{path}");
    }
}

#[test]
fn credentials_are_redacted_but_ordinary_words_survive() {
    // Assembled at run time so the source never carries a credential-shaped
    // literal (the collab security boundary scan rejects those).
    let brief = format!(
        "key {}-{}-abcdefghijklmnop and password: hunter2 for {}_0123456789abcdefghij",
        "sk", "ant-api03", "ghp"
    );
    let text = sanitize_share_text(&brief, &[]);
    assert!(!text.contains("sk-ant"), "{text}");
    assert!(!text.contains("hunter2"), "{text}");
    assert!(!text.contains("ghp_"), "{text}");
    let long = "Q2hhbmdlTWUxMjM0NTY3ODkwYWJjZGVmZ2hpamtsbW5vcA";
    assert_eq!(sanitize_share_text(long, &[]), SHARE_REDACTED);
    // Design briefs talk about tokens and passwords all the time.
    let plain = "A password manager app with design tokens, 5 slides, 16:9, https://example.com";
    assert_eq!(sanitize_share_text(plain, &[]), plain);
}

#[test]
fn extra_terms_redact_whole_words_only() {
    let text = sanitize_share_text("made by dana, dance the deck", &["dana"]);
    assert_eq!(text, format!("made by {SHARE_REDACTED}, dance the deck"));
    // Too-short terms would shred ordinary words; they are ignored.
    assert_eq!(sanitize_share_text("an app", &["an"]), "an app");
}

#[test]
fn sanitized_recipe_drops_a_style_guide_that_is_not_a_registry_name() {
    let mut raw = recipe("deck for /Users/alice/q4.key");
    raw.style_guide = Some("/Users/alice/styles/mine.md".to_string());
    let clean = raw.sanitized(&["alice"]);
    assert_eq!(clean.style_guide, None);
    assert!(!clean.brief.contains("alice"), "{}", clean.brief);
    assert_eq!(
        recipe("x").sanitized(&[]).style_guide.as_deref(),
        Some("editorial-dark")
    );
}

#[test]
fn brief_is_clamped() {
    let long = "字".repeat(SHARE_BRIEF_MAX_CHARS + 50);
    let clean = recipe("").with_brief(&long, &[]);
    assert_eq!(clean.brief.chars().count(), SHARE_BRIEF_MAX_CHARS);
}

#[test]
fn a_live_run_is_the_recipe_and_it_is_sanitized() {
    let mut ui = EditorUiState::default();
    assert_eq!(ui.share_recipe(), None);
    let options = TaskDraft {
        ratio: SlideRatio::Classic43,
        ..TaskDraft::default()
    };
    ui.open_workspace_for_generation(
        HomeFamily::Presentation,
        "年终汇报 /Users/alice/data.xlsx",
        options,
        0,
        1,
        None,
    );
    ui.pinned_style_guide = Some("swiss-grid".to_string());
    let captured = ui.share_recipe().expect("a live run has a recipe");
    assert_eq!(captured.family, HomeFamily::Presentation);
    assert_eq!(captured.ratio, SlideRatio::Classic43);
    assert_eq!(captured.style_guide.as_deref(), Some("swiss-grid"));
    assert_eq!(captured.brief, format!("年终汇报 {SHARE_REDACTED}"));
}

#[test]
fn an_opened_recipe_opens_the_workspace_settled_with_the_banner() {
    let mut ui = EditorUiState::default();
    assert!(!ui.open_workspace_for_shared_recipe(5));
    ui.home.recipe = Some(recipe("五页产品介绍"));
    assert!(ui.open_workspace_for_shared_recipe(5));
    assert!(ui.workspace.visible && ui.workspace.shared_view);
    assert_eq!(ui.workspace.phase, WorkspacePhase::Done);
    assert_eq!(ui.workspace.family, HomeFamily::Presentation);
    assert!(ui.workspace.make_same_banner_visible());
    assert!(
        !ui.sidebar_open,
        "a shared document opens with the dock shut"
    );
    // The shared view reports the file's recipe, not a run of its own.
    assert_eq!(ui.share_recipe(), ui.home.recipe);
}

#[test]
fn make_same_prefills_home_and_pins_the_style() {
    let mut ui = EditorUiState::default();
    ui.home.recipe = Some(recipe("五页产品介绍"));
    ui.open_workspace_for_shared_recipe(5);
    assert!(ui.begin_make_same(9));
    assert!(ui.home.visible);
    assert_eq!(ui.home.task, HomeFamily::Presentation);
    assert_eq!(ui.home.draft, "五页产品介绍");
    assert_eq!(ui.home.input.text(), "五页产品介绍");
    assert_eq!(ui.home.task_draft().ratio, SlideRatio::Classic43);
    assert_eq!(ui.pinned_style_guide.as_deref(), Some("editorial-dark"));
    assert!(
        ui.home.generation_prompt().is_some(),
        "the brief is sendable as is"
    );
    // The brief stays editable: typing appends to the pre-filled text.
    ui.home.insert_text("，蓝色", 10);
    assert_eq!(ui.home.draft, "五页产品介绍，蓝色");

    // Home's send swaps in a fresh document, which drops every pin; the
    // staged press puts the recipe's style back exactly once.
    ui.pinned_style_guide = None;
    ui.restore_make_same_pin();
    assert_eq!(ui.pinned_style_guide.as_deref(), Some("editorial-dark"));
    ui.pinned_style_guide = None;
    ui.restore_make_same_pin();
    assert_eq!(ui.pinned_style_guide, None);
}

#[test]
fn make_same_without_a_recipe_does_nothing() {
    let mut ui = EditorUiState::default();
    assert!(!ui.begin_make_same(1));
    assert!(!ui.home.visible);
    assert!(ui.home.make_same.is_none());
}

#[test]
fn a_new_run_or_document_retires_the_shared_view() {
    let mut ui = EditorUiState::default();
    ui.home.recipe = Some(recipe("x"));
    ui.open_workspace_for_shared_recipe(1);
    ui.workspace.reset_for_new_document();
    assert!(!ui.workspace.shared_view);
    ui.open_workspace_for_shared_recipe(1);
    ui.open_workspace_for_generation(HomeFamily::Web, "y", TaskDraft::default(), 0, 2, None);
    assert!(!ui.workspace.shared_view);
    assert!(!ui.workspace.make_same_banner_visible());
}
