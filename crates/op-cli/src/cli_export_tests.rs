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
    assert!(target.unwrap_err().contains("--item and --selection"));

    let format = parse_args(&args(&[
        "export",
        "--output",
        "/tmp/node.png",
        "--format",
        "png",
        "--formats",
        "jpeg",
    ]));
    assert!(format.unwrap_err().contains("--format and --formats"));
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
