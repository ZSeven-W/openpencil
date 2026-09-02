//! Tree-heuristic regressions for media, badges, overlays and the
//! accent/banner repaints.
//!
//! Split out of `tree_heuristics_tests.rs` (pure code motion) so both files
//! stay under the repo's 800-line cap; the nav / section-fill cases stay in
//! the original file.

use super::*;
use serde_json::json;

#[test]
fn card_image_corners_clipped() {
    let mut card = json!({
        "type":"frame","role":"card","cornerRadius":12,"children":[
            {"type":"image","name":"Photo","cornerRadius":12},
            {"type":"text","content":"Title"}
        ]
    });
    clip_card_image_corners(&mut card);
    assert_eq!(
        card["clipContent"],
        json!(true),
        "card clips to its outer radius"
    );
    assert!(
        card["children"][0].get("cornerRadius").is_none(),
        "image's own radius removed (card clips instead)"
    );
}

#[test]
fn standalone_image_not_clipped() {
    // single image (no title sibling) → not a card-image pattern, left alone.
    let mut frame = json!({
        "type":"frame","cornerRadius":12,"children":[
            {"type":"image","cornerRadius":12}
        ]
    });
    clip_card_image_corners(&mut frame);
    assert!(
        frame.get("clipContent").is_none(),
        "1-child image frame untouched"
    );
}

// ── fill_card_leading_image_width ────────────────────────────────────────

#[test]
fn dish_card_leading_image_fills_width() {
    // 160 px image inside a 252 px vertical dish card → white gap on the
    // right. The header image should span the card width.
    let mut card = json!({
        "type":"frame","layout":"vertical","width":252.0,"children":[
            {"type":"image","name":"Photo","width":160.0,"height":120.0},
            {"type":"frame","name":"Info","children":[{"type":"text","content":"Truffle Burger"}]}
        ]
    });
    fill_card_leading_image_width(&mut card);
    assert_eq!(
        card["children"][0]["width"],
        json!("fill_container"),
        "leading header image spans the card width"
    );
}

#[test]
fn centered_leading_avatar_not_widened() {
    // a centered leading image is an avatar / logo, not a full-bleed header.
    let mut card = json!({
        "type":"frame","layout":"vertical","alignItems":"center","children":[
            {"type":"image","name":"Avatar","width":96.0,"height":96.0,"cornerRadius":48.0},
            {"type":"text","content":"Jane Doe"}
        ]
    });
    fill_card_leading_image_width(&mut card);
    assert_eq!(
        card["children"][0]["width"],
        json!(96.0),
        "centered avatar keeps its fixed width"
    );
}

#[test]
fn horizontal_card_side_image_not_widened() {
    // image-left / text-right row → the leading image is a side thumbnail.
    let mut row = json!({
        "type":"frame","layout":"horizontal","children":[
            {"type":"image","name":"Thumb","width":120.0,"height":120.0},
            {"type":"text","content":"Order #1"}
        ]
    });
    fill_card_leading_image_width(&mut row);
    assert_eq!(
        row["children"][0]["width"],
        json!(120.0),
        "horizontal-row side image keeps its fixed width"
    );
}

#[test]
fn small_leading_icon_not_widened() {
    // a small (< 80 px) leading image is an inline icon/badge, left alone.
    let mut card = json!({
        "type":"frame","layout":"vertical","children":[
            {"type":"image","name":"Icon","width":48.0,"height":48.0},
            {"type":"text","content":"Label"}
        ]
    });
    fill_card_leading_image_width(&mut card);
    assert_eq!(
        card["children"][0]["width"],
        json!(48.0),
        "small leading icon keeps its fixed width"
    );
}

// ── fix_notification_badge_overlay ───────────────────────────────────────

