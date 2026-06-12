//! Named theme presets — app-level snapshots of the document's theme
//! axes + variable definitions, plus the `.optheme` file wire format.
//!
//! TS sources mirrored:
//! - `apps/web/src/stores/theme-preset-store.ts` — preset list +
//!   save/delete/rename ops, persisted app-level (localStorage key
//!   `openpencil-theme-presets`; the Rust desktop host persists the
//!   same JSON array in `<config>/openpencil/theme-presets.json`,
//!   see op-host-desktop `theme_preset_host.rs`).
//! - `apps/web/src/utils/theme-preset-io.ts` — `.optheme` file
//!   build / validate for import/export.
//! - `packages/pen-types/src/theme-preset.ts` — `ThemePreset` /
//!   `ThemePresetFile` shapes (camelCase `createdAt` on the wire).
//! - `apps/web/src/components/panels/variable-theme-manager.tsx:131-164`
//!   — `handleSavePreset` / `handleLoadPreset` / import / export flows.
//!
//! Documented divergences:
//! - Preset ids keep the TS `preset-{epoch_ms}-{6 base36 chars}`
//!   shape (`theme-preset-store.ts:22-24`), but the 6-char suffix is
//!   derived from a counter-salted hash instead of `Math.random()`
//!   (no rand dependency; unique within a session, and the id is
//!   only a local list key).
//! - JSON object key ORDER differs from TS `JSON.stringify`
//!   (serde_json emits map keys sorted); wire-compatible since every
//!   consumer parses by key. op-mcp `theme_presets.rs` already emits
//!   the same shape.
//! - TS `hydrate()` trusts any JSON array verbatim; the Rust parse is
//!   typed, so malformed entries are skipped instead of hydrated as
//!   garbage.
//! - TS `validatePresetFile` only type-checks the top level; the
//!   Rust parse also requires `themes` / `variables` to deserialize
//!   into their typed maps (a malformed entry rejects the file
//!   instead of failing later at apply time).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use jian_ops_schema::variable::VariableDefinition;
use serde_json::{json, Value};

use crate::EditorState;

/// `ThemePresetFile.type` discriminator (`theme-preset-io.ts:23`).
pub const THEME_PRESET_FILE_TYPE: &str = "openpencil-theme-preset";
/// `ThemePresetFile.version` literal (`pen-types/theme-preset.ts:13`).
pub const THEME_PRESET_FILE_VERSION: &str = "1.0.0";
/// Fixed export name (`variable-theme-manager.tsx:162`).
pub const THEME_PRESET_EXPORT_NAME: &str = "theme-preset";

/// One saved preset — mirrors TS `ThemePreset`
/// (`packages/pen-types/src/theme-preset.ts:3-9`).
#[derive(Debug, Clone, PartialEq)]
pub struct ThemePreset {
    pub id: String,
    pub name: String,
    pub themes: BTreeMap<String, Vec<String>>,
    pub variables: BTreeMap<String, VariableDefinition>,
    /// Epoch ms — serialized as `createdAt`.
    pub created_at: u64,
}

static PRESET_ID_SALT: AtomicU64 = AtomicU64::new(0);

/// `preset-{epoch_ms}-{6 base36 chars}` — same shape as TS
/// `generateId()` (`theme-preset-store.ts:22-24`).
pub fn generate_preset_id(now_ms: u64) -> String {
    let salt = PRESET_ID_SALT.fetch_add(1, Ordering::Relaxed);
    // FNV-style mix so two saves in the same millisecond still get
    // distinct, non-sequential suffixes.
    let mut h = now_ms ^ 0xcbf2_9ce4_8422_2325;
    h = h.wrapping_mul(0x0000_0100_0000_01b3).wrapping_add(salt);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    format!("preset-{now_ms}-{}", base36(h, 6))
}

fn base36(mut v: u64, len: usize) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = vec![b'0'; len];
    for slot in out.iter_mut().rev() {
        *slot = DIGITS[(v % 36) as usize];
        v /= 36;
    }
    String::from_utf8(out).expect("base36 digits are ASCII")
}

/// File-name sanitization for exports — TS
/// `name.replace(/[^a-zA-Z0-9_-]/g, '_')` (`theme-preset-io.ts:40`).
pub fn sanitize_preset_file_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

