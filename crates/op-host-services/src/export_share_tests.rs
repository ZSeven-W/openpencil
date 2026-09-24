//! Share-package tests: the page carries the viewer and the document,
//! navigates every board, and never carries what must not leave.

use super::*;
use crate::export_share_template::DOCUMENT_ELEMENT_ID;
use op_editor_core::{HomeFamily, TaskDraft};

fn state_from(source: &str) -> EditorState {
    let doc = jian_ops_schema::load_str(source)
        .expect("fixture JSON parses")
        .value;
    EditorState::from_document(doc)
}

fn two_boards() -> EditorState {
    state_from(
        r##"{"version":"1.0.0","name":"Coffee deck","children":[
            {"type":"frame","id":"f1","name":"Cover","x":0,"y":0,"width":400,"height":300,
             "fill":[{"type":"solid","color":"#ff0000"}],"children":[
               {"type":"text","id":"t1","x":20,"y":20,"width":300,"height":48,
                "content":"Morning Roast","fontSize":32,
                "fill":[{"type":"solid","color":"#101828"}]}]},
            {"type":"frame","id":"f2","name":"Menu","x":500,"y":0,"width":400,"height":300,
             "fill":[{"type":"solid","color":"#00ff00"}]}
        ]}"##,
    )
}

fn options() -> ShareOptions {
    ShareOptions {
        redact_terms: vec!["alice".to_string()],
        document_dir: None,
        locale: Locale::EnUs,
    }
}

/// The embedded `.op`, parsed.
fn embedded_document(html: &str) -> serde_json::Value {
    let open = format!("id=\"{DOCUMENT_ELEMENT_ID}\">");
    let start = html.find(&open).expect("document block") + open.len();
    let end = start + html[start..].find("</script>").expect("block end");
    serde_json::from_str(&html[start..end]).expect("embedded document is JSON")
}

fn with_live_run(mut state: EditorState, brief: &str) -> EditorState {
    state.editor_ui.open_workspace_for_generation(
        HomeFamily::Presentation,
        brief,
        TaskDraft::default(),
        0,
        1,
        None,
    );
    state.editor_ui.pinned_style_guide = Some("editorial-dark".to_string());
    state
}

#[test]
fn the_page_contains_every_board_the_viewer_and_the_document() {
    let state = with_live_run(two_boards(), "五页咖啡品牌介绍");
    let (html, package) = render_share_html(&state, &options()).expect("share renders");

    assert_eq!(package.boards, 2);
    assert_eq!(html.matches("class=\"slot").count(), 2);
    assert!(html.contains(">Morning Roast<"), "text is real text");
    // The viewer: navigation, counter, fit, and the download action.
    for needle in [
        "ArrowRight",
        "ArrowLeft",
        "addEventListener('resize', fit)",
        "<span id=\"counter\">1 / 2</span>",
        "id=\"make\"",
        "link.download",
        "Make one like this",
    ] {
        assert!(html.contains(needle), "missing {needle}");
    }
    // Self-contained: nothing is fetched.
    assert!(!html.contains("<link"), "{html}");
    assert!(!html.contains("src=\"http"), "{html}");

    // The document round-trips and carries its recipe.
    let doc = embedded_document(&html);
    assert_eq!(doc["name"], "Coffee deck");
    let recipe = &doc["editorMeta"]["shareRecipe"];
    assert_eq!(recipe["family"], "presentation");
    assert_eq!(recipe["brief"], "五页咖啡品牌介绍");
    assert_eq!(recipe["styleGuide"], "editorial-dark");
    let meta = op_pen_loader::extract_editor_meta(&serde_json::to_string(&doc).unwrap())
        .and_then(|meta| meta.share_recipe)
        .expect("the downloaded .op offers make-same");
    assert_eq!(meta.family, HomeFamily::Presentation);
    assert!(html.contains("data-file=\"Coffee deck.op\""), "{html}");
}