#[test]
fn notification_badge_becomes_corner_circle() {
    // live structure: 44×44 horizontal button > [8×8 filled dot, 22×22 bell].
    let mut button = json!({
        "type":"frame","name":"Notification Button","width":44.0,"height":44.0,
        "layout":"horizontal","gap":8.0,"cornerRadius":12.0,
        "children":[
            {"type":"frame","name":"Badge","width":8.0,"height":8.0,
             "fill":[{"type":"solid","color":"$--chart-6"}],"children":[]},
            {"type":"icon_font","name":"Bell","iconFontName":"bell","width":22.0,"height":22.0}
        ]
    });
    fix_notification_badge_overlay(&mut button);
    assert_eq!(
        button["layout"],
        json!("none"),
        "button switches to absolute"
    );
    assert!(button.get("gap").is_none(), "flex gap dropped");
    // badge: rounded to a circle, pinned to the bell's top-right corner.
    let badge = &button["children"][0];
    assert_eq!(
        badge["cornerRadius"],
        json!(4.0),
        "square dot rounded to a circle"
    );
    assert_eq!(badge["x"], json!(29.0), "badge x at bell top-right");
    assert_eq!(badge["y"], json!(7.0), "badge y at bell top-right");
    // bell: centered in the 44×44 button.
    let bell = &button["children"][1];
    assert_eq!(bell["x"], json!(11.0), "bell centered x");
    assert_eq!(bell["y"], json!(11.0), "bell centered y");
}

#[test]
fn switch_like_frame_not_treated_as_badge() {
    // a non-square pill (44×24) is a switch/toggle, not an icon button.
    let mut sw = json!({
        "type":"frame","width":44.0,"height":24.0,"layout":"horizontal","cornerRadius":12.0,
        "children":[
            {"type":"ellipse","width":20.0,"height":20.0,"fill":[{"type":"solid","color":"#fff"}]},
            {"type":"icon_font","iconFontName":"check","width":12.0,"height":12.0}
        ]
    });
    let before = sw.clone();
    fix_notification_badge_overlay(&mut sw);
    assert_eq!(sw, before, "non-square pill left untouched");
}

#[test]
fn icon_button_without_dot_unchanged() {
    // a plain icon button (icon + text label) is not a badge pattern.
    let mut btn = json!({
        "type":"frame","width":44.0,"height":44.0,"layout":"horizontal",
        "children":[
            {"type":"icon_font","iconFontName":"heart","width":22.0,"height":22.0},
            {"type":"text","content":"12"}
        ]
    });
    let before = btn.clone();
    fix_notification_badge_overlay(&mut btn);
    assert_eq!(btn, before, "icon+text button left untouched");
}

// ── convert_stacked_overlay_to_absolute ──────────────────────────────────

#[test]
fn stacked_overlay_switches_to_absolute() {
    // full-bleed image + gradient overlay (both full-height) in a vertical
    // fixed-height frame → layered hero → layout:none (fixes the collapsed
    // "Featured Restaurants" promo card: image h=0 + content overflow).
    let mut hero = json!({
        "type":"frame","layout":"vertical","height":180.0,"children":[
            {"type":"image","height":180.0},
            {"type":"frame","name":"Gradient","height":"fill_container"},
            {"type":"frame","name":"Content","children":[{"type":"text","content":"30% Off"}]}
        ]
    });
    convert_stacked_overlay_to_absolute(&mut hero);
    assert_eq!(
        hero["layout"],
        json!("none"),
        "layered hero (2+ full-height bg children) → absolute stacking"
    );
}

#[test]
fn normal_vertical_stack_not_converted() {
    // a normal content stack (1 image, rest text) → stays vertical.
    let mut col = json!({
        "type":"frame","layout":"vertical","height":200.0,"children":[
            {"type":"image","height":120.0},
            {"type":"text","content":"Title"},
            {"type":"text","content":"Body"}
        ]
    });
    convert_stacked_overlay_to_absolute(&mut col);
    assert_eq!(
        col["layout"],
        json!("vertical"),
        "only 1 bg-like child → not a hero overlay, left alone"
    );
}

// ── dominant_design_accent ───────────────────────────────────────────────

#[test]
fn dominant_accent_prefers_most_used_chart_token() {
    // glm uses $--chart-6 as the de-facto accent (3×) over $--primary (1×).
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"r","name":"Page","children":[
            {"type":"frame","id":"b","name":"Btn","role":"button","fill":[{"type":"solid","color":"$--chart-6"}]},
            {"type":"frame","id":"c","name":"Chip","role":"chip","fill":[{"type":"solid","color":"$--chart-6"}]},
            {"type":"text","id":"t","name":"T","content":"x","fill":[{"type":"solid","color":"$--chart-6"}]},
            {"type":"frame","id":"a","name":"A","fill":[{"type":"solid","color":"$--primary"}]}
        ]
    }))
    .unwrap()];
    assert_eq!(
        dominant_design_accent(&nodes).as_deref(),
        Some("$--chart-6"),
        "the most-used accent token wins over the palette default"
    );
}

// ── strip_nested_card_decoration ─────────────────────────────────────────

