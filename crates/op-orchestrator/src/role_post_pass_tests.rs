use super::*;
use serde_json::json;

// ── fixButtonForegroundContrast ──────────────────────────────────────────

#[test]
fn button_dark_bg_gets_white_text() {
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"#1E40AF"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert_eq!(
        btn["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
}

#[test]
fn button_light_bg_gets_dark_text() {
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert_eq!(
        btn["children"][0]["fill"],
        json!([{"type":"solid","color":"#0F172A"}])
    );
}

#[test]
fn button_icon_copies_sibling_text_color() {
    // text has an explicit white fill → the unfilled icon should match it.
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"#1E40AF"}],
        "children":[
            {"type":"text","fill":[{"type":"solid","color":"#FFFFFF"}],"content":"Go"},
            {"type":"icon_font","id":"i"}
        ]
    });
    fix_button_foreground_contrast(&mut btn);
    assert_eq!(
        btn["children"][1]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
}

#[test]
fn button_transparent_fill_skipped() {
    // No visible bg → nothing to compute contrast against; text untouched.
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"transparent"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert!(btn["children"][0].get("fill").is_none());
}

#[test]
fn button_unresolved_ref_bg_skipped() {
    // $color-accent doesn't resolve to a hex in the Rust context → skip.
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"$color-accent"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert!(btn["children"][0].get("fill").is_none());
}

// ── fixOrphanContainerContrast ───────────────────────────────────────────

#[test]
fn orphan_card_gets_fill_and_shadow() {
    let mut card = json!({
        "type":"frame","role":"card","cornerRadius":12,
        "children":[{"type":"text","content":"x"}]
    });
    // Parent exists (Some) but has no fill (Null) → orphan fix fires.
    fix_orphan_container_contrast(&mut card, Some(&Value::Null));
    assert_eq!(card["fill"], json!([{"type":"solid","color":"#FFFFFF"}]));
    assert!(card["effects"].is_array());
}

#[test]
fn orphan_skipped_when_parent_has_fill() {
    let mut card = json!({
        "type":"frame","role":"card","cornerRadius":12,
        "children":[{"type":"text","content":"x"}]
    });
    let parent_fill = json!([{"type":"solid","color":"#EEEEEE"}]);
    fix_orphan_container_contrast(&mut card, Some(&parent_fill));
    assert!(
        card.get("fill").is_none(),
        "child on a filled parent stays transparent"
    );
}

#[test]
fn orphan_skipped_for_root_and_structural_roles() {
    // Root (no parent) → skip.
    let mut card =
        json!({"type":"frame","role":"card","cornerRadius":12,"children":[{"type":"text"}]});
    fix_orphan_container_contrast(&mut card, None);
    assert!(card.get("fill").is_none());
    // Structural role (section) → skip even with cornerRadius + no parent fill.
    let mut sect =
        json!({"type":"frame","role":"section","cornerRadius":12,"children":[{"type":"text"}]});
    fix_orphan_container_contrast(&mut sect, Some(&Value::Null));
    assert!(sect.get("fill").is_none());
}

#[test]
fn orphan_skipped_without_corner_radius() {
    let mut card = json!({"type":"frame","role":"card","children":[{"type":"text"}]});
    fix_orphan_container_contrast(&mut card, Some(&Value::Null));
    assert!(
        card.get("fill").is_none(),
        "no cornerRadius → not a card silhouette"
    );
}

// ── fixSectionAlternation ────────────────────────────────────────────────

#[test]
fn section_alternation_paints_unfilled_runs() {
    let mut col = json!({
        "type":"frame","layout":"vertical","children":[
            {"type":"frame","role":"section","children":[]},
            {"type":"frame","role":"section","children":[]},
            {"type":"frame","role":"section","children":[]}
        ]
    });
    fix_section_alternation(&mut col);
    assert_eq!(
        col["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
    assert_eq!(
        col["children"][1]["fill"],
        json!([{"type":"solid","color":"#F8FAFC"}])
    );
    assert_eq!(
        col["children"][2]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
}

#[test]
fn section_alternation_skipped_on_dark_parent() {
    let mut col = json!({
        "type":"frame","layout":"vertical","fill":[{"type":"solid","color":"#0F172A"}],
        "children":[
            {"type":"frame","role":"section","children":[]},
            {"type":"frame","role":"section","children":[]},
            {"type":"frame","role":"section","children":[]}
        ]
    });
    fix_section_alternation(&mut col);
    assert!(
        col["children"][0].get("fill").is_none(),
        "dark page: no white strips"
    );
}

// ── fixInputSiblingConsistency ───────────────────────────────────────────

#[test]
fn input_siblings_unified_to_first() {
    let mut form = json!({
        "type":"frame","layout":"vertical","children":[
            {"type":"frame","role":"input","fill":[{"type":"solid","color":"#F8FAFC"}]},
            {"type":"frame","role":"input","fill":[{"type":"solid","color":"#FF0000"}]}
        ]
    });
    fix_input_sibling_consistency(&mut form);
    assert_eq!(
        form["children"][1]["fill"],
        json!([{"type":"solid","color":"#F8FAFC"}]),
        "second input adopts the first input's fill"
    );
}

// ── post_pass_forest integration (round-trips through PenNode) ────────────

#[test]
fn post_pass_forest_round_trips_and_fills_orphan_card() {
    // A section root whose child card has no fill + cornerRadius → card filled.
    let mut nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Section","children":[
            {"type":"frame","id":"card","role":"card","cornerRadius":12,
             "children":[{"type":"text","id":"t","content":"hi"}]}
        ]
    }))
    .unwrap()];
    post_pass_forest(&mut nodes);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(
        v["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "orphan card inside an unfilled section root gets a white fill"
    );
}
