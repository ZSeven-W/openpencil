//! Fingerprint-assignment, cache/seeding-adjacent, foreign-session
//! and nested-forwarding tests — carved off `tests.rs` at the
//! 800-line cap.

use super::tests::{
    derived_with, guid, guid_path, obj, ov_with, size, sized_leaf, symbol_root, transform_m02,
};
use super::*;

// ── Fingerprint assignment (Strategy 2 upgrade) ───────────────────

pub(super) fn text_leaf(name: &str, lid: u32, chars: &str, w: f32, h: f32) -> TreeNode {
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

fn image_paint(hash_byte: u8) -> FigValue {
    obj(vec![
        ("type", FigValue::Str("IMAGE".into())),
        (
            "image",
            obj(vec![("hash", FigValue::Bytes(vec![hash_byte; 4]))]),
        ),
    ])
}

fn solid_paint(opacity: f32) -> FigValue {
    obj(vec![
        ("type", FigValue::Str("SOLID".into())),
        ("opacity", FigValue::Float(opacity)),
        (
            "color",
            obj(vec![
                ("r", FigValue::Float(0.5)),
                ("g", FigValue::Float(0.5)),
                ("b", FigValue::Float(0.5)),
            ]),
        ),
    ])
}

/// Real-world shape (Test.fig orderCard thumbnails): the per-row
/// IMAGE fill override entry carries NO size / transform / text —
/// only fillPaints. Walk order maps its pk elsewhere, but an IMAGE
/// override can only mean the (unique) IMAGE-filled node — the fill
/// fingerprint must route it there instead of dropping it.
#[test]
fn image_fill_override_routes_to_image_filled_node() {
    let mut img = sized_leaf("thumb", 12, 49.0, 49.0, 0.0);
    img.figma
        .set("fillPaints", FigValue::Array(vec![image_paint(0x11)]));
    let sym = symbol_root(vec![
        sized_leaf("a", 10, 100.0, 100.0, 0.0),
        sized_leaf("b", 11, 100.0, 100.0, 0.0),
        img,
    ]);
    let derived = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
    ];
    // Walk order would map 9:50 → "a"; the IMAGE evidence must win.
    let over = vec![ov_with(
        vec![guid(9, 50)],
        vec![("fillPaints", FigValue::Array(vec![image_paint(0x22)]))],
    )];
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let hash_of = |name: &str| -> Vec<u8> {
        out.iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get_array("fillPaints").and_then(|a| a.first()))
            .and_then(|p| p.get("image"))
            .and_then(|i| match i.get("hash") {
                Some(FigValue::Bytes(b)) => Some(b.clone()),
                _ => None,
            })
            .unwrap_or_default()
    };
    assert_eq!(
        hash_of("thumb"),
        vec![0x22; 4],
        "IMAGE override must land on the image-filled node"
    );
    let a_has_fill = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("a"))
        .and_then(|n| n.figma.get_array("fillPaints"))
        .is_some();
    assert!(!a_has_fill, "walk-order neighbour must not take the image");
}

/// A LOW-OPACITY solid fill override (status-chip tint) is rare
/// enough to identify its node: it must route to the sibling whose
/// authored fill has the matching opacity, not the walk-order one.
#[test]
fn low_opacity_fill_override_routes_by_opacity_match() {
    let mut plain = sized_leaf("plain", 10, 100.0, 100.0, 0.0);
    plain
        .figma
        .set("fillPaints", FigValue::Array(vec![solid_paint(1.0)]));
    let mut chip = sized_leaf("chip", 11, 100.0, 100.0, 0.0);
    chip.figma
        .set("fillPaints", FigValue::Array(vec![solid_paint(0.12)]));
    let sym = symbol_root(vec![plain, chip, sized_leaf("c", 12, 100.0, 100.0, 0.0)]);
    let derived = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
    ];
    // Walk order would map 9:50 → "plain" (and the root-walk guard
    // would otherwise drop it); opacity affinity says chip. The
    // override tint is GREEN so the assertion can see it landed.
    let green = obj(vec![
        ("type", FigValue::Str("SOLID".into())),
        ("opacity", FigValue::Float(0.12)),
        (
            "color",
            obj(vec![
                ("r", FigValue::Float(0.2)),
                ("g", FigValue::Float(1.0)),
                ("b", FigValue::Float(0.44)),
            ]),
        ),
    ]);
    let over = vec![ov_with(
        vec![guid(9, 50)],
        vec![("fillPaints", FigValue::Array(vec![green]))],
    )];
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let g_of = |name: &str| -> f64 {
        out.iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get_array("fillPaints").and_then(|a| a.first()))
            .and_then(|p| p.get("color"))
            .and_then(|c| c.get_f64("g"))
            .unwrap_or(-1.0)
    };
    assert!(
        (g_of("chip") - 1.0).abs() < 0.001,
        "low-opacity override must land on the opacity-matching chip, got g={}",
        g_of("chip")
    );
    assert!(
        (g_of("plain") - 0.5).abs() < 0.001,
        "plain sibling keeps its own fill, got g={}",
        g_of("plain")
    );
}

