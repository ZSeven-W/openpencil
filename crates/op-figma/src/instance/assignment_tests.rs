//! Assignment-cache and pooled-seeding tests kept separate from the
//! main instance strategy matrix to keep each test module small.

use super::*;
use std::rc::Rc;

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

fn sized_leaf(name: &str, lid: u32, w: f32, h: f32) -> TreeNode {
    TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("TEXT".into())),
            ("guid", guid(1, lid)),
            ("name", FigValue::Str(name.into())),
            ("size", size(w, h)),
        ]),
        children: vec![],
    }
}

fn text_leaf(name: &str, lid: u32, chars: &str, w: f32, h: f32) -> TreeNode {
    let mut n = sized_leaf(name, lid, w, h);
    n.figma.set(
        "textData",
        obj(vec![("characters", FigValue::Str(chars.into()))]),
    );
    n
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

/// Instances of the same SYMBOL share one virtual-ID space. Evidence
/// found while converting one instance (a numeric text override pins
/// 9:51 to the value node) must be reused for the next instance,
/// whose entry on 9:51 is a signal-less `visible=false`.
#[test]
fn assignment_cache_shares_evidence_across_instances() {
    let make_sym = || {
        symbol_root(vec![
            text_leaf("label", 10, "Customers", 60.0, 17.0),
            text_leaf("value", 11, "0", 20.0, 24.0),
        ])
    };
    let mut cache: HashMap<String, String> = HashMap::new();

    let sym = make_sym();
    let derived_a = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
        guid_path(vec![guid(9, 53)]),
    ];
    let over_a = vec![ov_with(
        vec![guid(9, 51)],
        vec![(
            "textData",
            obj(vec![("characters", FigValue::Str("1,250".into()))]),
        )],
    )];
    let out_a =
        apply_instance_overrides_cached(&sym, Some(&over_a), Some(&derived_a), None, &mut cache);
    let a_value_chars = out_a
        .iter()
        .find(|n| n.figma.get_str("name") == Some("value"))
        .and_then(|n| n.figma.get("textData"))
        .and_then(|t| t.get_str("characters").map(|s| s.to_string()))
        .unwrap_or_default();
    assert_eq!(a_value_chars, "1,250", "instance A pins 9:51 -> value");

    let sym_b = make_sym();
    let derived_b = vec![
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
        guid_path(vec![guid(9, 53)]),
        guid_path(vec![guid(9, 54)]),
    ];
    let over_b = vec![ov_with(
        vec![guid(9, 51)],
        vec![("visible", FigValue::Bool(false))],
    )];
    let out_b =
        apply_instance_overrides_cached(&sym_b, Some(&over_b), Some(&derived_b), None, &mut cache);
    let vis_of = |name: &str| {
        out_b
            .iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get_bool("visible"))
    };
    assert_eq!(
        vis_of("value"),
        Some(false),
        "cached pin must route the visible=false to the value node"
    );
    assert_ne!(
        vis_of("label"),
        Some(false),
        "label (the walk-order guess) must stay visible"
    );
}

/// Pooled pre-seeding: before conversion, evidence from all instances
/// of a symbol is pooled per virtual pk and assigned once.
#[test]
fn pooled_seeding_assigns_by_cross_instance_evidence() {
    let sym = symbol_root(vec![
        text_leaf("label", 10, "Customers", 60.0, 17.0),
        text_leaf("value", 11, "0", 20.0, 24.0),
        text_leaf("delta", 12, "+0.00%", 40.0, 14.0),
    ]);
    let mut symbol_tree = HashMap::new();
    symbol_tree.insert("0:0".to_string(), Rc::new(sym));

    let make_instance = |lid: u32, ov: Vec<FigValue>, dv: Vec<FigValue>| TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            ("guid", guid(1, lid)),
            (
                "symbolData",
                obj(vec![
                    ("symbolID", guid(0, 0)),
                    ("symbolOverrides", FigValue::Array(ov)),
                ]),
            ),
            ("derivedSymbolData", FigValue::Array(dv)),
        ]),
        children: vec![],
    };
    let inst_a = make_instance(
        100,
        vec![
            ov_with(
                vec![guid(9, 51)],
                vec![(
                    "textData",
                    obj(vec![("characters", FigValue::Str("$4,000,000.00".into()))]),
                )],
            ),
            ov_with(
                vec![guid(9, 52)],
                vec![(
                    "textData",
                    obj(vec![("characters", FigValue::Str("+20.00%".into()))]),
                )],
            ),
        ],
        vec![],
    );
    let inst_b = make_instance(
        101,
        vec![ov_with(
            vec![guid(9, 51)],
            vec![(
                "textData",
                obj(vec![("characters", FigValue::Str("1,250".into()))]),
            )],
        )],
        vec![],
    );
    let root = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("guid", guid(1, 1)),
        ]),
        children: vec![inst_a, inst_b],
    };

    let mut cache: HashMap<String, String> = HashMap::new();
    seed_assignments_from_instances(&root, &symbol_tree, &mut cache);

    assert_eq!(
        cache.get("0:0|9:51").map(String::as_str),
        Some("1:11"),
        "pooled numeric+currency evidence pins 9:51 -> value node, cache={cache:?}"
    );
    assert_eq!(
        cache.get("0:0|9:52").map(String::as_str),
        Some("1:12"),
        "percent evidence pins 9:52 -> delta node"
    );
}

