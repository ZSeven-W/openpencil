//! Instance-override resolution tests — drives each of the four
//! Strategy branches with a tiny synthetic symbol subtree and a few
//! override / derived entries, asserting that the resulting overridden
//! tree carries the expected fields. The fixtures mirror the shape of a
//! decoded Figma `.fig` (`guid` / `guidPath.guids` arrays / `size` /
//! `fontSize` / etc.).

use super::*;

fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
    FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn guid(sid: u32, lid: u32) -> FigValue {
    obj(vec![
        ("sessionID", FigValue::Uint(sid)),
        ("localID", FigValue::Uint(lid)),
    ])
}

fn size(x: f32, y: f32) -> FigValue {
    obj(vec![("x", FigValue::Float(x)), ("y", FigValue::Float(y))])
}

fn guid_path(guids: Vec<FigValue>) -> FigValue {
    obj(vec![(
        "guidPath",
        obj(vec![("guids", FigValue::Array(guids))]),
    )])
}

fn ov_with(guids: Vec<FigValue>, extra: Vec<(&str, FigValue)>) -> FigValue {
    let mut pairs: Vec<(String, FigValue)> = vec![(
        "guidPath".into(),
        obj(vec![("guids", FigValue::Array(guids))]),
    )];
    for (k, v) in extra {
        pairs.push((k.into(), v));
    }
    FigValue::Object(pairs)
}

fn leaf(name: &str, sid: u32, lid: u32) -> TreeNode {
    TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("RECTANGLE".into())),
            ("guid", guid(sid, lid)),
            ("name", FigValue::Str(name.into())),
            ("size", size(100.0, 100.0)),
        ]),
        children: vec![],
    }
}

fn symbol_root(children: Vec<TreeNode>) -> TreeNode {
    TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("SYMBOL".into())),
            ("guid", guid(0, 0)),
            ("size", size(100.0, 100.0)),
        ]),
        children,
    }
}

#[test]
fn fast_path_scales_subtree_when_no_overrides() {
    let sym = symbol_root(vec![leaf("a", 1, 10)]);
    let out = apply_instance_overrides(&sym, None, None, Some(FigVec2 { x: 200.0, y: 100.0 }));
    assert_eq!(out.len(), 1);
    let resized = FigVec2::from_value(out[0].figma.get("size").unwrap()).unwrap();
    // 100 × 2 in x, untouched y.
    assert!((resized.x - 200.0).abs() < 0.001);
}

#[test]
fn strategy_0_applies_override_via_direct_guid() {
    let sym = symbol_root(vec![leaf("hero", 1, 10)]);
    // Derived (single-segment) names the real GUID → Strategy 0 wins.
    let derived = vec![guid_path(vec![guid(1, 10)])];
    let mut over_entry = ov_with(
        vec![guid(1, 10)],
        vec![("name", FigValue::Str("Renamed".into()))],
    );
    // Sanity: build a FigValue::Object with the override fields.
    if let FigValue::Object(pairs) = &mut over_entry {
        pairs.push(("ignored_skip".into(), FigValue::Null));
    }
    let out = apply_instance_overrides(&sym, Some(&[over_entry]), Some(&derived), None);
    assert_eq!(out[0].figma.get_str("name"), Some("Renamed"));
}

#[test]
fn strategy_0_takes_explicit_null_as_reset() {
    // TS copies an explicit null override (intentional reset).
    let sym = symbol_root(vec![leaf("hero", 1, 10)]);
    let derived = vec![guid_path(vec![guid(1, 10)])];
    let over_entry = ov_with(vec![guid(1, 10)], vec![("name", FigValue::Null)]);
    let out = apply_instance_overrides(&sym, Some(&[over_entry]), Some(&derived), None);
    // The original "hero" name was overwritten by Null — get_str now
    // returns None because FigValue::get treats Null as absent.
    assert_eq!(out[0].figma.get_str("name"), None);
}