/// A strong fill hint whose evidence is INAPPLICABLE (no candidate
/// carries a matching fill at all — e.g. the override ADDS a tinted
/// fill to an unfilled node) must fall back to walk order like any
/// other fill-only entry, not silently vanish. Hard-signal
/// rejections (size/transform/text contradicted) still drop.
#[test]
fn unmatched_low_opacity_fill_falls_back_to_walk_order() {
    // No child has any authored fill → the RareSolid hint can't score
    // a match anywhere and the entry is fingerprint-rejected.
    let sym = symbol_root(vec![
        sized_leaf("a", 10, 100.0, 100.0, 0.0),
        sized_leaf("b", 11, 100.0, 100.0, 0.0),
        sized_leaf("c", 12, 100.0, 100.0, 0.0),
    ]);
    let derived = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
    ];
    // Walk order maps 9:51 -> "b".
    let over = vec![ov_with(
        vec![guid(9, 51)],
        vec![("fillPaints", FigValue::Array(vec![solid_paint(0.12)]))],
    )];
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let b_opacity = out
        .iter()
        .find(|n| n.figma.get_str("name") == Some("b"))
        .and_then(|n| n.figma.get_array("fillPaints").and_then(|a| a.first()))
        .and_then(|p| p.get_f64("opacity"));
    let b_opacity = b_opacity.expect("unmatched low-opacity fill must apply via walk order");
    assert!(
        (b_opacity - 0.12).abs() < 0.001,
        "walk-order fill kept its opacity, got {b_opacity}"
    );
}

/// Component-swap stale-derived shape (Test.fig Sales-card icon): an
/// `overriddenSymbolID` swap leaves the OLD component's derived
/// entries in the array (listed FIRST, so they seed the virtual-GUID
/// base and claim the walk-order prior) alongside the NEW component's
/// entries. A near-EXACT geometric match must beat the stale entry's
/// walk-order advantage so the swapped-in frame gets its true size.
#[test]
fn near_exact_geometry_beats_stale_walk_order_prior() {
    // Swapped-in Graph icon subtree: frame 19.39, Stroke1 16.13,
    // Stroke3 8.92 (master sizes from Test.fig).
    let stroke3 = sized_leaf("stroke3", 12, 8.92, 8.79, 0.0);
    let stroke1 = sized_leaf("stroke1", 11, 16.13, 16.04, 0.0);
    let mut frame = sized_leaf("frame", 10, 19.39, 19.84, 0.0);
    frame.figma.set("type", FigValue::Str("FRAME".into()));
    let frame = TreeNode {
        children: vec![stroke1, stroke3],
        ..frame
    };
    // Real Graph symbol is 24×24 → instance 20×20 gives ratio 0.833.
    let sym = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("SYMBOL".into())),
            ("guid", guid(0, 0)),
            ("size", size(24.0, 24.0)),
        ]),
        children: vec![frame],
    };
    // 8 derived entries as production forwards them: the STALE 2-User
    // component (13723-13727, FIRST — seeds the base + walk prior)
    // then the correct Graph entries (13627-13629). Instance box 20×20.
    let d = |lid: u32, x: f32, y: f32| derived_with(vec![guid(2, lid)], vec![("size", size(x, y))]);
    let derived = vec![
        d(13723, 15.4, 14.63),
        d(13724, 11.4, 4.67),
        d(13725, 7.31, 7.31),
        d(13726, 2.36, 5.43),
        d(13727, 2.2, 2.9),
        d(13627, 16.16, 16.54),
        d(13628, 13.44, 13.37),
        d(13629, 7.43, 7.32),
    ];
    // Production also forwards strokePaints overrides on the two
    // Graph strokes (13628/13629) — include them so the scoring
    // matches the real path.
    let stroke_paint = || FigValue::Array(vec![obj(vec![("type", FigValue::Str("SOLID".into()))])]);
    let over = vec![
        ov_with(vec![guid(2, 13628)], vec![("strokePaints", stroke_paint())]),
        ov_with(vec![guid(2, 13629)], vec![("strokePaints", stroke_paint())]),
    ];
    let out = apply_instance_overrides(
        &sym,
        Some(&over),
        Some(&derived),
        Some(FigVec2 { x: 20.0, y: 20.0 }),
    );
    fn find<'a>(nodes: &'a [TreeNode], name: &str) -> Option<&'a TreeNode> {
        for n in nodes {
            if n.figma.get_str("name") == Some(name) {
                return Some(n);
            }
            if let Some(h) = find(&n.children, name) {
                return Some(h);
            }
        }
        None
    }
    let frame = find(&out, "frame").expect("frame present");
    let fsz = FigVec2::from_value(frame.figma.get("size").unwrap()).unwrap();
    assert!(
        (fsz.x - 16.16).abs() < 0.3 && (fsz.y - 16.54).abs() < 0.3,
        "near-exact Graph frame derived must win over stale 2-User derived, got {fsz:?}"
    );
    let stroke1 = find(&out, "stroke1").expect("stroke1 present");
    let s1 = FigVec2::from_value(stroke1.figma.get("size").unwrap()).unwrap();
    assert!(
        (s1.x - 13.44).abs() < 0.3,
        "stroke1 must keep its own derived size, got {s1:?}"
    );
}