impl EditorState {
    /// TS `savePreset` (`theme-preset-store.ts:29-39`) behind the
    /// `handleSavePreset` trim guard
    /// (`variable-theme-manager.tsx:131-136`). Snapshots the live
    /// document themes + variables. Returns `false` on a blank name.
    pub fn save_theme_preset(&mut self, name: &str, now_ms: u64) -> bool {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return false;
        }
        let preset = ThemePreset {
            id: generate_preset_id(now_ms),
            name: trimmed.to_string(),
            themes: self.doc.themes.clone().unwrap_or_default(),
            variables: self.doc.variables.clone().unwrap_or_default(),
            created_at: now_ms,
        };
        self.theme_presets.push(preset);
        self.theme_presets_dirty = true;
        true
    }

    /// TS `deletePreset` (`theme-preset-store.ts:41-44`).
    pub fn delete_theme_preset(&mut self, id: &str) -> bool {
        let before = self.theme_presets.len();
        self.theme_presets.retain(|p| p.id != id);
        let removed = self.theme_presets.len() != before;
        if removed {
            self.theme_presets_dirty = true;
        }
        removed
    }

    /// TS `renamePreset` (`theme-preset-store.ts:46-51`) — no trim /
    /// empty guard there either (no UI calls it; store API parity).
    pub fn rename_theme_preset(&mut self, id: &str, name: &str) -> bool {
        let Some(preset) = self.theme_presets.iter_mut().find(|p| p.id == id) else {
            return false;
        };
        preset.name = name.to_string();
        self.theme_presets_dirty = true;
        true
    }

    /// TS `handleLoadPreset` (`variable-theme-manager.tsx:138-151`):
    /// `{ ...themes, ...preset.themes }` spread-merge, then each
    /// preset variable written when missing or different.
    /// `set_themes_bulk(.., false)` / `set_variables_bulk(.., false)`
    /// reach the same end state (insert-or-overwrite, never delete),
    /// in the same themes-then-variables order.
    pub fn merge_theme_preset_payload(
        &mut self,
        themes: BTreeMap<String, Vec<String>>,
        variables: BTreeMap<String, VariableDefinition>,
    ) {
        self.set_themes_bulk(themes, false);
        self.set_variables_bulk(variables, false);
    }

    /// Apply the stored preset at `index` to the live document.
    /// Returns `false` for an out-of-range index. History is the
    /// caller's job (one snapshot per user gesture, matching the
    /// other variables-panel mutations).
    pub fn load_theme_preset(&mut self, index: usize) -> bool {
        let Some(preset) = self.theme_presets.get(index) else {
            return false;
        };
        let themes = preset.themes.clone();
        let variables = preset.variables.clone();
        self.merge_theme_preset_payload(themes, variables);
        true
    }
}

// ---------------------------------------------------------------------------
// Preset-list wire format (TS localStorage value shape)
// ---------------------------------------------------------------------------

fn preset_to_value(preset: &ThemePreset) -> Option<Value> {
    Some(json!({
        "id": preset.id,
        "name": preset.name,
        "themes": serde_json::to_value(&preset.themes).ok()?,
        "variables": serde_json::to_value(&preset.variables).ok()?,
        "createdAt": preset.created_at,
    }))
}

fn preset_from_value(value: &Value) -> Option<ThemePreset> {
    let obj = value.as_object()?;
    Some(ThemePreset {
        id: obj.get("id")?.as_str()?.to_string(),
        name: obj.get("name")?.as_str()?.to_string(),
        themes: serde_json::from_value(obj.get("themes")?.clone()).ok()?,
        variables: serde_json::from_value(obj.get("variables")?.clone()).ok()?,
        // TS `Date.now()` is an integer; tolerate a float on read.
        created_at: obj.get("createdAt")?.as_f64()? as u64,
    })
}

/// Serialize the preset list — the exact JSON array TS `persist()`
/// writes under `openpencil-theme-presets`.
pub fn presets_to_json(presets: &[ThemePreset]) -> String {
    Value::Array(presets.iter().filter_map(preset_to_value).collect()).to_string()
}

