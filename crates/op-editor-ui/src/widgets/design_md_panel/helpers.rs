use crate::Color;

pub(super) use crate::util::truncate_ellipsis as truncate;

pub(super) fn hex_to_color(hex: &str) -> Color {
    match op_editor_core::parse_hex_rgb(hex) {
        Some((r, g, b)) => Color { r, g, b, a: 1.0 },
        None => Color {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        },
    }
}

// Test-only call counter for `DesignMdPanel::layout` — lets tests prove a
// single `paint` / `hit_test` pass resolves the section layout exactly
// once instead of the pre-fix up-to-3×.
#[cfg(test)]
thread_local! {
    static LAYOUT_CALL_COUNT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn tick_layout_call() {
    LAYOUT_CALL_COUNT.with(|c| c.set(c.get() + 1));
}

#[cfg(test)]
pub(crate) fn layout_call_count() -> u64 {
    LAYOUT_CALL_COUNT.with(std::cell::Cell::get)
}
