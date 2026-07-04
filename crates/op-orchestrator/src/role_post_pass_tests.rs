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
fn accent_token_button_flips_dark_icon_to_white() {
    // Regression: a `$color-accent` (or `$color-primary`) button binds its bg
    // hex only at render time, so the contrast pass could not read its
    // luminance and left the model's default-dark icon on the orange accent.
    let mut btn = json!({
        "type":"frame","role":"icon-button",
        "fill":[{"type":"solid","color":"$color-accent"}],
        "children":[{"type":"icon_font","iconFontName":"sliders",
            "fill":[{"type":"solid","color":"#0F172A"}]}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert_eq!(
        btn["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "dark icon on an accent-token button must flip to white"
    );

    // An already-light icon on the accent stays put.
    let mut ok = json!({
        "type":"frame","role":"icon-button",
        "fill":[{"type":"solid","color":"$color-primary"}],
        "children":[{"type":"icon_font","fill":[{"type":"solid","color":"#FFFFFF"}]}]
    });
    fix_button_foreground_contrast(&mut ok);
    assert_eq!(
        ok["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );

    // A NON-accent token bg (surface) still can't be resolved → pass skips,
    // no accidental white on a light button.
    let mut surface = json!({
        "type":"frame","role":"icon-button",
        "fill":[{"type":"solid","color":"$color-surface"}],
        "children":[{"type":"icon_font","fill":[{"type":"solid","color":"#0F172A"}]}]
    });
    fix_button_foreground_contrast(&mut surface);
    assert_eq!(
        surface["children"][0]["fill"],
        json!([{"type":"solid","color":"#0F172A"}])
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
fn button_unresolved_non_accent_ref_bg_skipped() {
    // A non-accent design token can't resolve to a hex here AND isn't a known
    // saturated-accent family, so the pass can't pick a safe fg → skip. (An
    // accent token like $color-accent now DOES flip children to white — see
    // `accent_token_button_flips_dark_icon_to_white`.)
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"$color-surface-raised"}],
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

#[test]
fn search_filter_button_uses_neutral_surface_with_accent_icon() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[
            {"type":"frame","role":"search-bar","name":"Search Input","children":[]},
            {"type":"frame","role":"icon-button","name":"Filter Button",
             "fill":[{"type":"solid","color":"#FF5A1F"}],
             "children":[
                {"type":"icon_font","name":"Sliders Icon","iconFontName":"sliders",
                 "fill":[{"type":"solid","color":"#0F172A"}]}
             ]}
        ]
    });
    normalize_nested_search_shell(&mut row);
    assert_eq!(
        row["children"][1]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "default filter button should stay neutral, not a large accent block"
    );
    assert_eq!(
        row["children"][1]["stroke"],
        json!({"thickness":1,"fill":[{"type":"solid","color":"#E5E7EB"}]}),
        "neutral filter button keeps a subtle border"
    );
    assert_eq!(
        row["children"][1]["children"][0]["fill"],
        json!([{"type":"solid","color":"$color-accent"}]),
        "filter icon carries the accent without flooding the control"
    );
}

#[test]
fn search_filter_detects_icon_only_sliders_control() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","fill":[{"type":"solid","color":"#FFFCF6"}],
        "children":[
            {"type":"text_input","name":"搜索美食、餐厅","children":[]},
            {"type":"frame","role":"button","name":"快捷操作",
             "fill":[{"type":"solid","color":"#FF5A1F"}],
             "children":[
                {"type":"icon_font","iconFontName":"sliders",
                 "fill":[{"type":"solid","color":"#111111"}]}
             ]}
        ]
    });
    normalize_nested_search_shell(&mut row);
    assert_eq!(
        row["children"][1]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "icon-only sliders control beside search is a filter affordance, not a primary CTA"
    );
    assert_eq!(
        row["children"][1]["children"][0]["fill"],
        json!([{"type":"solid","color":"$color-accent"}]),
        "only the icon carries the brand accent"
    );
}

#[test]
fn search_filter_controls_use_eight_radius_and_twelve_gap() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","gap":24,
        "children":[
            {"type":"text_input","name":"Search restaurants","children":[]},
            {"type":"frame","role":"icon-button","name":"Filter",
             "cornerRadius":18,
             "children":[{"type":"icon_font","iconFontName":"sliders"}]}
        ]
    });
    normalize_nested_search_shell(&mut row);
    assert_eq!(
        row["gap"],
        json!(12),
        "search row gap should use the 12px spacing token"
    );
    assert_eq!(row["children"][0]["cornerRadius"], json!(8));
    assert_eq!(row["children"][1]["cornerRadius"], json!(8));
}

