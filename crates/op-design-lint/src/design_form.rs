//! What kind of SURFACE a root frame is, judged from the artboard itself.
//!
//! **Why this lives in the lint crate rather than the orchestrator.** The
//! classifier is a pure judgement over two numbers, and its two consumers sit
//! on opposite sides of the crate graph: the orchestrator's repair passes
//! (`spacing_repair`, the geometry validation loop, `role_post_pass`) and the
//! detectors in [`crate::detectors`]. `op-orchestrator` depends on
//! `op-design-lint`, so the only placement that keeps ONE judge for both is
//! the lower crate. `op_orchestrator::design_type` re-exports every item here,
//! so the orchestrator-side import paths are unchanged.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;

/// What kind of SURFACE an assembled root frame is, judged from the artboard
/// itself rather than from the prompt.
///
/// `op_orchestrator::design_type::detect_design_type` answers "what did the
/// user ask for", and only the prompt / plan layer can call it — repair passes
/// run on an assembled tree, and on the agentic-loop path there is no plan at
/// all. They need the same distinction derived from what is actually on the
/// canvas.
///
/// **This is that single judge.** A repair pass must not re-derive the form
/// from a width comparison of its own: the workspace already carries six
/// separate `480.0` literals (`mobile_reflow`, `mobile_content_rail`,
/// `geometry_bottom_gap`, `cleanup_mobile_dense`, `cleanup_root_and_nav`,
/// `role_defaults`), which is exactly the drift this exists to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignForm {
    /// A phone-sized viewport. Chrome contracts (status bar, bottom nav),
    /// edge-to-edge content and tight rhythm apply here.
    MobileScreen,
    /// A scrolling page wide enough for a desktop browser — marketing site,
    /// landing page, desktop app screen. Sections own a vertical rhythm the
    /// root's gap cannot express, and content sits inside gutters.
    Page,
    /// A fixed 16:9 projector board. Neither a viewport nor a scroll surface.
    Deck,
    /// A fixed PORTRAIT independent board — a standalone knowledge / XHS
    /// card designed as its own surface, not a section of a scroll page.
    /// Evidence band (measured 0815 on the 0815 v4-pro card corpus): the XHS
    /// 3:4 card board is 1080x1440 and the 9:16 card 1080x1920, so the band
    /// reads authored width 900..=1280, height > width, and height/width
    /// <= 2.0. Everything outside the band keeps its previous judgement —
    /// cards never steal from the phone band (<=480) or the projector band
    /// (>=1600), and a width between 480 and 900 stays `Unknown`.
    Card,
    /// Not enough evidence to classify — an unsized root, a `fill_container`
    /// width, or a width between the phone and desktop bands. Passes MUST
    /// treat this as "no type information", never as a default form.
    Unknown,
}

impl DesignForm {
    /// A surface the reader scrolls through, where the root's direct children
    /// are page sections rather than viewport chrome.
    pub fn is_scrolling_page(self) -> bool {
        matches!(self, DesignForm::Page)
    }

    /// A fixed projector board. Content that overflows one is not clipped or
    /// shrunk — it moves to another board (deck-system spec §3.1), which no
    /// geometry pass can decide on its own.
    pub fn is_deck_board(self) -> bool {
        matches!(self, DesignForm::Deck)
    }

    /// A fixed portrait card board — the same "independent board, not a
    /// scroll surface" semantics as a deck, in a portrait aspect. Repair
    /// passes that only need "fixed board" (the margin floor) accept both.
    pub fn is_card_board(self) -> bool {
        matches!(self, DesignForm::Card)
    }
}

