//! Phase B1 + B2 widget facade smoke tests.
//!
//! B1 piece: proves the `Widget` trait + `PaintCx` / `LayoutCx` shape
//! compiles + that a recording backend plugs in via `&mut dyn
//! RenderBackend`. B2 piece: proves the four inspector widgets (Tree /
//! PropertyRow / Dropdown / TextInput) paint and emit accesskit nodes
//! with the expected semantic roles.

use openpencil_shell_core::widgets::{
    rect, Dropdown, DropdownState, LayoutBox, LayoutCx, PaintCx, PropertyRow, TextInput,
    TextInputState, TreeWidget, Widget, WidgetId, ROOT_WIDGET_ID,
};
use openpencil_shell_core::{
    Color, ImeEvent, ImeKind, KeyCode, KeyEvent, KeyLocation, KeyState, KeyValue, Modifiers,
    NamedKey, Point2D, Rect, RenderBackend, TextLayout,
};

#[derive(Default)]
struct RecordingBackend {
    rects: usize,
    strokes: usize,
    text: usize,
    saves: usize,
    restores: usize,
    clips: usize,
    translates: usize,
}

impl RenderBackend for RecordingBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _rect: Rect, _color: Color) {
        self.rects += 1;
    }
    fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {
        self.strokes += 1;
    }
    fn draw_text(&mut self, _layout: &TextLayout, _origin: Point2D) {
        self.text += 1;
    }
    fn clip_rect(&mut self, _rect: Rect) {
        self.clips += 1;
    }
    fn save(&mut self) {
        self.saves += 1;
    }
    fn restore(&mut self) {
        self.restores += 1;
    }
    fn translate(&mut self, _offset: Point2D) {
        self.translates += 1;
    }
    fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {
        // Counted alongside strokes for the existing test asserts;
        // a separate counter is unnecessary for Step 4 visual lift.
        self.strokes += 1;
    }
    fn fill_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color) {
        self.rects += 1;
    }
    fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {
        self.strokes += 1;
    }
    fn stroke_svg_path(
        &mut self,
        _d: &str,
        _top_left: Point2D,
        _size: f32,
        _color: Color,
        _width: f32,
    ) {
        self.strokes += 1;
    }
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn paint_cx_dispatches_through_dyn_backend() {
    let mut backend = RecordingBackend::default();
    {
        let cx = PaintCx {
            backend: &mut backend,
        };
        cx.backend.fill_rect(rect(0.0, 0.0, 10.0, 10.0), Color::RED);
        cx.backend
            .stroke_rect(rect(1.0, 2.0, 8.0, 8.0), Color::WHITE, 1.5);
        cx.backend.save();
        cx.backend.translate(Point2D::new(2.0, 3.0));
        cx.backend.clip_rect(rect(0.0, 0.0, 4.0, 4.0));
        cx.backend.restore();
    }
    assert_eq!(backend.rects, 1, "fill_rect dispatch");
    assert_eq!(backend.strokes, 1, "stroke_rect dispatch");
    assert_eq!(backend.saves, 1, "save dispatch");
    assert_eq!(backend.restores, 1, "restore dispatch");
    assert_eq!(backend.translates, 1, "translate dispatch");
    assert_eq!(backend.clips, 1, "clip_rect dispatch");
}

/// A trivial widget impl proves the trait shape. Uses
/// `accesskit::Role::GenericContainer` (canonical "intentional placeholder",
/// ARIA `none`/`presentation`) so the test stays stable across unrelated
/// accesskit version bumps; B2 widgets use semantic roles.
struct StubWidget {
    id: WidgetId,
    box_rect: Rect,
}

impl Widget for StubWidget {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: self.box_rect,
        }
    }
    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, Color::WHITE);
    }
    fn access_node(&self) -> accesskit::Node {
        // `GenericContainer` (ARIA `none` / `presentation`) is the canonical
        // "intentional placeholder" role. Real B2 widgets use semantic
        // roles (TreeItem / EditableText / etc); see codex B1 review NIT-5.
        accesskit::Node::new(accesskit::Role::GenericContainer)
    }
}