#[test]
fn mobile_section_header_see_all_text_becomes_arrow_icon() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","name":"Categories Header",
        "justifyContent":"space_between","alignItems":"center",
        "children":[
            {"type":"text","role":"heading","content":"分类","fontSize":20,"fontWeight":700},
            {"type":"text","name":"See All Link","content":"查看全部","fontSize":14,
             "fontWeight":600,"fill":[{"type":"solid","color":"#FF5A1F"}]}
        ]
    });
    post_pass_value(&mut row, Some(Value::Null), 402.0);
    assert_eq!(
        row["children"][1]["type"],
        json!("icon_font"),
        "short see-all text in a mobile section header should become an icon-only affordance"
    );
    assert_eq!(row["children"][1]["iconFontName"], json!("chevron-right"));
    assert_eq!(
        row["children"][1]["fill"],
        json!([{"type":"solid","color":"$color-accent"}])
    );
    assert!(
        row["children"][1].get("content").is_none(),
        "the text label is removed from the visible design"
    );
}

#[test]
fn category_chip_row_preserves_all_chips_and_spreads() {
    // The chip COUNT follows the model (no truncate-to-4), and 3+ chips spread
    // across the row via space_between instead of clustering left
    // (user direction 2026-06-23). Each chip is sized fit_content so the set
    // fits the row.
    let mut row = json!({
        "type":"frame","layout":"horizontal","name":"Category Chips",
        "gap":16,"width":"fill_container",
        "children":[
            {"type":"frame","role":"chip","name":"Pizza","width":132,"children":[{"type":"text","content":"Pizza"}]},
            {"type":"frame","role":"chip","name":"Sushi","width":120,"children":[{"type":"text","content":"Sushi"}]},
            {"type":"frame","role":"chip","name":"Burgers","width":140,"children":[{"type":"text","content":"Burgers"}]},
            {"type":"frame","role":"chip","name":"Dessert","width":130,"children":[{"type":"text","content":"Dessert"}]},
            {"type":"frame","role":"chip","name":"Drinks","width":120,"children":[{"type":"text","content":"Drinks"}]}
        ]
    });
    post_pass_value(&mut row, Some(Value::Null), 390.0);
    let children = row["children"].as_array().expect("children");
    assert_eq!(
        children.len(),
        5,
        "category chip count must follow the model — not be truncated to four"
    );
    assert_eq!(row["gap"], json!(12));
    assert_eq!(row["height"], json!("fit_content"));
    assert_eq!(
        row["justifyContent"],
        json!("space_between"),
        "3+ chips spread across the row instead of clustering left"
    );
    assert_eq!(row["alignItems"], json!("center"));
    for child in children {
        assert_eq!(child["width"], json!("fit_content"));
        assert_eq!(child["cornerRadius"], json!(8));
    }
}

