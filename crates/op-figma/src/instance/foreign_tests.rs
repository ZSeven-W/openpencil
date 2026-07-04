//! Foreign-session anchoring, uniform-family fallback, swap-cluster
//! and nested-forwarding tests — carved off `fingerprint_tests.rs`
//! at the 800-line cap.

use super::fingerprint_tests::text_leaf;
use super::tests::{derived_with, guid, guid_path, obj, ov_with, size, sized_leaf, symbol_root};
use super::*;

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