/// Tolerant hydrate — mirrors TS `hydrate()`'s ignore-on-error
/// behavior (`theme-preset-store.ts:62-73`); entries that don't
/// parse are skipped (documented divergence: TS trusts the array).
pub fn presets_from_json(raw: &str) -> Vec<ThemePreset> {
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    items.iter().filter_map(preset_from_value).collect()
}

// ---------------------------------------------------------------------------
// `.optheme` file wire format
// ---------------------------------------------------------------------------

/// Build the export payload — TS `buildPresetFile` +
/// `JSON.stringify(preset, null, 2)` (`theme-preset-io.ts:5-17,39`).
pub fn preset_file_to_json(
    name: &str,
    themes: &BTreeMap<String, Vec<String>>,
    variables: &BTreeMap<String, VariableDefinition>,
) -> String {
    let value = json!({
        "type": THEME_PRESET_FILE_TYPE,
        "version": THEME_PRESET_FILE_VERSION,
        "name": name,
        "themes": serde_json::to_value(themes).unwrap_or_else(|_| json!({})),
        "variables": serde_json::to_value(variables).unwrap_or_else(|_| json!({})),
    });
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
}

/// Parse + validate an imported `.optheme` payload — TS
/// `validatePresetFile` (`theme-preset-io.ts:19-30`): `type` literal,
/// string `name`, object `themes` / `variables`. Returns
/// `(name, themes, variables)`; `None` mirrors TS resolving `null`.
#[allow(clippy::type_complexity)]
pub fn parse_preset_file(
    raw: &str,
) -> Option<(
    String,
    BTreeMap<String, Vec<String>>,
    BTreeMap<String, VariableDefinition>,
)> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let obj = value.as_object()?;
    if obj.get("type")?.as_str()? != THEME_PRESET_FILE_TYPE {
        return None;
    }
    let name = obj.get("name")?.as_str()?.to_string();
    let themes = serde_json::from_value(obj.get("themes")?.clone()).ok()?;
    let variables = serde_json::from_value(obj.get("variables")?.clone()).ok()?;
    Some((name, themes, variables))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::variable::{VariableKind, VariableScalar};

    fn state_with_brand_color() -> EditorState {
        let mut state = EditorState::new();
        assert!(state.create_variable(
            "brand",
            VariableKind::Color,
            VariableScalar::Str("#ff0000".into()),
        ));
        state
            .doc
            .themes
            .get_or_insert_with(BTreeMap::new)
            .insert("Theme-1".into(), vec!["Light".into(), "Dark".into()]);
        state
    }

    #[test]
    fn preset_id_keeps_the_ts_shape() {
        let id = generate_preset_id(1718000000000);
        let parts: Vec<&str> = id.splitn(3, '-').collect();
        assert_eq!(parts[0], "preset");
        assert_eq!(parts[1], "1718000000000");
        assert_eq!(parts[2].len(), 6);
        assert!(parts[2]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        // Two saves in the same millisecond stay distinct.
        assert_ne!(id, generate_preset_id(1718000000000));
    }

    #[test]
    fn save_theme_preset_snapshots_doc_and_rejects_blank_names() {
        let mut state = state_with_brand_color();
        assert!(!state.save_theme_preset("   ", 42));
        assert!(state.theme_presets.is_empty());
        assert!(!state.theme_presets_dirty);

        assert!(state.save_theme_preset("  Brand kit  ", 42));
        assert_eq!(state.theme_presets.len(), 1);
        let preset = &state.theme_presets[0];
        assert_eq!(preset.name, "Brand kit"); // trimmed
        assert_eq!(preset.created_at, 42);
        assert!(preset.variables.contains_key("brand"));
        assert_eq!(
            preset.themes.get("Theme-1").map(Vec::len),
            Some(2),
            "doc theme axes snapshot into the preset"
        );
        assert!(state.theme_presets_dirty);
    }

    #[test]
    fn delete_and_rename_follow_the_ts_store() {
        let mut state = state_with_brand_color();
        assert!(state.save_theme_preset("a", 1));
        assert!(state.save_theme_preset("b", 2));
        let id_a = state.theme_presets[0].id.clone();

        state.theme_presets_dirty = false;
        assert!(!state.delete_theme_preset("preset-nope"));
        assert!(!state.theme_presets_dirty);
        assert!(state.delete_theme_preset(&id_a));
        assert_eq!(state.theme_presets.len(), 1);
        assert!(state.theme_presets_dirty);

        let id_b = state.theme_presets[0].id.clone();
        assert!(state.rename_theme_preset(&id_b, "renamed"));
        assert_eq!(state.theme_presets[0].name, "renamed");
        assert!(!state.rename_theme_preset("preset-nope", "x"));
    }

    #[test]
    fn load_theme_preset_merges_like_ts_handle_load_preset() {
        let mut donor = state_with_brand_color();
        assert!(donor.save_theme_preset("kit", 7));
        let preset = donor.theme_presets[0].clone();

        // Target doc with its own axis + an overlapping variable.
        let mut state = EditorState::new();
        assert!(state.create_variable(
            "brand",
            VariableKind::Color,
            VariableScalar::Str("#0000ff".into()),
        ));
        assert!(state.create_variable("keep-me", VariableKind::Number, VariableScalar::Num(4.0),));
        state
            .doc
            .themes
            .get_or_insert_with(BTreeMap::new)
            .insert("Density".into(), vec!["Compact".into()]);
        state.theme_presets.push(preset);

        assert!(state.load_theme_preset(0));
        let themes = state.doc.themes.as_ref().unwrap();
        // Spread-merge: preset axis added, existing axis preserved.
        assert!(themes.contains_key("Theme-1"));
        assert!(themes.contains_key("Density"));
        let vars = state.doc.variables.as_ref().unwrap();
        // Preset variable overwrote the differing one; extras kept.
        assert!(vars.contains_key("keep-me"));
        assert_eq!(
            state.resolve_variable("brand"),
            Some(&VariableScalar::Str("#ff0000".into()))
        );
        assert!(!state.load_theme_preset(5));
    }

    #[test]
    fn preset_list_json_round_trips_with_camel_case_created_at() {
        let mut state = state_with_brand_color();
        assert!(state.save_theme_preset("kit", 99));
        let raw = presets_to_json(&state.theme_presets);
        assert!(raw.contains("\"createdAt\":99"), "wire key is camelCase");
        let parsed = presets_from_json(&raw);
        assert_eq!(parsed, state.theme_presets);
    }

    #[test]
    fn presets_from_json_is_tolerant_like_ts_hydrate() {
        assert!(presets_from_json("not json").is_empty());
        assert!(presets_from_json("{\"a\":1}").is_empty());
        // A malformed entry is skipped, good entries survive.
        let raw = r#"[{"bogus":true},{"id":"preset-1-aaaaaa","name":"ok",
            "themes":{},"variables":{},"createdAt":1}]"#;
        let parsed = presets_from_json(raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "ok");
    }

    #[test]
    fn preset_file_round_trips_and_validates_the_type_literal() {
        let state = state_with_brand_color();
        let raw = preset_file_to_json(
            THEME_PRESET_EXPORT_NAME,
            state.doc.themes.as_ref().unwrap(),
            state.doc.variables.as_ref().unwrap(),
        );
        assert!(raw.contains("\"type\": \"openpencil-theme-preset\""));
        assert!(raw.contains("\"version\": \"1.0.0\""));
        let (name, themes, variables) = parse_preset_file(&raw).expect("valid file");
        assert_eq!(name, "theme-preset");
        assert_eq!(themes.get("Theme-1").map(Vec::len), Some(2));
        assert!(variables.contains_key("brand"));

        // TS validatePresetFile rejections.
        assert!(parse_preset_file("[]").is_none());
        assert!(parse_preset_file(
            "{\"type\":\"other\",\"name\":\"x\",\"themes\":{},\"variables\":{}}"
        )
        .is_none());
        assert!(parse_preset_file(
            "{\"type\":\"openpencil-theme-preset\",\"themes\":{},\"variables\":{}}"
        )
        .is_none());
    }

    #[test]
    fn sanitize_preset_file_stem_matches_the_ts_regex() {
        assert_eq!(sanitize_preset_file_stem("theme-preset"), "theme-preset");
        assert_eq!(sanitize_preset_file_stem("My Kit (v2)!"), "My_Kit__v2__");
        assert_eq!(sanitize_preset_file_stem("深色主题"), "____");
    }
}
