//! Layout tests for the workspace's variants bar.

use super::*;

fn canvas() -> Rect {
    Rect::xywh(320.0, 108.0, 1120.0, 792.0)
}

#[test]
fn pills_sit_centred_along_the_canvas_floor_in_slot_order() {
    let labels = vec![
        (0, "方案 A · Zen Paper Light".to_string()),
        (1, "方案 B · Noir Elegant Dark".to_string()),
        (2, "方案 C · Pastel Soft Mobile Light".to_string()),
    ];
    let items = variant_bar_layout(canvas(), &labels, "用这个");
    assert_eq!(items.len(), 3);
    assert_eq!(
        items.iter().map(|item| item.index).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    let canvas = canvas();
    let left = items[0].pill.origin.x - canvas.origin.x;
    let last = &items[2].pill;
    let right = canvas.origin.x + canvas.size.x - (last.origin.x + last.size.x);
    assert!((left - right).abs() < 1.0, "centred: {left} vs {right}");
    for item in &items {
        assert_eq!(
            item.label, labels[item.index].1,
            "wide canvas: no shortening"
        );
        assert!(item.pill.origin.y + item.pill.size.y <= canvas.origin.y + canvas.size.y);
        // The button sits inside its own pill.
        assert!(item.button.origin.x > item.pill.origin.x);
        assert!(item.button.origin.x + item.button.size.x <= item.pill.origin.x + item.pill.size.x);
    }
}

#[test]
fn a_narrow_canvas_shortens_labels_instead_of_overflowing() {
    let long = "方案 A · A Very Long Imported Brand Guide Name That Goes On".to_string();
    let labels: Vec<(usize, String)> = (0..4).map(|index| (index, long.clone())).collect();
    let narrow = Rect::xywh(0.0, 0.0, 700.0, 500.0);
    let items = variant_bar_layout(narrow, &labels, "Use this");
    assert_eq!(items.len(), 4);
    for item in &items {
        assert!(item.label.ends_with('…'), "{}", item.label);
        assert!(item.pill.origin.x >= 0.0);
        assert!(item.pill.origin.x + item.pill.size.x <= 700.0 + 0.5);
    }
}

#[test]
fn nothing_to_lay_out_paints_nothing() {
    assert!(variant_bar_layout(canvas(), &[], "用这个").is_empty());
    assert!(variant_bar_layout(Rect::ZERO, &[(0, "A".into())], "用这个").is_empty());
}