#[test]
fn category_icon_label_items_without_chip_role_do_not_keep_huge_spacing() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","name":"Category Items",
        "width":900,"height":156,"gap":180,
        "justifyContent":"space_between","alignItems":"center",
        "children":[
            {"type":"frame","name":"Pizza","layout":"vertical","width":110,"gap":20,
             "children":[
                {"type":"frame","name":"Pizza Icon Tile","width":72,"height":72,"children":[{"type":"icon_font","iconFontName":"pizza"}]},
                {"type":"text","content":"Pizza"}
             ]},
            {"type":"frame","name":"Burger","layout":"vertical","width":110,"gap":20,
             "children":[
                {"type":"frame","name":"Burger Icon Tile","width":72,"height":72,"children":[{"type":"icon_font","iconFontName":"hamburger"}]},
                {"type":"text","content":"Burger"}
             ]},
            {"type":"frame","name":"Sushi","layout":"vertical","width":110,"gap":20,
             "children":[
                {"type":"frame","name":"Sushi Icon Tile","width":72,"height":72,"children":[{"type":"icon_font","iconFontName":"fish"}]},
                {"type":"text","content":"Sushi"}
             ]},
            {"type":"frame","name":"Salad","layout":"vertical","width":110,"gap":20,
             "children":[
                {"type":"frame","name":"Salad Icon Tile","width":72,"height":72,"children":[{"type":"icon_font","iconFontName":"salad"}]},
                {"type":"text","content":"Salad"}
             ]}
        ]
    });
    post_pass_value(&mut row, Some(Value::Null), 390.0);
    assert_eq!(row["width"], json!("fill_container"));
    assert_eq!(row["height"], json!("fit_content"));
    // The huge input gap (180) is normalized to a sane floor (12)...
    assert_eq!(row["gap"], json!(12));
    // ...and the 4 chips spread evenly (space_between) rather than clustering.
    assert_eq!(row["justifyContent"], json!("space_between"));
    let children = row["children"].as_array().expect("children");
    assert_eq!(children.len(), 4);
    for child in children {
        assert_eq!(child["width"], json!("fit_content"));
        assert_eq!(child["height"], json!("fit_content"));
        assert_eq!(child["gap"], json!(8));
    }
}