#[test]
fn nothing_identifying_or_secret_leaves_in_the_package() {
    let mut state = state_from(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"Board","x":0,"y":0,"width":200,"height":200,
             "fill":[{"type":"solid","color":"#ffffff"}],"children":[
               {"type":"image","id":"i1","x":0,"y":0,"width":50,"height":50,
                "src":"/Users/alice/Pictures/never-there.png"}]}
        ]}"##,
    );
    state = with_live_run(
        state,
        "deck by alice from /Users/alice/brief.txt key sk-ant-api03-SECRETSECRETSECRET token=abc123",
    );
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(op_editor_core::BuiltinAgentConfig {
            id: "anthropic".into(),
            preset: op_editor_core::BuiltinAgentPresetKey::Anthropic,
            display_name: "Anthropic".into(),
            kind: op_editor_core::BuiltinAgentKind::Anthropic,
            api_key: "sk-live-DO-NOT-SHIP-0123456789".into(),
            models: vec!["claude".into()],
            base_url: "https://api.anthropic.com".into(),
            enabled: true,
        });
    state.editor_ui.file_name_display = Some("/Users/alice/Documents/secret-plans.op".into());

    let (html, _) = render_share_html(&state, &options()).expect("share renders");
    for forbidden in [
        "/Users/",
        "alice",
        "sk-ant",
        "SECRETSECRET",
        "abc123",
        "sk-live",
        "DO-NOT-SHIP",
        "api.anthropic.com",
        "apiKey",
        "secret-plans",
    ] {
        assert!(!html.contains(forbidden), "package leaked {forbidden:?}");
    }
    let doc = embedded_document(&html);
    assert_eq!(doc["children"][0]["children"][0]["src"], "");
    let brief = doc["editorMeta"]["shareRecipe"]["brief"]
        .as_str()
        .expect("brief");
    assert!(brief.starts_with("deck by "), "{brief}");
}

#[test]
fn every_page_contributes_its_boards_in_order() {
    let state = state_from(
        r##"{"version":"1.0.0","pages":[
            {"id":"p1","name":"Mobile","children":[
              {"type":"frame","id":"a","name":"A","x":0,"y":0,"width":100,"height":100,
               "fill":[{"type":"solid","color":"#111111"}]}]},
            {"id":"p2","name":"Desktop","children":[
              {"type":"frame","id":"b","name":"B","x":0,"y":0,"width":300,"height":100,
               "fill":[{"type":"solid","color":"#222222"}]}]}
        ]}"##,
    );
    let (html, package) = render_share_html(&state, &options()).expect("share renders");
    assert_eq!(package.boards, 2);
    let mobile = html.find("data-page=\"Mobile\"").expect("page one");
    let desktop = html.find("data-page=\"Desktop\"").expect("page two");
    assert!(mobile < desktop);
    // No recipe (not a Studio document): the page still shares, with no
    // how-it-was-made line.
    assert!(!html.contains("class=\"recipe\""), "{html}");
}

#[test]
fn script_breaking_text_cannot_escape_the_document_block() {
    let state = state_from(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"x</script><script>alert(1)</script>","x":0,"y":0,
             "width":100,"height":100,"fill":[{"type":"solid","color":"#ffffff"}]}
        ]}"##,
    );
    let (html, _) = render_share_html(&state, &options()).expect("share renders");
    // The document block, the chrome table, the chrome swap and the viewer.
    assert_eq!(html.matches("</script>").count(), 4, "{html}");
    assert!(!html.contains("<script>alert"), "{html}");
    let doc = embedded_document(&html);
    assert_eq!(
        doc["children"][0]["name"],
        "x</script><script>alert(1)</script>"
    );
}

#[test]
fn a_document_with_no_visible_board_writes_nothing() {
    let state = state_from(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","x":0,"y":0,"width":40,"height":20,"visible":false}
        ]}"##,
    );
    let target = std::env::temp_dir().join(format!("op-share-empty-{}.html", std::process::id()));
    assert_eq!(
        export_share_html(&state, &target, &options()),
        Err(ExportError::NothingToExport)
    );
    assert!(!target.exists());
}