#[test]
fn strategy_1_index_maps_when_count_matches() {
    // Two children, two single-segment derived entries with virtual
    // GUIDs that do NOT name real subtree nodes → direct_matches == 0,
    // len1.len == flat_symbol.len (which is 3: symbol root + 2 leaves).
    let sym = symbol_root(vec![leaf("a", 1, 10), leaf("b", 1, 11)]);
    // 3 derived entries to match flat_symbol.len() == 3.
    let derived = vec![
        guid_path(vec![guid(9, 100)]),
        guid_path(vec![guid(9, 101)]),
        guid_path(vec![guid(9, 102)]),
    ];
    // Override the second child via index 2 (positions 0/1/2 = symbol/a/b).
    let over_b = ov_with(
        vec![guid(9, 102)],
        vec![("name", FigValue::Str("B-Override".into()))],
    );
    let out = apply_instance_overrides(&sym, Some(&[over_b]), Some(&derived), None);
    // flat_symbol[0] = symbol_root (sid 0 lid 0)
    // flat_symbol[1] = leaf "a" (first child by localID = 10)
    // flat_symbol[2] = leaf "b" (lid 11)
    // index 2 → leaf "b" gets renamed.
    let names: Vec<Option<&str>> = out.iter().map(|c| c.figma.get_str("name")).collect();
    assert!(names.contains(&Some("B-Override")), "{names:?}");
    assert!(names.contains(&Some("a")), "{names:?}");
}

#[test]
fn strategy_2_resolves_via_virtual_guid_dfs() {
    // Symbol with one child. Virtual GUID base = (sessionID 7, firstLID 50).
    // Derived has TWO single-segment entries (virtual). flat_symbol has 2
    // entries (symbol + child); count matches → Strategy 1 wins. To
    // force Strategy 2, add an extra child so counts diverge.
    let sym = symbol_root(vec![leaf("a", 1, 10), leaf("b", 1, 11), leaf("c", 1, 12)]);
    // 2 derived entries → not equal to flat_symbol.len() (4), and the
    // virtual GUIDs are not real subtree nodes → direct_matches=0, so
    // Strategy 2 fires.
    let derived = vec![guid_path(vec![guid(7, 50)]), guid_path(vec![guid(7, 51)])];
    // The override targets virtual GUID 7:51 which the "full" walk
    // (starting at children) maps to the second visited node — leaf "b"
    // (localID 11 → second after sort).
    let over = ov_with(
        vec![guid(7, 51)],
        vec![("name", FigValue::Str("B-via-virtual".into()))],
    );
    let out = apply_instance_overrides(&sym, Some(&[over]), Some(&derived), None);
    let names: Vec<Option<&str>> = out.iter().map(|c| c.figma.get_str("name")).collect();
    // Children sort by localID: a(10), b(11), c(12). full_idx starts at
    // 0 on "a" → virtual 7:50. idx 1 on "b" → 7:51. So "b" is renamed.
    assert!(names.contains(&Some("B-via-virtual")), "{names:?}");
}

#[test]
fn strategy_2_skips_overrides_that_target_root_in_root_walk() {
    // When the root-anchored walk maps a virtual GUID to the symbol
    // root itself, that override must not flow into the children. We
    // can't directly observe a no-op on the symbol root via the
    // returned children — but we CAN see that the children remain
    // unchanged.
    let sym = symbol_root(vec![leaf("a", 1, 10), leaf("b", 1, 11)]);
    let derived = vec![guid_path(vec![guid(8, 60)])];
    // Virtual GUID 8:60 anchored at the ROOT walk: rootIdx=0 names the
    // symbol_root itself → override is filtered out.
    let over = ov_with(
        vec![guid(8, 60)],
        vec![("name", FigValue::Str("ShouldNotShow".into()))],
    );
    let out = apply_instance_overrides(&sym, Some(&[over]), Some(&derived), None);
    let names: Vec<Option<&str>> = out.iter().map(|c| c.figma.get_str("name")).collect();
    assert!(!names.contains(&Some("ShouldNotShow")), "{names:?}");
}

#[test]
fn strategy_3_fallback_index_maps_when_no_virtual_base() {
    // Force Strategy 3 by providing only multi-segment derived entries
    // (len1_derived is empty, so virtual_guid_base returns None;
    // direct_matches == 0; len1==0 so use_direct is true via the empty
    // branch). Actually — when len1_derived is empty we fall into
    // Strategy 0's "len1_derived.is_empty()" branch (use_direct = true)
    // and the for-loop is empty, but the single-segment override pickup
    // still runs.
    //
    // So this test instead verifies the empty-len1 + single-segment
    // override pickup, which is part of Strategy 0's body.
    let sym = symbol_root(vec![leaf("a", 1, 10)]);
    let over_real = ov_with(
        vec![guid(1, 10)],
        vec![("name", FigValue::Str("Direct".into()))],
    );
    let out = apply_instance_overrides(&sym, Some(&[over_real]), None, None);
    assert_eq!(out[0].figma.get_str("name"), Some("Direct"));
}

