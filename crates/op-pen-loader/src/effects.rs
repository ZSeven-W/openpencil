//! Node-effects (drop shadow) round-trip for the desktop's private
//! `DocPayload` save format. Carved off `persistence.rs` to keep
//! that file under the 800-line cap.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenEffect;
use openpencil_shell_core::layout_scene::{DropShadow, Effect};
use serde::{Deserialize, Serialize};

/// Serializable mirror of `document::DropShadow`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShadowPayload {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: [f32; 4],
}

/// Serialize a node's `effects` into payload form.
pub fn effects_to_payload(effects: &[Effect]) -> Vec<ShadowPayload> {
    effects
        .iter()
        .map(|e| {
            let Effect::DropShadow(s) = e;
            ShadowPayload {
                offset_x: s.offset_x,
                offset_y: s.offset_y,
                blur: s.blur,
                color: [s.color.r, s.color.g, s.color.b, s.color.a],
            }
        })
        .collect()
}

/// Rebuild a node's `effects` from payload form.
pub fn effects_from_payload(payload: Vec<ShadowPayload>) -> Vec<Effect> {
    payload.iter().map(shadow_payload_to_effect).collect()
}

/// Borrowing variant of [`effects_from_payload`] — rebuilds `effects`
/// from a payload slice without consuming it. Used by the
/// `LayoutScene` builder, which reads `NodePayload.effects` by
/// reference.
pub fn effects_from_payload_ref(payload: &[ShadowPayload]) -> Vec<Effect> {
    payload.iter().map(shadow_payload_to_effect).collect()
}

fn shadow_payload_to_effect(s: &ShadowPayload) -> Effect {
    Effect::DropShadow(DropShadow {
        offset_x: s.offset_x,
        offset_y: s.offset_y,
        blur: s.blur,
        color: openpencil_shell_core::Color {
            r: s.color[0],
            g: s.color[1],
            b: s.color[2],
            a: s.color[3],
        },
    })
}

/// Per-variant accessor for a canonical node's `effects` list.
/// Frame / Group / Rectangle carry effects on their `container`
/// style; the leaf shapes carry it directly. IconFont / Ref have
/// none.
fn canonical_node_effects(node: &PenNode) -> Option<&[PenEffect]> {
    match node {
        PenNode::Frame(n) => n.container.effects.as_deref(),
        PenNode::Group(n) => n.container.effects.as_deref(),
        PenNode::Rectangle(n) => n.container.effects.as_deref(),
        PenNode::Ellipse(n) => n.effects.as_deref(),
        PenNode::Line(n) => n.effects.as_deref(),
        PenNode::Polygon(n) => n.effects.as_deref(),
        PenNode::Path(n) => n.effects.as_deref(),
        PenNode::Text(n) => n.effects.as_deref(),
        PenNode::TextInput(n) => n.effects.as_deref(),
        PenNode::Image(n) => n.effects.as_deref(),
        PenNode::IconFont(_) | PenNode::Ref(_) => None,
    }
}

/// Convert a canonical node's `PenEffect` list into desktop payload
/// `ShadowPayload`s. Only `PenEffect::Shadow` maps — blur variants
/// are dropped (no renderer path yet).
pub fn shadows_from_canonical(node: &PenNode) -> Vec<ShadowPayload> {
    let Some(effects) = canonical_node_effects(node) else {
        return Vec::new();
    };
    effects
        .iter()
        .filter_map(|e| match e {
            // `parse_css_color` handles hex AND `rgba()` — canonical
            // shadow colours use the functional form, which plain
            // hex parsing rejected (codex stop-gate: rgba shadows
            // were first imported as opaque black, then dropped).
            // Genuinely unparseable colours still drop the shadow
            // rather than fabricate a wrong one.
            PenEffect::Shadow(s) => {
                parse_css_color(&s.color).map(|color| ShadowPayload {
                    offset_x: s.offset_x,
                    offset_y: s.offset_y,
                    blur: s.blur,
                    color,
                })
            }
            PenEffect::Blur(_) | PenEffect::BackgroundBlur(_) => None,
        })
        .collect()
}

/// Parse a CSS colour string into `[r,g,b,a]` 0..1. Handles hex
/// (`#rgb` / `#rrggbb` / `#rrggbbaa`) AND the functional
/// `rgb(r,g,b)` / `rgba(r,g,b,a)` form (`r`/`g`/`b` 0-255, `a`
/// 0..1) that canonical `.op` shadow colours use. Returns `None`
/// only for genuinely unparseable input.
fn parse_css_color(s: &str) -> Option<[f32; 4]> {
    let t = s.trim();
    if let Some(rgba) = parse_hex(t) {
        return Some(rgba);
    }
    let lower = t.to_ascii_lowercase();
    let inner = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let r = parts[0].parse::<f32>().ok()?;
    let g = parts[1].parse::<f32>().ok()?;
    let b = parts[2].parse::<f32>().ok()?;
    let a = if parts.len() >= 4 {
        parts[3].parse::<f32>().ok()?
    } else {
        1.0
    };
    Some([
        (r / 255.0).clamp(0.0, 1.0),
        (g / 255.0).clamp(0.0, 1.0),
        (b / 255.0).clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    ])
}

fn parse_hex(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    let (r, g, b, a) = match s.len() {
        3 => (
            u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?,
            255u8,
        ),
        6 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            255u8,
        ),
        8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            u8::from_str_radix(&s[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some([
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpencil_shell_core::Color;

    #[test]
    fn parse_css_color_handles_rgba_and_hex() {
        // Functional rgba() — canonical shadow form.
        let rgba = parse_css_color("rgba(0, 0, 0, 0.5)").expect("rgba");
        assert_eq!(rgba, [0.0, 0.0, 0.0, 0.5]);
        // rgb() defaults alpha to 1.
        let rgb = parse_css_color("rgb(255, 128, 0)").expect("rgb");
        assert!((rgb[0] - 1.0).abs() < 1e-6);
        assert!((rgb[3] - 1.0).abs() < 1e-6);
        // Hex still works.
        assert_eq!(parse_css_color("#000000"), Some([0.0, 0.0, 0.0, 1.0]));
        // Genuinely unparseable input is rejected, not faked.
        assert_eq!(parse_css_color("not-a-colour"), None);
    }

    #[test]
    fn effects_survive_payload_round_trip() {
        let original = vec![
            Effect::DropShadow(DropShadow {
                offset_x: 4.0,
                offset_y: 6.0,
                blur: 12.0,
                color: Color { r: 0.0, g: 0.0, b: 0.0, a: 0.5 },
            }),
            Effect::DropShadow(DropShadow {
                offset_x: -2.0,
                offset_y: 0.0,
                blur: 3.0,
                color: Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 },
            }),
        ];
        // Through the actual serde JSON path the `.op` file uses.
        let payload = effects_to_payload(&original);
        let json = serde_json::to_string(&payload).expect("serialize");
        let back: Vec<ShadowPayload> =
            serde_json::from_str(&json).expect("deserialize");
        let restored = effects_from_payload(back);
        assert_eq!(restored, original);
    }

    #[test]
    fn empty_effects_round_trip_to_empty() {
        let payload = effects_to_payload(&[]);
        assert!(payload.is_empty());
        assert!(effects_from_payload(payload).is_empty());
    }
}