#[test]
fn category_icon_tiles_inside_food_rows_are_compact_and_light() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","name":"美食分类 Category Items",
        "width":900,"height":156,"gap":120,
        "justifyContent":"space_between","alignItems":"center",
        "children":[
            {"type":"frame","name":"川菜","layout":"vertical","width":96,"height":104,"gap":20,
             "children":[
                {"type":"frame","name":"川菜 Icon Tile","width":72,"height":72,
                 "cornerRadius":0,
                 "children":[{"type":"icon_font","iconFontName":"flame"}]},
                {"type":"text","content":"川菜"}
             ]},
            {"type":"frame","name":"日料","layout":"vertical","width":96,"height":104,"gap":20,
             "children":[
                {"type":"frame","name":"日料 Icon Tile","width":72,"height":72,
                 "cornerRadius":0,
                 "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#D7C6B8"}]},
                 "children":[{"type":"icon_font","iconFontName":"fish"}]},
                {"type":"text","content":"日料"}
             ]},
            {"type":"frame","name":"甜品","layout":"vertical","width":96,"height":104,"gap":20,
             "children":[
                {"type":"frame","name":"甜品 Icon Tile","width":72,"height":72,
                 "cornerRadius":0,
                 "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#D7C6B8"}]},
                 "children":[{"type":"icon_font","iconFontName":"cake-slice"}]},
                {"type":"text","content":"甜品"}
             ]}
        ]
    });

    post_pass_value(&mut row, Some(Value::Null), 390.0);

    let children = row["children"].as_array().expect("children");
    let active_tile = &children[0]["children"][0];
    assert_eq!(active_tile["width"], json!(56));
    assert_eq!(active_tile["height"], json!(56));
    assert_eq!(active_tile["cornerRadius"], json!(8));
    assert_eq!(
        active_tile["fill"],
        json!([{"type":"solid","color":"#FFF0E3"}]),
        "the selected category gets only a subtle warm tint"
    );
    assert!(
        active_tile.get("stroke").is_none(),
        "the selected category tile should not have a heavy square outline"
    );
    assert_eq!(
        active_tile["children"][0]["fill"],
        json!([{"type":"solid","color":"$color-accent"}])
    );

    let inactive_tile = &children[1]["children"][0];
    assert_eq!(inactive_tile["width"], json!(56));
    assert_eq!(inactive_tile["height"], json!(56));
    assert_eq!(inactive_tile["cornerRadius"], json!(8));
    assert_eq!(
        inactive_tile["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
    assert_eq!(
        inactive_tile["stroke"],
        json!({"thickness":1,"fill":[{"type":"solid","color":"#EAD8C8"}]})
    );
    assert_eq!(
        inactive_tile["children"][0]["fill"],
        json!([{"type":"solid","color":"#8A5F49"}])
    );
}

#[test]
fn category_section_with_generic_item_row_does_not_keep_huge_spacing() {
    let mut section = json!({
        "type":"frame","layout":"vertical","name":"Categories Section",
        "width":"fill_container","height":220,"gap":24,
        "children":[
            {"type":"frame","layout":"horizontal","name":"Section Header",
             "justifyContent":"space_between",
             "children":[
                {"type":"text","role":"heading","content":"Categories"},
                {"type":"text","name":"See All Link","content":"See all"}
             ]},
            {"type":"frame","layout":"horizontal","name":"Options Row",
             "width":"fill_container","height":132,"justifyContent":"space_between","gap":0,
             "children":[
                {"type":"frame","name":"Pizza","layout":"vertical","width":96,"height":104,"gap":20,
                 "children":[
                    {"type":"frame","name":"Pizza Icon Tile","width":72,"height":72,
                     "children":[{"type":"icon_font","iconFontName":"pizza"}]},
                    {"type":"text","content":"Pizza"}
                 ]},
                {"type":"frame","name":"Burger","layout":"vertical","width":96,"height":104,"gap":20,
                 "children":[
                    {"type":"frame","name":"Burger Icon Tile","width":72,"height":72,
                     "children":[{"type":"icon_font","iconFontName":"hamburger"}]},
                    {"type":"text","content":"Burger"}
                 ]}
             ]}
        ]
    });

    post_pass_value(&mut section, Some(Value::Null), 390.0);

    assert_eq!(
        section["height"],
        json!("fit_content"),
        "category section wrappers should not keep a large fixed height"
    );
    assert_eq!(
        section["gap"],
        json!(12),
        "category section vertical spacing should use the 12px token"
    );
    let row = &section["children"][1];
    assert_eq!(row["height"], json!("fit_content"));
    assert_eq!(row["gap"], json!(12));
    assert_eq!(row["justifyContent"], json!("start"));
    for child in row["children"].as_array().expect("children") {
        assert_eq!(child["width"], json!("fit_content"));
        assert_eq!(child["height"], json!("fit_content"));
        assert_eq!(child["gap"], json!(8));
    }
}

#[test]
fn category_section_normalizes_plain_item_row_even_without_loose_spacing() {
    let mut section = json!({
        "type":"frame","layout":"vertical","name":"Categories",
        "width":"fill_container","height":188,"gap":18,
        "children":[
            {"type":"frame","layout":"horizontal","name":"Header",
             "justifyContent":"space_between",
             "children":[
                {"type":"text","role":"heading","content":"Categories"},
                {"type":"text","content":"See all"}
             ]},
            {"type":"frame","layout":"horizontal","name":"Options",
             "width":"fill_container","height":112,"justifyContent":"start","gap":28,
             "children":[
                {"type":"frame","name":"Burgers","layout":"vertical","width":86,"height":98,"gap":16,
                 "children":[
                    {"type":"frame","name":"Burger Icon Tile","width":72,"height":72,
                     "children":[{"type":"icon_font","iconFontName":"hamburger"}]},
                    {"type":"text","content":"Burgers"}
                 ]},
                {"type":"frame","name":"Pizza","layout":"vertical","width":86,"height":98,"gap":16,
                 "children":[
                    {"type":"frame","name":"Pizza Icon Tile","width":72,"height":72,
                     "children":[{"type":"icon_font","iconFontName":"pizza"}]},
                    {"type":"text","content":"Pizza"}
                 ]},
                {"type":"frame","name":"Sushi","layout":"vertical","width":86,"height":98,"gap":16,
                 "children":[
                    {"type":"frame","name":"Sushi Icon Tile","width":72,"height":72,
                     "children":[{"type":"icon_font","iconFontName":"fish"}]},
                    {"type":"text","content":"Sushi"}
                 ]},
                {"type":"frame","name":"Healthy","layout":"vertical","width":86,"height":98,"gap":16,
                 "children":[
                    {"type":"frame","name":"Healthy Icon Tile","width":72,"height":72,
                     "children":[{"type":"icon_font","iconFontName":"salad"}]},
                    {"type":"text","content":"Healthy"}
                 ]}
             ]}
        ]
    });

    post_pass_value(&mut section, Some(Value::Null), 390.0);

    assert_eq!(
        section["height"],
        json!("fit_content"),
        "category section should not reserve a tall fixed band"
    );
    assert_eq!(section["gap"], json!(12));
    let row = &section["children"][1];
    assert_eq!(row["height"], json!("fit_content"));
    assert_eq!(row["gap"], json!(12));
    // 4 chips spread across the row (space_between) instead of clustering left.
    assert_eq!(row["justifyContent"], json!("space_between"));
    let first_tile = &row["children"][0]["children"][0];
    assert_eq!(first_tile["width"], json!(56));
    assert_eq!(first_tile["height"], json!(56));
    assert_eq!(first_tile["cornerRadius"], json!(8));
}

#[test]
fn mixed_category_header_and_chips_is_not_truncated_as_chip_row() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","name":"热门分类 Category Section",
        "gap":16,"width":"fill_container",
        "children":[
            {"type":"text","role":"heading","content":"热门分类","fontSize":22,"fontWeight":700},
            {"type":"text","name":"See All","content":"查看全部","fontSize":14,"fontWeight":600},
            {"type":"frame","role":"chip","name":"Burger Category","children":[{"type":"icon_font","iconFontName":"hamburger"}]},
            {"type":"frame","role":"chip","name":"Sushi Category","children":[{"type":"icon_font","iconFontName":"fish"}]},
            {"type":"frame","role":"chip","name":"Dessert Category","children":[{"type":"icon_font","iconFontName":"cake"}]},
            {"type":"frame","role":"chip","name":"Drinks Category","children":[{"type":"icon_font","iconFontName":"cup-soda"}]},
            {"type":"frame","role":"chip","name":"Salad Category","children":[{"type":"icon_font","iconFontName":"salad"}]}
        ]
    });
    post_pass_value(&mut row, Some(Value::Null), 390.0);
    assert_eq!(
        row["children"].as_array().expect("children").len(),
        7,
        "a mixed header + category-items row must not be treated as the chip rail and truncated"
    );
    assert_eq!(
        row["children"][1]["type"],
        json!("icon_font"),
        "the header action can still be converted to an icon"
    );
}