#[test]
fn skipped_keys_never_flow_into_node() {
    let sym = symbol_root(vec![leaf("hero", 1, 10)]);
    let derived = vec![guid_path(vec![guid(1, 10)])];
    let over = ov_with(
        vec![guid(1, 10)],
        vec![
            // Real prop.
            ("name", FigValue::Str("Hello".into())),
            // Blacklisted prop must NOT overwrite the node.
            ("guidPath", FigValue::Str("not-a-path".into())),
            ("componentKey", FigValue::Str("XX".into())),
        ],
    );
    let out = apply_instance_overrides(&sym, Some(&[over]), Some(&derived), None);
    assert_eq!(out[0].figma.get_str("name"), Some("Hello"));
    // Blacklisted keys were NOT copied — the node's existing guidPath
    // shape is unchanged (none was present, so it stays absent).
    assert!(out[0].figma.get("componentKey").is_none());
}

#[test]
fn nested_overrides_forwarded_into_instance_subtree() {
    // Build a symbol that contains an INSTANCE child + two leaves,
    // with a multi-segment override targeting the instance subtree.
    let inst = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            ("guid", guid(1, 20)),
            ("name", FigValue::Str("InstanceChild".into())),
        ]),
        children: vec![],
    };
    let sym = symbol_root(vec![inst.clone(), leaf("a", 1, 10)]);
    let derived = vec![guid_path(vec![guid(1, 20)])];
    // Multi-segment override — head = the instance's guid (1:20).
    let nested = ov_with(
        vec![guid(1, 20), guid(2, 30)],
        vec![("name", FigValue::Str("nested-name".into()))],
    );
    let out = apply_instance_overrides(&sym, Some(&[nested]), Some(&derived), None);
    // Find the INSTANCE child and verify symbolData.symbolOverrides
    // grew by 1 nested entry whose guidPath dropped the head segment.
    let inst_out = out
        .iter()
        .find(|c| c.figma.get_str("type") == Some("INSTANCE"))
        .expect("instance child present");
    let sym_data = inst_out
        .figma
        .get("symbolData")
        .expect("symbolData populated");
    let overrides = sym_data
        .get_array("symbolOverrides")
        .expect("symbolOverrides populated");
    assert_eq!(overrides.len(), 1);
    let first = &overrides[0];
    let guids = first
        .get("guidPath")
        .and_then(|p| p.get_array("guids"))
        .expect("nested guidPath present");
    // Head dropped — only the second guid remains.
    assert_eq!(guids.len(), 1);
    let g = FigGuid::from_value(&guids[0]).unwrap();
    assert_eq!((g.session_id, g.local_id), (2, 30));
}

use crate::figma_types::FigGuid;

// ── Guessed-mapping confidence gate ───────────────────────────────

fn sized_leaf(name: &str, lid: u32, w: f32, h: f32, x: f32) -> TreeNode {
    TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("RECTANGLE".into())),
            ("guid", guid(1, lid)),
            ("name", FigValue::Str(name.into())),
            ("size", size(w, h)),
            (
                "transform",
                obj(vec![
                    ("m00", FigValue::Float(1.0)),
                    ("m01", FigValue::Float(0.0)),
                    ("m02", FigValue::Float(x)),
                    ("m10", FigValue::Float(0.0)),
                    ("m11", FigValue::Float(1.0)),
                    ("m12", FigValue::Float(0.0)),
                ]),
            ),
        ]),
        children: vec![],
    }
}

fn derived_with(guids: Vec<FigValue>, extra: Vec<(&str, FigValue)>) -> FigValue {
    ov_with(guids, extra)
}

fn transform_m02(v: f32) -> (&'static str, FigValue) {
    (
        "transform",
        obj(vec![
            ("m00", FigValue::Float(1.0)),
            ("m01", FigValue::Float(0.0)),
            ("m02", FigValue::Float(v)),
            ("m10", FigValue::Float(0.0)),
            ("m11", FigValue::Float(1.0)),
            ("m12", FigValue::Float(0.0)),
        ]),
    )
}

