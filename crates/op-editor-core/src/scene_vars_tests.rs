//! Tests for `scene_vars`. Kept in a sibling file so the types +
//! impl stay under the 800-line cap.

use super::{
    ThemeAxis, ThemedValue, Variable, VariableKind, VariableScalar, VariableTable, VariableValue,
};
use std::collections::BTreeMap;

fn axis(name: &str, value: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert(name.to_string(), value.to_string());
    m
}

#[test]
fn scalar_variable_resolves_regardless_of_theme() {
    let v = Variable {
        name: "color-1".into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str("#ff0000".into())),
    };
    let theme = axis("mode", "dark");
    match v.resolve(&theme).unwrap() {
        VariableScalar::Str(s) => assert_eq!(s, "#ff0000"),
        _ => panic!(),
    }
}

#[test]
fn themed_variable_picks_active_axis() {
    let v = Variable {
        name: "bg".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![
            ThemedValue {
                value: VariableScalar::Str("#ffffff".into()),
                theme: Some(axis("mode", "light")),
            },
            ThemedValue {
                value: VariableScalar::Str("#000000".into()),
                theme: Some(axis("mode", "dark")),
            },
        ]),
    };
    let dark = axis("mode", "dark");
    let light = axis("mode", "light");
    match v.resolve(&dark).unwrap() {
        VariableScalar::Str(s) => assert_eq!(s, "#000000"),
        _ => panic!(),
    }
    match v.resolve(&light).unwrap() {
        VariableScalar::Str(s) => assert_eq!(s, "#ffffff"),
        _ => panic!(),
    }
}

#[test]
fn themed_variable_falls_back_to_default_when_no_match() {
    let v = Variable {
        name: "accent".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![
            ThemedValue {
                value: VariableScalar::Str("#888888".into()),
                theme: None,
            },
            ThemedValue {
                value: VariableScalar::Str("#000000".into()),
                theme: Some(axis("mode", "dark")),
            },
        ]),
    };
    let light = axis("mode", "light");
    match v.resolve(&light).unwrap() {
        VariableScalar::Str(s) => assert_eq!(s, "#888888"),
        _ => panic!(),
    }
}

#[test]
fn resolve_color_parses_rrggbb_hex() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "accent".into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str("#ff8040".into())),
    });
    let c = tbl.resolve_color("accent").unwrap();
    assert!((c.r - 1.0).abs() < 0.01);
    assert!((c.g - (128.0 / 255.0)).abs() < 0.01);
    assert!((c.b - (64.0 / 255.0)).abs() < 0.01);
    assert_eq!(c.a, 1.0);
}

#[test]
fn resolve_color_picks_themed_active_value() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "bg".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![
            ThemedValue {
                value: VariableScalar::Str("#ffffff".into()),
                theme: Some(axis("mode", "light")),
            },
            ThemedValue {
                value: VariableScalar::Str("#000000".into()),
                theme: Some(axis("mode", "dark")),
            },
        ]),
    });
    tbl.active_theme = axis("mode", "dark");
    let c = tbl.resolve_color("bg").unwrap();
    assert_eq!(c.r, 0.0);
    assert_eq!(c.g, 0.0);
    assert_eq!(c.b, 0.0);
}

#[test]
fn resolve_color_rejects_non_color_variables() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "spacing".into(),
        kind: VariableKind::Number,
        value: VariableValue::Scalar(VariableScalar::Num(12.0)),
    });
    assert!(tbl.resolve_color("spacing").is_none());
}

#[test]
fn fill_for_resolves_registered_node_ref_to_themed_color() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "accent".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![
            ThemedValue {
                value: VariableScalar::Str("#ff0000".into()),
                theme: Some(axis("mode", "light")),
            },
            ThemedValue {
                value: VariableScalar::Str("#00ff00".into()),
                theme: Some(axis("mode", "dark")),
            },
        ]),
    });
    let node = crate::NodeId::new("n42");
    tbl.set_fill_ref(node.clone(), "accent");
    tbl.active_theme = axis("mode", "dark");
    let c = tbl.fill_for(&node).unwrap();
    assert!((c.g - 1.0).abs() < 0.01, "dark mode → green; got {c:?}");
    // Switch theme; same ref resolves differently.
    tbl.active_theme = axis("mode", "light");
    let c2 = tbl.fill_for(&node).unwrap();
    assert!((c2.r - 1.0).abs() < 0.01, "light mode → red; got {c2:?}");
}