/// Component-swap SAME-SIZED-frame shape (Test.fig Folder/Graph
/// cards): the stale pre-swap "2 User" frame derived (15.4) and the
/// swapped-in Folder frame's true size are nearly IDENTICAL, so the
/// near-exact bonus can't disambiguate them and the stale entry wins
/// on walk-order. The stale cluster must be dropped by geometric fit
/// before fingerprinting.
#[test]
fn swap_drops_stale_cluster_with_same_sized_frame() {
    // Swapped-in Folder icon: frame 18.5, two 8-ish strokes.
    let s1 = sized_leaf("fstroke1", 12, 8.7, 8.7, 0.0);
    let s2 = sized_leaf("fstroke2", 11, 8.6, 8.6, 0.0);
    let mut frame = sized_leaf("folderframe", 10, 18.5, 17.55, 0.0);
    frame.figma.set("type", FigValue::Str("FRAME".into()));
    let frame = TreeNode {
        children: vec![s2, s1],
        ..frame
    };
    let sym = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("SYMBOL".into())),
            ("guid", guid(0, 0)),
            ("size", size(24.0, 24.0)),
        ]),
        children: vec![frame],
    };
    // Instance 20×20 → ratio 0.833: Folder frame → 15.42, strokes → ~7.25.
    let d = |lid: u32, x: f32, y: f32| derived_with(vec![guid(2, lid)], vec![("size", size(x, y))]);
    // STALE 2-User cluster (13723-13727) FIRST — frame 15.4 ≈ Folder
    // frame's scaled 15.42, so it's indistinguishable by size.
    let derived = vec![
        d(13723, 15.4, 14.63),
        d(13724, 11.4, 4.67),
        d(13725, 7.31, 7.31),
        d(13726, 2.36, 5.43),
        d(13727, 2.2, 2.9),
        // VALID Folder cluster (13617-13619) — all near-exact to Folder.
        d(13617, 15.42, 14.63),
        d(13618, 7.25, 7.25),
        d(13619, 7.17, 7.17),
    ];
    let out = filter_and_apply_swap(&sym, &derived, FigVec2 { x: 20.0, y: 20.0 });
    fn find<'a>(nodes: &'a [TreeNode], name: &str) -> Option<&'a TreeNode> {
        for n in nodes {
            if n.figma.get_str("name") == Some(name) {
                return Some(n);
            }
            if let Some(h) = find(&n.children, name) {
                return Some(h);
            }
        }
        None
    }
    let frame = find(&out, "folderframe").expect("frame present");
    let fsz = FigVec2::from_value(frame.figma.get("size").unwrap()).unwrap();
    // Whichever cluster wins, the frame must end ~15.4 (both agree) —
    // the real signal is the STROKES: they must be Folder's ~7.25,
    // NOT the stale 2-User's 11.4/2.36/2.2.
    assert!((fsz.x - 15.42).abs() < 0.3, "frame size, got {fsz:?}");
    let stroke = find(&out, "fstroke1").expect("stroke present");
    let ssz = FigVec2::from_value(stroke.figma.get("size").unwrap()).unwrap();
    assert!(
        (ssz.x - 7.25).abs() < 0.5,
        "Folder stroke must get its own ~7.25 size, not a stale 2-User size, got {ssz:?}"
    );
}

