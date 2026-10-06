//! Tests for `round_missing_compact_pill_radius` — the structural fallback
//! for CTA/pill corner rounding. Fixtures mirror 0724-1-gm-3.op's real
//! shape: hug-sized (`fit_content`) buttons authored via padding + content,
//! not literal pixel width/height — n180 "已完成" (icon + text), n192
//! "继续", n203 "开始", none carrying a `cornerRadius`, sitting in a screen
//! that already has several `cornerRadius: 9999` badges (n144 "Level
//! Badge", n168, n209 "Count Badge", n233 "Rank Badge").

use super::*;

fn run(mut nodes: Vec<PenNode>) -> serde_json::Value {
    enforce_surface_color_discipline(&mut nodes);
    serde_json::to_value(&nodes[0]).unwrap()
}

/// A hug-sized painted badge/pill, `fit_content` on both axes, matching the
/// real corpus's authoring convention (padding + content, no literal size).
fn hug_badge(id: &str, content: &str, corner_radius: Option<f64>) -> serde_json::Value {
    let mut v = json!({
        "type":"frame","id":id,
        "width":"fit_content","height":"fit_content",
        "padding":[4.0, 10.0],
        "fill":[{"type":"solid","color":"$--accent"}],
        "children":[{"type":"text","id":format!("{id}-t"),"width":"fit_content",
                     "height":"fit_content","content":content}]
    });
    if let Some(r) = corner_radius {
        v["cornerRadius"] = json!(r);
    }
    v
}

#[test]
fn unrounded_hug_cta_buttons_are_rounded_when_the_design_already_uses_rounded_badges() {
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Today Page","width":375,"height":"fit_content",
        "layout":"vertical",
        "children":[
            hug_badge("n144", "LVL 12", Some(9999.0)),
            hug_badge("n168", "2/3 完成", Some(9999.0)),
            // Real gm-3 shape: icon + text CTA, no cornerRadius.
            {"type":"frame","id":"n180","layout":"horizontal","gap":4.0,
             "padding":[6.0, 12.0],"alignItems":"center",
             "fill":[{"type":"solid","color":"#22C55E1A"}],
             "children":[
                {"type":"icon_font","id":"n181","iconFontName":"check","width":14.0,"height":14.0,
                 "fill":[{"type":"solid","color":"#22C55E"}]},
                {"type":"text","id":"n182","width":"fit_content","height":"fit_content",
                 "content":"已完成"}
             ]},
            // Real gm-3 shape: text-only CTA, no cornerRadius.
            {"type":"frame","id":"n192","layout":"horizontal","padding":[8.0, 16.0],
             "justifyContent":"center","alignItems":"center",
             "fill":[{"type":"solid","color":"$--primary"}],
             "children":[{"type":"text","id":"n193","width":"fit_content","height":"fit_content",
                          "content":"继续"}]}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    let children = v["children"].as_array().unwrap();
    let by_id = |id: &str| children.iter().find(|c| c["id"] == id).unwrap();

    assert_eq!(
        by_id("n180")["cornerRadius"],
        json!(10.0),
        "icon+text CTA (fit_content) must get a fallback radius: {v}"
    );
    assert_eq!(
        by_id("n192")["cornerRadius"],
        json!(10.0),
        "text-only CTA (fit_content) must get a fallback radius: {v}"
    );
    // Already-rounded badges are untouched — their authored radius stands.
    assert_eq!(by_id("n144")["cornerRadius"], json!(9999.0));
    assert_eq!(by_id("n168")["cornerRadius"], json!(9999.0));
}

#[test]
fn all_sharp_corner_design_is_left_alone() {
    // No OTHER rounded compact capsule anywhere in the screen — the
    // consistency gate has no evidence this design uses rounded pills, so
    // the CTA (structurally identical to the positive case) must stay
    // untouched: a deliberate all-sharp-corners system is not
    // strong-armed into rounding.
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Screen","width":375,"height":"fit_content",
        "layout":"vertical",
        "children":[
            {"type":"frame","id":"n192","layout":"horizontal","padding":[8.0, 16.0],
             "justifyContent":"center","alignItems":"center",
             "fill":[{"type":"solid","color":"$--primary"}],
             "children":[{"type":"text","id":"n193","width":"fit_content","height":"fit_content",
                          "content":"继续"}]}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    assert!(
        v["children"][0].get("cornerRadius").is_none(),
        "CTA must stay untouched with no rounded-pill evidence: {v}"
    );
}

#[test]
fn icon_only_box_is_never_rounded_even_with_rounded_badge_evidence() {
    // 0724-1-gm-3.op's real n173/n185/n196 shape: a 44x44 painted icon-only
    // tap target sitting right next to CTA buttons — must never be rounded
    // by this pass, however strong the consistency-gate evidence, because
    // it has no text child.
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Screen","width":375,"height":"fit_content",
        "layout":"vertical",
        "children":[
            hug_badge("n144", "LVL 12", Some(9999.0)),
            hug_badge("n168", "2/3 完成", Some(9999.0)),
            {"type":"frame","id":"n173","width":44.0,"height":44.0,"layout":"horizontal",
             "justifyContent":"center","alignItems":"center",
             "fill":[{"type":"solid","color":"#22C55E1A"}],
             "children":[{"type":"icon_font","id":"n174","iconFontName":"headphones",
                          "width":20.0,"height":20.0,
                          "fill":[{"type":"solid","color":"#22C55E"}]}]}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    let icon_box = v["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "n173")
        .unwrap();
    assert!(
        icon_box.get("cornerRadius").is_none(),
        "icon-only box (no text child) must never be rounded by this pass: {v}"
    );
}

#[test]
fn literal_height_capsule_uses_half_height_when_under_ten() {
    // A capsule WITH an authored literal height (not the common fit_content
    // case) stays under half its own height instead of always defaulting
    // to 10, so a very short pill doesn't get an oversized radius.
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"root","name":"Screen","width":375,"height":"fit_content",
        "layout":"vertical",
        "children":[
            hug_badge("n144", "LVL 12", Some(9999.0)),
            hug_badge("n168", "2/3 完成", Some(9999.0)),
            {"type":"frame","id":"short-pill","width":80.0,"height":16.0,"layout":"horizontal",
             "justifyContent":"center","alignItems":"center",
             "fill":[{"type":"solid","color":"$--primary"}],
             "children":[{"type":"text","id":"sp-t","width":"fit_content","height":"fit_content",
                          "content":"New"}]}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    let pill = v["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "short-pill")
        .unwrap();
    assert_eq!(
        pill["cornerRadius"],
        json!(8.0),
        "16px-tall literal-height pill must get height/2=8, not the 10 fallback: {v}"
    );
}
