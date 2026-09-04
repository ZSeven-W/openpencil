use super::*;
use serde_json::json;

// ── inject_missing_nav_surface_fill ──────────────────────────────────────

#[test]
fn nav_without_fill_gets_surface_and_upward_shadow() {
    let mut nav = json!({
        "type":"frame","role":"bottom-tab-bar",
        "children":[{"type":"frame","role":"button","name":"Home"}]
    });
    inject_nav_surface_for_section(&mut nav);
    assert_eq!(
        nav["fill"],
        json!([{"type":"solid","color":"$--card"}]),
        "transparent bottom nav anchored with surface fill"
    );
    // bottom nav → shadow points up (offsetY < 0)
    assert_eq!(nav["effects"][0]["offsetY"], json!(-4));
}

#[test]
fn nav_with_existing_fill_left_alone() {
    let mut nav = json!({
        "type":"frame","role":"bottom-tab-bar","fill":[{"type":"solid","color":"#222222"}],
        "children":[{"type":"frame","role":"button"}]
    });
    inject_nav_surface_for_section(&mut nav);
    assert_eq!(
        nav["fill"],
        json!([{"type":"solid","color":"#222222"}]),
        "explicit nav fill preserved (intent signal)"
    );
    assert!(
        nav.get("effects").is_none(),
        "no shadow stamped over intent"
    );
}

#[test]
fn nav_wrapped_in_single_child_section_reached_one_hop() {
    let mut wrap = json!({
        "type":"frame","role":"section",
        "children":[{"type":"frame","role":"bottom-tab-bar","children":[{"type":"frame"}]}]
    });
    inject_nav_surface_for_section(&mut wrap);
    assert_eq!(
        wrap["children"][0]["fill"],
        json!([{"type":"solid","color":"$--card"}]),
        "nav nested one hop under a wrapper section still anchored"
    );
}

// ── round_active_nav_tab ─────────────────────────────────────────────────

#[test]
fn active_nav_tab_square_rounded_to_pill() {
    // tt5: the active tab is a solid-filled frame with NO cornerRadius → a sharp
    // square poking out of the rounded nav pill. Round it to a pill; leave the
    // inactive (fill-less) tab and the already-rounded bar container alone.
    let mut nav = json!({
        "type":"frame","role":"bottom-tab-bar","cornerRadius":100,
        "fill":[{"type":"solid","color":"$--card"}],
        "children":[
            {"type":"frame","name":"Home Tab","fill":[{"type":"solid","color":"$--primary"}],
             "children":[{"type":"icon_font","iconFontName":"home"},{"type":"text","content":"Home"}]},
            {"type":"frame","name":"Search Tab",
             "children":[{"type":"icon_font","iconFontName":"search"},{"type":"text","content":"Search"}]}
        ]
    });
    round_active_nav_tab(&mut nav, "$--chart-6");
    assert_eq!(
        nav["children"][0]["cornerRadius"],
        json!(999.0),
        "active filled tab rounded to a pill"
    );
    assert!(
        nav["children"][1].get("cornerRadius").is_none(),
        "inactive fill-less tab untouched"
    );
    assert_eq!(
        nav["cornerRadius"],
        json!(100),
        "the bar container's own radius is preserved"
    );
}

#[test]
fn rounded_nav_bar_gets_clip_content() {
    // A full-height sharp active block can be TALLER than the bar — rounding it
    // alone can't stop the vertical overflow. A rounded-pill bar must clip its
    // children to the pill silhouette (image 52: the orange HOME square pokes
    // above + below the white pill).
    let mut nav = json!({
        "type":"frame","role":"bottom-tab-bar","cornerRadius":32,
        "fill":[{"type":"solid","color":"$--card"}],
        "children":[
            {"type":"frame","name":"Home Tab","height":"fill_container",
             "fill":[{"type":"solid","color":"$--primary"}],
             "children":[{"type":"text","content":"Home"}]}
        ]
    });
    round_active_nav_tab(&mut nav, "$--chart-6");
    assert_eq!(
        nav["clipContent"],
        json!(true),
        "rounded bar clips children so a tall active block can't overflow"
    );
    assert_eq!(
        nav["children"][0]["cornerRadius"],
        json!(999.0),
        "the filled active tab is still rounded to a pill"
    );
}

