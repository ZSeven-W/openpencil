//! A correctly sized artboard is not a successful delivery if it hides text.
use super::*;

pub fn fixed_board_content_diagnostics(state: &EditorState) -> Vec<String> {
    let rects = resolved_rects(state);
    let mut out = Vec::new();
    for root in state.active_children() {
        if let Ok(root) = serde_json::to_value(root) {
            collect(&root, &rects, &mut out);
        }
    }
    out
}

pub(super) fn collect(root: &Value, rects: &HashMap<String, Rect>, out: &mut Vec<String>) {
    if root.get("type").and_then(Value::as_str) != Some("frame")
        || root.get("clipContent").and_then(Value::as_bool) == Some(false)
        || root.get("height").and_then(Value::as_f64).is_none()
    {
        return;
    }
    let Some(board) = root
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
    else {
        return;
    };
    if board.w <= 0.0 || board.h <= 0.0 {
        return;
    }
    walk(root, root, board, rects, out, false);
}

fn walk(
    node: &Value,
    root: &Value,
    board: &Rect,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
    in_horizontal_scroll: bool,
) {
    let in_horizontal_scroll = in_horizontal_scroll
        || (board.w <= 480.0
            && board.h >= 500.0
            && op_editor_core::scroll_viewport::is_horizontal_scroll_viewport(node)
            && node
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| rects.get(id))
                .is_some_and(|r| {
                    r.x >= board.x - 1.0
                        && r.y >= board.y - 1.0
                        && r.x + r.w <= board.x + board.w + 1.0
                        && r.y + r.h <= board.y + board.h + 1.0
                }));
    // A real clipped scroll viewport deliberately paints only its visible
    // window. Its descendants are reachable by scrolling, unlike root chrome
    // pushed out of the artboard. Plain clipping does not earn this exception.
    if node.get("clipContent").and_then(Value::as_bool) == Some(true)
        && node
            .get("events")
            .and_then(|e| e.get("onScroll"))
            .and_then(Value::as_array)
            .is_some_and(|actions| !actions.is_empty())
        && node.get("id") != root.get("id")
    {
        return;
    }
    if out.len() >= MAX_DIAGNOSTICS
        || node.get("visible").and_then(Value::as_bool) == Some(false)
        || node.get("opacity").and_then(Value::as_f64) == Some(0.0)
    {
        return;
    }
    if node.get("type").and_then(Value::as_str) == Some("text")
        && node
            .get("content")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
    {
        if let Some(r) = node
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        {
            if (!in_horizontal_scroll
                && (r.x < board.x - 1.0 || r.x + r.w > board.x + board.w + 1.0))
                || r.y < board.y - 1.0
                || r.y + r.h > board.y + board.h + 1.0
            {
                out.push(format!("fixed-board-content-outside: {} extends outside {}'s fixed viewport. Reflow the content or reduce excess whitespace; do not treat a correctly sized cropped export as complete.", diag_label(node), diag_label(root)));
            }
        }
    }
    for child in children(node) {
        walk(child, root, board, rects, out, in_horizontal_scroll);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn reachable_phone_carousels_only_exempt_horizontal_board_overflow() {
        let mut root = json!({"type":"frame","id":"root","width":375,"height":812,"children":[
            {"type":"frame","id":"viewport","name":"课程横向轨道","layout":"vertical","clipContent":true,"children":[
                {"type":"frame","layout":"horizontal","children":[
                    {"type":"text","id":"outside-x","content":"Last course"},
                    {"type":"text","id":"outside-y","content":"Cut footer"}
                ]}
            ]}
        ]});
        let rects = HashMap::from([
            (
                "root".into(),
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 375.0,
                    h: 812.0,
                },
            ),
            (
                "viewport".into(),
                Rect {
                    x: 0.0,
                    y: 100.0,
                    w: 375.0,
                    h: 120.0,
                },
            ),
            (
                "outside-x".into(),
                Rect {
                    x: 700.0,
                    y: 130.0,
                    w: 150.0,
                    h: 20.0,
                },
            ),
            (
                "outside-y".into(),
                Rect {
                    x: 700.0,
                    y: 850.0,
                    w: 150.0,
                    h: 20.0,
                },
            ),
        ]);
        let mut out = vec![];
        collect(&root, &rects, &mut out);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("outside-y"));
        root["children"][0]["name"] = json!("Clipped media card");
        out.clear();
        collect(&root, &rects, &mut out);
        assert_eq!(out.len(), 2);
        root["children"][0]["name"] = json!("Horizontal scroll viewport");
        let mut misplaced = rects.clone();
        misplaced.get_mut("viewport").unwrap().x = 40.0;
        out.clear();
        collect(&root, &misplaced, &mut out);
        assert_eq!(out.len(), 2);
    }
    #[test]
    fn fixed_viewports_report_hidden_text_but_explicit_open_frames_do_not() {
        let root = serde_json::json!({"type":"frame","id":"root","width":100,"height":50,"children":[
            {"type":"text","id":"text","content":"Delivery"}
        ]});
        let rects = HashMap::from([
            (
                "root".into(),
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 100.0,
                    h: 50.0,
                },
            ),
            (
                "text".into(),
                Rect {
                    x: 0.0,
                    y: 45.0,
                    w: 80.0,
                    h: 20.0,
                },
            ),
        ]);
        let mut out = vec![];
        collect(&root, &rects, &mut out);
        assert_eq!(out.len(), 1);
        let mut open = root.clone();
        open["clipContent"] = serde_json::json!(false);
        out.clear();
        collect(&open, &rects, &mut out);
        assert!(out.is_empty());
        open = root.clone();
        open["children"][0]["visible"] = serde_json::json!(false);
        collect(&open, &rects, &mut out);
        assert!(out.is_empty());
    }
}
