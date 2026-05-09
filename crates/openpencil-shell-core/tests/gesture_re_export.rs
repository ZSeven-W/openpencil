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
fn key_event_is_re_exported_from_jian() {
    let event = KeyEvent {
        key: KeyValue::Named(NamedKey::Enter),
        code: KeyCode::Enter,
        location: KeyLocation::Standard,
        modifiers: Modifiers::empty(),
        state: KeyState::Pressed,
        repeat: false,
        is_composing: false,
    };
    let _: jian_core::gesture::KeyEvent = event.clone();
    assert_eq!(event.key, KeyValue::Named(NamedKey::Enter));
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
    match event.kind {
        ImeKind::CompositionUpdate { selection } => assert_eq!(selection, Some(0..6)),
        _ => panic!("expected CompositionUpdate"),
    }
}

#[test]
fn focus_event_is_re_exported_from_jian() {
    let event = FocusEvent {
        gained: true,
        node_id_hint: Some(11),
        related_node_id_hint: Some(7),
    };
    let _: jian_core::gesture::FocusEvent = event;
    assert!(event.gained);
}

#[test]
fn wheel_event_is_re_exported_from_jian() {
    let event = WheelEvent::simple(
        jian_core::geometry::Point::new(0.0, 0.0),
        jian_core::geometry::Point::new(0.0, 120.0),
    );
    assert_eq!(event.mode, ScrollMode::Pixel);
    assert_eq!(event.delta_z, 0.0);
}
