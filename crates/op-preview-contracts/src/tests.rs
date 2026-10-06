//! Contract unit tests — the frozen DTO invariants hosts build against.

#![cfg(test)]

use crate::activation::UserActivationId;
use crate::capability::{PreviewCapability, PreviewHostCapabilities};
use crate::effect::{
    EffectSource, HapticStyle, PreviewEffect, PreviewEffectFailure, PreviewEffectFailureCode,
    PreviewEffectResult, SharePayload,
};
use crate::platform_support::{platform_support, HostSupport, PreviewInteraction, PreviewPlatform};

fn source(capability: PreviewCapability) -> EffectSource {
    EffectSource {
        node_id: "btn-1".to_owned(),
        event: "onTap".to_owned(),
        activation: Some(UserActivationId::from_raw(7)),
        required_capability: capability,
    }
}

/// Absence is fail-closed: `none()` denies everything, so the legacy
/// `enter` wrapper can never read as consent.
#[test]
fn no_capabilities_deny_everything() {
    let caps = PreviewHostCapabilities::none();
    for capability in [
        PreviewCapability::OpenUrl,
        PreviewCapability::Clipboard,
        PreviewCapability::Share,
        PreviewCapability::Haptics,
        PreviewCapability::DismissKeyboard,
        PreviewCapability::Notifications,
        PreviewCapability::Focus,
    ] {
        assert!(!caps.supports(capability), "{capability:?} must be denied");
    }
}

/// A declared host capability maps onto its effect classes.
#[test]
fn declared_capability_satisfies_its_effects() {
    let caps = PreviewHostCapabilities {
        clipboard: true,
        ..PreviewHostCapabilities::none()
    };
    assert!(caps.supports(PreviewCapability::Clipboard));
    assert!(!caps.supports(PreviewCapability::Share));
}

/// Effects carry their id, class name, capability, and source; ids are
/// host-opaque handles.
#[test]
fn effect_metadata_is_self_describing() {
    let effect = PreviewEffect::Haptic {
        id: 3,
        style: HapticStyle::Success,
        source: source(PreviewCapability::Haptics),
    };
    assert_eq!(effect.id(), 3);
    assert_eq!(effect.kind(), "haptic");
    assert_eq!(effect.required_capability(), PreviewCapability::Haptics);
    assert_eq!(
        effect.source().activation,
        Some(UserActivationId::from_raw(7))
    );
}

/// The activation id is an opaque handle with a stable raw form for FFI.
#[test]
fn activation_id_round_trips_raw() {
    let id = UserActivationId::from_raw(9001);
    assert_eq!(id.raw(), 9001);
    assert_eq!(
        serde_json::to_value(id).unwrap(),
        serde_json::json!(9001),
        "transparent serde for host marshalling"
    );
}

/// Effect results serialize stably for host → runtime reporting.
#[test]
fn effect_result_round_trip() {
    let result = PreviewEffectResult::Failed(PreviewEffectFailure {
        code: PreviewEffectFailureCode::InvalidUrlScheme,
        detail: Some("ftp://".to_owned()),
    });
    let back: PreviewEffectResult =
        serde_json::from_value(serde_json::to_value(&result).unwrap()).unwrap();
    assert_eq!(back, result);
}

/// The approved adaptations: Hover and ContextMenu are ADAPTED (touch
/// fallbacks); everything the table lists is Complete on both platforms.
#[test]
fn platform_support_table_matches_the_contract() {
    for platform in [PreviewPlatform::Native, PreviewPlatform::Web] {
        assert_eq!(
            platform_support(platform, PreviewInteraction::Hover),
            HostSupport::Adapted,
            "Hover falls back to Pressed/Focus on touch"
        );
        assert_eq!(
            platform_support(platform, PreviewInteraction::ContextMenu),
            HostSupport::Adapted,
            "ContextMenu falls back to LongPress on touch"
        );
        for complete in [
            PreviewInteraction::Tap,
            PreviewInteraction::DoubleTap,
            PreviewInteraction::LongPress,
            PreviewInteraction::Press,
            PreviewInteraction::Pan,
            PreviewInteraction::Swipe,
            PreviewInteraction::Scale,
            PreviewInteraction::Rotate,
            PreviewInteraction::Key,
            PreviewInteraction::TextInput,
            PreviewInteraction::Ime,
            PreviewInteraction::Scroll,
            PreviewInteraction::Back,
        ] {
            assert_eq!(
                platform_support(platform, complete),
                HostSupport::Complete,
                "{complete:?} is Complete on {platform:?}"
            );
        }
    }
}

/// Share payloads keep absent fields absent (never guessed).
#[test]
fn share_payload_preserves_absent_fields() {
    let payload = SharePayload {
        text: Some("hi".to_owned()),
        ..SharePayload::default()
    };
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json, serde_json::json!({"text": "hi"}));
}
