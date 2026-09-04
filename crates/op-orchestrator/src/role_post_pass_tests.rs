use super::*;
use serde_json::json;

// ── I4: layout-property fixes ─────────────────────────────────────────────

#[test]
fn card_row_equalized_to_fill_container() {
    // Two fixed-width cards of unequal width (240 vs 120 → ratio 0.5 < 0.6) and
    // similar height → widths are promoted, while authored heights stay fixed.
    let mut row = json!({
        "type":"frame","layout":"horizontal","children":[
            {"type":"frame","width":240,"height":200,"children":[]},
            {"type":"frame","width":120,"height":210,"children":[]}
        ]
    });
    equalize_card_row(&mut row);
    assert_eq!(row["children"][0]["width"], json!("fill_container"));
    assert_eq!(row["children"][1]["width"], json!("fill_container"));
    assert_eq!(row["children"][0]["height"], json!(200));
    assert_eq!(row["children"][1]["height"], json!(210));
}

#[test]
fn card_row_left_alone_when_widths_similar() {
    // 200 vs 190 → ratio 0.95 ≥ 0.6 → leave widths as-is.
    let mut row = json!({
        "type":"frame","layout":"horizontal","children":[
            {"type":"frame","width":200,"height":200,"children":[]},
            {"type":"frame","width":190,"height":200,"children":[]}
        ]
    });
    equalize_card_row(&mut row);
    assert_eq!(row["children"][0]["width"], json!(200));
}

#[test]
fn badge_pill_tag_rows_are_not_equalized() {
    // The dashboard-pass (equalizeHorizontalSiblings) excludes badge/pill/tag,
    // so a row of those keeps its widths even with a low width ratio.
    for role in ["badge", "pill", "tag"] {
        let mut row = json!({
            "type":"frame","layout":"horizontal","children":[
                {"type":"frame","role":role,"width":240,"height":200,"children":[]},
                {"type":"frame","role":role,"width":120,"height":210,"children":[]}
            ]
        });
        equalize_card_row(&mut row);
        assert_eq!(
            row["children"][0]["width"],
            json!(240),
            "{role} row must not be equalized"
        );
    }
}

#[test]
fn form_inputs_promoted_when_fill_sibling_present() {
    let mut form = json!({
        "type":"frame","layout":"vertical","children":[
            {"type":"frame","role":"input","width":"fill_container"},
            {"type":"frame","role":"input","width":200}
        ]
    });
    normalize_form_input_widths(&mut form);
    assert_eq!(form["children"][1]["width"], json!("fill_container"));
}

#[test]
fn trailing_icon_pushes_text_to_fill() {
    let mut input = json!({
        "type":"frame","role":"input","children":[
            {"type":"text","content":"Search"},
            {"type":"frame","role":"icon","width":20,"height":20}
        ]
    });
    normalize_input_trailing_icon_alignment(&mut input);
    assert_eq!(input["children"][0]["width"], json!("fill_container"));
    assert_eq!(input["children"][0]["textGrowth"], json!("fixed-width"));
}

#[test]
fn horizontal_overflow_reduces_gap_before_expanding_parent() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","width":300,"padding":[16,16,16,16],"gap":24,
        "children":[
            {"type":"frame","width":130,"height":44},
            {"type":"frame","width":130,"height":44}
        ]
    });
    fix_horizontal_overflow(&mut row, 375.0, DesignForm::Unknown, &mut Vec::new());
    assert_eq!(row["gap"], json!(8.0));
    assert_eq!(row["width"], json!(300));
}

#[test]
fn horizontal_overflow_uses_fill_when_needed_width_nears_canvas() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","width":220,"padding":[16,16,16,16],"gap":16,
        "children":[
            {"type":"frame","width":180,"height":44},
            {"type":"frame","width":180,"height":44}
        ]
    });
    fix_horizontal_overflow(&mut row, 375.0, DesignForm::Unknown, &mut Vec::new());
    assert_eq!(row["width"], json!("fill_container"));
}

#[test]
fn horizontal_overflow_beyond_viewport_clips_instead_of_spilling() {
    // The food-app category bug: 6 chips that physically can't fit a 375px phone.
    // Widening is futile (children sum > canvas), so the row spans the viewport and
    // clips at the edge instead of letting chips spill off-canvas into the void.
    let mut row = json!({
        "type":"frame","layout":"horizontal","width":327,"gap":12,
        "children":[
            {"type":"frame","width":46,"height":34},
            {"type":"frame","width":84,"height":34},
            {"type":"frame","width":85,"height":34},
            {"type":"frame","width":91,"height":34},
            {"type":"frame","width":89,"height":34},
            {"type":"frame","width":96,"height":34}
        ]
    });
    fix_horizontal_overflow(&mut row, 375.0, DesignForm::Unknown, &mut Vec::new());
    assert_eq!(row["width"], json!("fill_container"));
    assert_eq!(
        row["clipContent"],
        json!(true),
        "an over-viewport horizontal row clips at the edge (scroll-row floor)"
    );
}

