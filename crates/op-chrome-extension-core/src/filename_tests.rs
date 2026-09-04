//! Tests for [`crate::filename`] — download-name sanitisation of an
//! attacker-controlled page title. The offline download is a `.op` document,
//! so the sanitised stem carries the `.op` suffix.

use crate::filename::{design_md_filename, op_filename};

#[test]
fn the_design_system_download_uses_the_conventional_fixed_name() {
    assert_eq!(design_md_filename(), "design.md");
}

#[test]
fn keeps_an_ordinary_title_readable() {
    assert_eq!(op_filename("Example Domain"), "Example Domain.op");
    assert_eq!(op_filename("周报 2026"), "周报 2026.op");
}

#[test]
fn falls_back_when_the_title_sanitises_to_nothing() {
    for title in ["", "   ", "...", "///", "\u{0}\u{1}", "..", "."] {
        assert_eq!(op_filename(title), "page.op", "title {title:?}");
    }
}

#[test]
fn no_title_can_produce_a_parent_directory_component() {
    // `chrome.downloads` rejects a name containing `..` outright, which would
    // turn a capture into an opaque failure.
    for title in [
        "../../etc/passwd",
        "..\\..\\windows",
        "a/../../b",
        "....//....//x",
    ] {
        let name = op_filename(title);
        assert!(!name.contains(".."), "{title:?} produced {name:?}");
        assert!(!name.contains('/'), "{title:?} produced {name:?}");
        assert!(!name.contains('\\'), "{title:?} produced {name:?}");
    }
}

#[test]
fn strips_path_separators_and_reserved_characters() {
    assert_eq!(
        op_filename("a/b\\c:d*e?f\"g<h>i|j%k"),
        "a b c d e f g h i j k.op"
    );
}

#[test]
fn strips_control_characters_including_newlines_and_nul() {
    assert_eq!(op_filename("a\u{0}b\nc\rd\te"), "a b c d e.op");
    // A control run collapses to a single space, like the JS `+` quantifier.
    assert_eq!(op_filename("a\u{1}\u{2}\u{3}b"), "a b.op");
}

#[test]
fn never_produces_a_dot_file() {
    assert_eq!(op_filename(".bashrc"), "bashrc.op");
    assert_eq!(op_filename("   .hidden"), "hidden.op");
}

#[test]
fn collapses_dot_runs_but_keeps_a_single_dot() {
    assert_eq!(op_filename("report.v2"), "report.v2.op");
    assert_eq!(op_filename("report...v2"), "report.v2.op");
}

#[test]
fn caps_the_stem_and_trims_what_the_cut_leaves_behind() {
    let long = "x".repeat(200);
    assert_eq!(op_filename(&long), format!("{}.op", "x".repeat(80)));

    // The 80th unit lands on a space, which Windows would silently drop.
    let with_space = format!("{} tail", "y".repeat(79));
    assert_eq!(op_filename(&with_space), format!("{}.op", "y".repeat(79)));

    // …and on a dot.
    let with_dot = format!("{}.tail", "z".repeat(79));
    assert_eq!(op_filename(&with_dot), format!("{}.op", "z".repeat(79)));
}

#[test]
fn measures_the_cap_in_utf16_code_units_like_js_does() {
    // 40 astral characters are 80 UTF-16 code units — exactly the cap.
    let emoji = "😀".repeat(50);
    let name = op_filename(&emoji);
    let stem = name.trim_end_matches(".op");
    assert_eq!(stem.chars().count(), 40);
}

#[test]
fn collapses_whitespace_runs_including_the_byte_order_mark() {
    assert_eq!(op_filename("a \t\n b"), "a b.op");
    assert_eq!(op_filename("\u{feff}title\u{feff}"), "title.op");
    assert_eq!(op_filename("a\u{a0}\u{3000}b"), "a b.op");
}