/// Widest artboard (inclusive) that reads as a phone viewport.
///
/// `op_orchestrator::plan_normalize::MOBILE_MAX_WIDTH` aliases this constant
/// so the plan layer and the tree layer agree on the band by construction.
pub const MOBILE_MAX_WIDTH: f64 = 480.0;
/// Narrowest artboard that reads as a desktop browser page. Between this and
/// [`MOBILE_MAX_WIDTH`] sits the tablet band, which is deliberately
/// [`DesignForm::Unknown`] — neither set of contracts is safe to assume there.
const PAGE_MIN_WIDTH: f64 = 1024.0;
/// Narrowest artboard that can be a projector board (the 1920 preset, minus
/// room for a model that rounds down).
const DECK_MIN_WIDTH: f64 = 1600.0;
/// 16:9 is 0.5625. The band accepts a board a model sized slightly off while
/// still excluding any page tall enough to scroll.
const DECK_ASPECT_RANGE: std::ops::RangeInclusive<f64> = 0.50..=0.65;
/// Card band (inclusive) on the authored width. 900 is the small end of the
/// portrait knowledge-card presets (1080 - one design step); 1280 stops
/// short of the 1600 projector band so Deck can never be shadowed.
const CARD_MIN_WIDTH: f64 = 900.0;
const CARD_MAX_WIDTH: f64 = 1280.0;
/// Tallest aspect that still reads as one fixed board: 9:16 is 0.5625
/// inverted -> 1.78; 2.0 leaves room for a slightly taller card while a
/// genuine scroll page (h/w > 2.0, e.g. 1200x2977) keeps its Page form.
const CARD_MAX_ASPECT: f64 = 2.0;

/// Classify a root frame from its artboard size. `width` / `height` are the
/// authored numeric values; a non-numeric (`fill_container`, `fit_content`) or
/// absent size is passed as `None` and yields [`DesignForm::Unknown`].
pub fn classify_root_form(width: Option<f64>, height: Option<f64>) -> DesignForm {
    let Some(width) = width.filter(|w| *w > 0.0) else {
        return DesignForm::Unknown;
    };
    if width <= MOBILE_MAX_WIDTH {
        return DesignForm::MobileScreen;
    }
    if width >= DECK_MIN_WIDTH {
        if let Some(height) = height.filter(|h| *h > 0.0) {
            if DECK_ASPECT_RANGE.contains(&(height / width)) {
                return DesignForm::Deck;
            }
        }
    }
    // Card runs BEFORE Page: the card band (900..=1280 portrait, h/w <= 2.0)
    // overlaps the Page band's lower end, and a 1080x1440 card must read as
    // Card, not as a scroll page. Every input outside the card band keeps its
    // previous judgement: phone (<=480) and deck (>=1600) bands never overlap,
    // a square (h <= w) or taller-than-2.0 board stays Page (when >= 1024) or
    // Unknown, and 480..900 wide stays Unknown either way.
    if (CARD_MIN_WIDTH..=CARD_MAX_WIDTH).contains(&width) {
        if let Some(height) = height.filter(|h| *h > 0.0) {
            if height > width && height / width <= CARD_MAX_ASPECT {
                return DesignForm::Card;
            }
        }
    }
    if width >= PAGE_MIN_WIDTH {
        return DesignForm::Page;
    }
    DesignForm::Unknown
}

/// [`classify_root_form`] over a root node's JSON. Sizes that are strings
/// (`"fill_container"`) read as unknown, matching the numeric contract above.
pub fn classify_root_form_value(root: &serde_json::Value) -> DesignForm {
    let number = |key: &str| root.get(key).and_then(serde_json::Value::as_f64);
    classify_root_form(number("width"), number("height"))
}

/// [`classify_root_form`] over a typed root node.
///
/// Only the three container variants can BE an artboard; anything else is a
/// leaf that happens to sit at the top level, and reading its box as a board
/// would classify a stray text node as a page. Reads the typed fields rather
/// than round-tripping through `serde_json` — the detectors call this on every
/// root, and serializing a whole design to read two numbers is not free.
pub fn classify_root_form_node(root: &PenNode) -> DesignForm {
    let container = match root {
        PenNode::Frame(node) => &node.container,
        PenNode::Group(node) => &node.container,
        PenNode::Rectangle(node) => &node.container,
        _ => return DesignForm::Unknown,
    };
    classify_root_form(size_px(&container.width), size_px(&container.height))
}

