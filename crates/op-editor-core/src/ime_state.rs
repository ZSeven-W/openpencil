//! In-flight IME composition (preedit) state.
//!
//! Native CJK input: winit delivers `Ime::Preedit` updates while the
//! user composes; the committed candidate arrives as `Ime::Commit`.
//! The native host consumes preedit updates without painting a
//! separate floating bubble; committed text arrives through
//! `Ime::Commit`.

/// The composition string currently held by the system IME, plus the
/// IME's caret/highlight range within it (byte offsets, when the
/// platform reports one).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImePreedit {
    /// In-flight composition text (UTF-8).
    pub text: String,
    /// Caret / highlighted segment within `text` in bytes, as
    /// reported by winit's `Ime::Preedit(_, cursor)`.
    pub cursor: Option<(usize, usize)>,
}