#[test]
fn nested_card_redundant_shadow_and_radius_stripped() {
    let mut outer = json!({
        "type":"frame","role":"card","cornerRadius":12,
        "effects":[{"type":"shadow","offsetX":0,"offsetY":1,"blur":3,"color":"#0000001A"}],
        "children":[{
            "type":"frame","role":"card","cornerRadius":10,
            "effects":[{"type":"shadow","offsetX":0,"offsetY":1,"blur":2,"color":"#0000000F"}],
            "children":[{"type":"text","content":"hi"}]
        }]
    });
    strip_nested_card_decoration(&mut outer, DecoFlags::default());
    let inner = &outer["children"][0];
    assert!(
        inner.get("effects").is_none(),
        "inner card's redundant shadow stripped (kills box-in-box)"
    );
    assert!(
        inner.get("cornerRadius").is_none(),
        "inner card's redundant radius stripped"
    );
    // outer keeps its decoration
    assert_eq!(outer["cornerRadius"], json!(12));
}

// ── fix_invisible_text_band (glm banner-fill floor) ──────────────────────

#[test]
fn invisible_white_text_band_gets_accent_on_light_page() {
    // glm's transparent promo banner: white copy, no fill → invisible on cream.
    let mut band = json!({"type":"frame","role":"section","children":[
        {"type":"text","content":"50% OFF","fill":[{"type":"solid","color":"$--card"}]},
        {"type":"text","content":"first order","fill":[{"type":"solid","color":"#FFFFFF"}]}
    ]});
    fix_invisible_text_band(&mut band, Theme::Light, "$--primary");
    assert_eq!(
        band["fill"],
        json!([{"type":"solid","color":"$--primary"}]),
        "all-white-text band on a light page → accent fill so copy is readable"
    );
}

#[test]
fn banner_with_filled_button_still_filled() {
    // white headline + an "Order Now" button (own white fill, accent text). The
    // button's accent text sits on its OWN surface → must not count → the
    // all-light headline alone triggers the banner fill. (gen9 Promo Content.)
    let mut band = json!({"type":"frame","role":"section","children":[
        {"type":"text","content":"50% OFF","fill":[{"type":"solid","color":"$--card"}]},
        {"type":"text","content":"Code: X","fill":[{"type":"solid","color":"$--muted"}]},
        {"type":"frame","role":"button","fill":[{"type":"solid","color":"$--card"}],"children":[
            {"type":"text","content":"Order Now","fill":[{"type":"solid","color":"$--primary"}]}
        ]}
    ]});
    fix_invisible_text_band(&mut band, Theme::Light, "$--primary");
    assert_eq!(
        band["fill"],
        json!([{"type":"solid","color":"$--primary"}]),
        "button's accent text excluded (own surface) → light headline → filled"
    );
}

#[test]
fn light_surface_filled_banner_repainted_accent() {
    // tt5: a feature-card carrying a `$--card` (light) fill with white
    // headline + subtext + a translucent-white badge + a dark CTA button. The
    // light surface makes the white headline INVISIBLE. The light-surface fill
    // must be treated like no fill → repaint with the design accent.
    let mut band = json!({
        "type":"frame","role":"feature-card","cornerRadius":12,
        "fill":[{"type":"solid","color":"$--card"}],
        "children":[
            {"type":"frame","name":"Promo Content","children":[
                {"type":"frame","role":"badge","fill":[{"type":"solid","color":"#FFFFFF30"}],"children":[
                    {"type":"text","content":"FLASH","fill":[{"type":"solid","color":"$--card"}]}
                ]},
                {"type":"text","content":"Get 30% off","fill":[{"type":"solid","color":"$--card"}]},
                {"type":"text","content":"first order","fill":[{"type":"solid","color":"#FFFFFFCC"}]},
                {"type":"frame","role":"button","fill":[{"type":"solid","color":"#1C1410"}],"children":[
                    {"type":"text","content":"Order Now","fill":[{"type":"solid","color":"$--card"}]}
                ]}
            ]}
        ]
    });
    fix_invisible_text_band(&mut band, Theme::Light, "$--chart-6");
    assert_eq!(
        band["fill"],
        json!([{"type":"solid","color":"$--chart-6"}]),
        "light-surface banner with all-white copy → repainted with the accent"
    );
}

