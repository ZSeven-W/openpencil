use super::*;
use serde_json::json;
use std::collections::HashMap;

fn rects(entries: &[(&str, f64, f64, f64, f64)]) -> HashMap<String, Rect> {
    entries
        .iter()
        .map(|(id, x, y, w, h)| {
            (
                (*id).to_string(),
                Rect {
                    x: *x,
                    y: *y,
                    w: *w,
                    h: *h,
                },
            )
        })
        .collect()
}

#[test]
fn pill_chip_text_overflow_is_not_converted_to_fill_wrap() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Guest Chip","width":48,"height":40,
        "layout":"horizontal","cornerRadius":9999,"children":[
            {"type":"text","id":"guest-text","name":"Guest Text","content":"2 Guests, 1 Room","width":"fit_content","textGrowth":"auto"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 48.0, 40.0),
        ("guest-text", 0.0, 0.0, 140.0, 18.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&chip, &rects, &mut cmds);

    assert!(
        cmds.is_empty(),
        "pill chip text must stay single-line instead of fill+wrap: {cmds:?}"
    );
}

#[test]
fn pill_chip_text_already_fill_wrapped_is_restored_to_single_line() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Guest Chip","width":60,"height":40,
        "layout":"horizontal","cornerRadius":9999,"children":[
            {"type":"text","id":"guest-text","name":"Guest Text","content":"2 Guests, 1 Room","width":"fill_container","textGrowth":"fixed-width"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 60.0, 40.0),
        ("guest-text", 0.0, 0.0, 32.0, 65.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&chip, &rects, &mut cmds);

    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == "width" && k == "fit_content"
        )
    }));
    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == "textGrowth" && k == "auto"
        )
    }));
}

#[test]
fn ordinary_card_text_overflow_still_converts_to_fill_wrap() {
    let card = json!({
        "type":"frame","id":"card","name":"Narrow Card","width":48,"height":90,
        "layout":"vertical","cornerRadius":8,"children":[
            {"type":"text","id":"copy","name":"Copy","content":"Long text","width":"fit_content","textGrowth":"auto"}
        ]
    });
    let rects = rects(&[
        ("card", 0.0, 0.0, 48.0, 90.0),
        ("copy", 0.0, 0.0, 140.0, 18.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&card, &rects, &mut cmds);

    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == "width" && k == "fill_container"
        )
    }));
    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                if property == "textGrowth" && k == "fixed-width"
        )
    }));
}

#[test]
fn overfull_all_pill_chip_row_clips_instead_of_flexifying_chips() {
    let row = json!({
        "type":"frame","id":"chips","name":"Chips Row","width":240,"height":48,"layout":"horizontal","gap":8,"children":[
            {"type":"frame","id":"date","name":"Date Chip","width":120,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"dt","content":"Jun 12"}]},
            {"type":"frame","id":"guest","name":"Guest Chip","width":100,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"gt","content":"2 Guests"}]},
            {"type":"frame","id":"map","name":"Map Chip","width":90,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"mt","content":"Map"}]}
        ]
    });
    let rects = rects(&[
        ("chips", 0.0, 0.0, 240.0, 48.0),
        ("date", 0.0, 0.0, 120.0, 40.0),
        ("guest", 128.0, 0.0, 100.0, 40.0),
        ("map", 236.0, 0.0, 90.0, 40.0),
    ]);
    let mut cmds = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut cmds, false);

    assert!(cmds.iter().any(|cmd| {
        matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Bool(true) }
                if node_id.as_str() == "chips" && property == "clipContent"
        )
    }));
    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Keyword(k), .. }
                    if property == "width" && k == "fill_container"
            )
        }),
        "pill chips must not be flexified: {cmds:?}"
    );
}