/// Geometry pre-seeding (Test.fig category icon): a sibling instance
/// with rich derived data (size + transform) proves the pk -> node
/// mapping; an instance carrying only a plain fill override on the
/// same pk must reuse that pin instead of its (shifted) walk order.
#[test]
fn geometry_seeding_pins_from_rich_sibling_instance() {
    let make_leaf = |name: &str, lid: u32, w: f32, h: f32, x: f32| {
        let mut n = sized_leaf(name, lid, w, h);
        n.figma.set("type", FigValue::Str("RECTANGLE".into()));
        n.figma.set(
            "transform",
            obj(vec![
                ("m02", FigValue::Float(x)),
                ("m12", FigValue::Float(0.0)),
            ]),
        );
        n
    };
    let make_sym = || {
        symbol_root(vec![
            make_leaf("combined", 10, 20.0, 20.0, 0.0),
            make_leaf("accent", 11, 8.0, 8.0, 11.0),
        ])
    };
    let mut symbol_tree = HashMap::new();
    symbol_tree.insert("0:0".to_string(), Rc::new(make_sym()));

    let make_instance = |lid: u32, ov: Vec<FigValue>, dv: Vec<FigValue>| TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            ("guid", guid(1, lid)),
            ("size", size(100.0, 100.0)),
            (
                "symbolData",
                obj(vec![
                    ("symbolID", guid(0, 0)),
                    ("symbolOverrides", FigValue::Array(ov)),
                ]),
            ),
            ("derivedSymbolData", FigValue::Array(dv)),
        ]),
        children: vec![],
    };
    let geom = |pk_lid: u32, w: f32, h: f32, x: f32| {
        ov_with(
            vec![guid(9, pk_lid)],
            vec![
                ("size", size(w, h)),
                (
                    "transform",
                    obj(vec![
                        ("m02", FigValue::Float(x)),
                        ("m12", FigValue::Float(0.0)),
                    ]),
                ),
            ],
        )
    };
    // Rich instance: derived geometry proves 9:50 -> accent (8x8 at
    // x=11) even though walk order would say combined.
    let rich = make_instance(
        100,
        vec![],
        vec![geom(50, 8.0, 8.0, 11.0), geom(51, 20.0, 20.0, 0.0)],
    );
    let root = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("guid", guid(1, 1)),
        ]),
        children: vec![rich],
    };
    let mut cache: HashMap<String, String> = HashMap::new();
    seed_assignments_from_instances(&root, &symbol_tree, &mut cache);
    assert_eq!(
        cache.get("0:0|9:50").map(String::as_str),
        Some("1:11"),
        "rich geometry must pin 9:50 -> accent, cache={cache:?}"
    );

    // Poor instance: a plain fill on 9:50 (walk order says combined)
    // must follow the seeded pin onto accent.
    let sym_b = make_sym();
    let over_b = vec![ov_with(
        vec![guid(9, 50)],
        vec![(
            "fillPaints",
            FigValue::Array(vec![obj(vec![("type", FigValue::Str("SOLID".into()))])]),
        )],
    )];
    // 4 bare entries != 3 subtree nodes so Strategy 1's index
    // mapping (which never consults the cache) stays out of the way.
    let derived_b = vec![
        guid_path(vec![guid(9, 50)]),
        guid_path(vec![guid(9, 51)]),
        guid_path(vec![guid(9, 52)]),
        guid_path(vec![guid(9, 53)]),
    ];
    let out_b =
        apply_instance_overrides_cached(&sym_b, Some(&over_b), Some(&derived_b), None, &mut cache);
    let has_fill = |name: &str| {
        out_b
            .iter()
            .find(|n| n.figma.get_str("name") == Some(name))
            .and_then(|n| n.figma.get_array("fillPaints"))
            .map(|a| !a.is_empty())
            .unwrap_or(false)
    };
    assert!(
        has_fill("accent"),
        "seeded pin routes the plain fill to accent"
    );
    assert!(!has_fill("combined"), "walk-order guess must not win");
}

