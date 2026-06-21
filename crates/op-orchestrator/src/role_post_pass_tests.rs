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

#[test]
fn orphan_skipped_for_roleless_container() {
    // A roleless rounded container is NOT assumed to be a card. Rust role
    // inference is thinner than TS, so roleless wrappers (header / section /
    // banner) are common; white-washing them is what produced the spurious
    // panels behind the header / banner / search row.
    let mut wrap = json!({
        "type":"frame","cornerRadius":16,
        "children":[{"type":"frame","role":"section","children":[{"type":"text"}]}]
    });
    fix_orphan_container_contrast(&mut wrap, Some(&Value::Null));
    assert!(
        wrap.get("fill").is_none(),
        "roleless container is not white-washed into a card"
    );
}

#[test]
fn orphan_skipped_when_child_container_paints_surface() {
    // glm wraps an orange promo banner in a bare `feature-card` frame. The child
    // (Promo Card) carries the full-bleed gradient, so the wrapper must NOT be
    // whitewashed — doing so leaks the injected drop-shadow out as a gray ghost
    // box around the orange child (the user's "mysterious bg + rounded border").
    let mut wrap = json!({
        "type":"frame","role":"feature-card","cornerRadius":12,
        "children":[{
            "type":"frame","role":"card",
            "fill":[{"type":"linear_gradient","angle":135,"stops":[
                {"offset":0.0,"color":"#FB923C"},{"offset":1.0,"color":"#F97316"}]}],
            "children":[{"type":"text","content":"50% Off"}]
        }]
    });
    fix_orphan_container_contrast(&mut wrap, Some(&Value::Null));
    assert!(
        wrap.get("fill").is_none(),
        "wrapper around a filled card stays transparent"
    );
    assert!(
        wrap.get("effects").is_none(),
        "no ghost shadow injected onto the bare wrapper"
    );
}

// ── orphaned-shadow strip (ghost-box cleanup) ────────────────────────────

#[test]
fn orphaned_shadow_stripped_from_fill_less_frame() {
    // A drop-shadow with no surface (no fill, no stroke) is a gray ghost box.
    let mut node = json!({
        "type":"frame","cornerRadius":12,
        "effects":[{"type":"shadow","offsetX":0,"offsetY":1,"blur":3,"spread":0,"color":"#0000001A"}],
        "children":[{"type":"frame","role":"card","fill":[{"type":"solid","color":"#FB923C"}]}]
    });
    fix_surface_color_discipline(&mut node, false);
    assert!(
        node.get("effects").is_none(),
        "shadow on a fill-less, stroke-less frame is stripped"
    );
}

#[test]
fn shadow_kept_when_node_has_visible_fill() {
    // A real card (visible fill) keeps its elevation shadow.
    let mut node = json!({
        "type":"frame","role":"card","cornerRadius":12,
        "fill":[{"type":"solid","color":"#FFFFFF"}],
        "effects":[{"type":"shadow","offsetX":0,"offsetY":1,"blur":3,"spread":0,"color":"#0000001A"}],
    });
    fix_surface_color_discipline(&mut node, false);
    assert!(node["effects"].is_array(), "a filled card keeps its shadow");
}

// ── fixStructuralWrapperTransparency ─────────────────────────────────────

#[test]
fn structural_wrapper_white_fill_stripped() {
    // A section wrapper that glm gave an explicit white fill → forced transparent.
    let mut sect = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text","content":"x"}]
    });
    fix_structural_wrapper_transparency(&mut sect);
    assert_eq!(
        sect["fill"],
        json!([]),
        "white structural wrapper → transparent"
    );
}

#[test]
fn structural_wrapper_keeps_card_fill() {
    // card is in CARD_LIKE_ALLOWLIST — an intentional surface, fill stays.
    let mut card = json!({
        "type":"frame","role":"card","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text","content":"x"}]
    });
    fix_structural_wrapper_transparency(&mut card);
    assert_eq!(
        card["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "card keeps its white surface"
    );
}

