use openpencil_shell_core::{
    FocusEvent, ImeEvent, ImeKind, KeyCode, KeyEvent, KeyLocation, KeyState, KeyValue, Modifiers,
    MouseButtons, NamedKey, PointerEvent, PointerId, PointerKind, PointerPhase, ScrollMode,
    WheelEvent,
};

#[test]
fn pointer_event_is_re_exported_from_jian() {
    let event = PointerEvent::simple(
        7,
        PointerPhase::Down,
        jian_core::geometry::Point::new(12.0, 34.0),
    );

    let _: PointerEvent = event.clone();
    let _: jian_core::gesture::PointerEvent = event.clone();

    assert_eq!(event.id, PointerId(7));
    assert_eq!(event.kind, PointerKind::Touch);
    assert_eq!(event.phase, PointerPhase::Down);
    assert_eq!(event.position.x, 12.0);
    assert_eq!(event.position.y, 34.0);
    assert!(event.buttons.contains(MouseButtons::LEFT));
    assert!(event.modifiers.is_empty());
}

#[test]
fn pointer_modifier_and_button_flags_keep_jian_names() {
    let mods = Modifiers::SHIFT | Modifiers::CMD;
    assert!(mods.contains(Modifiers::SHIFT));
    assert!(mods.contains(Modifiers::CMD));
    assert!(!mods.contains(Modifiers::CTRL));

    let buttons = MouseButtons::LEFT | MouseButtons::RIGHT;
    assert!(buttons.contains(MouseButtons::LEFT));
    assert!(buttons.contains(MouseButtons::RIGHT));
    assert!(!buttons.contains(MouseButtons::MIDDLE));
}

#[test]
fn key_event_is_re_exported_from_jian_with_all_w3c_fields() {
    let event = KeyEvent {
        key: KeyValue::Named(NamedKey::Enter),
        code: KeyCode::Enter,
        location: KeyLocation::Right,
        modifiers: Modifiers::SHIFT,
        state: KeyState::Pressed,
        repeat: true,
        is_composing: true,
    };
    let _: jian_core::gesture::KeyEvent = event.clone();
    // Round 2 Q5 fix: assert every W3C field reads back the value we set
    // so cross-crate type identity AND field-level binary compat are
    // both verified through the OP re-export path.
    assert_eq!(event.key, KeyValue::Named(NamedKey::Enter));
    assert_eq!(event.code, KeyCode::Enter);
    assert_eq!(event.location, KeyLocation::Right);
    assert!(event.modifiers.contains(Modifiers::SHIFT));
    assert_eq!(event.state, KeyState::Pressed);
    assert!(event.repeat);
    assert!(event.is_composing);
}

#[test]
fn ime_event_is_re_exported_from_jian() {
    let event = ImeEvent {
        kind: ImeKind::CompositionUpdate {
            selection: Some(0..6),
        },
        text: "你好".to_string(),
    };
    let _: jian_core::gesture::ImeEvent = event.clone();
    assert_eq!(event.text, "你好");
    match event.kind {
        ImeKind::CompositionUpdate { selection } => assert_eq!(selection, Some(0..6)),
        _ => panic!("expected CompositionUpdate"),
    }
}

#[test]
fn focus_event_is_re_exported_from_jian_with_all_w3c_fields() {
    let event = FocusEvent {
        gained: false,
        node_id_hint: Some(11),
        related_node_id_hint: Some(7),
    };
    let _: jian_core::gesture::FocusEvent = event;
    // Round 2 Q5 fix: assert all three W3C fields, not just gained.
    assert!(!event.gained);
    assert_eq!(event.node_id_hint, Some(11));
    assert_eq!(event.related_node_id_hint, Some(7));
}

#[test]
fn wheel_event_is_re_exported_from_jian_with_w3c_fields() {
    let mut event = WheelEvent::simple(
        jian_core::geometry::Point::new(10.0, 20.0),
        jian_core::geometry::Point::new(0.0, 120.0),
    );
    // Defaults from WheelEvent::simple
    assert_eq!(event.mode, ScrollMode::Pixel);
    assert_eq!(event.delta_z, 0.0);
    // Round 2 Q5 fix: assert mode + delta_z mutability + roundtrip
    // through the OP re-export path matches Jian's behavior.
    event.mode = ScrollMode::Line;
    event.delta_z = -3.0;
    assert_eq!(event.mode, ScrollMode::Line);
    assert_eq!(event.delta_z, -3.0);
    assert_eq!(event.delta.x, 0.0);
    assert_eq!(event.delta.y, 120.0);
}
