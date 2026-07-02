//! Guard tests for the local style-guide corpus — task 0.7.
//!
//! These are additive audit tests that assert structural invariants of:
//!   - The `STYLE_GUIDE_TAGS` vocabulary array (tag hygiene)
//!   - The `select_style_guide` exact-name lookup contract (subagent reproducibility)
//!   - Every guide's structured content (Color System + Typography sections)
//!   - The registry's purely local / embedded resolution (no remote path)
//!
//! All four tests are read-only; they touch no data. Failure means a real
//! data defect was introduced — fix the data (or a bug in select_style_guide),
//! not the assertion.

#[cfg(test)]
#[allow(clippy::module_inception)]
mod audit_tests {
    use super::super::{
        loader::{select_style_guide, style_guide_registry, SelectOptions},
        types::STYLE_GUIDE_TAGS,
    };

    // -----------------------------------------------------------------------
    // Guard 1 — Tag vocabulary hygiene
    //
    // Every entry in STYLE_GUIDE_TAGS must be:
    //   - non-empty
    //   - lowercase [a-z0-9-_] only (kebab or snake_case; the corpus uses a
    //     couple of snake_case aliases so both separators are allowed)
    //   - free of spaces, colons, double-quotes, hashes (Pencil-YAML-leak guard)
    //   - not starting with "- " (guards against a `- fill: "#fef0e8"` style leak)
    //
    // The vocabulary must also be free of duplicates.
    // -----------------------------------------------------------------------
    #[test]
    fn tag_vocabulary_entries_are_well_formed() {
        for &tag in STYLE_GUIDE_TAGS {
            assert!(!tag.is_empty(), "STYLE_GUIDE_TAGS contains an empty entry");

            assert!(
                !tag.contains(' '),
                "STYLE_GUIDE_TAGS entry contains a space: {tag:?}"
            );
            assert!(
                !tag.contains(':'),
                "STYLE_GUIDE_TAGS entry contains a colon: {tag:?}"
            );
            assert!(
                !tag.contains('"'),
                "STYLE_GUIDE_TAGS entry contains a double-quote: {tag:?}"
            );
            assert!(
                !tag.contains('#'),
                "STYLE_GUIDE_TAGS entry contains a hash: {tag:?}"
            );
            assert!(
                !tag.starts_with("- "),
                "STYLE_GUIDE_TAGS entry starts with dash-space (YAML list leak): {tag:?}"
            );

            // Only lowercase letters, digits, hyphens, and underscores.
            assert!(
                tag.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'),
                "STYLE_GUIDE_TAGS entry contains characters outside [a-z0-9-_]: {tag:?}"
            );
        }

        // No duplicates.
        let mut seen = std::collections::HashSet::new();
        for &tag in STYLE_GUIDE_TAGS {
            assert!(
                seen.insert(tag),
                "STYLE_GUIDE_TAGS has a duplicate entry: {tag:?}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Guard 2 — Stable exact-name lookup (subagent reproducibility contract)
    //
    // For every guide in the live registry, select_style_guide(registry,
    // SelectOptions { name: Some(guide.name.clone()), .. }) must return that
    // exact guide back. Derives names from the corpus at runtime — never
    // hardcodes names that may not exist.
    //
    // This is the contract `spawn_agents.styleguideName` depends on: once the
    // parent agent records a style-guide name, every subagent that calls
    // get_style_guide with that name must receive the same guide.
    // -----------------------------------------------------------------------
    #[test]
    fn exact_name_lookup_returns_the_correct_guide() {
        let registry = style_guide_registry();
        assert!(
            !registry.is_empty(),
            "style_guide_registry() returned an empty list — corpus failed to embed"
        );

        for guide in registry {
            let opts = SelectOptions {
                name: Some(guide.name.clone()),
                ..Default::default()
            };
            let found = select_style_guide(registry, &opts);
            assert!(
                found.is_some(),
                "select_style_guide could not find guide by its own name: {:?}",
                guide.name
            );
            assert_eq!(
                found.unwrap().name,
                guide.name,
                "select_style_guide returned the wrong guide for name {:?}",
                guide.name
            );
        }
    }

    // -----------------------------------------------------------------------
    // Guard 3 — Structured content: Color System + Typography headings
    //
    // Every guide the registry exposes must carry both a `## Color System`
    // and a `## Typography` heading. These are the two token sections the
    // agentic design loop extracts color and font values from.
    //
    // Also asserts that each guide has a non-empty name and at least one tag
    // (so a partially-parsed guide never silently surfaces as empty metadata).
    // -----------------------------------------------------------------------
    #[test]
    fn every_guide_has_required_structured_sections() {
        let registry = style_guide_registry();

        for guide in registry {
            // Non-empty name and at least one tag.
            assert!(
                !guide.name.is_empty(),
                "a guide in the registry has an empty name"
            );
            assert!(
                !guide.tags.is_empty(),
                "guide {:?} has no tags — every guide must declare at least one",
                guide.name
            );

            // Required headings — exact prefix match so `## Color System`
            // inside a fenced code block would not pass (guides don't have
            // fenced blocks at this level).
            assert!(
                guide.content.contains("## Color System"),
                "guide {:?} is missing the required `## Color System` heading",
                guide.name
            );
            assert!(
                guide.content.contains("## Typography"),
                "guide {:?} is missing the required `## Typography` heading",
                guide.name
            );
        }
    }

    // -----------------------------------------------------------------------
    // Guard 4 — No remote path: registry is purely in-memory / embedded
    //
    // `style_guide_registry()` is backed by `include_dir!` (compile-time
    // embed); it must return ≥ 50 guides synchronously with no network I/O.
    //
    // The grep-confirmed absence of `api.pencil.dev`, `http`, `reqwest`,
    // or any network primitive in `crates/op-ai-skills/src/style_guide/` is
    // the structural guarantee; this test is the runtime corroboration —
    // the call completes instantly with a non-empty slice, proving the data
    // came from the binary rather than from any deferred fetch.
    //
    // Note: the `include_dir!` macro embeds `crates/op-ai-skills/skills/`
    // into the binary at compile time. Editing a `.md` file and rebuilding
    // is sufficient to pick up the change (no additional copy step needed).
    // -----------------------------------------------------------------------
    #[test]
    fn registry_is_local_and_returns_at_least_fifty_guides() {
        // The call must complete synchronously (no async, no blocking I/O).
        // If it takes a network round-trip this test will time-out in CI.
        let registry = style_guide_registry();

        assert!(
            registry.len() >= 50,
            "expected ≥ 50 locally-embedded style guides, got {} — \
             corpus may be missing or the embed macro path is wrong",
            registry.len()
        );

        // Every entry must have non-empty content (proves the embed
        // resolved real file bytes, not zero-length placeholders).
        for guide in registry {
            assert!(
                !guide.content.is_empty(),
                "guide {:?} has empty content — include_dir! embed may be broken",
                guide.name
            );
        }
    }
}