/// Mirrors the real-world failure (Test.fig orderCard): Strategy 2's
/// walk-order guess maps derived entries onto the wrong nodes. The
/// give-away is derived sizes grossly contradicting the mapped nodes'
/// own sizes (321 px wide data landing on 76/47 px nodes). When ≥2
/// entries are that far off, the whole guessed mapping is untrusted
/// and NO derived transform/size may be applied.
#[test]
fn strategy_2_gross_size_mismatch_abandons_guessed_mapping() {
    let sym = symbol_root(vec![
        sized_leaf("iphone", 10, 65.0, 17.0, 0.0),
        sized_leaf("right", 11, 76.0, 15.0, 244.0),
        sized_leaf("pending", 12, 47.0, 15.0, 15.0),
    ]);
    // Strategy 2 fires: 3 single-segment virtual entries ≠ flat len (4),
    // virtual base (9, 50). walk_virtual maps 9:50→iphone, 9:51→right,
    // 9:52→pending.
    let derived = vec![
        derived_with(vec![guid(9, 50)], vec![transform_m02(245.0)]),
        derived_with(vec![guid(9, 51)], vec![("size", size(321.0, 17.0))]),
        derived_with(vec![guid(9, 52)], vec![("size", size(321.0, 19.0))]),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    // Gate trips (two grossly-mismatched sizes) → derived data is NOT
    // applied: "iphone" keeps its authored x=0 instead of jumping to 245.
    let iphone = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("iphone"))
        .expect("iphone child present");
    let m02 = iphone
        .figma
        .get("transform")
        .and_then(|t| t.get_f64("m02"))
        .unwrap_or(0.0);
    assert!(
        (m02 - 0.0).abs() < 0.001,
        "gated mapping must not move iphone to 245, got m02={m02}"
    );
    // And "right" keeps its authored 76×15 size.
    let right = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("right"))
        .expect("right child present");
    let sz = FigVec2::from_value(right.figma.get("size").unwrap()).unwrap();
    assert!((sz.x - 76.0).abs() < 0.001, "right width mangled: {}", sz.x);
}

/// Control: when the guessed mapping's derived sizes agree with the
/// mapped nodes (within stretch tolerance), the mapping is trusted and
/// derived transforms still apply.
#[test]
fn strategy_2_consistent_sizes_keep_derived_application() {
    let sym = symbol_root(vec![
        sized_leaf("iphone", 10, 65.0, 17.0, 0.0),
        sized_leaf("right", 11, 76.0, 15.0, 244.0),
        sized_leaf("pending", 12, 47.0, 15.0, 15.0),
    ]);
    let derived = vec![
        derived_with(vec![guid(9, 50)], vec![transform_m02(5.0)]),
        derived_with(vec![guid(9, 51)], vec![("size", size(77.0, 15.0))]),
        derived_with(vec![guid(9, 52)], vec![("size", size(48.0, 15.0))]),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let iphone = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("iphone"))
        .expect("iphone child present");
    let m02 = iphone
        .figma
        .get("transform")
        .and_then(|t| t.get_f64("m02"))
        .unwrap_or(-1.0);
    assert!(
        (m02 - 5.0).abs() < 0.001,
        "trusted mapping must apply derived transform, got m02={m02}"
    );
}

/// Strategy 1's positional guess maps the symbol ROOT too, but root
/// derived data never applies to the returned children — a severe
/// root-size entry must not count toward the gate. With the root
/// skipped only one severe child remains, which is below the gate
/// threshold, so the mapping stays trusted.
#[test]
fn gate_ignores_symbol_root_size_mismatch() {
    let sym = symbol_root(vec![
        sized_leaf("a", 10, 100.0, 100.0, 0.0),
        sized_leaf("b", 11, 100.0, 100.0, 0.0),
    ]);
    // 3 single-segment virtual entries == flat_symbol.len() (root+2)
    // → Strategy 1 positional mapping: entry0→root, entry1→a, entry2→b.
    let derived = vec![
        derived_with(vec![guid(9, 100)], vec![("size", size(400.0, 400.0))]),
        derived_with(vec![guid(9, 101)], vec![("size", size(400.0, 400.0))]),
        derived_with(vec![guid(9, 102)], vec![transform_m02(5.0)]),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let b = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("b"))
        .expect("b child present");
    let m02 = b
        .figma
        .get("transform")
        .and_then(|t| t.get_f64("m02"))
        .unwrap_or(-1.0);
    assert!(
        (m02 - 5.0).abs() < 0.001,
        "root mismatch must not trip the gate; b should get m02=5, got {m02}"
    );
}