#[test]
fn cart_count_badge_becomes_tiny_circle_on_neutral_button() {
    let mut button = json!({
        "type":"frame","role":"icon-button","name":"Cart Button",
        "width":54,"height":44,"cornerRadius":12,
        "fill":[{"type":"solid","color":"#FFFFFF"}],
        "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#EAD8C8"}]},
        "children":[
            {"type":"frame","name":"Cart Count Badge","width":20,"height":20,
             "cornerRadius":2,"fill":[{"type":"solid","color":"#FF5A1F"}],
             "children":[{"type":"text","content":"3","fontSize":12,"fill":[{"type":"solid","color":"#FFFFFF"}]}]},
            {"type":"icon_font","iconFontName":"shopping-cart","width":22,"height":22,
             "fill":[{"type":"solid","color":"#21140F"}]}
        ]
    });
    post_pass_value(&mut button, Some(Value::Null), 390.0);
    assert_eq!(
        button["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "cart action is a neutral header control, not an accent block"
    );
    assert_eq!(button["cornerRadius"], json!(8));
    assert_eq!(button["children"][0]["role"], json!("badge"));
    assert_eq!(button["children"][0]["width"], json!(16));
    assert_eq!(button["children"][0]["height"], json!(16));
    assert_eq!(button["children"][0]["cornerRadius"], json!(999));
    assert_eq!(button["children"][0]["children"][0]["fontSize"], json!(10));
}