/// The pixel value of an authored size, or `None` for a keyword / expression.
fn size_px(size: &Option<SizingBehavior>) -> Option<f64> {
    match size {
        Some(SizingBehavior::Number(px)) => Some(*px),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn a_typed_root_classifies_the_same_as_its_json() {
        let deck = node(json!({
            "type": "frame", "id": "board", "width": 1920, "height": 1080
        }));
        assert_eq!(classify_root_form_node(&deck), DesignForm::Deck);
        assert_eq!(
            classify_root_form_value(&json!({"width": 1920, "height": 1080})),
            DesignForm::Deck
        );
    }

    #[test]
    fn a_keyword_sized_root_is_unknown_not_a_default() {
        let root = node(json!({
            "type": "frame", "id": "root", "width": "fill_container", "height": 1080
        }));
        assert_eq!(classify_root_form_node(&root), DesignForm::Unknown);
    }

    #[test]
    fn a_leaf_at_the_top_level_is_never_an_artboard() {
        // A stray 1920x1080 image is not a projector board — only a container
        // variant can be one.
        let image = node(json!({
            "type": "image", "id": "hero", "src": "x.png", "width": 1920, "height": 1080
        }));
        assert_eq!(classify_root_form_node(&image), DesignForm::Unknown);
    }

    #[test]
    fn a_deck_board_reports_itself_as_one() {
        assert!(DesignForm::Deck.is_deck_board());
        assert!(!DesignForm::Page.is_deck_board());
        assert!(!DesignForm::Unknown.is_deck_board());
    }

    #[test]
    fn a_card_board_reports_itself_as_one() {
        assert!(DesignForm::Card.is_card_board());
        assert!(!DesignForm::Deck.is_card_board());
        assert!(!DesignForm::Page.is_card_board());
        assert!(!DesignForm::Unknown.is_card_board());
        // A card is a fixed board, but never a deck board (16:9 gate).
        assert!(!DesignForm::Card.is_deck_board());
    }

    #[test]
    fn a_portrait_card_classifies_as_card() {
        // The 0815 v4-pro card corpus: XHS 3:4 and the generic 9:16 card.
        assert_eq!(
            classify_root_form(Some(1080.0), Some(1440.0)),
            DesignForm::Card
        );
        assert_eq!(
            classify_root_form(Some(1080.0), Some(1920.0)),
            DesignForm::Card
        );
        // Band edges: the exact corners stay inside.
        assert_eq!(
            classify_root_form(Some(900.0), Some(1000.0)),
            DesignForm::Card
        );
        assert_eq!(
            classify_root_form(Some(1280.0), Some(1600.0)),
            DesignForm::Card
        );
        assert_eq!(
            classify_root_form(Some(1280.0), Some(2560.0)),
            DesignForm::Card
        );
    }

    #[test]
    fn a_square_or_landscape_board_is_not_a_card() {
        // XHS square 1:1 — a fixed board but not portrait, so it keeps its
        // previous Page judgement.
        assert_eq!(
            classify_root_form(Some(1080.0), Some(1080.0)),
            DesignForm::Page
        );
        assert_eq!(
            classify_root_form(Some(1200.0), Some(800.0)),
            DesignForm::Page
        );
        // A phone inside its own band keeps MobileScreen — the card band never
        // shadows the phone band.
        assert_eq!(
            classify_root_form(Some(390.0), Some(844.0)),
            DesignForm::MobileScreen
        );
    }

    #[test]
    fn outside_the_card_band_the_previous_judgement_holds() {
        // Narrower than 900 stays Unknown (the tablet band is deliberate).
        assert_eq!(
            classify_root_form(Some(899.0), Some(1798.0)),
            DesignForm::Unknown
        );
        assert_eq!(
            classify_root_form(Some(768.0), Some(1024.0)),
            DesignForm::Unknown
        );
        // Wider than 1280 stays Page (a 1600-wide board is not a card).
        assert_eq!(
            classify_root_form(Some(1281.0), Some(2000.0)),
            DesignForm::Page
        );
        // Taller than 2:1 is a scroll page, not one fixed board.
        assert_eq!(
            classify_root_form(Some(1200.0), Some(2977.0)),
            DesignForm::Page
        );
        assert_eq!(
            classify_root_form(Some(1280.0), Some(2561.0)),
            DesignForm::Page
        );
        // The projector band is untouched: 1920x1080 stays Deck.
        assert_eq!(
            classify_root_form(Some(1920.0), Some(1080.0)),
            DesignForm::Deck
        );
    }
}