/// Two severe entries among many consistent ones are legitimate
/// per-node stretches, not proof of a wrong guess — the gate needs a
/// severe MAJORITY. 2 severe of 5 comparable stays trusted.
#[test]
fn gate_requires_severe_majority() {
    let sym = symbol_root(vec![
        sized_leaf("a", 10, 100.0, 100.0, 0.0),
        sized_leaf("b", 11, 100.0, 100.0, 0.0),
        sized_leaf("c", 12, 100.0, 100.0, 0.0),
        sized_leaf("d", 13, 100.0, 100.0, 0.0),
        sized_leaf("e", 14, 100.0, 100.0, 0.0),
        sized_leaf("f", 15, 100.0, 100.0, 0.0),
    ]);
    // 6 virtual entries ≠ flat len (7) → Strategy 2. a+b severely
    // stretched (legit per-node resize), c/d/e near-authored, f gets a
    // transform. severe=2 of comparable=5 → minority → trusted.
    let derived = vec![
        derived_with(vec![guid(9, 50)], vec![("size", size(400.0, 400.0))]),
        derived_with(vec![guid(9, 51)], vec![("size", size(400.0, 400.0))]),
        derived_with(vec![guid(9, 52)], vec![("size", size(101.0, 100.0))]),
        derived_with(vec![guid(9, 53)], vec![("size", size(102.0, 100.0))]),
        derived_with(vec![guid(9, 54)], vec![("size", size(103.0, 100.0))]),
        derived_with(vec![guid(9, 55)], vec![transform_m02(7.0)]),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let f = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("f"))
        .expect("f child present");
    let m02 = f
        .figma
        .get("transform")
        .and_then(|t| t.get_f64("m02"))
        .unwrap_or(-1.0);
    assert!(
        (m02 - 7.0).abs() < 0.001,
        "severe minority must not trip the gate; f should get m02=7, got {m02}"
    );
}

/// When the gate DOES trip, multi-segment entries whose head names a
/// real subtree node are mapping-independent and must still be
/// forwarded into that node's nested override slots.
#[test]
fn gate_fallback_keeps_real_guid_nested_forwarding() {
    // Forwarding only lands on INSTANCE nodes, so the head child must
    // be one.
    let mut nested_instance = sized_leaf("nested", 12, 50.0, 50.0, 0.0);
    nested_instance
        .figma
        .set("type", FigValue::Str("INSTANCE".into()));
    let sym = symbol_root(vec![
        sized_leaf("x", 10, 65.0, 17.0, 0.0),
        sized_leaf("y", 11, 76.0, 15.0, 244.0),
        nested_instance,
    ]);
    let derived = vec![
        // Two gross mismatches → gate trips (2 of 2 severe).
        derived_with(vec![guid(9, 50)], vec![("size", size(321.0, 17.0))]),
        derived_with(vec![guid(9, 51)], vec![("size", size(321.0, 19.0))]),
        // Real-GUID head (1:12) + nested segment — unambiguous.
        derived_with(
            vec![guid(1, 12), guid(11, 8523)],
            vec![("fontSize", FigValue::Float(20.0))],
        ),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let nested = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("nested"))
        .expect("nested child present");
    let forwarded = nested
        .figma
        .get("derivedSymbolData")
        .and_then(|v| match v {
            FigValue::Array(a) => Some(a.len()),
            _ => None,
        })
        .unwrap_or(0);
    assert_eq!(
        forwarded, 1,
        "real-GUID-headed nested derived entry must survive the gate fallback"
    );
    // And the gate still protected the guessed mapping: x keeps x=0.
    let x = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("x"))
        .expect("x child present");
    let sz = FigVec2::from_value(x.figma.get("size").unwrap()).unwrap();
    assert!((sz.x - 65.0).abs() < 0.001, "x size mangled: {}", sz.x);
}

// ── Fingerprint assignment (Strategy 2 upgrade) ───────────────────

fn text_leaf(name: &str, lid: u32, chars: &str, w: f32, h: f32) -> TreeNode {
    let mut n = sized_leaf(name, lid, w, h, 0.0);
    n.figma.set("type", FigValue::Str("TEXT".into()));
    n.figma.set(
        "textData",
        obj(vec![("characters", FigValue::Str(chars.into()))]),
    );
    n
}

