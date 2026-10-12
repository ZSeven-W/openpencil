//! `strip_nested_card_decoration` — drop a nested card's own shadow/radius
//! / border when an ancestor already paints one.

use super::*;

// ── strip_nested_card_decoration (port of strip-nested-card-decoration.ts) ────

pub(super) const KEEP_DECORATION_ROLES: &[&str] = &[
    "button",
    "icon-button",
    "fab",
    "tag",
    "chip",
    "badge",
    "status-badge",
    "pill",
    "input",
    "search-bar",
    "form-field",
    "textarea",
    "select",
    "combobox",
    "avatar",
    "avatar-stack",
    "switch",
    "checkbox",
    "radio",
    "toolbar",
    "segmented-control",
];
pub(super) const MEDIA_CLIP_ROLES: &[&str] = &[
    "image",
    "image-card",
    "image-placeholder",
    "video",
    "video-placeholder",
    "media",
    "media-thumbnail",
    "thumbnail",
    "cover",
    "cover-image",
    "gallery-item",
];

#[derive(Clone, Copy, Default)]
pub(super) struct DecoFlags {
    stroke: bool,
    corner: bool,
    shadow: bool,
}

pub(super) fn read_decoration(node: &Value) -> DecoFlags {
    let stroke = node
        .get("stroke")
        .and_then(|s| s.get("thickness"))
        .and_then(Value::as_f64)
        .map(|t| t > 0.0)
        .unwrap_or(false);
    let corner = match node.get("cornerRadius") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) > 0.0,
        Some(Value::Array(a)) => a.first().and_then(Value::as_f64).unwrap_or(0.0) > 0.0,
        _ => false,
    };
    let shadow = node
        .get("effects")
        .and_then(Value::as_array)
        .map(|e| {
            e.iter()
                .any(|x| x.get("type").and_then(Value::as_str) == Some("shadow"))
        })
        .unwrap_or(false);
    DecoFlags {
        stroke,
        corner,
        shadow,
    }
}

pub(super) fn role_lower(node: &Value) -> String {
    role_of(node).unwrap_or("").to_lowercase()
}

pub(super) fn is_role_protected(node: &Value) -> bool {
    KEEP_DECORATION_ROLES.contains(&role_lower(node).as_str())
}

pub(super) fn is_media_clipper(node: &Value) -> bool {
    let role = role_lower(node);
    if MEDIA_CLIP_ROLES.contains(&role.as_str()) {
        return true;
    }
    if node.get("clipContent").and_then(Value::as_bool) != Some(true) {
        return false;
    }
    children_of(node).iter().any(|c| {
        c.get("type").and_then(Value::as_str) == Some("image")
            || MEDIA_CLIP_ROLES.contains(&role_lower(c).as_str())
    })
}

/// A compact icon surface is a shape, not another content card. Its radius
/// defines the circle/squircle even when an outer panel already has corners.
fn is_icon_surface(node: &Value) -> bool {
    let size = |key| node.get(key).and_then(Value::as_f64);
    let (Some(w), Some(h)) = (size("width"), size("height")) else {
        return false;
    };
    let kids = children_of(node);
    w > 0.0
        && h > 0.0
        && w <= 80.0
        && h <= 80.0
        && kids.len() == 1
        && kids[0].get("type").and_then(Value::as_str) == Some("icon_font")
}

pub(super) fn strip_nested_card_decoration(node: &mut Value, ancestor: DecoFlags) {
    let is_frame = node.get("type").and_then(Value::as_str) == Some("frame");
    let mut next_ancestor = ancestor;
    if is_frame {
        let own = read_decoration(node);
        if !is_role_protected(node) {
            let media = is_media_clipper(node);
            let icon_surface = is_icon_surface(node);
            if let Some(obj) = node.as_object_mut() {
                if own.stroke && ancestor.stroke {
                    obj.remove("stroke");
                }
                if own.corner && ancestor.corner && !media && !icon_surface {
                    obj.remove("cornerRadius");
                }
                if own.shadow && ancestor.shadow {
                    obj.remove("effects");
                }
            }
        }
        // Accumulate THIS node's (original) decoration for descendants.
        next_ancestor = DecoFlags {
            stroke: ancestor.stroke || own.stroke,
            // A transparent, non-clipping wrapper paints no rounded surface.
            corner: ancestor.corner
                || (own.corner && (has_renderable_fill(node) || is_media_clipper(node))),
            shadow: ancestor.shadow || own.shadow,
        };
    }
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children.iter_mut() {
            strip_nested_card_decoration(child, next_ancestor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transparent_wrapper_does_not_flatten_category_circle() {
        let mut node = json!({"type":"frame","cornerRadius":8,"children":[{
            "type":"frame","width":52,"height":52,"cornerRadius":26,
            "fill":[{"type":"solid","color":"#FFF0E3"}],"children":[
                {"type":"icon_font","width":22,"height":22,"iconFontName":"coffee"}
            ]
        }]});
        strip_nested_card_decoration(&mut node, DecoFlags::default());
        assert_eq!(node["children"][0]["cornerRadius"], json!(26));
    }

    #[test]
    fn painted_card_keeps_compact_icon_surface_but_strips_nested_panel() {
        let mut node = json!({"type":"frame","role":"card","cornerRadius":12,
        "fill":[{"type":"solid","color":"#FFFFFF"}],"children":[
            {"type":"frame","width":44,"height":44,"cornerRadius":22,
             "fill":[{"type":"solid","color":"#FFF0E3"}],"children":[
                {"type":"icon_font","width":20,"height":20,"iconFontName":"coffee"}]},
            {"type":"frame","role":"card","width":300,"height":200,"cornerRadius":8,
             "fill":[{"type":"solid","color":"#FFFFFF"}],"children":[
                {"type":"text","content":"Nested panel"}]}
        ]});
        strip_nested_card_decoration(&mut node, DecoFlags::default());
        assert_eq!(node["children"][0]["cornerRadius"], json!(22));
        assert!(node["children"][1].get("cornerRadius").is_none());
    }
}
