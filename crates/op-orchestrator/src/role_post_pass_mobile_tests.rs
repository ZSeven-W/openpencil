//! Tests for `fix_input_sibling_consistency` and the mobile normalizer
//! clusters (search shell, section headers, category rows, product cards,
//! cart badges, promo panes).

use super::*;
use serde_json::json;

/// The walk with every tier enabled, on an unclassified surface. These tests
/// exercise the normalizers themselves — not the repair-tier gate in front of
/// one of them (see `crate::repair_tier`), and not the deck gate on the
/// overflow clip floor (see `crate::deck_echo`) — so they call the shape the
/// pass had before either gate existed and stay readable as normalizer tests.
fn post_pass_value(node: &mut Value, parent_fill: Option<Value>, canvas_width: f64) {
    super::post_pass_value(
        node,
        parent_fill,
        canvas_width,
        true,
        DesignForm::Unknown,
        &mut Vec::new(),
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
fn leaf_children_search_bar_keeps_its_authored_chrome() {
    // test0711-1-ds: the search bar ITSELF is the input (bare icon/text
    // leaves inside role=search-bar). The nested-shell normalizer used to
    // misfire here: shell fill stripped, search glyph painted white
    // (invisible on the light field), filter glyph re-inked with a
    // dangling $--primary that rendered fallback blue.
    let mut bar = json!({
        "type":"frame","role":"search-bar","layout":"horizontal",
        "fill":[{"type":"solid","color":"#F4EDE3"}],
        "padding":[0,16],
        "children":[
            {"type":"icon_font","iconFontName":"search","fill":[{"type":"solid","color":"#9A8F80"}]},
            {"type":"text","content":"Where to?","fill":[{"type":"solid","color":"#0F172A"}]},
            {"type":"icon_font","name":"Filter","iconFontName":"filter","fill":[{"type":"solid","color":"#F97316"}]}
        ]
    });
    let before = bar.clone();
    normalize_nested_search_shell(&mut bar);
    assert_eq!(bar, before, "a leaf-children search bar is untouched");
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
        json!([{"type":"solid","color":"#FF5A1F"}]),
        "filter icon carries the button's own demoted accent (concrete hex, never a dangling $ref)"
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
        json!([{"type":"solid","color":"#FF5A1F"}]),
        "only the icon carries the brand accent - the button's own demoted hex"
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
        json!([{"type":"solid","color":"$--primary"}])
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
fn overflowing_mobile_category_rail_keeps_every_chip() {
    let chip = |id: &str| {
        json!({
            "type":"frame","id":id,"role":"chip","name":format!("{id} Category"),
            "children":[{"type":"text","id":format!("{id}-label"),"content":id}]
        })
    };
    let mut row = json!({
        "type":"frame","id":"rail","layout":"horizontal","name":"Category Chips",
        "children":[chip("one"), chip("two"), chip("three"), chip("four"), chip("five"), chip("six")]
    });

    post_pass_value(&mut row, Some(Value::Null), 390.0);

    assert_eq!(row["children"].as_array().expect("children").len(), 6);
    assert_eq!(row["justifyContent"], json!("start"));
    assert_eq!(row["clipContent"], json!(true));
}

#[test]
fn desktop_category_rail_preserves_all_authored_tiles() {
    let tile = |id: &str, label: &str, glyph: &str| {
        json!({
            "type":"frame","id":id,"name":format!("{label} tile"),
            "layout":"vertical","width":196,"height":150,
            "children":[
                {"type":"icon_font","id":format!("{id}-icon"),
                 "name":format!("{label} icon tile"),"iconFontName":glyph},
                {"type":"text","id":format!("{id}-label"),
                 "name":format!("{label} label"),"content":label}
            ]
        })
    };
    let mut row = json!({
        "type":"frame","id":"rail","layout":"horizontal","name":"Category tiles",
        "width":"fill_container","gap":24,"justifyContent":"space_between",
        "children":[
            tile("home", "Home", "house"),
            tile("bags", "Bags", "shopping-bag"),
            tile("apparel", "Apparel", "shirt"),
            tile("lighting", "Lighting", "lamp"),
            tile("kitchen", "Kitchen", "cooking-pot"),
            tile("furniture", "Furniture", "sofa")
        ]
    });
    let before = row.clone();

    post_pass_value(&mut row, Some(Value::Null), 1440.0);

    assert_eq!(
        row, before,
        "phone-only category normalization must not restyle or truncate a desktop rail"
    );
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
        json!([{"type":"solid","color":"$--primary"}])
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
fn image_cards_with_favorite_icons_are_not_category_chips() {
    let card = |name: &str| {
        json!({
            "type":"frame", "name":name, "layout":"vertical",
            "width":"fill_container", "height":"fill_container", "gap":12,
            "cornerRadius":16, "children":[
                {"type":"frame", "name":"Photo Wrap", "layout":"none",
                 "width":"fill_container", "height":140, "clipContent":true,
                 "children":[
                    {"type":"image", "name":"Destination Photo", "src":"",
                     "width":"fill_container", "height":140},
                    {"type":"frame", "name":"Favorite", "children":[
                        {"type":"icon_font", "iconFontName":"heart", "width":18, "height":18}
                    ]}
                 ]},
                {"type":"text", "content":name, "width":"fit_content", "height":"fit_content"}
            ]
        })
    };
    let mut row = json!({
        "type":"frame", "name":"Cards Row", "layout":"horizontal",
        "width":"fit_content", "height":"fit_content", "gap":12,
        "justifyContent":"space_between",
        "children":[card("Santorini Card"), card("Kyoto Card"), card("Lisbon Card")]
    });

    post_pass_value(&mut row, Some(Value::Null), 375.0);

    assert_eq!(row["width"], json!("fit_content"));
    assert_eq!(row["children"][0]["cornerRadius"], json!(16));
    assert_eq!(row["children"][0]["children"][0]["height"], json!(140));
    assert_eq!(
        row["children"][0]["children"][0]["layout"],
        json!("none"),
        "a photo with an overlaid favorite icon remains media, not a 56px category tile"
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
