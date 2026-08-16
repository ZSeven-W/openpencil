use super::*;
use base64::Engine as _;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn export_item_maps_to_dedicated_command() {
    let parsed = parse_args(&args(&[
        "export",
        "--item",
        "page-2",
        "--output",
        "/tmp/page.png",
        "--format",
        "png",
        "--scale",
        "2",
    ]))
    .expect("parse export");
    assert_eq!(
        parsed.command,
        Command::Export {
            item_id: Some("page-2".into()),
            selection: false,
            output: "/tmp/page.png".into(),
            format: "png".into(),
            scale: Some("2".into()),
        }
    );
}

#[test]
fn export_without_item_means_live_selection() {
    let parsed = parse_args(&args(&[
        "export",
        "--output",
        "/tmp/selected.png",
        "--format",
        "png",
    ]))
    .expect("parse selection export");
    assert!(matches!(
        parsed.command,
        Command::Export {
            item_id: None,
            selection: false,
            ..
        }
    ));
}

#[test]
fn export_selection_flag_means_live_selection() {
    let parsed = parse_args(&args(&[
        "export",
        "--selection",
        "--output",
        "/tmp/selected.png",
    ]))
    .expect("parse --selection export");
    assert!(matches!(
        parsed.command,
        Command::Export {
            item_id: None,
            selection: true,
            ..
        }
    ));
}

#[test]
fn export_accepts_issue_formats_alias() {
    let parsed = parse_args(&args(&[
        "export",
        "--item",
        "page-1",
        "--output",
        "/tmp/page.png",
        "--formats",
        "png",
    ]))
    .expect("parse --formats alias");
    assert!(matches!(
        parsed.command,
        Command::Export { format, .. } if format == "png"
    ));
}

#[test]
fn export_rejects_conflicting_target_and_format_flags() {
    let target = parse_args(&args(&[
        "export",
        "--item",
        "n1",
        "--selection",
        "--output",
        "/tmp/node.png",
    ]));
    assert!(target
        .unwrap_err()
        .to_string()
        .contains("--item and --selection"));

    let format = parse_args(&args(&[
        "export",
        "--output",
        "/tmp/node.png",
        "--format",
        "png",
        "--formats",
        "jpeg",
    ]));
    assert!(format
        .unwrap_err()
        .to_string()
        .contains("--format and --formats"));
}

#[test]
fn write_export_response_decodes_png_to_exact_path() {
    let path =
        std::env::temp_dir().join(format!("op-cli-export-{}-selected.png", std::process::id()));
    let png = [0x89, b'P', b'N', b'G', 13, 10, 26, 10];
    let response = serde_json::json!({
        "itemId": "n1",
        "itemType": "node",
        "format": "png",
        "bytes_base64": base64::engine::general_purpose::STANDARD.encode(png),
    })
    .to_string();

    let output = export_cli::write_export_response(&response, &path).expect("write export");
    assert_eq!(std::fs::read(&path).expect("read export"), png);
    assert!(output.contains("\"itemType\":\"node\""), "{output}");
    std::fs::remove_file(path).ok();
}

#[test]
fn write_export_response_rejects_invalid_payloads() {
    let path = std::env::temp_dir().join("op-cli-export-invalid.png");
    assert!(export_cli::write_export_response("not-json", &path).is_err());
    assert!(export_cli::write_export_response(
        r#"{"itemId":"n1","itemType":"node","format":"png","bytes_base64":"%%%"}"#,
        &path,
    )
    .is_err());
}

#[test]
fn export_deck_defaults_to_pptx_and_requires_an_output() {
    let parsed = parse_args(&args(&["export-deck", "--output", "/tmp/deck.pptx"]))
        .expect("parse export-deck");
    assert!(matches!(
        parsed.command,
        Command::ExportDeck { ref format, .. } if format == "pptx"
    ));

    // PowerPoint is the format users reach for by name, so it is the default;
    // the other two must still be selectable.
    for format in ["html", "pdf"] {
        let parsed = parse_args(&args(&[
            "export-deck",
            "--output",
            "/tmp/deck.out",
            "--format",
            format,
        ]))
        .expect("parse deck format");
        assert!(matches!(
            parsed.command,
            Command::ExportDeck { format: ref parsed_format, .. } if parsed_format == format
        ));
    }

    assert!(parse_args(&args(&["export-deck"]))
        .unwrap_err()
        .to_string()
        .contains("--output"));
}