#[test]
fn overfull_pill_chip_row_with_spacer_still_clips() {
    let row = json!({
        "type":"frame","id":"chips","name":"Chips Row","width":240,"height":"fit_content","layout":"horizontal","gap":10,"children":[
            {"type":"frame","id":"date","name":"Date Chip","height":40,"cornerRadius":9999,"children":[{"type":"text","id":"dt","content":"Jun 12"}]},
            {"type":"frame","id":"guest","name":"Guest Chip","width":120,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"gt","content":"2 Guests"}]},
            {"type":"frame","id":"spacer","name":"Spacer","role":"spacer","width":"fill_container","height":1,"children":[]},
            {"type":"frame","id":"sort","name":"Sort Chip","width":90,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"st","content":"Sort"}]}
        ]
    });
    let rects = rects(&[
        ("chips", 0.0, 0.0, 240.0, 48.0),
        ("date", 0.0, 0.0, 86.0, 40.0),
        ("guest", 96.0, 0.0, 120.0, 40.0),
        ("spacer", 226.0, 0.0, 1.0, 1.0),
        ("sort", 237.0, 0.0, 90.0, 40.0),
    ]);
    let mut cmds = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut cmds, false);

    assert!(
        cmds.iter().any(|cmd| {
            matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Bool(true) }
                    if node_id.as_str() == "chips" && property == "clipContent"
            )
        }),
        "spacer should not prevent chip-row clipping: {cmds:?}"
    );
}

#[test]
fn fitted_pill_chip_rail_with_flexible_spacer_still_clips() {
    let row = json!({
        "type":"frame","id":"chips","name":"Chips Row","width":"fill_container","height":"fit_content","layout":"horizontal","gap":10,"children":[
            {"type":"frame","id":"date","name":"Date Chip","height":40,"cornerRadius":9999,"children":[{"type":"text","id":"dt","content":"Jun 12"}]},
            {"type":"frame","id":"guest","name":"Guest Chip","width":"fill_container","height":40,"cornerRadius":9999,"children":[{"type":"text","id":"gt","content":"2 Guests"}]},
            {"type":"frame","id":"spacer","name":"Spacer","role":"spacer","width":"fill_container","height":1,"children":[]},
            {"type":"frame","id":"sort","name":"Sort Chip","width":"fill_container","height":40,"cornerRadius":9999,"children":[{"type":"text","id":"st","content":"Sort"}]}
        ]
    });
    let rects = rects(&[
        ("chips", 0.0, 0.0, 335.0, 40.0),
        ("date", 0.0, 0.0, 149.0, 40.0),
        ("guest", 159.0, 0.0, 60.0, 40.0),
        ("spacer", 229.0, 20.0, 36.0, 1.0),
        ("sort", 275.0, 0.0, 60.0, 40.0),
    ]);
    let mut cmds = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut cmds, false);

    assert!(
        cmds.iter().any(|cmd| {
            matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Bool(true) }
                    if node_id.as_str() == "chips" && property == "clipContent"
            )
        }),
        "all-pill chip rails clip their overflow instead of flexifying chips: {cmds:?}"
    );
}

#[test]
fn mixed_overfull_row_does_not_get_chip_clip_exemption() {
    let row = json!({
        "type":"frame","id":"row","name":"Mixed Row","width":240,"height":48,"layout":"horizontal","gap":8,"children":[
            {"type":"frame","id":"filter","name":"Filter Panel","width":140,"height":40,"cornerRadius":8,"children":[{"type":"text","id":"ft","content":"Filter"}]},
            {"type":"frame","id":"guest","name":"Guest Chip","width":100,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"gt","content":"2 Guests"}]},
            {"type":"frame","id":"map","name":"Map Chip","width":90,"height":40,"cornerRadius":9999,"children":[{"type":"text","id":"mt","content":"Map"}]}
        ]
    });
    let rects = rects(&[
        ("row", 0.0, 0.0, 240.0, 48.0),
        ("filter", 0.0, 0.0, 140.0, 40.0),
        ("guest", 148.0, 0.0, 100.0, 40.0),
        ("map", 256.0, 0.0, 90.0, 40.0),
    ]);
    let mut cmds = Vec::new();

    collect_row_overfull_fixes(&row, &rects, &mut cmds, false);

    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { property, value: LayoutPropValue::Bool(true), .. }
                    if property == "clipContent"
            )
        }),
        "mixed row must not get pill-row clipping: {cmds:?}"
    );
}
