//! Token-table dialect tests.
//!
//! The fixture is a same-shape sample written here, not a copied third-party
//! file: the point is that the *structure* found in the ecosystem parses, and
//! no external guide's content belongs in this repository.

use super::*;
use crate::style_guide::parser::extract_style_guide_values;

/// A community `DESIGN.md` in the shape that returned eight `None`s: a token
/// table with the value in a code span, a CSS custom property, and the role in
/// prose — plus a gradient accent whose only colour is an `rgba()`.
const TOKEN_TABLE_GUIDE: &str = "\
# Midnight Reference

## Tokens — Colors

| Name | Value | Token | Role |
| --- | --- | --- | --- |
| Void Canvas | `#0a0a0a` | `--color-void-canvas` | Primary page background, base surface for all views |
| Panel Glass | `#141418` | `--color-panel-glass` | Elevated panel and card fills, frosted layers |
| Ink Black | `#000000` | `--color-ink-black` | Icon strokes, SVG fill, deep contrast elements |
| Bright Snow | `#f5f5f7` | `--color-bright-snow` | Primary CTA fill, headline text on dark surfaces |
| Dusk Violet | `linear-gradient(90deg, rgba(0, 0, 0, 0), rgba(107, 98, 242, 0.565) 50%, rgba(0, 0, 0, 0))` | `--color-dusk-violet` | The only chromatic accent in the system |
| Bone | `#ededed` | `--color-bone` | Primary readable text on dark surfaces |
| Ash Grey | `#9a9aa2` | `--color-ash-grey` | Secondary text, supporting copy |
| Faint Smoke | `#5a5a62` | `--color-faint-smoke` | Muted text, disabled states |
| Hairline | `#242428` | `--color-hairline` | 1px borders on dark-surface controls |

## Typography

### DM Sans — Interface
- **Weights:** 500, 600
- Used for every UI surface.

### JetBrains Mono — Numerics
- **Weights:** 400

## Shape

Components are round: 9999px pill buttons, 10px UI radii, 24px card radii.
";

#[test]
fn a_token_table_guide_yields_its_palette() {
    let v = extract_style_guide_values(TOKEN_TABLE_GUIDE);
    assert_eq!(v.colors.background.as_deref(), Some("#0A0A0A"));
    assert_eq!(v.colors.surface.as_deref(), Some("#141418"));
    assert_eq!(v.colors.text_primary.as_deref(), Some("#EDEDED"));
    assert_eq!(v.colors.text_secondary.as_deref(), Some("#9A9AA2"));
    assert_eq!(v.colors.text_muted.as_deref(), Some("#5A5A62"));
    assert_eq!(v.colors.border.as_deref(), Some("#242428"));
}

/// The single chromatic accent is the most recognizable thing about a guide
/// like this, and it is written only as an `rgba()` inside a gradient. Missing
/// it means the imported style can never show its own colour.
#[test]
fn a_gradient_accents_rgba_is_extracted() {
    let v = extract_style_guide_values(TOKEN_TABLE_GUIDE);
    assert_eq!(v.colors.accent.as_deref(), Some("#6B62F2"));
}

/// "Primary page background, base surface for all views" describes one colour.
/// Letting it satisfy both fields would paint every card the page colour.
#[test]
fn a_row_is_claimed_by_one_field_only() {
    let v = extract_style_guide_values(TOKEN_TABLE_GUIDE);
    assert_ne!(v.colors.background, v.colors.surface);
}

#[test]
fn heading_fonts_and_prose_radii_are_read() {
    let v = extract_style_guide_values(TOKEN_TABLE_GUIDE);
    assert_eq!(v.typography.display_font.as_deref(), Some("DM Sans"));
    assert_eq!(v.typography.data_font.as_deref(), Some("JetBrains Mono"));
    assert_eq!(v.radius.button, Some(9999), "pill controls");
    assert_eq!(v.radius.card, Some(24));
}

#[test]
fn colour_literals_normalize_to_six_digit_hex() {
    assert_eq!(read_color("`#abc`").as_deref(), Some("#AABBCC"));
    assert_eq!(read_color("`#0a0a0aff`").as_deref(), Some("#0A0A0A"));
    assert_eq!(read_color("rgb(255, 0, 8)").as_deref(), Some("#FF0008"));
    assert_eq!(
        read_color("rgba(107 98 242 / 0.5)").as_deref(),
        Some("#6B62F2")
    );
    // Values out of range are clamped rather than wrapping around.
    assert_eq!(read_color("rgb(300, -5, 0)").as_deref(), Some("#FF0000"));
    assert_eq!(read_color("no colour at all"), None);
    assert_eq!(read_color("issue #1234567890"), None);
}

/// A file with nothing extractable must produce nothing, not a guess.
#[test]
fn a_guide_with_no_values_stays_empty() {
    let v = extract_style_guide_values("# Prose Only\n\nSoft, quiet, generous.\n");
    assert_eq!(v.colors, StyleColors::default());
    assert_eq!(v.typography, StyleTypography::default());
    assert_eq!(v.radius, StyleRadius::default());
}

/// A spacing table is not a radius table; reading one as the other would put
/// an invented corner radius on every card.
#[test]
fn radii_are_only_read_from_lines_that_claim_to_be_radii() {
    let v = extract_style_guide_values(
        "# G\n\n## Spacing\n\nThe base unit is 8px; gutters are 24px.\n",
    );
    assert_eq!(v.radius, StyleRadius::default());
}