#[test]
fn stroke_color_for_resolves_registered_ref() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "border".into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str("#0000ff".into())),
    });
    let node = crate::NodeId::new("n7");
    tbl.set_stroke_ref(node.clone(), "border");
    let c = tbl.stroke_color_for(&node).unwrap();
    assert!((c.b - 1.0).abs() < 0.01);
}

#[test]
fn set_active_theme_round_trips_through_axis_picker() {
    let mut tbl = super::VariableTable::default();
    tbl.set_active_theme("mode", "dark");
    assert_eq!(tbl.active_theme.get("mode"), Some(&"dark".to_string()));
    tbl.set_active_theme("density", "comfortable");
    assert_eq!(tbl.active_theme.len(), 2);
    tbl.clear_active_axis("mode");
    assert!(!tbl.active_theme.contains_key("mode"));
    assert_eq!(tbl.active_theme.len(), 1);
}

#[test]
fn fill_for_returns_none_when_no_ref_registered() {
    let tbl = super::VariableTable::default();
    assert!(tbl.fill_for(&crate::NodeId::new("n99")).is_none());
}

#[test]
fn resolve_color_rejects_invalid_hex() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "broken".into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str("not-hex".into())),
    });
    assert!(tbl.resolve_color("broken").is_none());
}

#[test]
fn themed_variable_returns_none_when_empty_and_no_default() {
    let v = Variable {
        name: "x".into(),
        kind: VariableKind::Number,
        value: VariableValue::Themed(vec![]),
    };
    let theme = BTreeMap::new();
    assert!(v.resolve(&theme).is_none());
}

#[test]
fn set_color_hex_writes_scalar_variable() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "color-1".into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str("#ff0000".into())),
    });
    assert!(tbl.set_color_hex("color-1", "#00ff00"));
    assert_eq!(
        tbl.resolve_color("color-1"),
        Some(crate::Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        })
    );
}

#[test]
fn set_color_hex_rejects_malformed_input() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "color-1".into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str("#ff0000".into())),
    });
    assert!(!tbl.set_color_hex("color-1", "not-hex"));
    // Scalar must be unchanged after a rejected write.
    assert_eq!(
        tbl.resolve_color("color-1"),
        Some(crate::Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        })
    );
}

#[test]
fn set_color_hex_returns_false_for_unknown_variable() {
    let mut tbl = super::VariableTable::default();
    assert!(!tbl.set_color_hex("does-not-exist", "#ffffff"));
}

#[test]
fn set_color_hex_returns_false_for_wrong_kind() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "spacing".into(),
        kind: VariableKind::Number,
        value: VariableValue::Scalar(VariableScalar::Num(16.0)),
    });
    // Number-kind variable: write rejected.
    assert!(!tbl.set_color_hex("spacing", "#ffffff"));
}

#[test]
fn set_color_hex_changes_resolved_value_under_subset_active_theme() {
    // Codex stop-gate regression: when the themed entry's theme
    // is a SUBSET of the active theme (e.g. entry only specifies
    // `mode: dark`; active is `mode: dark, density: compact`),
    // `resolve` still picks that entry — so a write must update
    // that entry in place, NOT push a new entry at the end that
    // `resolve` will never reach.
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "bg".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![ThemedValue {
            value: VariableScalar::Str("#111111".into()),
            theme: Some({
                let mut m = BTreeMap::new();
                m.insert("mode".into(), "dark".into());
                m
            }),
        }]),
    });
    tbl.set_active_theme("mode", "dark");
    tbl.set_active_theme("density", "compact");
    // Sanity: resolves through the subset-matching entry.
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0x11 as f32 / 255.0,
            g: 0x11 as f32 / 255.0,
            b: 0x11 as f32 / 255.0,
            a: 1.0,
        })
    );
    // Write under the wider active theme. The original entry
    // (only mode=dark) is the resolved one — the write MUST go
    // through it, not append a new entry that resolve never
    // reaches.
    assert!(tbl.set_color_hex("bg", "#22ff44"));
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0x22 as f32 / 255.0,
            g: 1.0,
            b: 0x44 as f32 / 255.0,
            a: 1.0,
        }),
        "themed write must update the entry resolve picks"
    );
}

