use super::*;
use op_editor_core::EditorCommand;

fn kit() -> BrandKit {
    op_brand::extract_from_html(
        "<style>body{background:#FFFDF8;color:#1C1917} .cta{background:#0E7C66;color:#fff;border-radius:10px}</style>\
         <a class=cta>Go</a>",
        &[],
        "Verdant Loop",
    )
}

fn args(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn the_marker_literal_matches_across_crates() {
    assert_eq!(op_brand::BRAND_KIT_MARKER, op_editor_core::BRAND_KIT_MARKER);
}

#[test]
fn extract_only_returns_the_kit_without_a_command() {
    let tool = brand_extract_snapshot(&EditorState::new());
    let out = tool.call_with(&args(&[("url", "https://verdant.example")]), &|source| {
        assert_eq!(
            source,
            &BrandSourceArg::Url("https://verdant.example".into())
        );
        Ok(kit())
    });
    let ToolOutcome::Ok(map) = out else {
        panic!("plain Ok without apply");
    };
    assert_eq!(map["applied"], "false");
    let json: serde_json::Value = serde_json::from_str(&map["kit"]).unwrap();
    assert_eq!(json["light"]["--primary"], "#0E7C66");
    assert_eq!(map["variableCount"], "40");
}

#[test]
fn apply_emits_one_batch_the_host_applies_as_one_undo_step() {
    let mut state = EditorState::new();
    let tool = brand_extract_snapshot(&state);
    let out = tool.call_with(
        &args(&[("imagePath", "/fixtures/brand.png"), ("apply", "true")]),
        &|source| {
            assert!(matches!(source, BrandSourceArg::ImagePath(_)));
            Ok(kit())
        },
    );
    let ToolOutcome::OkWithCommand(map, command) = out else {
        panic!("apply returns a command");
    };
    assert_eq!(map["designMdWritten"], "true");
    assert!(matches!(command, EditorCommand::Batch { .. }));
    let before = state.history.past.len();
    assert!(state.apply(command));
    assert_eq!(state.history.past.len(), before + 1);
    assert!(state
        .doc
        .variables
        .as_ref()
        .unwrap()
        .contains_key("--primary"));
}

#[test]
fn argument_errors_are_typed() {
    let tool = brand_extract_snapshot(&EditorState::new());
    let never = |_: &BrandSourceArg| -> Result<BrandKit, BrandError> { panic!("not reached") };
    assert!(matches!(
        tool.call_with(&BTreeMap::new(), &never),
        ToolOutcome::Err(ToolErrorCode::MissingArgument, _)
    ));
    assert!(matches!(
        tool.call_with(
            &args(&[("url", "https://a.example"), ("imagePath", "x.png")]),
            &never
        ),
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
    ));
    assert!(matches!(
        tool.call_with(
            &args(&[("url", "https://a.example"), ("apply", "maybe")]),
            &never
        ),
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
    ));
    let failing = |_: &BrandSourceArg| -> Result<BrandKit, BrandError> {
        Err(BrandError::Fetch {
            url: "https://a.example".into(),
            detail: "timed out".into(),
        })
    };
    assert!(matches!(
        tool.call_with(&args(&[("url", "https://a.example")]), &failing),
        ToolOutcome::Err(ToolErrorCode::ToolFailed, _)
    ));
}

#[test]
fn the_screened_fetcher_refuses_private_hosts_without_dialing() {
    // Screening happens before any socket opens, so this stays offline.
    for url in [
        "http://127.0.0.1/",
        "http://localhost/",
        "http://169.254.169.254/",
    ] {
        assert!(matches!(
            ScreenedBrandFetcher.fetch(url, FetchKind::Page),
            Err(BrandError::Fetch { .. })
        ));
    }
}

#[test]
fn image_path_errors_do_not_panic() {
    assert!(matches!(
        extract_brand_from_image_path("/definitely/not/here.png"),
        Err(BrandError::ImageDecode { .. })
    ));
}