#[test]
fn export_deck_rejects_a_node_export_format() {
    // png is valid for `op export` and meaningless for a deck; accepting it
    // would reach the daemon and fail there instead of at the typo.
    let error = parse_args(&args(&[
        "export-deck",
        "--output",
        "/tmp/deck.pptx",
        "--format",
        "png",
    ]))
    .unwrap_err()
    .to_string();
    assert!(error.contains("unsupported deck format"), "{error}");
}

#[test]
fn use_template_accepts_a_bare_id_or_a_flag() {
    for argv in [
        vec!["use-template", "slide-deck"],
        vec!["use-template", "--template", "slide-deck"],
    ] {
        let parsed = parse_args(&args(&argv)).expect("parse use-template");
        assert!(matches!(
            parsed.command,
            Command::UseTemplate { ref template_id } if template_id == "slide-deck"
        ));
    }
    assert!(parse_args(&args(&["use-template"]))
        .unwrap_err()
        .to_string()
        .contains("template id"));
}

#[test]
fn templates_filters_are_optional() {
    let bare = parse_args(&args(&["templates"])).expect("parse templates");
    assert!(matches!(
        bare.command,
        Command::Templates {
            scene: None,
            tag: None
        }
    ));
    let filtered = parse_args(&args(&["templates", "--scene", "slides"])).expect("parse filter");
    assert!(matches!(
        filtered.command,
        Command::Templates { scene: Some(ref scene), .. } if scene == "slides"
    ));
}

#[test]
fn styles_takes_a_bare_id_or_filters() {
    let listing = parse_args(&args(&["styles"])).expect("parse styles");
    assert!(matches!(
        listing.command,
        Command::Styles {
            id: None,
            tag: None,
            platform: None
        }
    ));

    let one = parse_args(&args(&["styles", "user:my-brand"])).expect("parse styles id");
    assert!(matches!(
        one.command,
        Command::Styles { id: Some(ref id), .. } if id == "user:my-brand"
    ));

    let filtered =
        parse_args(&args(&["styles", "--platform", "slides"])).expect("parse styles filter");
    assert!(matches!(
        filtered.command,
        Command::Styles { platform: Some(ref platform), .. } if platform == "slides"
    ));
}

#[test]
fn export_frames_requires_a_directory_and_rejects_pdf() {
    let parsed = parse_args(&args(&["export-frames", "--output-dir", "/tmp/frames"]))
        .expect("parse export-frames");
    assert!(matches!(
        parsed.command,
        Command::ExportFrames { ref format, .. } if format == "png"
    ));

    assert!(parse_args(&args(&["export-frames"]))
        .unwrap_err()
        .to_string()
        .contains("--output-dir"));

    // A frame batch is images; pdf belongs to `op export` / `op export-deck`.
    assert!(parse_args(&args(&[
        "export-frames",
        "--output-dir",
        "/tmp/frames",
        "--format",
        "pdf",
    ]))
    .unwrap_err()
    .to_string()
    .contains("unsupported frame format"));
}

#[test]
fn every_mcp_backed_alias_uses_running_endpoint_discovery() {
    for argv in [
        vec!["export-deck", "--output", "/tmp/deck.pptx"],
        vec!["export-frames", "--output-dir", "/tmp/frames"],
        vec!["templates"],
        vec!["use-template", "slide-deck"],
        vec!["styles"],
    ] {
        let parsed = parse_args(&args(&argv)).expect("parse MCP-backed alias");
        assert!(
            !parsed.port_explicit,
            "the fixture must exercise discovery rather than an explicit port: {argv:?}"
        );
        assert!(
            command_needs_server(&parsed.command),
            "MCP-backed alias must discover the running endpoint: {argv:?}"
        );
    }
}