#[test]
fn text_heights_removed_unless_fixed_width_height() {
    let mut root = json!({
        "type":"frame","layout":"vertical","children":[
            {"type":"text","content":"Long address text","height":18,"textGrowth":"fixed-width"},
            {"type":"text","content":"Pinned","height":18,"textGrowth":"fixed-width-height"}
        ]
    });
    fix_text_heights(&mut root);
    assert!(root["children"][0].get("height").is_none());
    assert_eq!(root["children"][1]["height"], json!(18));
}

#[test]
fn clip_content_set_for_rounded_image_frame() {
    let mut card = json!({
        "type":"frame","cornerRadius":12,"children":[{"type":"image","id":"img"}]
    });
    apply_clip_content_for_image(&mut card);
    assert_eq!(card["clipContent"], json!(true));
    // No image → untouched.
    let mut plain = json!({"type":"frame","cornerRadius":12,"children":[{"type":"text"}]});
    apply_clip_content_for_image(&mut plain);
    assert!(plain.get("clipContent").is_none());
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
    post_pass_forest(&mut nodes, 375.0);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(
        v["children"][0]["fill"],
        json!([{"type":"solid","color":"$--card"}]),
        "orphan card inside an unfilled section root gets a semantic surface fill"
    );
}

#[test]
fn text_token_container_fill_flips_to_surface_with_its_dark_text() {
    // ATELIER's verbatim slot error: a search pill filled with
    // `$--foreground` (white capsule on the dark theme), its
    // placeholder styled #404040 FOR that accidental white. The container
    // flips to the surface slot; the dark literal text joins the ladder.
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"pill","name":"Search Container","layout":"horizontal","cornerRadius":8,
        "fill":[{"type":"solid","color":"$--foreground"}],
        "children":[
            {"type":"text","id":"ph","content":"Search clients...","fill":[{"type":"solid","color":"#404040"}]},
            {"type":"text","id":"gold","content":"FILTER","fill":[{"type":"solid","color":"$--primary"}]}
        ]
    }))
    .unwrap()];
    enforce_surface_color_discipline(&mut nodes);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(
        v["fill"][0]["color"].as_str(),
        Some("$--muted"),
        "container fill rebound to the surface slot: {v}"
    );
    assert_eq!(
        v["children"][0]["fill"][0]["color"].as_str(),
        Some("$--muted-foreground"),
        "dark literal placeholder joins the text ladder"
    );
    assert_eq!(
        v["children"][1]["fill"][0]["color"].as_str(),
        Some("$--primary"),
        "token-bound text is left alone"
    );
}

#[test]
fn text_nodes_keep_text_tokens() {
    // The rule targets CONTAINERS — a text node filled with a text token is
    // exactly right and must not be touched.
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![serde_json::from_value(json!({
        "type":"text","id":"t","content":"Heading",
        "fill":[{"type":"solid","color":"$--foreground"}]
    }))
    .unwrap()];
    enforce_surface_color_discipline(&mut nodes);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(v["fill"][0]["color"].as_str(), Some("$--foreground"));
}

#[test]
fn count_badge_without_radius_becomes_a_pill() {
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"badge","layout":"horizontal","padding":[3,8],
        "fill":[{"type":"solid","color":"#C9A96220"}],
        "children":[{"type":"text","id":"n","content":"12","fontSize":11}]
    }))
    .unwrap()];
    enforce_surface_color_discipline(&mut nodes);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(v["cornerRadius"].as_f64(), Some(100.0), "{v}");
}

#[test]
fn authored_radius_and_word_chips_stay() {
    // cornerRadius 0 (sharp luxury) is a decision; a WORD chip ("VIP") is
    // not a count badge.
    let sharp: jian_ops_schema::node::PenNode = serde_json::from_value(json!({
        "type":"frame","id":"b1","layout":"horizontal","cornerRadius":0,"padding":[3,8],
        "fill":[{"type":"solid","color":"#C9A96220"}],
        "children":[{"type":"text","id":"n1","content":"12"}]
    }))
    .unwrap();
    let word: jian_ops_schema::node::PenNode = serde_json::from_value(json!({
        "type":"frame","id":"b2","layout":"horizontal","padding":[3,8],
        "fill":[{"type":"solid","color":"#22C55E18"}],
        "children":[{"type":"text","id":"n2","content":"VIP"}]
    }))
    .unwrap();
    let mut nodes = vec![sharp, word];
    enforce_surface_color_discipline(&mut nodes);
    let v0 = serde_json::to_value(&nodes[0]).unwrap();
    let v1 = serde_json::to_value(&nodes[1]).unwrap();
    assert_eq!(v0["cornerRadius"].as_f64(), Some(0.0));
    assert!(v1.get("cornerRadius").is_none() || v1["cornerRadius"].is_null());
}