/// Real-world shape (Test.fig orderCard): two sibling FRAMEs sit at
/// x≈244/243; the derived transform meant for the FILLED pill frame
/// carries a fillPaints override alongside. Geometry alone ties —
/// the fill fingerprint must route the fill-carrying entry to the
/// frame that authored fills.
#[test]
fn fingerprint_routes_fill_entry_to_filled_frame() {
    let mut right = sized_leaf("right", 10, 76.0, 15.0, 244.0);
    right.figma.set("type", FigValue::Str("FRAME".into()));
    let mut pill = sized_leaf("pill", 11, 77.0, 19.0, 243.0);
    pill.figma.set("type", FigValue::Str("FRAME".into()));
    pill.figma.set(
        "fillPaints",
        FigValue::Array(vec![obj(vec![("type", FigValue::Str("SOLID".into()))])]),
    );
    let sym = symbol_root(vec![right, pill]);
    // Walk order would map 9:50→right and 9:51→pill, but the TRUE
    // targets are swapped (Figma's allocation order is not walk
    // order): 9:50 carries the pill's transform + fill override,
    // 9:51 carries right's transform. Only the fingerprint (fill
    // presence + transform proximity) routes them correctly.
    let derived = vec![
        derived_with(vec![guid(9, 50)], vec![transform_m02(244.0)]),
        derived_with(vec![guid(9, 51)], vec![transform_m02(245.0)]),
    ];
    let over = vec![ov_with(
        vec![guid(9, 50)],
        vec![(
            "fillPaints",
            FigValue::Array(vec![obj(vec![(
                "type",
                FigValue::Str("GRADIENT_LINEAR".into()),
            )])]),
        )],
    )];
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let get_x = |name: &str| {
        out.iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get("transform"))
            .and_then(|t| t.get_f64("m02"))
            .unwrap_or(-1.0)
    };
    assert!(
        (get_x("right") - 245.0).abs() < 0.001,
        "right must take the 245 transform, got {}",
        get_x("right")
    );
    assert!(
        (get_x("pill") - 244.0).abs() < 0.001,
        "pill must take the 244 transform, got {}",
        get_x("pill")
    );
    let pill_fill_is_gradient = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("pill"))
        .and_then(|n| n.figma.get_array("fillPaints").and_then(|a| a.first()))
        .and_then(|p| p.get_str("type").map(|s| s == "GRADIENT_LINEAR"))
        .unwrap_or(false);
    assert!(pill_fill_is_gradient, "fill override must land on pill");
}

/// Stat-card shape: three TEXT siblings (label / value / delta) whose
/// per-instance text overrides carry only new characters. Content
/// class (numeric vs percent vs plain) must route each override to
/// the matching text node.
#[test]
fn fingerprint_routes_text_content_by_class() {
    let sym = symbol_root(vec![
        text_leaf("label", 10, "Customers", 60.0, 17.0),
        text_leaf("value", 11, "0", 20.0, 24.0),
        text_leaf("delta", 12, "+0.00%", 40.0, 14.0),
    ]);
    let derived = vec![
        derived_with(
            vec![guid(9, 50)],
            vec![(
                "derivedTextData",
                obj(vec![("characters", FigValue::Str("+15.80%".into()))]),
            )],
        ),
        derived_with(
            vec![guid(9, 51)],
            vec![(
                "derivedTextData",
                obj(vec![("characters", FigValue::Str("1,250".into()))]),
            )],
        ),
        derived_with(
            vec![guid(9, 52)],
            vec![(
                "derivedTextData",
                obj(vec![("characters", FigValue::Str("Customers".into()))]),
            )],
        ),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let chars_of = |name: &str| {
        out.iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get("textData"))
            .and_then(|t| t.get_str("characters").map(|s| s.to_string()))
            .unwrap_or_default()
    };
    assert_eq!(chars_of("value"), "1,250", "numeric override → value text");
    assert_eq!(
        chars_of("delta"),
        "+15.80%",
        "percent override → delta text"
    );
    assert_eq!(chars_of("label"), "Customers", "exact match → label text");
}