#[test]
fn malformed_tables_do_not_panic() {
    for sample in [
        "| |\n|---|\n",
        "|||||\n",
        "| a | `#` | b |\n",
        "| a | rgba( | b |\n",
        "| a | rgb(1) | b |\n",
        "| 温暖 | `#E07A5F` | 主色 |\n",
        "|",
    ] {
        let _ = extract_style_guide_values(sample);
    }
}

// ─── Regression lock ───────────────────────────────────────────────────

/// The dialect is a gap filler. Every guide in the shipped corpus must reach
/// byte-identical values whether or not it runs — otherwise this change moved
/// the palette of fifty guides that were working.
#[test]
fn the_token_dialect_never_changes_a_corpus_guides_values() {
    for guide in crate::style_guide::style_guide_registry() {
        let corpus_only = crate::style_guide::parser::extract_corpus_values(&guide.content);
        let with_dialect = extract_style_guide_values(&guide.content);
        assert_eq!(
            corpus_only, with_dialect,
            "{} changed once the token dialect ran",
            guide.name
        );
    }
}

// ─── Traps found on a real imported guide ──────────────────────────────
//
// Each of these was a wrong value the dialect produced on a user's actual
// file, not a hypothetical. They are asserted separately from the palette
// test so a regression names the specific mistake it reintroduced.

/// A gradient that fades in from nothing starts with a fully transparent
/// stop. Taking the first `rgba()` reported the signature violet accent as
/// `#000000` — the one colour that makes the style recognizable, replaced by
/// the absence of colour.
#[test]
fn a_transparent_gradient_stop_is_not_a_colour() {
    assert_eq!(
        read_color("linear-gradient(90deg, rgba(0, 0, 0, 0), rgba(107, 98, 242, 0.565) 50%)")
            .as_deref(),
        Some("#6B62F2")
    );
    // Zero alpha is skipped; a low-but-present alpha is a real colour.
    assert_eq!(
        read_color("rgba(12, 34, 56, 0.01)").as_deref(),
        Some("#0C2238")
    );
    assert_eq!(read_color("rgba(9, 9, 9, 0)"), None);
}

/// "Icon strokes, SVG fill" is not a border. It claimed the field on a real
/// guide and reported the border colour as pure black, three rows above the
/// `Hairline` row that said "1px borders".
#[test]
fn icon_strokes_do_not_claim_the_border_field() {
    let v = extract_style_guide_values(TOKEN_TABLE_GUIDE);
    assert_eq!(v.colors.border.as_deref(), Some("#242428"));
    assert_ne!(v.colors.border.as_deref(), Some("#000000"));
}

/// A row saying "Primary CTA fill, headline text" sat above one saying
/// "Primary readable text" and took the field with it. Strong role cues have
/// to be scanned across the whole table before weak ones are considered.
#[test]
fn an_explicit_primary_text_row_beats_an_earlier_headline_row() {
    let v = extract_style_guide_values(TOKEN_TABLE_GUIDE);
    assert_eq!(
        v.colors.text_primary.as_deref(),
        Some("#EDEDED"),
        "the row that says 'primary readable text' wins over an earlier 'headline text'"
    );
}

/// One sentence names three radii. Classifying them against the whole line
/// makes every keyword true for every number, and the card radius came out as
/// the UI radius that happened to be listed first.
#[test]
fn each_radius_is_classified_by_its_own_words() {
    let v = extract_style_guide_values(
        "# G\n\n## Shape\n\nComponents are round: 9999px pill buttons, 10px UI radii, 24px card radii.\n",
    );
    assert_eq!(v.radius.card, Some(24), "not the 10px UI radius");
    assert_eq!(v.radius.button, Some(9999));
}

/// The floor under the summary tiers: colours come back with the author's own
/// words, and nothing is invented when there is nothing to find.
#[test]
fn the_palette_floor_reports_colours_with_their_own_role_text() {
    let samples = sample_palette(TOKEN_TABLE_GUIDE, 3);
    assert_eq!(samples.len(), 3);
    assert_eq!(samples[0].color, "#0A0A0A");
    assert!(samples[0].role.contains("primary page background"));
    assert!(sample_palette("# Nothing\n\nNo colours at all here.\n", 5).is_empty());

    // A prose guide with no table still yields its colours.
    let prose = sample_palette("The wash is #101014 and the spark is #6b62f2.\n", 5);
    assert_eq!(prose.len(), 2);
    assert_eq!(prose[0].color, "#101014");
}

/// A radius sentence whose description is not written in Latin script. The
/// per-number context is cut at 48 bytes, and a guide's prose is arbitrary
/// author text — most CJK and Hangul characters are three bytes each, so that
/// cut lands mid-character on ordinary input.
#[test]
fn a_radius_line_written_in_another_script_is_read_and_classified() {
    for line in [
        "Corner radius: 12px  卡片与面板容器的外框圆角，用于所有卡片与弹层",
        "Corner radius: 12px卡片与面板容器的外框圆角，用于所有卡片与弹层",
        "Corner radius: 12px 카드와 패널 컨테이너의 바깥 테두리 라운드에 사용",
    ] {
        let content = format!("# G\n\n## Shape\n\n{line}\n");
        let values = extract_style_guide_values(&content);
        // Nothing in these says which role the radius is for, so nothing is
        // invented — but the scan has to reach that answer, not panic on
        // the way.
        assert_eq!(values.radius, StyleRadius::default(), "line: {line}");
    }

    // The same sentence with an English role cue still classifies.
    let mixed = extract_style_guide_values(
        "# G\n\n## Shape\n\nCorner radius: 24px card 카드와 패널 컨테이너의 바깥 테두리\n",
    );
    assert_eq!(mixed.radius.card, Some(24));
}