#[test]
fn widget_trait_dispatches_layout_and_paint() {
    let widget = StubWidget {
        id: WidgetId::new(7),
        box_rect: rect(0.0, 0.0, 100.0, 24.0),
    };
    let layout_cx = LayoutCx {
        available_width: 320.0,
        dpi: 1.0,
    };
    let layout = widget.layout(&layout_cx);
    assert_eq!(layout.rect.size.x, 100.0);

    let mut backend = RecordingBackend::default();
    {
        let mut paint_cx = PaintCx {
            backend: &mut backend,
        };
        widget.paint(&mut paint_cx, layout.rect);
    }
    assert_eq!(backend.rects, 1);
    assert_eq!(widget.id(), WidgetId::new(7));
    // Sanity check the root id constant survives codegen.
    assert_eq!(ROOT_WIDGET_ID.0, 0);

    // Trait surface check: access_node returns the placeholder
    // `Role::GenericContainer` advertised by `StubWidget`. Real B2 widgets
    // assert their semantic roles in the tests below.
    let node = widget.access_node();
    assert_eq!(node.role(), accesskit::Role::GenericContainer);
}

// ---------------------------------------------------------------------
// B2: four inspector widgets paint static content + expose semantic
// accesskit roles.
// ---------------------------------------------------------------------

#[test]
fn four_inspector_widgets_paint_static_content() {
    let layout = LayoutCx {
        available_width: 240.0,
        dpi: 1.0,
    };
    let widgets: Vec<Box<dyn Widget>> = vec![
        Box::new(TreeWidget::sample()),
        Box::new(PropertyRow::new(200, "Width", "960")),
        Box::new(Dropdown::sample()),
        Box::new(TextInput::sample()),
    ];
    let mut backend = RecordingBackend::default();
    for widget in widgets {
        let box_ = widget.layout(&layout);
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        widget.paint(&mut cx, box_.rect);
    }

    // Each widget paints at least its background fill (4); Tree adds one
    // more for the selected row → ≥ 5. Stroked rects: PropertyRow,
    // Dropdown, TextInput each stroke their border (3). Text runs:
    // PropertyRow has 2 (label+value); Tree has 3 items; Dropdown has 1;
    // TextInput has 1 → ≥ 7 total.
    assert!(
        backend.rects >= 5,
        "fill_rect dispatch ≥ 5 (got {})",
        backend.rects
    );
    assert!(
        backend.strokes >= 3,
        "stroke_rect dispatch ≥ 3 (got {})",
        backend.strokes
    );
    assert!(
        backend.text >= 7,
        "draw_text dispatch ≥ 7 (got {})",
        backend.text
    );
}

#[test]
fn tree_widget_advertises_tree_role_and_layers_label() {
    let tree = TreeWidget::sample();
    let node = tree.access_node();
    assert_eq!(node.role(), accesskit::Role::Tree);
    // accesskit::Node exposes label() returning Option<&str> in 0.24.
    assert_eq!(node.label(), Some("Layers"));
    // Sample tree has 3 items.
    assert_eq!(tree.items.len(), 3);
    assert!(tree.items.iter().any(|item| item.selected));
}

#[test]
fn property_row_advertises_label_and_value() {
    let row = PropertyRow::new(201, "Width", "960");
    let node = row.access_node();
    // `Role::Group` (not GenericContainer) so the label survives ARIA
    // filtering — see codex B2 R1 CONCERN + the fix in prop_row.rs.
    assert_eq!(node.role(), accesskit::Role::Group);
    assert_eq!(node.label(), Some("Width 960"));
}

#[test]
fn dropdown_advertises_combobox_role() {
    let drop = Dropdown::sample();
    let node = drop.access_node();
    assert_eq!(node.role(), accesskit::Role::ComboBox);
    assert_eq!(node.label(), Some("Blend"));
    // Sample preserves the closed/first-selected state.
    assert_eq!(drop.state.selected, 0);
    assert!(!drop.state.open);
}

#[test]
fn text_input_advertises_text_input_role_and_value() {
    let input = TextInput::sample();
    let node = input.access_node();
    assert_eq!(node.role(), accesskit::Role::TextInput);
    assert_eq!(node.label(), Some("Name"));
    assert_eq!(node.value(), Some("Frame 1"));
}