#[test]
fn set_color_hex_under_active_theme_does_not_clobber_default() {
    // Codex stop-gate (round 2): when active_theme is non-empty
    // and no subset-matching themed entry exists, the write
    // must scope to the active theme (push a new entry), NOT
    // mutate the `theme: None` default. Mutating the default
    // would silently change the value under EVERY OTHER theme
    // axis the user hasn't explicitly authored, which is
    // never what an editor user clicking a colour swatch in
    // dark mode means.
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "bg".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![
            ThemedValue {
                value: VariableScalar::Str("#ffffff".into()),
                theme: Some({
                    let mut m = BTreeMap::new();
                    m.insert("mode".into(), "light".into());
                    m
                }),
            },
            ThemedValue {
                value: VariableScalar::Str("#888888".into()),
                theme: None,
            },
        ]),
    });
    tbl.set_active_theme("mode", "dark"); // matches neither
                                          // Pre-write: resolves to the default (#888) under dark.
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0x88 as f32 / 255.0,
            g: 0x88 as f32 / 255.0,
            b: 0x88 as f32 / 255.0,
            a: 1.0,
        })
    );
    // Write under active=dark must scope to dark only.
    assert!(tbl.set_color_hex("bg", "#000000"));
    // Under dark — new value reads back.
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }),
    );
    // The default entry must be untouched. Flipping to a
    // never-authored axis value (e.g. mode=sepia) should still
    // resolve to the original default #888888, NOT the new dark
    // value.
    tbl.set_active_theme("mode", "sepia");
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0x88 as f32 / 255.0,
            g: 0x88 as f32 / 255.0,
            b: 0x88 as f32 / 255.0,
            a: 1.0,
        }),
        "default entry must NOT have been clobbered by the dark write"
    );
    // Light entry also untouched.
    tbl.set_active_theme("mode", "light");
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }),
        "light entry must NOT have been clobbered"
    );
}

#[test]
fn set_color_hex_does_not_shadow_existing_entries_on_other_axes() {
    // Codex stop-gate (round 3): when the write pushes a new
    // themed entry, it must NOT shadow pre-existing entries
    // whose theme references a different axis. The pathological
    // case is an existing entry keyed to `{density:compact}`
    // and a write under `{mode:dark}`. A naive front-insert
    // would mean active=`{mode:dark, density:compact}` resolves
    // to the new entry instead of the compact entry, silently
    // changing the resolved colour for the combined axis.
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "bg".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![ThemedValue {
            value: VariableScalar::Str("#11ccaa".into()),
            theme: Some({
                let mut m = BTreeMap::new();
                m.insert("density".into(), "compact".into());
                m
            }),
        }]),
    });
    // Active theme = mode:dark only (no compact axis set, so the
    // existing entry doesn't subset-match — the write must push
    // a new entry).
    tbl.set_active_theme("mode", "dark");
    assert!(tbl.set_color_hex("bg", "#000000"));
    // Under mode:dark — the new entry is reachable.
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        })
    );
    // Now flip active to the combined axes — mode:dark +
    // density:compact. Under that resolve walk:
    //   - The pre-existing {density:compact} entry must still
    //     win (it's earlier in the vec AND its theme is a
    //     subset of the active combo).
    //   - The new {mode:dark} entry, end-pushed, is also a
    //     subset of the active combo, but it's later. First
    //     subset match wins.
    // Pre-fix (front-insert) this would have read #000000
    // instead — that was the codex BLOCK.
    tbl.set_active_theme("density", "compact");
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0x11 as f32 / 255.0,
            g: 0xcc as f32 / 255.0,
            b: 0xaa as f32 / 255.0,
            a: 1.0,
        }),
        "compact entry must not be shadowed by the end-pushed dark entry"
    );
}

