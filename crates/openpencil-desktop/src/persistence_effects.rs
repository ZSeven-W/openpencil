//! Node-effects (drop shadow) round-trip for the desktop's private
//! `DocPayload` save format. Carved off `persistence.rs` to keep
//! that file under the 800-line cap.

use openpencil_shell_core::document::{DropShadow, Effect};
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
    payload
        .into_iter()
        .map(|s| {
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
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpencil_shell_core::Color;

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