/// Helper: run the swap-stale-derived filter then apply, mirroring
/// what `convert_instance` does when `overriddenSymbolID` swaps.
fn filter_and_apply_swap(sym: &TreeNode, derived: &[FigValue], isz: FigVec2) -> Vec<TreeNode> {
    let filtered = crate::instance::filter_swap_stale_derived(derived, sym, Some(isz));
    apply_instance_overrides(sym, None, Some(&filtered), Some(isz))
}

/// Space-between drift signature (Test.fig summary-card filter): the
/// derived transform moves a node along ONE axis only (x pushed by
/// justify, y identical). d=dx+dy lands in the dead zone between the
/// proximity tiers, but an exact-match axis plus a moderate other
/// axis still identifies the node.
#[test]
fn single_axis_transform_drift_routes_to_matching_node() {
    let icon = sized_leaf("icon", 10, 36.0, 36.0, 0.0);
    let mut filter = sized_leaf("filter", 11, 82.0, 16.0, 218.0);
    filter.figma.set(
        "transform",
        obj(vec![
            ("m02", FigValue::Float(218.0)),
            ("m12", FigValue::Float(10.0)),
        ]),
    );
    let mid = sized_leaf("mid", 12, 50.0, 50.0, 100.0);
    let sym = symbol_root(vec![icon, filter, mid]);
    // Walk order maps 9:50 -> icon; the true target is filter
    // (m12=10 exact, m02 drifted 218 -> 311.3 by space-between).
    let derived = vec![
        derived_with(
            vec![guid(9, 50)],
            vec![(
                "transform",
                obj(vec![
                    ("m02", FigValue::Float(311.3)),
                    ("m12", FigValue::Float(10.0)),
                ]),
            )],
        ),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
    ];
    let out = apply_instance_overrides(&sym, None, Some(&derived), None);
    let m02_of = |name: &str| {
        out.iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get("transform"))
            .and_then(|t| t.get_f64("m02"))
            .unwrap_or(-1.0)
    };
    assert!(
        (m02_of("filter") - 311.3).abs() < 0.001,
        "single-axis drift must route to the y-matching node, got {}",
        m02_of("filter")
    );
    assert!(
        (m02_of("icon")).abs() < 0.001,
        "walk-order neighbour keeps its own transform, got {}",
        m02_of("icon")
    );
}

/// When the subtree DOES carry a hint-matching node but this entry
/// LOST it (another entry claimed it, or the evidence contradicts),
/// the rejection is a conflict — reviving the entry through walk
/// order would paint its fill onto an arbitrary sibling.
#[test]
fn conflicted_image_fill_reject_stays_dropped() {
    let mut img = sized_leaf("thumb", 11, 49.0, 49.0, 0.0);
    img.figma
        .set("fillPaints", FigValue::Array(vec![image_paint(0x11)]));
    let sym = symbol_root(vec![
        sized_leaf("a", 10, 100.0, 100.0, 0.0),
        img,
        sized_leaf("c", 12, 100.0, 100.0, 0.0),
    ]);
    let derived = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
    ];
    // TWO image overrides compete for the single image-filled node;
    // the loser must be dropped, not walk-ordered onto "c".
    let over = vec![
        ov_with(
            vec![guid(9, 51)],
            vec![("fillPaints", FigValue::Array(vec![image_paint(0x22)]))],
        ),
        ov_with(
            vec![guid(9, 52)],
            vec![("fillPaints", FigValue::Array(vec![image_paint(0x33)]))],
        ),
    ];
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let fills_of = |name: &str| -> bool {
        out.iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get_array("fillPaints"))
            .map(|a| !a.is_empty())
            .unwrap_or(false)
    };
    assert!(fills_of("thumb"), "winner applies to the image node");
    assert!(
        !fills_of("a") && !fills_of("c"),
        "conflicted loser must not revive onto a sibling via walk order"
    );
}