/// Seeding regression shape (Test.fig orderCard chip): the chip and
/// its sibling "right" frame TIE on geometry. The instance's fill
/// override on the same pk is the tie-breaker — seeding without it
/// pins the pk to the wrong sibling and the cache then poisons every
/// later conversion.
#[test]
fn geometry_seeding_uses_fill_override_as_tie_breaker() {
    let make_leaf = |name: &str, lid: u32, w: f32, h: f32, x: f32, filled: bool| {
        let mut n = sized_leaf(name, lid, w, h);
        n.figma.set("type", FigValue::Str("FRAME".into()));
        n.figma.set(
            "transform",
            obj(vec![
                ("m02", FigValue::Float(x)),
                ("m12", FigValue::Float(0.0)),
            ]),
        );
        if filled {
            n.figma.set(
                "fillPaints",
                FigValue::Array(vec![obj(vec![
                    ("type", FigValue::Str("SOLID".into())),
                    ("opacity", FigValue::Float(0.12)),
                ])]),
            );
        }
        n
    };
    // Walk order: right(10) first, chip(11) second; geometry ties.
    let make_sym = || {
        symbol_root(vec![
            make_leaf("right", 10, 76.0, 15.0, 244.0, false),
            make_leaf("chip", 11, 77.0, 19.0, 243.0, true),
        ])
    };
    let mut symbol_tree = HashMap::new();
    symbol_tree.insert("0:0".to_string(), Rc::new(make_sym()));

    let rich = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            ("guid", guid(1, 100)),
            ("size", size(100.0, 100.0)),
            (
                "symbolData",
                obj(vec![
                    ("symbolID", guid(0, 0)),
                    (
                        "symbolOverrides",
                        FigValue::Array(vec![ov_with(
                            vec![guid(9, 50)],
                            vec![(
                                "fillPaints",
                                FigValue::Array(vec![obj(vec![
                                    ("type", FigValue::Str("SOLID".into())),
                                    ("opacity", FigValue::Float(0.12)),
                                ])]),
                            )],
                        )]),
                    ),
                ]),
            ),
            (
                "derivedSymbolData",
                FigValue::Array(vec![ov_with(
                    vec![guid(9, 50)],
                    vec![
                        ("size", size(77.0, 19.0)),
                        (
                            "transform",
                            obj(vec![
                                ("m02", FigValue::Float(243.0)),
                                ("m12", FigValue::Float(0.0)),
                            ]),
                        ),
                    ],
                )]),
            ),
        ]),
        children: vec![],
    };
    let root = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("guid", guid(1, 1)),
        ]),
        children: vec![rich],
    };
    let mut cache: HashMap<String, String> = HashMap::new();
    seed_assignments_from_instances(&root, &symbol_tree, &mut cache);
    let pinned = cache.get("0:0|9:50").map(String::as_str);
    assert!(
        pinned.is_none() || pinned == Some("1:11"),
        "a geometry tie must not pin the fill-carrying pk to the unfilled sibling, got {pinned:?}"
    );
}