#[test]
fn redundant_colored_wrapper_strips_light_surface() {
    // feature-card with a $color-surface fill wrapping a single full-bleed
    // gradient banner child → its surface is a redundant box, strip it.
    let mut wrap = json!({
        "type":"frame","role":"feature-card","cornerRadius":12,
        "fill":[{"type":"solid","color":"$color-surface"}],
        "children":[
            {
                "type":"frame","role":"card","width":"fill_container",
                "fill":[{"type":"linear_gradient","angle":135,"stops":[
                    {"offset":0.0,"color":"$color-chart-6"},{"offset":1.0,"color":"#FB923C"}]}],
                "children":[{"type":"text","content":"50% Off"}]
            },
            {"type":"text","content":"-30%"}
        ]
    });
    fix_structural_wrapper_transparency(&mut wrap);
    assert_eq!(
        wrap["fill"],
        json!([]),
        "redundant wrapper around a full-bleed colored card → transparent"
    );
}

#[test]
fn card_with_small_colored_child_keeps_fill() {
    // A real card whose colored child is NOT full-bleed (a small icon tile) is
    // a genuine surface — must keep its fill.
    let mut card = json!({
        "type":"frame","role":"card","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[
            {
                "type":"frame","role":"icon-tile","width":48,
                "fill":[{"type":"solid","color":"#FB923C"}],
                "children":[]
            },
            {"type":"text","content":"Title"}
        ]
    });
    fix_structural_wrapper_transparency(&mut card);
    assert_eq!(
        card["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "card with a small colored tile keeps its surface"
    );
}

#[test]
fn structural_wrapper_keeps_dark_fill() {
    // A dark section fill (luminance well below 0.85) is a deliberate band → kept.
    let mut sect = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"#1A1A1A"}],
        "children":[{"type":"text","content":"x"}]
    });
    fix_structural_wrapper_transparency(&mut sect);
    assert_eq!(
        sect["fill"],
        json!([{"type":"solid","color":"#1A1A1A"}]),
        "dark structural band is intentional, not stripped"
    );
}