#[test]
fn text_input_paints_preedit_underline_when_composing() {
    // The preedit-underline painting branch only fires when
    // `state.preedit` is non-empty; verify it via the recording backend.
    let mut input = TextInput::sample();
    input.state.preedit = "你好".to_string();
    let layout_cx = LayoutCx {
        available_width: 240.0,
        dpi: 1.0,
    };
    let layout = input.layout(&layout_cx);
    let mut backend = RecordingBackend::default();
    {
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        input.paint(&mut cx, layout.rect);
    }
    // 1 fill (background) + 2 strokes (border + preedit underline) +
    // 1 text run (preedit content).
    assert_eq!(backend.rects, 1);
    assert_eq!(backend.strokes, 2);
    assert_eq!(backend.text, 1);
}

#[test]
fn dropdown_state_independent_state_struct() {
    // DropdownState lives on its own so input handling can swap it
    // without taking ownership of the surrounding Dropdown widget.
    let s = DropdownState {
        selected: 2,
        open: true,
    };
    assert_eq!(s.selected, 2);
    assert!(s.open);
}

#[test]
fn text_input_state_default_is_empty() {
    let s = TextInputState::default();
    assert_eq!(s.value, "");
    assert_eq!(s.preedit, "");
}

// ---------------------------------------------------------------------
// C2 widget event handlers (apply_ime / apply_key)
// ---------------------------------------------------------------------

fn keydown(named: NamedKey) -> KeyEvent {
    KeyEvent {
        key: KeyValue::Named(named),
        code: KeyCode::Unknown(String::new()),
        location: KeyLocation::Standard,
        modifiers: Modifiers::empty(),
        state: KeyState::Pressed,
        repeat: false,
        is_composing: false,
    }
}

fn keyup(named: NamedKey) -> KeyEvent {
    KeyEvent {
        state: KeyState::Released,
        ..keydown(named)
    }
}

#[test]
fn text_input_apply_ime_start_clears_preedit() {
    let mut state = TextInputState {
        value: "Frame 1".into(),
        preedit: "stale".into(),
    };
    state.apply_ime(&ImeEvent {
        kind: ImeKind::CompositionStart,
        text: String::new(),
    });
    assert_eq!(state.preedit, "");
    // CompositionStart must NOT touch the committed value.
    assert_eq!(state.value, "Frame 1");
}

#[test]
fn text_input_apply_ime_update_replaces_preedit() {
    let mut state = TextInputState {
        value: "Frame ".into(),
        preedit: String::new(),
    };
    state.apply_ime(&ImeEvent {
        kind: ImeKind::CompositionUpdate { selection: None },
        text: "你".into(),
    });
    assert_eq!(state.preedit, "你");
    state.apply_ime(&ImeEvent {
        kind: ImeKind::CompositionUpdate { selection: None },
        text: "你好".into(),
    });
    assert_eq!(state.preedit, "你好");
    // Update path NEVER mutates value.
    assert_eq!(state.value, "Frame ");
}

#[test]
fn text_input_apply_ime_end_appends_to_value_and_clears_preedit() {
    let mut state = TextInputState {
        value: "Frame ".into(),
        preedit: "你好".into(),
    };
    state.apply_ime(&ImeEvent {
        kind: ImeKind::CompositionEnd,
        text: "你好".into(),
    });
    assert_eq!(state.value, "Frame 你好");
    assert_eq!(state.preedit, "");
}

#[test]
fn text_input_apply_ime_double_start_clears_preedit_each_time() {
    // Pathological host state machine: a CompositionStart followed by
    // another CompositionStart without an intervening End. Each Start
    // unconditionally resets preedit; the committed value never moves.
    // (Codex C2.1 R1 CONCERN-1.)
    let mut state = TextInputState {
        value: "Frame ".into(),
        preedit: "old".into(),
    };
    state.apply_ime(&ImeEvent {
        kind: ImeKind::CompositionStart,
        text: String::new(),
    });
    assert_eq!(state.preedit, "");
    state.preedit = "leftover".into();
    state.apply_ime(&ImeEvent {
        kind: ImeKind::CompositionStart,
        text: String::new(),
    });
    assert_eq!(state.preedit, "");
    assert_eq!(state.value, "Frame ");
}