#[test]
fn full_pipeline_rounds_nested_active_tab_in_pill() {
    // tt8 (image 53) EXACT structure: bottom-tab-bar > Nav Pill(cr=100) >
    // Home Tab(fill orange, no radius, fill_container/vertical). Run the FULL
    // apply_tree_heuristics (incl. the PenNode JSON round-trip) and confirm the
    // active Home Tab actually comes out rounded to a pill — a square here is
    // the sharp block overflowing the rounded pill the user keeps seeing.
    let mut forest: Vec<PenNode> = serde_json::from_value(json!([{
        "type":"frame","id":"bar","role":"bottom-tab-bar","name":"Bottom Navigation",
        "width":"fill_container","fill":[{"type":"solid","color":"$--card"}],
        "children":[{
            "type":"frame","id":"pill","name":"Nav Pill","width":"fill_container","layout":"horizontal",
            "cornerRadius":100,"fill":[{"type":"solid","color":"$--card"}],
            "children":[
                {"type":"frame","id":"home","name":"Home Tab","width":"fill_container","height":"fit_content",
                 "layout":"vertical","fill":[{"type":"solid","color":"$--chart-6"}],
                 "children":[
                    {"type":"icon_font","id":"hi","name":"Home Icon","iconFontName":"home","width":20,"height":20},
                    {"type":"text","id":"hl","name":"Home Label","content":"HOME"}
                 ]},
                {"type":"frame","id":"search","name":"Search Tab","width":"fill_container","height":"fit_content",
                 "layout":"vertical","children":[
                    {"type":"icon_font","id":"si","name":"Search Icon","iconFontName":"search","width":20,"height":20}
                 ]}
            ]
        }]
    }]))
    .expect("nav forest json");
    apply_tree_heuristics(
        &mut forest,
        Some("#FFF8F0"),
        Theme::Light,
        Some("$--chart-6"),
    );
    let out = serde_json::to_value(&forest[0]).expect("serialize");
    let home = &out["children"][0]["children"][0];
    assert_eq!(home["name"], json!("Home Tab"));
    assert_eq!(
        home["cornerRadius"],
        json!(999.0),
        "filled active Home Tab must round to a pill end-to-end (got {:?})",
        home.get("cornerRadius")
    );
}

#[test]
fn full_pipeline_rounds_manifest_nav_item_active() {
    // glm-5.1 / manifest (element-builder) nav: bottom-tab-bar > nav-item-active
    // tab (role="nav-item-active", width=fit_content) carrying an accent fill
    // (the orange active block). This is the structure the USER actually gets
    // (their model is glm-5.1, the manifest path) — distinct from glm-5.2's
    // `bottom-tab-bar > Nav Pill > Home Tab`. The active tab must round to a pill.
    let mut forest: Vec<PenNode> = serde_json::from_value(json!([{
        "type":"frame","id":"bar","role":"bottom-tab-bar","name":"Bottom Tab Bar",
        "width":"fill_container","cornerRadius":100,
        "fill":[{"type":"solid","color":"$--card"}],
        "children":[
            {"type":"frame","id":"home","role":"nav-item-active","name":"Tab (Home)",
             "width":"fit_content","height":"fit_content","layout":"vertical","padding":[4,12],
             "fill":[{"type":"solid","color":"$--chart-6"}],
             "children":[
                {"type":"icon_font","id":"hi","name":"Icon","iconFontName":"house","width":24,"height":24},
                {"type":"text","id":"hl","name":"Label","content":"Home"}
             ]},
            {"type":"frame","id":"search","role":"nav-item","name":"Tab (Search)",
             "width":"fit_content","height":"fit_content","layout":"vertical","padding":[4,12],
             "children":[
                {"type":"icon_font","id":"si","name":"Icon","iconFontName":"search","width":24,"height":24}
             ]}
        ]
    }]))
    .expect("manifest nav json");
    apply_tree_heuristics(
        &mut forest,
        Some("#FFF8F0"),
        Theme::Light,
        Some("$--chart-6"),
    );
    let out = serde_json::to_value(&forest[0]).expect("serialize");
    let active = &out["children"][0];
    assert_eq!(active["role"], json!("nav-item-active"));
    assert_eq!(
        active["cornerRadius"],
        json!(999.0),
        "manifest nav-item-active tab must round to a pill (got {:?})",
        active.get("cornerRadius")
    );
}

