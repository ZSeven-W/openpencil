use super::*;
use serde_json::json;

#[test]
fn layout_none_repair_centres_against_resolved_child_size() {
    let ring = json!({
        "type":"frame","id":"ring","width":"fill_container","height":120,"layout":"none",
        "children":[
            {"type":"frame","id":"center","width":98,"height":43},
            {"type":"ellipse","id":"progress","width":120,"height":120,
             "innerRadius":0.86,"sweepAngle":264},
            {"type":"ellipse","id":"track","width":120,"height":120,"innerRadius":0.86}
        ]
    });
    let rects = HashMap::from([
        ("ring".to_string(), Rect { w: 287.0, h: 120.0 }),
        ("center".to_string(), Rect { w: 98.0, h: 47.0 }),
    ]);

    let commands = radial_stack_repair(&ring, &rects).expect("radial repair");
    let center_update = commands.iter().find_map(|command| match command {
        EditorCommand::UpdateNode {
            node_id,
            x,
            y,
            width,
            height,
            ..
        } if node_id.as_str() == "center" => Some((*x, *y, *width, *height)),
        _ => None,
    });

    assert_eq!(
        center_update,
        Some((Some(95), Some(37), None, None)),
        "position must use the final 47px layout height without rewriting the authored size"
    );
}