#[test]
fn dropdown_apply_key_arrow_down_advances_and_opens() {
    let mut state = DropdownState {
        selected: 0,
        open: false,
    };
    state.apply_key(&keydown(NamedKey::ArrowDown), 3);
    assert_eq!(state.selected, 1);
    assert!(state.open);
    state.apply_key(&keydown(NamedKey::ArrowDown), 3);
    assert_eq!(state.selected, 2);
    // Saturates at option_count - 1; does not wrap.
    state.apply_key(&keydown(NamedKey::ArrowDown), 3);
    assert_eq!(state.selected, 2);
}

#[test]
fn dropdown_apply_key_arrow_up_retreats_with_saturating_sub() {
    let mut state = DropdownState {
        selected: 1,
        open: false,
    };
    state.apply_key(&keydown(NamedKey::ArrowUp), 3);
    assert_eq!(state.selected, 0);
    assert!(state.open);
    // Already at 0 — saturating_sub holds at 0, no panic.
    state.apply_key(&keydown(NamedKey::ArrowUp), 3);
    assert_eq!(state.selected, 0);
}

#[test]
fn dropdown_apply_key_enter_and_escape_close() {
    let mut state = DropdownState {
        selected: 1,
        open: true,
    };
    state.apply_key(&keydown(NamedKey::Enter), 3);
    assert!(!state.open);
    assert_eq!(state.selected, 1);

    state.open = true;
    state.apply_key(&keydown(NamedKey::Escape), 3);
    assert!(!state.open);
    // Escape closes the menu but does NOT mutate selection — confirm
    // both halves of the close path leave selected stable (codex C2.1
    // R1 CONCERN-2).
    assert_eq!(state.selected, 1);
}

#[test]
fn dropdown_apply_key_ignores_keyup() {
    let mut state = DropdownState {
        selected: 1,
        open: false,
    };
    state.apply_key(&keyup(NamedKey::ArrowDown), 3);
    // Released event is a no-op.
    assert_eq!(state.selected, 1);
    assert!(!state.open);
}

#[test]
fn dropdown_apply_key_ignores_zero_options() {
    let mut state = DropdownState {
        selected: 0,
        open: false,
    };
    state.apply_key(&keydown(NamedKey::ArrowDown), 0);
    // No options → no-op rather than panic on `option_count - 1`.
    assert_eq!(state.selected, 0);
    assert!(!state.open);
}

#[test]
fn dropdown_apply_key_ignores_unrelated_keys() {
    let mut state = DropdownState {
        selected: 1,
        open: true,
    };
    state.apply_key(&keydown(NamedKey::Tab), 3);
    // Tab is not bound; state should be unchanged.
    assert_eq!(state.selected, 1);
    assert!(state.open);
}

#[test]
fn dropdown_apply_key_ignores_keys_during_ime_composition() {
    // Spec §2.4 widget rule: skip key dispatch while is_composing is
    // true. ArrowDown during a CJK composition belongs to the IME's
    // candidate-picker UI, NOT to the dropdown selection cycle.
    // Codex Phase C stop-hook (#2): "focused IME textarea lets
    // composing keys mutate dropdown state."
    let mut state = DropdownState {
        selected: 0,
        open: false,
    };
    let composing_arrow = KeyEvent {
        is_composing: true,
        ..keydown(NamedKey::ArrowDown)
    };
    state.apply_key(&composing_arrow, 3);
    assert_eq!(state.selected, 0, "composing ArrowDown must not advance");
    assert!(!state.open, "composing ArrowDown must not open");

    // Same protection for Enter / Escape — IME 'commit' shouldn't
    // close the dropdown.
    let mut state2 = DropdownState {
        selected: 1,
        open: true,
    };
    let composing_enter = KeyEvent {
        is_composing: true,
        ..keydown(NamedKey::Enter)
    };
    state2.apply_key(&composing_enter, 3);
    assert!(state2.open, "composing Enter must not close");
}