#[test]
fn promo_banner_with_cta_uses_fit_content_height() {
    let mut banner = json!({
        "type":"frame","role":"banner","name":"Limited Offer Promo Card",
        "layout":"horizontal","height":156,"clipContent":true,
        "children":[
            {"type":"frame","layout":"vertical","children":[
                {"type":"text","content":"50% OFF"},
                {"type":"frame","role":"button","name":"Order Now Button",
                 "children":[{"type":"text","content":"Order Now"}]}
            ]},
            {"type":"image","height":156}
        ]
    });
    post_pass_value(&mut banner, Some(Value::Null), 375.0);
    assert_eq!(
        banner["height"],
        json!("fit_content"),
        "promo/banner cards with CTAs must not keep a clipping fixed height"
    );
}

#[test]
fn horizontal_food_promo_constrains_headline_to_text_pane() {
    let mut banner = json!({
        "type":"frame","role":"banner","name":"今日特惠 Promo Banner",
        "layout":"horizontal","width":"fill_container","height":220,"clipContent":true,
        "children":[
            {"type":"frame","name":"Promo Copy","layout":"vertical","width":240,"gap":18,
             "children":[
                {"type":"frame","role":"badge","name":"Deal Badge",
                 "children":[{"type":"text","content":"限时特惠","fontSize":11}]},
                {"type":"text","id":"headline","content":"今日特惠 限时5折","fontSize":34,
                 "fontWeight":700,"width":360,"textGrowth":"auto-width"},
                {"type":"text","content":"精选人气餐厅，今日下单立享半价","fontSize":16,
                 "width":320,"textGrowth":"auto-width"}
             ]},
            {"type":"image","name":"Food Photo","width":190,"height":220,
             "imageSearchQuery":"pasta plate"}
        ]
    });

    post_pass_value(&mut banner, Some(Value::Null), 390.0);

    let copy = &banner["children"][0];
    let headline = &copy["children"][1];
    assert_eq!(
        copy["width"],
        json!("fill_container"),
        "the text pane must be allowed to shrink beside the fixed photo"
    );
    assert_eq!(
        headline["width"],
        json!("fill_container"),
        "promo headline should wrap inside the copy pane, not overlap the image"
    );
    assert_eq!(headline["textGrowth"], json!("fixed-width"));
    assert_eq!(
        headline["fontSize"],
        json!(28),
        "long CJK promo headlines need a mobile-safe maximum size"
    );
    assert_eq!(headline["lineHeight"], json!(1.12));
    assert_eq!(copy["children"][2]["width"], json!("fill_container"));
    assert_eq!(copy["children"][2]["textGrowth"], json!("fixed-width"));
    assert!(
        copy["children"][0]["children"][0].get("width").is_none(),
        "tiny badge labels should keep their intrinsic width"
    );
}

#[test]
fn mobile_product_card_row_fits_two_cards_inside_content_rail() {
    let mut row = json!({
        "type":"frame","name":"Popular Now Cards","layout":"horizontal",
        "width":560,"height":"fit_content","gap":20,"justifyContent":"space_between",
        "children":[
            {"type":"frame","role":"card","name":"Truffle Carbonara Card","width":230,"height":260,"cornerRadius":16,
             "children":[
                {"type":"image","width":230,"height":150,"imageSearchQuery":"pasta plate"},
                {"type":"text","content":"Truffle Carbonara"},
                {"type":"text","content":"$14"}
             ]},
            {"type":"frame","role":"card","name":"Smash Deluxe Card","width":230,"height":260,"cornerRadius":16,
             "children":[
                {"type":"image","width":230,"height":150,"imageSearchQuery":"burger plate"},
                {"type":"text","content":"Smash Deluxe"},
                {"type":"text","content":"$16"}
             ]}
        ]
    });

    post_pass_value(&mut row, Some(Value::Null), 390.0);

    assert_eq!(row["width"], json!("fill_container"));
    assert_eq!(row["height"], json!("fit_content"));
    assert_eq!(row["gap"], json!(12));
    assert_eq!(row["justifyContent"], json!("start"));
    assert_eq!(row["alignItems"], json!("stretch"));
    assert_eq!(row["clipContent"], json!(false));
    for card in row["children"].as_array().unwrap() {
        assert_eq!(card["width"], json!("fill_container"));
        assert_eq!(card["height"], json!("fit_content"));
        assert_eq!(card["cornerRadius"], json!(8));
    }
}