#[test]
fn structural_wrapper_section_substring_and_header_stripped() {
    // role *containing* "section" (e.g. a custom "feature-section") → stripped.
    let mut feat = json!({
        "type":"frame","role":"feature-section","fill":[{"type":"solid","color":"#FAFAFA"}],
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut feat);
    assert_eq!(feat["fill"], json!([]), "*-section role → transparent");
    // header is structural too.
    let mut header = json!({
        "type":"frame","role":"header","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut header);
    assert_eq!(header["fill"], json!([]), "header → transparent");
}

#[test]
fn structural_wrapper_surface_variable_ref_stripped() {
    // glm emits UNRESOLVED $color-* refs (variable binding runs after post-pass),
    // so hex_luminance can't read them — the strip must match neutral surface
    // tokens by name. This is the actual header-background bug: role navbar +
    // fill $color-surface-2 was surviving because luminance("$color-surface-2")
    // returned None.
    let mut header = json!({
        "type":"frame","role":"navbar","fill":[{"type":"solid","color":"$color-surface-2"}],
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut header);
    assert_eq!(
        header["fill"],
        json!([]),
        "navbar with $color-surface-2 → transparent"
    );
    // A colored token (deliberate accent band) is NOT a neutral surface → kept.
    let mut band = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"$color-accent"}],
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut band);
    assert_eq!(
        band["fill"],
        json!([{"type":"solid","color":"$color-accent"}]),
        "deliberate colored band kept"
    );
}

#[test]
fn structural_wrapper_strips_border_with_fill() {
    // The mobile header (role navbar) carried a $color-surface fill AND a bottom
    // hairline stroke — the user flagged BOTH the background and the border.
    // Stripping the surface fill must drop the accompanying border too; a
    // transparent structural wrapper keeps no card/bar chrome.
    let mut header = json!({
        "type":"frame","role":"navbar","fill":[{"type":"solid","color":"$color-surface"}],
        "stroke":{"thickness":[0,0,1,0],"fill":[{"type":"solid","color":"$color-border"}]},
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut header);
    assert_eq!(header["fill"], json!([]), "surface fill stripped");
    assert!(
        header.get("stroke").is_none(),
        "bottom border stripped alongside the fill"
    );
}

#[test]
fn structural_wrapper_non_structural_role_left_alone() {
    // A plain content role that isn't structural and isn't card-like → untouched.
    let mut text = json!({
        "type":"text","role":"body","fill":[{"type":"solid","color":"#FFFFFF"}]
    });
    fix_structural_wrapper_transparency(&mut text);
    assert_eq!(
        text["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "non-structural role is out of scope"
    );
}

// ── fixSurfaceColorDiscipline ────────────────────────────────────────────

#[test]
fn state_bg_token_on_input_recolored_to_neutral() {
    // The pink-search bug: glm used $color-danger-bg as the input surface.
    let mut input = json!({
        "type":"text_input","name":"Search Input",
        "fill":[{"type":"solid","color":"$color-danger-bg"}]
    });
    fix_surface_color_discipline(&mut input, false);
    assert_eq!(
        input["fill"],
        json!([{"type":"solid","color":"$color-surface-2"}]),
        "danger-bg misused as input surface → neutral surface-2"
    );
}

#[test]
fn state_bg_token_kept_on_status_element() {
    // A real status element (name says "Error") legitimately uses danger-bg.
    let mut badge = json!({
        "type":"frame","role":"badge","name":"Error Badge",
        "fill":[{"type":"solid","color":"$color-danger-bg"}],
        "children":[{"type":"text","content":"Failed"}]
    });
    fix_surface_color_discipline(&mut badge, false);
    assert_eq!(
        badge["fill"],
        json!([{"type":"solid","color":"$color-danger-bg"}]),
        "status element keeps its semantic state color"
    );
}

#[test]
fn page_bg_token_stripped_from_inner_node_kept_on_root() {
    // Inner wrapper repainting the page bg (the cool grey panel behind search).
    let mut root = json!({
        "type":"frame","name":"Page","fill":[{"type":"solid","color":"$color-bg-deep"}],
        "children":[
            {"type":"frame","name":"Search & Categories",
             "fill":[{"type":"solid","color":"$color-bg-deep"}],
             "children":[{"type":"text_input","name":"Search"}]}
        ]
    });
    fix_surface_color_discipline(&mut root, true);
    assert_eq!(
        root["fill"],
        json!([{"type":"solid","color":"$color-bg-deep"}]),
        "page root keeps the page-bg token"
    );
    assert_eq!(
        root["children"][0]["fill"],
        json!([]),
        "inner wrapper using the page-bg token → transparent"
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

#[test]
fn non_ascii_color_does_not_panic() {
    // A malformed multi-byte color string must not panic the byte-slicer.
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"#héllo!"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn); // must not panic; bg unparseable → skip
    assert!(btn["children"][0].get("fill").is_none());
}

// ── I4: layout-property fixes ─────────────────────────────────────────────

#[test]
fn card_row_equalized_to_fill_container() {
    // Two fixed-width cards of unequal width (240 vs 120 → ratio 0.5 < 0.6) and
    // similar height → both promoted to fill_container.
    let mut row = json!({
        "type":"frame","layout":"horizontal","children":[
            {"type":"frame","width":240,"height":200,"children":[]},
            {"type":"frame","width":120,"height":210,"children":[]}
        ]
    });
    equalize_card_row(&mut row);
    assert_eq!(row["children"][0]["width"], json!("fill_container"));
    assert_eq!(row["children"][1]["width"], json!("fill_container"));
    assert_eq!(row["children"][0]["height"], json!("fill_container"));
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
    fix_horizontal_overflow(&mut row, 375.0);
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
    fix_horizontal_overflow(&mut row, 375.0);
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
    fix_horizontal_overflow(&mut row, 375.0);
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
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "orphan card inside an unfilled section root gets a white fill"
    );
}