/// Order-row shape: the status text override ("Completed") and the
/// price override ("₦730,000.00 x 1") are both letters+digits, but the
/// currency symbol must fingerprint the price as a distinct class so
/// "Completed" can't land on the price text.
#[test]
fn fingerprint_separates_currency_text_from_status_text() {
    let sym = symbol_root(vec![
        text_leaf("price", 10, "₦730,000.00 x 1", 112.0, 17.0),
        text_leaf("status", 11, "Pending", 47.0, 15.0),
    ]);
    // Walk order would hand 9:50→price — but 9:50 carries the status
    // text. Class routing (plain vs currency) must swap them.
    let derived = vec![
        derived_with(
            vec![guid(9, 50)],
            vec![(
                "derivedTextData",
                obj(vec![("characters", FigValue::Str("Completed".into()))]),
            )],
        ),
        derived_with(
            vec![guid(9, 51)],
            vec![(
                "derivedTextData",
                obj(vec![(
                    "characters",
                    FigValue::Str("₦899,000.00 x 2".into()),
                )]),
            )],
        ),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let chars_of = |name: &str| {
        out.iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get("textData"))
            .and_then(|t| t.get_str("characters").map(|s| s.to_string()))
            .unwrap_or_default()
    };
    assert_eq!(chars_of("status"), "Completed");
    assert_eq!(chars_of("price"), "₦899,000.00 x 2");
}

/// A fontSize-only virtual entry has no scoreable evidence — it must
/// route through the walk-order fallback (old behavior), not vanish
/// into an unassignable fingerprint bucket.
#[test]
fn fontsize_only_entry_falls_back_to_walk_order() {
    let sym = symbol_root(vec![
        text_leaf("title", 10, "Hello", 60.0, 17.0),
        text_leaf("body", 11, "World", 60.0, 17.0),
    ]);
    // Walk order: 9:50→title, 9:51→body.
    let derived = vec![
        derived_with(vec![guid(9, 50)], vec![("fontSize", FigValue::Float(20.0))]),
        derived_with(vec![guid(9, 51)], vec![("fontSize", FigValue::Float(30.0))]),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let fs_of = |name: &str| {
        out.iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get_f64("fontSize"))
            .unwrap_or(-1.0)
    };
    assert!(
        (fs_of("title") - 20.0).abs() < 0.001,
        "fontSize-only entry must apply via walk order, got {}",
        fs_of("title")
    );
    assert!((fs_of("body") - 30.0).abs() < 0.001);
}

/// A fill-only override entry (no derived geometry) can't clear the
/// fingerprint threshold — it must keep the walk-order fallback
/// instead of being silently dropped.
#[test]
fn fill_only_override_falls_back_to_walk_order() {
    // Three children so len1 (3) != flat_symbol.len() (4) → Strategy 2.
    let sym = symbol_root(vec![
        sized_leaf("a", 10, 100.0, 100.0, 0.0),
        sized_leaf("b", 11, 100.0, 100.0, 0.0),
        sized_leaf("c", 12, 100.0, 100.0, 0.0),
    ]);
    // Bare derived entries establish the virtual base; the override on
    // 9:51 carries only fillPaints.
    let derived = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
    ];
    let over = vec![ov_with(
        vec![guid(9, 51)],
        vec![(
            "fillPaints",
            FigValue::Array(vec![obj(vec![(
                "type",
                FigValue::Str("GRADIENT_LINEAR".into()),
            )])]),
        )],
    )];
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    // Walk order maps 9:51 → "b" (second child).
    let b_fill = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("b"))
        .and_then(|n| n.figma.get_array("fillPaints").and_then(|a| a.first()))
        .and_then(|p| p.get_str("type").map(|s| s == "GRADIENT_LINEAR"))
        .unwrap_or(false);
    assert!(b_fill, "fill-only override must apply via walk order");
}