#[test]
fn full_pipeline_rounds_user_tt5_exact_nav() {
    // The USER's actual tt5.op (glm-5.2): bottom-tab-bar(vertical) > Tab
    // Pill(role=None, cr=100, horizontal) > Home Tab(role=None, fill_container,
    // vertical, cornerRadius=0.0, $--chart-6 fill, white icon/label). This is
    // the EXACT square the user keeps seeing. Run the full pipeline; the active
    // Home Tab must come out rounded (cornerRadius=999).
    let src = r##"[{
        "type":"frame","id":"bar","role":"bottom-tab-bar","name":"Bottom Nav",
        "width":"fill_container","height":"fit_content","layout":"vertical",
        "fill":[{"type":"solid","color":"$--card"}],
        "children":[{
            "type":"frame","id":"pill","name":"Tab Pill",
            "width":"fill_container","height":"fit_content","layout":"horizontal","cornerRadius":100,
            "fill":[{"type":"solid","color":"$--card"}],
            "children":[
                {"type":"frame","id":"home","name":"Home Tab",
                 "width":"fill_container","height":"fit_content","layout":"vertical","cornerRadius":0.0,
                 "fill":[{"type":"solid","color":"$--chart-6"}],
                 "children":[
                    {"type":"icon_font","id":"hi","name":"Home Icon","iconFontName":"home","width":18,"height":18,
                     "fill":[{"type":"solid","color":"$--card"}]},
                    {"type":"text","id":"hl","name":"Home Label","content":"HOME",
                     "fill":[{"type":"solid","color":"$--card"}]}
                 ]},
                {"type":"frame","id":"sr","name":"Search Tab","role":"search-bar",
                 "width":"fill_container","height":"fit_content","layout":"vertical","cornerRadius":26,
                 "children":[
                    {"type":"icon_font","id":"si","name":"Search Icon","iconFontName":"search","width":18,"height":18}
                 ]}
            ]
        }]
    }]"##;
    let mut forest: Vec<PenNode> = serde_json::from_str(src).expect("user nav json");
    apply_tree_heuristics(
        &mut forest,
        Some("#FFF8F0"),
        Theme::Light,
        Some("$--chart-6"),
    );
    let out = serde_json::to_value(&forest[0]).expect("serialize");
    let home = &out["children"][0]["children"][0];
    assert_eq!(home["name"], json!("Home Tab"));
    assert_eq!(
        home["cornerRadius"],
        json!(999.0),
        "user's exact square Home Tab must round to a pill (got {:?})",
        home.get("cornerRadius")
    );
}

#[test]
fn bare_highlight_rect_in_nav_rounded() {
    // The active highlight is sometimes a bare filled rect (no icon/label
    // children) sitting behind the tab content. It must still round to a pill.
    let mut nav = json!({
        "type":"frame","role":"bottom-tab-bar","cornerRadius":28,
        "children":[
            {"type":"frame","name":"Tab Row","width":"fill_container","layout":"horizontal",
             "children":[
                {"type":"frame","name":"Active Highlight",
                 "fill":[{"type":"solid","color":"$--primary"}]}
             ]}
        ]
    });
    round_active_nav_tab(&mut nav, "$--chart-6");
    assert_eq!(
        nav["children"][0]["children"][0]["cornerRadius"],
        json!(999.0),
        "bare filled highlight rect rounded to a pill"
    );
    assert!(
        nav["children"][0].get("cornerRadius").is_none(),
        "the full-width horizontal tab ROW is not rounded"
    );
}

// ── strip_redundant_section_fill ─────────────────────────────────────────

