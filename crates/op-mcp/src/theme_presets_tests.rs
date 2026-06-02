//! Theme preset parity tests.

use std::collections::BTreeMap;
use std::path::PathBuf;

use jian_ops_schema::variable::{VariableKind, VariableScalar};

use super::test_fixtures::{add_theme_axis, add_variable, sample};
use super::{
    list_theme_presets_snapshot, load_theme_preset_snapshot, save_theme_preset_snapshot,
    EditorCommand, McpTool, ToolOutcome,
};

#[test]
fn save_theme_preset_writes_ts_preset_shape() {
    let mut state = sample();
    add_variable(
        &mut state,
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff0000".into()),
    );
    add_theme_axis(&mut state, "Mode", &["Light", "Dark"]);
    let dir = temp_dir("save");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let preset_path = dir.join("acme.optheme");

    let tool = save_theme_preset_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("presetPath".into(), preset_path.display().to_string());
    args.insert("name".into(), "Acme".into());

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("ok"), Some(&"true".to_string()));
            let saved: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&preset_path).expect("preset"))
                    .expect("preset json");
            assert_eq!(saved["type"], "openpencil-theme-preset");
            assert_eq!(saved["version"], "1.0.0");
            assert_eq!(saved["name"], "Acme");
            assert_eq!(saved["themes"]["Mode"][1], "Dark");
            assert_eq!(saved["variables"]["brand"]["value"], "#ff0000");
        }
        other => panic!("expected save ok, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_theme_preset_emits_merge_command() {
    let dir = temp_dir("load");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let preset_path = dir.join("dark.optheme");
    std::fs::write(
        &preset_path,
        r##"{"type":"openpencil-theme-preset","version":"1.0.0","name":"Dark","themes":{"Mode":["Light","Dark"]},"variables":{"brand":{"type":"color","value":"#101010"}}}"##,
    )
    .expect("preset file");

    let tool = load_theme_preset_snapshot();
    let mut args = BTreeMap::new();
    args.insert("presetPath".into(), preset_path.display().to_string());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, EditorCommand::MergeThemePreset { variables, themes }) => {
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            assert_eq!(out.get("variableCount"), Some(&"1".to_string()));
            assert_eq!(
                themes.get("Mode"),
                Some(&vec!["Light".to_string(), "Dark".to_string()])
            );
            assert!(variables.contains_key("brand"));
        }
        other => panic!("expected merge command, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_theme_presets_skips_invalid_files() {
    let dir = temp_dir("list");
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(
        dir.join("valid.optheme"),
        r#"{"type":"openpencil-theme-preset","version":"1.0.0","name":"Valid","themes":{},"variables":{}}"#,
    )
    .expect("valid preset");
    std::fs::write(dir.join("invalid.optheme"), r#"{"type":"other"}"#).expect("invalid preset");

    let tool = list_theme_presets_snapshot();
    let mut args = BTreeMap::new();
    args.insert("directory".into(), dir.display().to_string());

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"1".to_string()));
            let presets: serde_json::Value =
                serde_json::from_str(out.get("presets").expect("presets json"))
                    .expect("presets json");
            assert_eq!(presets[0]["name"], "Valid");
        }
        other => panic!("expected list ok, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "openpencil-theme-presets-{label}-{}-{nanos}",
        std::process::id()
    ))
}