#[test]
fn missing_semantic_micro_surfaces_are_rounded() {
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![
        serde_json::from_value(json!({
            "type":"frame","id":"done","name":"Done Badge","width":20,"height":20,
            "fill":[{"type":"solid","color":"#22C55E"}],
            "children":[{"type":"icon_font","id":"check","iconFontName":"check"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"status","name":"Status Pill","role":"stat-card",
            "width":"fill_container","height":28,
            "fill":[{"type":"solid","color":"#FF6B6B20"}],
            "children":[{"type":"text","id":"status-text","content":"进行中"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"tag","role":"tag","width":"fit_content","height":"fit_content",
            "fill":[{"type":"solid","color":"#F3F4F6"}],
            "children":[{"type":"text","id":"tag-text","content":"今日核心词"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"avatar","role":"avatar","width":36,"height":36,
            "fill":[{"type":"solid","color":"#E5E7EB"}],
            "children":[{"type":"text","id":"avatar-text","content":"A"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"active","name":"Active Indicator","width":8,"height":8,
            "fill":[{"type":"solid","color":"#FF6B6B"}],
            "children":[]
        }))
        .unwrap(),
    ];

    enforce_surface_color_discipline(&mut nodes);
    let values: Vec<Value> = nodes
        .iter()
        .map(|node| serde_json::to_value(node).unwrap())
        .collect();

    for value in &values[..4] {
        assert!(
            value["cornerRadius"]
                .as_f64()
                .is_some_and(|radius| radius >= 100.0),
            "badge/pill/tag/avatar should become a capsule: {value}"
        );
    }
    assert_eq!(
        values[4]["cornerRadius"].as_f64(),
        Some(4.0),
        "a fixed near-square active indicator should become a true circle"
    );
}

#[test]
fn semantic_capsule_rounding_skips_large_fixed_surfaces() {
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![
        serde_json::from_value(json!({
            "type":"frame","id":"avatar-card","name":"Avatar Card",
            "width":240,"height":180,
            "fill":[{"type":"solid","color":"#FFFFFF"}],
            "children":[{"type":"text","id":"avatar-card-title","content":"Profile"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"tag-cloud","name":"Tag Cloud",
            "width":320,"height":160,
            "fill":[{"type":"solid","color":"#F3F4F6"}],
            "children":[{"type":"text","id":"tag-cloud-title","content":"Topics"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"badge-preview","name":"Badge Preview",
            "width":200,"height":120,
            "fill":[{"type":"solid","color":"#F3F4F6"}],
            "children":[{"type":"text","id":"badge-preview-title","content":"Preview"}]
        }))
        .unwrap(),
    ];

    enforce_surface_color_discipline(&mut nodes);
    for node in &nodes {
        let value = serde_json::to_value(node).unwrap();
        assert!(
            value.get("cornerRadius").is_none() || value["cornerRadius"].is_null(),
            "large fixed semantic containers are not micro surfaces: {value}"
        );
    }
}

#[test]
fn semantic_capsule_rounding_honors_compact_boundaries_and_hug_anatomy() {
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![
        serde_json::from_value(json!({
            "type":"frame","id":"avatar-64","role":"avatar","name":"Avatar",
            "width":64,"height":64,
            "fill":[{"type":"solid","color":"#E5E7EB"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"avatar-65","role":"avatar","name":"Avatar",
            "width":65,"height":65,
            "fill":[{"type":"solid","color":"#E5E7EB"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"fit-tag","role":"tag","name":"Topic Tag",
            "width":"fit_content","height":"fit_content","layout":"horizontal",
            "padding":[4,10],"fill":[{"type":"solid","color":"#F3F4F6"}],
            "children":[{"type":"text","id":"fit-tag-text","content":"Travel","fontSize":12}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"fit-cloud","name":"Tag Cloud",
            "width":"fit_content","height":"fit_content","layout":"horizontal",
            "fill":[{"type":"solid","color":"#F3F4F6"}],
            "children":[
                {"type":"frame","id":"cloud-a","children":[]},
                {"type":"frame","id":"cloud-b","children":[]},
                {"type":"frame","id":"cloud-c","children":[]},
                {"type":"frame","id":"cloud-d","children":[]}
            ]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"numeric-preview","name":"Badge Preview",
            "width":240,"height":160,"layout":"horizontal",
            "fill":[{"type":"solid","color":"#F3F4F6"}],
            "children":[{"type":"text","id":"preview-count","content":"80"}]
        }))
        .unwrap(),
    ];

    enforce_surface_color_discipline(&mut nodes);
    let values: Vec<Value> = nodes
        .iter()
        .map(|node| serde_json::to_value(node).unwrap())
        .collect();

    assert_eq!(values[0]["cornerRadius"].as_f64(), Some(999.0));
    assert!(values[1].get("cornerRadius").is_none());
    assert_eq!(values[2]["cornerRadius"].as_f64(), Some(999.0));
    assert!(values[3].get("cornerRadius").is_none());
    assert!(
        values[4].get("cornerRadius").is_none(),
        "large numeric badge preview must not bypass the semantic gate via count-badge repair"
    );
}

#[test]
fn semantic_rounding_preserves_authored_sharp_and_structural_surfaces() {
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![
        serde_json::from_value(json!({
            "type":"frame","id":"sharp","name":"Done Badge","cornerRadius":0,
            "width":20,"height":20,"fill":[{"type":"solid","color":"#22C55E"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"transparent","name":"Status Pill","width":80,"height":28
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"status-bar","name":"Status Bar","width":"fill_container","height":62,
            "fill":[{"type":"solid","color":"#FFFFFF"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"row","name":"Badge Row","width":"fill_container","height":32,
            "fill":[{"type":"solid","color":"#F3F4F6"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"container","name":"Pill Container","width":"fill_container","height":40,
            "fill":[{"type":"solid","color":"#F3F4F6"}]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"nav","role":"bottom-tab-bar","name":"Bottom Navigation Bar",
            "width":375,"height":72,"fill":[{"type":"solid","color":"#FFFFFF"}]
        }))
        .unwrap(),
    ];

    enforce_surface_color_discipline(&mut nodes);
    let values: Vec<Value> = nodes
        .iter()
        .map(|node| serde_json::to_value(node).unwrap())
        .collect();

    assert_eq!(values[0]["cornerRadius"].as_f64(), Some(0.0));
    for value in &values[1..] {
        assert!(
            value.get("cornerRadius").is_none() || value["cornerRadius"].is_null(),
            "unpainted or structural surfaces must stay untouched: {value}"
        );
    }
}

#[test]
fn exact_icon_box_inside_rounded_card_gets_conservative_radius() {
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![
        serde_json::from_value(json!({
            "type":"frame","id":"named-card","name":"Listening Task Card","cornerRadius":18,
            "children":[{
                "type":"frame","id":"icon-a","name":"Icon Box","role":"icon",
                "width":32,"height":32,"fill":[{"type":"solid","color":"#22C55E20"}]
            }]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"role-card","role":"card","name":"Lesson Surface","cornerRadius":20,
            "children":[{
                "type":"frame","id":"icon-b","name":"Icon Box",
                "width":64,"height":64,"fill":[{"type":"solid","color":"#FF6B6B20"}]
            }]
        }))
        .unwrap(),
    ];

    enforce_surface_color_discipline(&mut nodes);
    let named = serde_json::to_value(&nodes[0]).unwrap();
    let role = serde_json::to_value(&nodes[1]).unwrap();
    assert_eq!(named["children"][0]["cornerRadius"].as_f64(), Some(8.0));
    assert_eq!(
        role["children"][0]["cornerRadius"].as_f64(),
        Some(12.0),
        "the contextual icon radius is capped at 12px"
    );
}

#[test]
fn icon_box_rounding_requires_rounded_card_context_and_missing_radius() {
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![
        serde_json::from_value(json!({
            "type":"frame","id":"outside","name":"Section",
            "children":[{
                "type":"frame","id":"icon-outside","name":"Icon Box",
                "width":32,"height":32,"fill":[{"type":"solid","color":"#F3F4F6"}]
            }]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"sharp-card","role":"card","cornerRadius":0,
            "children":[{
                "type":"frame","id":"icon-sharp","name":"Icon Box",
                "width":32,"height":32,"fill":[{"type":"solid","color":"#F3F4F6"}]
            }]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "type":"frame","id":"rounded-card","role":"card","cornerRadius":18,
            "children":[{
                "type":"frame","id":"icon-authored","name":"Icon Box","cornerRadius":0,
                "width":32,"height":32,"fill":[{"type":"solid","color":"#F3F4F6"}]
            }]
        }))
        .unwrap(),
    ];

    enforce_surface_color_discipline(&mut nodes);
    let outside = serde_json::to_value(&nodes[0]).unwrap();
    let sharp = serde_json::to_value(&nodes[1]).unwrap();
    let authored = serde_json::to_value(&nodes[2]).unwrap();

    assert!(outside["children"][0].get("cornerRadius").is_none());
    assert!(sharp["children"][0].get("cornerRadius").is_none());
    assert_eq!(
        authored["children"][0]["cornerRadius"].as_f64(),
        Some(0.0),
        "an explicit sharp icon box is an authored decision"
    );
}
