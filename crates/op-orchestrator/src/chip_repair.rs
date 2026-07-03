use serde_json::Value;

pub(crate) fn is_pill_chip(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("frame")
        && numeric(v, "height").is_some_and(|h| h > 0.0 && h <= 48.0)
        && numeric(v, "cornerRadius").is_some_and(|r| {
            let h = numeric(v, "height").unwrap_or(0.0);
            r >= h / 2.0
        })
}

pub(crate) fn all_children_are_pill_chips(children: &[&Value]) -> bool {
    let mut pill_count = 0;
    for child in children {
        if is_ignorable_chip_row_spacer(child) {
            continue;
        }
        if !is_pill_chip(child) {
            return false;
        }
        pill_count += 1;
    }
    pill_count > 0
}

pub(crate) fn is_fill_container_frame(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("frame")
        && v.get("width").and_then(Value::as_str) == Some("fill_container")
}

fn numeric(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn is_ignorable_chip_row_spacer(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("frame")
        && (v.get("role").and_then(Value::as_str) == Some("spacer")
            || v.get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case("spacer")))
        && numeric(v, "height").is_some_and(|h| h <= 2.0)
        && children(v).is_empty()
        && !has_paint(v)
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn has_paint(v: &Value) -> bool {
    v.get("fill").is_some_and(|fill| !fill.is_null())
        || v.get("stroke").is_some_and(|stroke| !stroke.is_null())
}

/// Adopt a CORNER BADGE into the image wrapper it was meant to ride.
///
/// Models author a deal card's discount tag as a card-level FLOW sibling —
/// `[Image Wrapper, "-35%" badge, Details]` — intending an on-image corner
/// overlay but omitting both the nesting and the x/y (measured: the badge
/// rendered straddling the image/content seam). A small painted short-text
/// badge sitting IMMEDIATELY after an image-bearing frame in a horizontal
/// card is that intent by construction: move it inside the wrapper at an
/// 8,8 corner inset (authored x/y = absolute placement in the engine).
pub(crate) fn adopt_corner_badges(root: &mut jian_ops_schema::node::PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !adopt_in_value(&mut v) {
        return false;
    }
    match serde_json::from_value::<jian_ops_schema::node::PenNode>(v) {
        Ok(new_node) => {
            *root = new_node;
            true
        }
        Err(_) => false,
    }
}

fn adopt_in_value(v: &mut Value) -> bool {
    let mut changed = false;
    if v.get("layout").and_then(Value::as_str) == Some("horizontal") {
        let adopt_at = {
            let kids = children(v);
            kids.windows(2)
                .enumerate()
                .find_map(|(i, w)| (is_image_wrapper(&w[0]) && is_corner_badge(&w[1])).then_some(i))
        };
        if let Some(i) = adopt_at {
            if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
                let mut badge = kids.remove(i + 1);
                if let Some(obj) = badge.as_object_mut() {
                    obj.insert("x".into(), serde_json::json!(8.0));
                    obj.insert("y".into(), serde_json::json!(8.0));
                }
                if let Some(wrapper_kids) =
                    kids[i].get_mut("children").and_then(Value::as_array_mut)
                {
                    wrapper_kids.push(badge);
                    changed = true;
                }
            }
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= adopt_in_value(c);
        }
    }
    changed
}

/// A frame that carries an image (directly or one level down) — the overlay
/// surface a corner badge belongs to. Bare `image` nodes can't take children,
/// so only FRAME wrappers qualify.
fn is_image_wrapper(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("frame")
        && children(v)
            .iter()
            .any(|c| c.get("type").and_then(Value::as_str) == Some("image"))
}

/// A small painted chip with one SHORT text child and no authored position —
/// "-35%", "NEW", "HOT". Anything positioned (x/y) is already placed on
/// purpose; anything wordy is content, not a corner tag.
fn is_corner_badge(v: &Value) -> bool {
    if v.get("type").and_then(Value::as_str) != Some("frame") {
        return false;
    }
    if v.get("x").is_some() || v.get("y").is_some() {
        return false;
    }
    if !has_paint(v) {
        return false;
    }
    let kids = children(v);
    kids.len() == 1
        && kids[0].get("type").and_then(Value::as_str) == Some("text")
        && kids[0]
            .get("content")
            .and_then(Value::as_str)
            .map(str::trim)
            .is_some_and(|c| !c.is_empty() && c.chars().count() <= 6)
}

/// Adopt a bell/notification DOT onto its icon's corner.
///
/// Models author `[icon_font, 8px painted square frame]` as FLOW siblings in
/// a button — intending the classic notification dot pinned on the bell's
/// top-right, but omitting both the radius and the x/y (measured: a green
/// SQUARE beside the bell). A tiny childless painted square immediately
/// after an icon is that intent by construction: round it to a circle and
/// pin it absolutely at the icon's top-right corner.
pub(crate) fn adopt_notification_dots(root: &mut jian_ops_schema::node::PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !adopt_dots_in_value(&mut v) {
        return false;
    }
    match serde_json::from_value::<jian_ops_schema::node::PenNode>(v) {
        Ok(new_node) => {
            *root = new_node;
            true
        }
        Err(_) => false,
    }
}

fn adopt_dots_in_value(v: &mut Value) -> bool {
    let mut changed = false;
    let hit = {
        let kids = children(v);
        kids.windows(2).enumerate().find_map(|(i, w)| {
            let icon_w = numeric(&w[0], "width")?;
            (w[0].get("type").and_then(Value::as_str) == Some("icon_font")
                && is_notification_dot(&w[1]))
            .then_some((i, icon_w))
        })
    };
    if let Some((i, icon_w)) = hit {
        if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
            if let Some(obj) = kids[i + 1].as_object_mut() {
                let dot_w = obj.get("width").and_then(Value::as_f64).unwrap_or(8.0);
                obj.insert("cornerRadius".into(), serde_json::json!(999.0));
                // Pin at the icon's top-right corner: the icon is the first
                // flow child at the content origin, so the dot's absolute
                // x/y are measured from there.
                obj.insert("x".into(), serde_json::json!(icon_w - dot_w / 2.0));
                obj.insert("y".into(), serde_json::json!(0.0));
                changed = true;
            }
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= adopt_dots_in_value(c);
        }
    }
    changed
}

/// A ≤12px painted childless square frame or ellipse with no authored
/// position — the notification-dot shape.
fn is_notification_dot(v: &Value) -> bool {
    if !matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "ellipse")
    ) {
        return false;
    }
    if v.get("x").is_some() || v.get("y").is_some() {
        return false;
    }
    let (Some(w), Some(h)) = (numeric(v, "width"), numeric(v, "height")) else {
        return false;
    };
    w <= 12.0 && (w - h).abs() < 0.5 && has_paint(v) && children(v).is_empty()
}