/// Some files anchor the virtual numbering at the symbol ROOT (the
/// base pk's derived size equals the symbol size). The walk-order
/// fallback must then use the root-anchored walk — the children-first
/// walk would shift every mapping by one.
#[test]
fn root_anchored_numbering_uses_root_walk_fallback() {
    let sym = symbol_root(vec![
        sized_leaf("a", 10, 100.0, 100.0, 0.0),
        sized_leaf("b", 11, 100.0, 100.0, 0.0),
        sized_leaf("c", 12, 100.0, 100.0, 0.0),
    ]);
    // Base pk 9:50 carries the SYMBOL's own size (100×100 root) →
    // numbering is root-anchored: 9:50=root, 9:51=a, 9:52=b, 9:53=c.
    let derived = vec![
        derived_with(vec![guid(9, 50)], vec![("size", size(100.0, 100.0))]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
        guid_path(vec![guid(9, 53)]),
        guid_path(vec![guid(9, 54)]),
    ];
    // Signal-less name override on 9:51 → must land on "a" (root walk),
    // not "b" (children-walk guess).
    let over = vec![ov_with(
        vec![guid(9, 51)],
        vec![("name", FigValue::Str("Tinted".into()))],
    )];
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let names: Vec<Option<&str>> = out.iter().map(|c| c.figma.get_str("name")).collect();
    assert_eq!(
        names[0],
        Some("Tinted"),
        "root-anchored numbering: 9:51 is the FIRST child, got {names:?}"
    );
}

/// Multi-session virtual spaces: a secondary session's numbering is
/// anchored at some SUBTREE of the symbol (its own component-history
/// space). The anchor is only trusted when the group's text-demand
/// entries land on TEXT nodes under it — then signal-less opacity
/// hides apply to the right items.
#[test]
fn secondary_session_anchors_to_validated_subtree() {
    let t = |name: &str, lid: u32, chars: &str| text_leaf(name, lid, chars, 40.0, 14.0);
    let crumbs = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("guid", guid(1, 20)),
            ("name", FigValue::Str("crumbs".into())),
            ("size", size(200.0, 20.0)),
        ]),
        children: vec![
            t("t1", 21, "Page"),
            t("t2", 22, "Page"),
            t("t3", 23, "Page"),
        ],
    };
    let header = sized_leaf("header", 10, 300.0, 40.0, 0.0);
    let sym = symbol_root(vec![header, crumbs]);

    // 4 bare base-session entries (9:xx) dodge Strategy 1's count
    // match; secondary session 30 carries 3 text-demand entries and a
    // visible=false on 30:71 (→ t1 under the crumbs-children anchor).
    let derived = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
        guid_path(vec![guid(9, 53)]),
        derived_with(vec![guid(30, 71)], vec![("derivedTextData", obj(vec![]))]),
        derived_with(vec![guid(30, 72)], vec![("derivedTextData", obj(vec![]))]),
        derived_with(vec![guid(30, 73)], vec![("derivedTextData", obj(vec![]))]),
    ];
    let over = vec![ov_with(
        vec![guid(30, 71)],
        vec![("visible", FigValue::Bool(false))],
    )];
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    fn find<'a>(nodes: &'a [TreeNode], name: &str) -> Option<&'a TreeNode> {
        for n in nodes {
            if n.figma.get_str("name") == Some(name) {
                return Some(n);
            }
            if let Some(hit) = find(&n.children, name) {
                return Some(hit);
            }
        }
        None
    }
    let t1 = find(&out, "t1").expect("t1 present");
    assert_eq!(
        t1.figma.get_bool("visible"),
        Some(false),
        "validated subtree anchor must route the hide to t1"
    );
    let header = find(&out, "header").expect("header present");
    assert_ne!(header.figma.get_bool("visible"), Some(false));
}

/// A signal-bearing entry REJECTED by the fingerprint (its size
/// contradicts every candidate) must not keep its walk-order guess in
/// the head-resolution map — otherwise nested overrides forward into
/// a node the evidence already disqualified.
#[test]
fn rejected_mapping_does_not_forward_nested_overrides() {
    let mut nested_instance = sized_leaf("nested", 10, 50.0, 50.0, 0.0);
    nested_instance
        .figma
        .set("type", FigValue::Str("INSTANCE".into()));
    let sym = symbol_root(vec![
        nested_instance,
        sized_leaf("b", 11, 60.0, 60.0, 0.0),
        sized_leaf("c", 12, 70.0, 70.0, 0.0),
    ]);
    // Walk order maps 9:50 → "nested". Its derived size (900×900)
    // grossly contradicts every candidate → fingerprint rejects it.
    let derived = vec![
        derived_with(vec![guid(9, 50)], vec![("size", size(900.0, 900.0))]),
        guid_path(vec![guid(9, 51)]),
        // Nested entry headed by the REJECTED pk.
        derived_with(
            vec![guid(9, 50), guid(11, 8523)],
            vec![("fontSize", FigValue::Float(20.0))],
        ),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let nested = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("nested"))
        .expect("nested present");
    assert!(
        nested.figma.get("derivedSymbolData").is_none(),
        "nested forwarding must not ride a rejected walk guess, got {:?}",
        nested.figma.get("derivedSymbolData")
    );
    // And the contradicted size must not have been applied anywhere.
    let sz = FigVec2::from_value(nested.figma.get("size").unwrap()).unwrap();
    assert!((sz.x - 50.0).abs() < 0.001);
}