#[test]
fn gradient_filled_banner_left_alone() {
    // A banner that already paints a real colored surface (a gradient) reads its
    // white text fine — must NOT be repainted.
    let mut band = json!({
        "type":"frame","role":"feature-card",
        "fill":[{"type":"linear_gradient","angle":135,"stops":[
            {"offset":0.0,"color":"$--chart-6"},{"offset":1.0,"color":"#FB923C"}]}],
        "children":[
            {"type":"text","content":"Get 30% off","fill":[{"type":"solid","color":"$--card"}]}
        ]
    });
    let before = band["fill"].clone();
    fix_invisible_text_band(&mut band, Theme::Light, "$--primary");
    assert_eq!(
        band["fill"], before,
        "gradient banner keeps its real surface"
    );
}

#[test]
fn white_text_band_untouched_on_dark_page() {
    let mut band = json!({"type":"frame","children":[
        {"type":"text","fill":[{"type":"solid","color":"#FFFFFF"}]}
    ]});
    fix_invisible_text_band(&mut band, Theme::Dark, "$--primary");
    assert!(
        band.get("fill").is_none(),
        "white text on a dark page is fine — no fill stamped"
    );
}

#[test]
fn mixed_text_band_not_filled() {
    // a dark text present → not an all-white invisible band → leave alone.
    let mut band = json!({"type":"frame","children":[
        {"type":"text","fill":[{"type":"solid","color":"#FFFFFF"}]},
        {"type":"text","fill":[{"type":"solid","color":"#1C1410"}]}
    ]});
    fix_invisible_text_band(&mut band, Theme::Light, "$--primary");
    assert!(band.get("fill").is_none(), "mixed text → not a hidden band");
}

#[test]
fn media_clipper_radius_preserved_under_decorated_card() {
    // A card with cornerRadius wrapping an image-placeholder: the placeholder's
    // own cornerRadius rounds the photo, not card decoration → keep it.
    let mut outer = json!({
        "type":"frame","role":"card","cornerRadius":12,
        "children":[{
            "type":"frame","role":"image-card","cornerRadius":12,"clipContent":true,
            "children":[{"type":"image","name":"photo"}]
        }]
    });
    strip_nested_card_decoration(&mut outer, DecoFlags::default());
    assert_eq!(
        outer["children"][0]["cornerRadius"],
        json!(12),
        "media-clipper radius preserved (rounds the photo)"
    );
}

#[test]
fn backdrop_sibling_fill_merges_into_the_nav_item() {
    // test07021's verbatim defect: "Dashboard Item" [Active Bg(fill×fill,
    // $--card), icon, labels] — the backdrop ate the row's left half
    // and shoved the label outside the sidebar. Its fill must move onto the
    // item and the backdrop must disappear.
    let mut nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"item","name":"Dashboard Item","layout":"horizontal","gap":16,
        "width":"fill_container","height":"fit_content","padding":[12,16],"children":[
            {"type":"rectangle","id":"bg","name":"Active Bg","width":"fill_container","height":"fill_container",
             "fill":[{"type":"solid","color":"$--card"}],"cornerRadius":8},
            {"type":"icon_font","id":"ic","iconFontName":"layout-dashboard","width":20,"height":20},
            {"type":"text","id":"lbl","content":"Dashboard","fontSize":14}
        ]
    }))
    .unwrap()];
    apply_tree_heuristics(&mut nodes, None, Theme::Dark, None);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    let kids = v["children"].as_array().unwrap();
    assert_eq!(kids.len(), 2, "backdrop removed: {kids:?}");
    assert_eq!(
        v["fill"][0]["color"], "$--card",
        "backdrop fill moved onto the item"
    );
    assert_eq!(v["cornerRadius"], json!(8.0), "radius moved along");
}

#[test]
fn thin_divider_and_fixed_avatar_are_not_backdrops() {
    // A thin divider (fixed 1px height) and a fixed-size avatar square carry
    // fills but are NOT backdrops — they must stay.
    let mut nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"row","name":"Row","layout":"horizontal","children":[
            {"type":"rectangle","id":"div","name":"Divider","width":"fill_container","height":1,
             "fill":[{"type":"solid","color":"#2A2A2A"}]},
            {"type":"text","id":"t","content":"After","fontSize":14}
        ]
    }))
    .unwrap()];
    apply_tree_heuristics(&mut nodes, None, Theme::Dark, None);
    let v = serde_json::to_value(&nodes[0]).unwrap();
    assert_eq!(
        v["children"].as_array().unwrap().len(),
        2,
        "divider survives"
    );
}