#[test]
fn set_color_hex_empty_active_theme_targets_default_entry() {
    // The flip side: when active_theme IS empty (no axis
    // selected), writes target the default `theme: None`
    // entry — that's what resolve falls back to in this state.
    // Editing with no theme selected is the explicit "edit the
    // universal value" path.
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "bg".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![ThemedValue {
            value: VariableScalar::Str("#888888".into()),
            theme: None,
        }]),
    });
    // No active theme set.
    assert!(tbl.set_color_hex("bg", "#aabbcc"));
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0xaa as f32 / 255.0,
            g: 0xbb as f32 / 255.0,
            b: 0xcc as f32 / 255.0,
            a: 1.0,
        })
    );
}

#[test]
fn set_color_hex_writes_themed_entry_matching_active_axis() {
    let mut tbl = super::VariableTable::default();
    tbl.variables.push(Variable {
        name: "bg".into(),
        kind: VariableKind::Color,
        value: VariableValue::Themed(vec![
            ThemedValue {
                value: VariableScalar::Str("#ffffff".into()),
                theme: Some({
                    let mut m = BTreeMap::new();
                    m.insert("mode".into(), "light".into());
                    m
                }),
            },
            ThemedValue {
                value: VariableScalar::Str("#000000".into()),
                theme: Some({
                    let mut m = BTreeMap::new();
                    m.insert("mode".into(), "dark".into());
                    m
                }),
            },
        ]),
    });
    tbl.set_active_theme("mode", "dark");
    // Resolves to dark first.
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        })
    );
    // Write under the active theme → updates only the dark entry.
    assert!(tbl.set_color_hex("bg", "#3344ff"));
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 0x33 as f32 / 255.0,
            g: 0x44 as f32 / 255.0,
            b: 1.0,
            a: 1.0,
        })
    );
    // Flip to light — original light entry untouched.
    tbl.set_active_theme("mode", "light");
    assert_eq!(
        tbl.resolve_color("bg"),
        Some(crate::Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        })
    );
}

#[test]
fn cycle_active_axis_seeds_first_value_when_absent() {
    // Axis defined in `themes` but not yet in active_theme →
    // cycle plants the first value.
    let mut tbl = VariableTable::default();
    tbl.themes.push(ThemeAxis {
        name: "mode".into(),
        values: vec!["light".into(), "dark".into()],
    });
    assert!(tbl.cycle_active_axis_value("mode"));
    assert_eq!(tbl.active_theme.get("mode"), Some(&"light".to_string()));
}

#[test]
fn cycle_active_axis_advances_to_next_value() {
    let mut tbl = VariableTable::default();
    tbl.themes.push(ThemeAxis {
        name: "mode".into(),
        values: vec!["light".into(), "dark".into(), "sepia".into()],
    });
    tbl.set_active_theme("mode", "light");
    // light → dark → sepia → light (wrap)
    assert!(tbl.cycle_active_axis_value("mode"));
    assert_eq!(tbl.active_theme.get("mode"), Some(&"dark".to_string()));
    assert!(tbl.cycle_active_axis_value("mode"));
    assert_eq!(tbl.active_theme.get("mode"), Some(&"sepia".to_string()));
    assert!(tbl.cycle_active_axis_value("mode"));
    assert_eq!(tbl.active_theme.get("mode"), Some(&"light".to_string()));
}

#[test]
fn cycle_active_axis_returns_false_for_unknown_axis() {
    let mut tbl = VariableTable::default();
    assert!(!tbl.cycle_active_axis_value("does-not-exist"));
    assert!(tbl.active_theme.is_empty());
}

#[test]
fn cycle_active_axis_returns_false_for_empty_values() {
    let mut tbl = VariableTable::default();
    tbl.themes.push(ThemeAxis {
        name: "empty".into(),
        values: Vec::new(),
    });
    assert!(!tbl.cycle_active_axis_value("empty"));
    assert!(!tbl.active_theme.contains_key("empty"));
}

#[test]
fn cycle_active_axis_falls_back_to_first_when_current_unknown() {
    // active_theme carries a value that isn't in themes.values
    // (e.g. an axis whose options changed since the file loaded).
    // Cycle wraps to the first known value.
    let mut tbl = VariableTable::default();
    tbl.themes.push(ThemeAxis {
        name: "mode".into(),
        values: vec!["light".into(), "dark".into()],
    });
    tbl.set_active_theme("mode", "ghost"); // not in values
    assert!(tbl.cycle_active_axis_value("mode"));
    assert_eq!(tbl.active_theme.get("mode"), Some(&"light".to_string()));
}