#[test]
fn section_safe_light_hedge_fill_stripped_with_chrome() {
    // Weak-model "search section" wrapper: white hedge fill + pill chrome.
    let mut sect = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"#FFFFFF"}],
        "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#E2E8F0"}]},
        "cornerRadius":16,
        "children":[{"type":"text","content":"x"}]
    });
    strip_redundant_section_fill(&mut sect, None);
    assert!(sect.get("fill").is_none(), "hedge fill stripped");
    assert!(sect.get("stroke").is_none(), "wrapper stroke stripped");
    assert!(
        sect.get("cornerRadius").is_none(),
        "wrapper radius stripped"
    );
}

#[test]
fn section_dark_hedge_fill_stripped() {
    let mut sect = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"#0A0A0A"}],
        "children":[{"type":"text"}]
    });
    strip_redundant_section_fill(&mut sect, None);
    assert!(sect.get("fill").is_none(), "safe-dark hedge stripped");
}

#[test]
fn card_fill_protected_from_strip() {
    let mut card = json!({
        "type":"frame","role":"card","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text","content":"x"}]
    });
    strip_redundant_section_fill(&mut card, None);
    assert_eq!(
        card["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "card is a protected surface, not a section"
    );
}

#[test]
fn misrolled_search_bar_wrapper_stripped() {
    // sub-agent emits Search Bar(role=search-bar) > Search Input(role=input,fill)
    // — the outer "search-bar" is a section wrapper, strip its hedge fill.
    let mut wrap = json!({
        "type":"frame","role":"search-bar","fill":[{"type":"solid","color":"#F8FAFC"}],
        "children":[{"type":"text_input","role":"input","fill":[{"type":"solid","color":"#FFFFFF"}]}]
    });
    strip_redundant_section_fill(&mut wrap, None);
    assert!(
        wrap.get("fill").is_none(),
        "misrolled search-bar wrapper around a real input → fill stripped"
    );
}

#[test]
fn section_fill_matching_page_bg_stripped() {
    let mut sect = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"#FFF8F0"}],
        "children":[{"type":"text"}]
    });
    strip_redundant_section_fill(&mut sect, Some("#FFF8F0"));
    assert!(
        sect.get("fill").is_none(),
        "section repainting the page bg is redundant → stripped"
    );
}

#[test]
fn screen_root_safe_dark_fill_preserved() {
    // Screen root (mobile artboard, width=390, height=844) with safe-dark fill
    // should NOT be stripped. A screen root is the ground level; its fill is never
    // redundant because nothing is behind it to paint.
    let mut root = json!({
        "type":"frame","width":390.0,"height":844.0,
        "fill":[{"type":"solid","color":"#0A0A0A"}],
        "children":[{"type":"text","content":"Content"}]
    });
    strip_redundant_section_fill(&mut root, None);
    assert_eq!(
        root["fill"],
        json!([{"type":"solid","color":"#0A0A0A"}]),
        "screen root's dark fill preserved (is the ground)"
    );
}

#[test]
fn screen_root_page_background_fill_preserved() {
    // Screen root with fill matching the page background. For a section, this
    // would be redundant, but a screen root is the ground — it has no background
    // behind it, so the fill is never redundant.
    let mut root = json!({
        "type":"frame","width":390.0,"height":844.0,
        "fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text","content":"Content"}]
    });
    strip_redundant_section_fill(&mut root, Some("#FFFFFF"));
    assert_eq!(
        root["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "screen root's background fill preserved (is the ground)"
    );
}

#[test]
fn fit_content_screen_root_fill_preserved() {
    // Screen root with fit_content height (generated mobile artboard) and safe-dark
    // fill should NOT be stripped.
    let mut root = json!({
        "type":"frame","width":390.0,"height":"fit_content",
        "fill":[{"type":"solid","color":"#1A1A1A"}],
        "children":[{"type":"text","content":"Content"}]
    });
    strip_redundant_section_fill(&mut root, None);
    assert_eq!(
        root["fill"],
        json!([{"type":"solid","color":"#1A1A1A"}]),
        "fit_content screen root's fill preserved"
    );
}

// ── clip_card_image_corners ──────────────────────────────────────────────
