use super::*;
use serde_json::json;

#[test]
fn horizontal_overflow_preserves_direct_arc_stack_wrapper_width() {
    // A progress-ring track + arc are visually stacked, not row siblings. The
    // generic row sum would otherwise widen this 120px wrapper to 320px before
    // the dedicated radial cleanup gets a chance to overlay the children.
    let mut ring = json!({
        "type":"frame","name":"Ring","layout":"horizontal","width":120,"height":120,"gap":0,
        "children":[
            {"type":"ellipse","name":"Ring Track","width":120,"height":120,"innerRadius":0.86},
            {"type":"ellipse","name":"Ring Progress","width":120,"height":120,
             "innerRadius":0.86,"startAngle":-90,"sweepAngle":264},
            {"type":"frame","name":"Ring Center","width":80,"height":44}
        ]
    });

    fix_horizontal_overflow(&mut ring, 375.0);

    assert_eq!(ring["width"], json!(120));
    assert!(ring.get("clipContent").is_none());
}

#[test]
fn horizontal_overflow_still_expands_row_of_plain_ellipses() {
    // The exemption is deliberately limited to arc/donut geometry. Ordinary
    // ellipse siblings remain a real horizontal row and keep the generic fix.
    let mut row = json!({
        "type":"frame","layout":"horizontal","width":120,"gap":0,
        "children":[
            {"type":"ellipse","width":120,"height":120},
            {"type":"ellipse","width":120,"height":120}
        ]
    });

    fix_horizontal_overflow(&mut row, 375.0);

    assert_eq!(row["width"], json!(240.0));
}

#[test]
fn horizontal_overflow_still_expands_wide_row_of_independent_arcs() {
    let mut row = json!({
        "type":"frame","layout":"horizontal","width":120,"height":48,"gap":8,
        "children":[
            {"type":"ellipse","width":40,"height":40,"innerRadius":0.75,"sweepAngle":180},
            {"type":"ellipse","width":40,"height":40,"innerRadius":0.75,"sweepAngle":220},
            {"type":"text","width":40,"height":20,"content":"Goals"}
        ]
    });

    fix_horizontal_overflow(&mut row, 375.0);

    assert_eq!(row["width"], json!(136.0));
}
