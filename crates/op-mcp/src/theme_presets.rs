//! TS-compatible theme preset tools.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use jian_ops_schema::variable::VariableDefinition;
use op_editor_core::{EditorCommand, EditorState};
use serde_json::{json, Value};

use super::{McpTool, ToolErrorCode, ToolOutcome};

type ToolCallError = (ToolErrorCode, String);
type ThemePresetPayload = (
    BTreeMap<String, Vec<String>>,
    BTreeMap<String, VariableDefinition>,
);

pub struct SaveThemePreset {
    themes: BTreeMap<String, Vec<String>>,
    variables: BTreeMap<String, VariableDefinition>,
}

pub struct LoadThemePreset;

pub struct ListThemePresets;

impl McpTool for SaveThemePreset {
    fn name(&self) -> &str {
        "save_theme_preset"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let preset_path = match required_path(args, "presetPath") {
            Ok(path) => path,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let name = args
            .get("name")
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| file_stem(&preset_path));
        let preset = json!({
            "type": "openpencil-theme-preset",
            "version": "1.0.0",
            "name": name,
            "themes": &self.themes,
            "variables": &self.variables,
        });
        let raw = match serde_json::to_string_pretty(&preset) {
            Ok(raw) => raw,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize theme preset failed: {e}"),
                );
            }
        };
        if let Err(e) = fs::write(&preset_path, raw) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("write theme preset failed: {e}"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("ok".into(), "true".into());
        out.insert("path".into(), preset_path.display().to_string());
        ToolOutcome::Ok(out)
    }
}

impl McpTool for LoadThemePreset {
    fn name(&self) -> &str {
        "load_theme_preset"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let preset_path = match required_path(args, "presetPath") {
            Ok(path) => path,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let (themes, variables) = match read_preset_payload(&preset_path) {
            Ok(payload) => payload,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let themes_json = match serde_json::to_string(&themes) {
            Ok(json) => json,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize themes failed: {e}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        out.insert("themes".into(), themes_json);
        out.insert("variableCount".into(), variables.len().to_string());
        ToolOutcome::OkWithCommand(out, EditorCommand::MergeThemePreset { variables, themes })
    }
}

impl McpTool for ListThemePresets {
    fn name(&self) -> &str {
        "list_theme_presets"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let directory = match required_path(args, "directory") {
            Ok(path) => path,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::ToolFailed,
                    format!("list theme presets failed: {e}"),
                );
            }
        };
        let mut paths = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "optheme")
            })
            .collect::<Vec<_>>();
        paths.sort();

        let mut presets = Vec::new();
        for path in paths {
            let Ok(value) = read_preset_value(&path) else {
                continue;
            };
            let name = value
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| file_stem(&path));
            presets.push(json!({
                "name": name,
                "path": path.display().to_string(),
            }));
        }

        let presets_json = match serde_json::to_string(&presets) {
            Ok(json) => json,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize theme preset list failed: {e}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("count".into(), presets.len().to_string());
        out.insert("presets".into(), presets_json);
        ToolOutcome::Ok(out)
    }
}

pub fn save_theme_preset_snapshot(state: &EditorState) -> SaveThemePreset {
    SaveThemePreset {
        themes: state.doc.themes.clone().unwrap_or_default(),
        variables: state.doc.variables.clone().unwrap_or_default(),
    }
}

pub fn load_theme_preset_snapshot() -> LoadThemePreset {
    LoadThemePreset
}

pub fn list_theme_presets_snapshot() -> ListThemePresets {
    ListThemePresets
}

fn required_path(args: &BTreeMap<String, String>, key: &str) -> Result<PathBuf, ToolCallError> {
    let Some(raw) = args.get(key).filter(|raw| !raw.trim().is_empty()) else {
        return Err((ToolErrorCode::MissingArgument, format!("{key} is required")));
    };
    Ok(resolve_path(raw))
}

fn resolve_path(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("theme")
        .to_string()
}

fn read_preset_payload(path: &Path) -> Result<ThemePresetPayload, ToolCallError> {
    let value = read_preset_value(path)?;
    let themes = serde_json::from_value::<BTreeMap<String, Vec<String>>>(
        value.get("themes").cloned().unwrap_or_else(|| json!({})),
    )
    .map_err(|e| {
        (
            ToolErrorCode::InvalidArgument,
            format!("theme preset themes must be an object of string arrays: {e}"),
        )
    })?;
    let variables = serde_json::from_value::<BTreeMap<String, VariableDefinition>>(
        value.get("variables").cloned().unwrap_or_else(|| json!({})),
    )
    .map_err(|e| {
        (
            ToolErrorCode::InvalidArgument,
            format!("theme preset variables must be variable definitions: {e}"),
        )
    })?;
    Ok((themes, variables))
}

fn read_preset_value(path: &Path) -> Result<Value, ToolCallError> {
    let raw = fs::read_to_string(path).map_err(|e| {
        (
            ToolErrorCode::ToolFailed,
            format!("read theme preset failed: {e}"),
        )
    })?;
    let value = serde_json::from_str::<Value>(&raw).map_err(|e| {
        (
            ToolErrorCode::InvalidArgument,
            format!("theme preset must be valid JSON: {e}"),
        )
    })?;
    if value.get("type").and_then(Value::as_str) != Some("openpencil-theme-preset") {
        return Err((
            ToolErrorCode::InvalidArgument,
            "Invalid theme preset file".into(),
        ));
    }
    Ok(value)
}