#[test]
fn the_sample_deck_shares_as_a_small_file() {
    let path = format!(
        "{}/../../packaging/ios/Resources/ppt-demo.op",
        env!("CARGO_MANIFEST_DIR")
    );
    let src = std::fs::read_to_string(path).expect("sample");
    let doc = op_pen_loader::load_canonical(&src)
        .expect("sample parses")
        .value;
    let state = with_live_run(EditorState::from_document(doc), "产品介绍 deck");
    let target = std::env::temp_dir().join(format!("op-share-sample-{}.html", std::process::id()));
    let package = export_share_html(&state, &target, &options()).expect("sample shares");
    let written = std::fs::metadata(&target).expect("written").len() as usize;
    assert_eq!(written, package.bytes);
    eprintln!(
        "share sample: {} boards, {} bytes, {} raster fallbacks",
        package.boards, package.bytes, package.raster_fallbacks
    );
    assert!(package.boards >= 1);
    assert!(package.bytes < 15 * 1024 * 1024, "{package:?}");
    // `OP_SHARE_SAMPLE_OUT=<file>` keeps a copy for a browser check.
    if let Ok(keep) = std::env::var("OP_SHARE_SAMPLE_OUT") {
        std::fs::copy(&target, keep).expect("keep the sample page");
    }
    let _ = std::fs::remove_file(&target);
}

/// The embedded chrome table, parsed.
fn embedded_chrome(html: &str) -> serde_json::Value {
    let open = format!("id=\"{}\">", crate::export_share_chrome::CHROME_ELEMENT_ID);
    let start = html.find(&open).expect("chrome block") + open.len();
    let end = start + html[start..].find("</script>").expect("block end");
    serde_json::from_str(&html[start..end]).expect("chrome table is JSON")
}

#[test]
fn the_chrome_follows_the_recipients_language_with_the_author_as_fallback() {
    let state = with_live_run(two_boards(), "five-slide coffee brand deck");
    let mut options = options();
    options.locale = Locale::De;
    let (html, _) = render_share_html(&state, &options).expect("share renders");

    // Server-rendered in the author's locale: the no-script fallback.
    assert!(html.contains("<html lang=\"de\">"), "{html}");
    let chrome = embedded_chrome(&html);
    assert_eq!(chrome["author"], "de");
    let locales = chrome["locales"].as_object().expect("locale map");
    assert_eq!(locales.len(), Locale::ALL.len());
    for locale in Locale::ALL {
        let entry = &locales[locale.code()];
        for field in ["make", "hint", "made", "prev", "next", "recipe"] {
            assert!(
                !entry[field].as_str().unwrap_or_default().is_empty(),
                "{} {field}",
                locale.code()
            );
        }
    }
    assert_eq!(locales["en-US"]["make"], "Make one like this");
    assert_ne!(locales["zh-CN"]["make"], locales["en-US"]["make"]);
    assert_ne!(locales["ja"]["recipe"], locales["en-US"]["recipe"]);
    // The swap runs from the recipient's browser languages.
    assert!(html.contains("navigator.languages"));
    assert!(html.contains("class=\"made\""));
}

#[test]
fn the_recipe_line_names_the_style_the_way_a_person_reads_it() {
    let state = with_live_run(two_boards(), "five-slide coffee brand deck");
    let (html, _) = render_share_html(&state, &options()).expect("share renders");
    let line_start = html.find("<div class=\"recipe\">").expect("recipe line");
    let line_end = line_start + html[line_start..].find("</div>").expect("line end");
    let line = &html[line_start..line_end];
    assert!(line.contains("Editorial Dark"), "{line}");
    assert!(!line.contains("editorial-dark"), "{line}");
    let chrome = embedded_chrome(&html);
    assert!(chrome["locales"]["zh-CN"]["recipe"]
        .as_str()
        .expect("zh-CN recipe")
        .contains("Editorial Dark"));
    // The recipe itself keeps the id the recipient's editor pins.
    assert_eq!(
        embedded_document(&html)["editorMeta"]["shareRecipe"]["styleGuide"],
        "editorial-dark"
    );
}

#[test]
fn style_display_names_humanize_corpus_ids() {
    use crate::export_share_chrome::style_display_name;
    assert_eq!(
        style_display_name("agency-editorial-light"),
        "Agency Editorial Light"
    );
}