/// A SWAPPED instance (`overriddenSymbolID` present) carries derived
/// data for the SWAPPED-in component, not its base `symbolID`. The
/// seeding must NOT pool/pin it under the base component's cache —
/// doing so poisons genuine base-component instances that reuse the
/// cache. Verified for geometry seeding.
#[test]
fn swapped_instance_does_not_seed_base_component_cache() {
    let make_leaf = |name: &str, lid: u32, w: f32, h: f32, x: f32| {
        let mut n = sized_leaf(name, lid, w, h);
        n.figma.set("type", FigValue::Str("RECTANGLE".into()));
        n.figma.set(
            "transform",
            obj(vec![
                ("m02", FigValue::Float(x)),
                ("m12", FigValue::Float(0.0)),
            ]),
        );
        n
    };
    // Base component 0:0 with two nodes at 20 / 8.
    let base_sym = symbol_root(vec![
        make_leaf("combined", 10, 20.0, 20.0, 0.0),
        make_leaf("accent", 11, 8.0, 8.0, 11.0),
    ]);
    let mut symbol_tree = HashMap::new();
    symbol_tree.insert("0:0".to_string(), Rc::new(base_sym));

    let geom = |pk_lid: u32, w: f32, h: f32, x: f32| {
        ov_with(
            vec![guid(9, pk_lid)],
            vec![
                ("size", size(w, h)),
                (
                    "transform",
                    obj(vec![
                        ("m02", FigValue::Float(x)),
                        ("m12", FigValue::Float(0.0)),
                    ]),
                ),
            ],
        )
    };
    // A SWAPPED instance: symbolID = base (0:0), overriddenSymbolID =
    // some other component. Its derived (sizes 8 then 20) would pin
    // 9:50 -> accent if seeded — poisoning the base cache.
    let swapped = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            ("guid", guid(1, 100)),
            ("size", size(100.0, 100.0)),
            ("overriddenSymbolID", guid(0, 99)),
            (
                "symbolData",
                obj(vec![
                    ("symbolID", guid(0, 0)),
                    ("symbolOverrides", FigValue::Array(vec![])),
                ]),
            ),
            (
                "derivedSymbolData",
                FigValue::Array(vec![geom(50, 8.0, 8.0, 11.0), geom(51, 20.0, 20.0, 0.0)]),
            ),
        ]),
        children: vec![],
    };
    let root = TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("guid", guid(1, 1)),
        ]),
        children: vec![swapped],
    };
    let mut cache: HashMap<String, String> = HashMap::new();
    seed_assignments_from_instances(&root, &symbol_tree, &mut cache);
    assert!(
        cache.keys().all(|k| !k.starts_with("0:0|")),
        "swapped instance must not seed the base 0:0 cache, cache={cache:?}"
    );
}

#[test]
fn pooled_seeding_does_not_overwrite_existing_pin() {
    let sym = symbol_root(vec![
        text_leaf("label", 10, "Customers", 60.0, 17.0),
        text_leaf("value", 11, "0", 20.0, 24.0),
    ]);
    let mut symbol_tree = HashMap::new();
    symbol_tree.insert("0:0".to_string(), Rc::new(sym));

    let make_root = || TreeNode {
        figma: obj(vec![
            ("type", FigValue::Str("FRAME".into())),
            ("guid", guid(1, 1)),
        ]),
        children: vec![TreeNode {
            figma: obj(vec![
                ("type", FigValue::Str("INSTANCE".into())),
                ("guid", guid(1, 100)),
                (
                    "symbolData",
                    obj(vec![
                        ("symbolID", guid(0, 0)),
                        (
                            "symbolOverrides",
                            FigValue::Array(vec![ov_with(
                                vec![guid(9, 51)],
                                vec![(
                                    "textData",
                                    obj(vec![("characters", FigValue::Str("Customers".into()))]),
                                )],
                            )]),
                        ),
                    ]),
                ),
            ]),
            children: vec![],
        }],
    };

    let mut fresh = HashMap::new();
    seed_assignments_from_instances(&make_root(), &symbol_tree, &mut fresh);
    assert_eq!(
        fresh.get("0:0|9:51").map(String::as_str),
        Some("1:10"),
        "fixture must produce a narrower label pin"
    );

    let mut cache = HashMap::from([("0:0|9:51".to_string(), "1:11".to_string())]);
    seed_assignments_from_instances(&make_root(), &symbol_tree, &mut cache);
    assert_eq!(
        cache.get("0:0|9:51").map(String::as_str),
        Some("1:11"),
        "existing global pin must not be overwritten by a later narrow seed"
    );
}