#[test]
fn mobile_featured_food_card_clamps_oversized_media_band() {
    let mut card = json!({
        "type":"frame","role":"card","name":"主题推荐 Featured Dish Card",
        "layout":"vertical","width":"fill_container","height":780,
        "cornerRadius":8,"fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[
            {"type":"image","name":"麻辣锅 Food Photo","width":"fill_container","height":616,
             "imageSearchQuery":"hotpot bowl"},
            {"type":"frame","name":"Dish Body","layout":"vertical","gap":12,
             "children":[
                {"type":"frame","role":"badge","name":"主题推荐 Badge",
                 "children":[{"type":"text","content":"主题推荐","fontSize":12}]},
                {"type":"text","content":"鲜香麻辣，现炒锅气十足","fontSize":18,
                 "fontWeight":700,"width":"fill_container","textGrowth":"fixed-width"},
                {"type":"frame","role":"button","name":"立即下单 Button",
                 "children":[{"type":"text","content":"立即下单"}]}
             ]}
        ]
    });

    post_pass_value(&mut card, Some(Value::Null), 390.0);

    assert_eq!(
        card["height"],
        json!("fit_content"),
        "mobile food cards should not keep a fixed height that leaves a blank media band"
    );
    assert_eq!(
        card["children"][0]["height"],
        json!(204),
        "image-top featured cards need a bounded mobile-safe media height"
    );
    assert_eq!(card["clipContent"], json!(true));
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

#[test]
fn text_token_container_fill_flips_to_surface_with_its_dark_text() {
    // ATELIER's verbatim slot error: a search pill filled with
    // `$color-text-primary` (white capsule on the dark theme), its
    // placeholder styled #404040 FOR that accidental white. The container
    // flips to the surface slot; the dark literal text joins the ladder.
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"pill","name":"Search Container","layout":"horizontal","cornerRadius":8,
        "fill":[{"type":"solid","color":"$color-text-primary"}],
        "children":[
            {"type":"text","id":"ph","content":"Search clients...","fill":[{"type":"solid","color":"#404040"}]},
            {"type":"text","id":"gold","content":"FILTER","fill":[{"type":"solid","color":"$color-accent"}]}
        ]
    }))
    .unwrap()];
    enforce_surface_color_discipline(&mut nodes);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(
        v["fill"][0]["color"].as_str(),
        Some("$color-surface-2"),
        "container fill rebound to the surface slot: {v}"
    );
    assert_eq!(
        v["children"][0]["fill"][0]["color"].as_str(),
        Some("$color-text-muted"),
        "dark literal placeholder joins the text ladder"
    );
    assert_eq!(
        v["children"][1]["fill"][0]["color"].as_str(),
        Some("$color-accent"),
        "token-bound text is left alone"
    );
}

#[test]
fn text_nodes_keep_text_tokens() {
    // The rule targets CONTAINERS — a text node filled with a text token is
    // exactly right and must not be touched.
    let mut nodes: Vec<jian_ops_schema::node::PenNode> = vec![serde_json::from_value(json!({
        "type":"text","id":"t","content":"Heading",
        "fill":[{"type":"solid","color":"$color-text-primary"}]
    }))
    .unwrap()];
    enforce_surface_color_discipline(&mut nodes);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(v["fill"][0]["color"].as_str(), Some("$color-text-primary"));
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
