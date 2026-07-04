//! Fingerprint-assignment, cache/seeding-adjacent, foreign-session
//! and nested-forwarding tests — carved off `tests.rs` at the
//! 800-line cap.

use super::tests::{
    derived_with, guid, guid_path, obj, ov_with, size, sized_leaf, symbol_root, transform_m02,
};
use super::*;

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

/// Breadcrumb regression shape (Test.fig Top Nav): a foreign group's
/// text demands ALL validate under a wrong lid-order anchor, but the
/// group also carries a nested-path HEAD pk — which that anchor maps
/// onto a plain FRAME. A head can only be an INSTANCE, so the anchor
/// must be rejected: the opacity=0 meant for an item row must never
/// hide the container frame.
fn breadcrumb_shape() -> (TreeNode, Vec<FigValue>, Vec<FigValue>) {
    let t = |name: &str, lid: u32, chars: &str| text_leaf(name, lid, chars, 40.0, 14.0);
    let item = |lid: u32, text_lid: u32| TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("guid", guid(1, lid)),
            ("name", FigValue::Str("item".into())),
            ("size", size(45.0, 15.0)),
        ]),
        children: vec![t("page", text_lid, "Page")],
    };
    let home = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            ("guid", guid(1, 21)),
            ("name", FigValue::Str("home".into())),
            ("size", size(16.0, 16.0)),
        ]),
        children: vec![],
    };
    let crumbs = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("guid", guid(1, 20)),
            ("name", FigValue::Str("crumbs".into())),
            ("size", size(200.0, 20.0)),
        ]),
        children: vec![home, item(22, 23), item(24, 25)],
    };
    let header = sized_leaf("header", 10, 300.0, 40.0, 0.0);
    let sym = symbol_root(vec![header, crumbs]);

    // Base-session bare entries (9:xx) dodge Strategy 1. Session 30:
    // two IDENTICAL opacity=0 overrides (the hidden items), two
    // derived-text entries, and a nested path headed at 30:75. Under
    // the crumbs-self lid-order walk (71=crumbs, 72=home, 73=item,
    // 74=page, 75=item, 76=page) the text demands 74/76 both land on
    // TEXT — but 75 (a nested HEAD, so an INSTANCE) lands on a FRAME.
    let derived = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        derived_with(vec![guid(30, 74)], vec![("derivedTextData", obj(vec![]))]),
        derived_with(vec![guid(30, 76)], vec![("derivedTextData", obj(vec![]))]),
    ];
    let over = vec![
        ov_with(vec![guid(30, 71)], vec![("opacity", FigValue::Float(0.0))]),
        ov_with(vec![guid(30, 73)], vec![("opacity", FigValue::Float(0.0))]),
        ov_with(
            vec![guid(30, 75), guid(29221, 5)],
            vec![("name", FigValue::Str("x".into()))],
        ),
    ];
    (sym, over, derived)
}

fn find_named<'a>(nodes: &'a [TreeNode], name: &str) -> Vec<&'a TreeNode> {
    let mut out = Vec::new();
    fn go<'a>(nodes: &'a [TreeNode], name: &str, out: &mut Vec<&'a TreeNode>) {
        for n in nodes {
            if n.figma.get_str("name") == Some(name) {
                out.push(n);
            }
            go(&n.children, name, out);
        }
    }
    go(nodes, name, &mut out);
    out
}

#[test]
fn foreign_anchor_rejected_when_nested_head_lands_on_non_instance() {
    let (sym, over, derived) = breadcrumb_shape();
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let crumbs = &find_named(&out, "crumbs")[0];
    assert_ne!(
        crumbs.figma.get_f64("opacity"),
        Some(0.0),
        "container frame must not take an item's opacity=0"
    );
    let home = &find_named(&out, "home")[0];
    assert_ne!(home.figma.get_f64("opacity"), Some(0.0));
}

/// When no anchor validates, K identical cosmetic overrides matching
/// a UNIQUE family of K same-named same-typed siblings apply to the
/// whole family — permutation-independent, so no guessing involved.
#[test]
fn foreign_identical_overrides_apply_to_unique_sibling_family() {
    let (sym, over, derived) = breadcrumb_shape();
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let items = find_named(&out, "item");
    assert_eq!(items.len(), 2);
    for it in &items {
        assert_eq!(
            it.figma.get_f64("opacity"),
            Some(0.0),
            "identical opacity=0 pair must land on the unique item family"
        );
    }
    let crumbs = &find_named(&out, "crumbs")[0];
    assert_ne!(crumbs.figma.get_f64("opacity"), Some(0.0));
}

/// The family pairing is order-ARBITRARY — it is only safe because
/// every entry carries the same cosmetic payload. Any per-pk derived
/// data riding on those pks (fontSize, sizes, …) must therefore be
/// dropped, not applied through the arbitrary pairing.
#[test]
fn family_fallback_applies_only_the_cosmetic_payload() {
    let (sym, over, mut derived) = breadcrumb_shape();
    // A derived fontSize rides on family payload pk 30:71 — fontSize
    // alone is not a fingerprint signal, so the pk still reaches the
    // family fallback with this data attached.
    derived.push(derived_with(
        vec![guid(30, 71)],
        vec![("fontSize", FigValue::Float(99.0))],
    ));
    let out = apply_instance_overrides(&sym, Some(&over), Some(&derived), None);
    let items = find_named(&out, "item");
    assert_eq!(items.len(), 2);
    for it in &items {
        assert_eq!(
            it.figma.get_f64("opacity"),
            Some(0.0),
            "cosmetic payload still applies to the family"
        );
        assert_ne!(
            it.figma.get_f64("fontSize"),
            Some(99.0),
            "per-pk derived data must not ride the arbitrary pairing"
        );
    }
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
