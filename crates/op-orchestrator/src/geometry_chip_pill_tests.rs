//! Fixed-width pill clipping: the measured-overflow branch that a
//! hug-width chip must not take.
//!
//! Split from `geometry_chip_private_tests.rs`, which is at the 800-line
//! ceiling.

use super::*;
use serde_json::json;

use super::chip_private_tests::rects;

#[test]
fn hug_width_pill_with_wrapped_text_gets_unwrapped_no_clip() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Hug Chip","layout":"horizontal","cornerRadius":20,"height":40,"children":[
            {"type":"text","id":"text","name":"Text","content":"Very long text here","width":"fill_container","textGrowth":"fixed-width"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 50.0, 40.0),
        ("text", 0.0, 0.0, 40.0, 35.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&chip, &rects, &mut cmds);

    assert!(
        cmds.iter().any(|cmd| {
            matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property,
                    value: LayoutPropValue::Keyword(k),
                } if node_id.as_str() == "text"
                    && property == "width"
                    && k == "fit_content"
            )
        }),
        "hug-width pill unwraps wrapped text: {cmds:?}"
    );
    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp {
                    property,
                    value: LayoutPropValue::Bool(true),
                    ..
                } if property == "clipContent"
            )
        }),
        "hug-width pill never sets clipContent: {cmds:?}"
    );
}

#[test]
fn numeric_width_pill_with_text_overflow_clips_once() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Search Pill","width":135,"height":40,
        "layout":"horizontal","cornerRadius":20,"children":[
            {"type":"text","id":"search-text","name":"Search","content":"Search","width":"fit_content","textGrowth":"auto"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 135.0, 40.0),
        ("search-text", 0.0, 0.0, 380.0, 18.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&chip, &rects, &mut cmds);

    let clip_cmds: Vec<_> = cmds
        .iter()
        .filter(|cmd| {
            matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp {
                    node_id,
                    property,
                    value: LayoutPropValue::Bool(true),
                } if node_id.as_str() == "chip" && property == "clipContent"
            )
        })
        .collect();
    assert_eq!(
        clip_cmds.len(),
        1,
        "exactly one clipContent command on pill"
    );
    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { property, .. } if property == "width"
            )
        }),
        "no width mutation on text or pill: {cmds:?}"
    );
    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp { property, .. } if property == "textGrowth"
            )
        }),
        "no textGrowth mutation: {cmds:?}"
    );
}

#[test]
fn numeric_width_pill_already_clipped_does_not_duplicate_command() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Search Pill","width":135,"height":40,
        "layout":"horizontal","cornerRadius":20,"clipContent":true,"children":[
            {"type":"text","id":"search-text","name":"Search","content":"Search","width":"fit_content","textGrowth":"auto"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 135.0, 40.0),
        ("search-text", 0.0, 0.0, 380.0, 18.0),
    ]);
    let mut cmds = Vec::new();

    collect_text_overflow_fixes(&chip, &rects, &mut cmds);

    assert!(
        cmds.iter().all(|cmd| {
            !matches!(
                cmd,
                EditorCommand::SetNodeLayoutProp {
                    property,
                    value: LayoutPropValue::Bool(true),
                    ..
                } if property == "clipContent"
            )
        }),
        "no duplicate clipContent command when already set: {cmds:?}"
    );
}

#[test]
fn diagnostic_numeric_width_pill_with_text_overflow_emits_message() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Search Pill","width":135,"height":40,
        "layout":"horizontal","cornerRadius":20,"children":[
            {"type":"text","id":"search-text","name":"Search","content":"Search","width":"fit_content","textGrowth":"auto"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 135.0, 40.0),
        ("search-text", 0.0, 0.0, 380.0, 18.0),
    ]);
    let mut diags = Vec::new();

    collect_diagnostics(&chip, &rects, &mut diags);

    assert!(
        diags
            .iter()
            .any(|d| d.contains("text") && d.contains("overflow")),
        "numeric-width pill with measured text overflow should emit diagnostic: {diags:?}"
    );
}

#[test]
fn diagnostic_hug_width_pill_with_text_overflow_stays_silent() {
    let chip = json!({
        "type":"frame","id":"chip","name":"Hug Pill","height":40,"layout":"horizontal","cornerRadius":20,"children":[
            {"type":"text","id":"search-text","name":"Search","content":"Search","width":"fit_content","textGrowth":"auto"}
        ]
    });
    let rects = rects(&[
        ("chip", 0.0, 0.0, 50.0, 40.0),
        ("search-text", 0.0, 0.0, 380.0, 18.0),
    ]);
    let mut diags = Vec::new();

    collect_diagnostics(&chip, &rects, &mut diags);

    assert!(
        diags.is_empty(),
        "hug-width pill should not emit diagnostic even with wide text: {diags:?}"
    );
}

#[test]
fn ordinary_horizontal_clipped_card_still_repairs_inner_overflows() {
    let tree = json!({
        "type":"frame","id":"card","name":"Media Card","width":120,"height":100,
        "layout":"horizontal","clipContent":true,"children":[{
            "type":"frame","id":"content","name":"Card Content","width":"fill_container",
            "height":"fit_content","layout":"horizontal","gap":8,"children":[
                {
                    "type":"frame","id":"visual","name":"Visual","width":"fit_content",
                    "height":40,"children":[]
                },
                {
                    "type":"text","id":"copy","name":"Copy","content":"Long card copy",
                    "width":"fit_content","textGrowth":"auto"
                }
            ]
        }]
    });
    let rects = rects(&[
        ("card", 0.0, 0.0, 120.0, 100.0),
        ("content", 0.0, 0.0, 120.0, 40.0),
        ("visual", 0.0, 0.0, 140.0, 40.0),
        ("copy", 148.0, 0.0, 140.0, 18.0),
    ]);

    let mut text_cmds = Vec::new();
    collect_text_overflow_fixes(&tree, &rects, &mut text_cmds);
    assert!(text_cmds.iter().any(|cmd| matches!(
        cmd,
        EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Keyword(k) }
            if node_id.as_str() == "copy" && property == "width" && k == "fill_container"
    )));

    let mut frame_cmds = Vec::new();
    collect_frame_overflow_fixes(&tree, &rects, &mut frame_cmds);
    assert!(frame_cmds.iter().any(|cmd| matches!(
        cmd,
        EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Keyword(k) }
            if node_id.as_str() == "visual" && property == "width" && k == "fill_container"
    )));

    let mut row_cmds = Vec::new();
    collect_row_overfull_fixes(&tree, &rects, &mut row_cmds, false);
    assert!(row_cmds.iter().any(|cmd| matches!(
        cmd,
        EditorCommand::SetNodeLayoutProp { node_id, property, value: LayoutPropValue::Keyword(k) }
            if node_id.as_str() == "visual" && property == "width" && k == "fill_container"
    )));
}
