//! Cross-locale catalog integrity tests: key sets, duplicates, placeholders.

use std::collections::BTreeSet;

type Lookup = fn(&str) -> Option<&'static str>;

fn tables() -> [(&'static str, &'static str, &'static str, Lookup); 15] {
    [
        (
            "en",
            include_str!("en.rs"),
            concat!(
                include_str!("en_git.rs"),
                include_str!("en_panel.rs"),
                include_str!("en_collab.rs")
            ),
            super::en::lookup,
        ),
        (
            "zh_cn",
            include_str!("zh_cn.rs"),
            concat!(
                include_str!("zh_cn_git.rs"),
                include_str!("zh_cn_panel.rs"),
                include_str!("zh_cn_collab.rs")
            ),
            super::zh_cn::lookup,
        ),
        (
            "zh_tw",
            include_str!("zh_tw.rs"),
            concat!(
                include_str!("zh_tw_git.rs"),
                include_str!("zh_tw_panel.rs"),
                include_str!("zh_tw_collab.rs")
            ),
            super::zh_tw::lookup,
        ),
        (
            "ja",
            include_str!("ja.rs"),
            concat!(
                include_str!("ja_git.rs"),
                include_str!("ja_panel.rs"),
                include_str!("ja_collab.rs")
            ),
            super::ja::lookup,
        ),
        (
            "ko",
            include_str!("ko.rs"),
            concat!(
                include_str!("ko_git.rs"),
                include_str!("ko_panel.rs"),
                include_str!("ko_collab.rs")
            ),
            super::ko::lookup,
        ),
        (
            "fr",
            include_str!("fr.rs"),
            concat!(
                include_str!("fr_git.rs"),
                include_str!("fr_panel.rs"),
                include_str!("fr_collab.rs")
            ),
            super::fr::lookup,
        ),
        (
            "es",
            include_str!("es.rs"),
            concat!(
                include_str!("es_git.rs"),
                include_str!("es_panel.rs"),
                include_str!("es_collab.rs")
            ),
            super::es::lookup,
        ),
        (
            "de",
            include_str!("de.rs"),
            concat!(
                include_str!("de_git.rs"),
                include_str!("de_panel.rs"),
                include_str!("de_collab.rs")
            ),
            super::de::lookup,
        ),
        (
            "pt",
            include_str!("pt.rs"),
            concat!(
                include_str!("pt_git.rs"),
                include_str!("pt_panel.rs"),
                include_str!("pt_collab.rs")
            ),
            super::pt::lookup,
        ),
        (
            "ru",
            include_str!("ru.rs"),
            concat!(
                include_str!("ru_git.rs"),
                include_str!("ru_panel.rs"),
                include_str!("ru_collab.rs")
            ),
            super::ru::lookup,
        ),
        (
            "hi",
            include_str!("hi.rs"),
            concat!(
                include_str!("hi_git.rs"),
                include_str!("hi_panel.rs"),
                include_str!("hi_collab.rs")
            ),
            super::hi::lookup,
        ),
        (
            "tr",
            include_str!("tr.rs"),
            concat!(
                include_str!("tr_git.rs"),
                include_str!("tr_panel.rs"),
                include_str!("tr_collab.rs")
            ),
            super::tr::lookup,
        ),
        (
            "th",
            include_str!("th.rs"),
            concat!(
                include_str!("th_git.rs"),
                include_str!("th_panel.rs"),
                include_str!("th_collab.rs")
            ),
            super::th::lookup,
        ),
        (
            "vi",
            include_str!("vi.rs"),
            concat!(
                include_str!("vi_git.rs"),
                include_str!("vi_panel.rs"),
                include_str!("vi_collab.rs")
            ),
            super::vi::lookup,
        ),
        (
            "id",
            include_str!("id.rs"),
            concat!(
                include_str!("id_git.rs"),
                include_str!("id_panel.rs"),
                include_str!("id_collab.rs")
            ),
            super::id::lookup,
        ),
    ]
}

fn source_keys<'a>(name: &str, source: &'a str) -> Vec<&'a str> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim_start();
            let (pattern, value) = line.split_once("=>")?;
            let pattern = pattern.trim();
            if pattern == "_" {
                return None;
            }
            let rest = pattern.strip_prefix('"').unwrap_or_else(|| {
                panic!(
                    "locale table `{name}` has a non-string match pattern on line {}: `{}`",
                    index + 1,
                    line.trim()
                )
            });
            let (key, suffix) = rest.split_once('"').unwrap_or_else(|| {
                panic!(
                    "locale table `{name}` has an unterminated key on line {}: `{}`",
                    index + 1,
                    line.trim()
                )
            });
            assert!(
                suffix.trim().is_empty(),
                "locale table `{name}` has an unsupported match pattern on line {}: `{}`; \
                 use exactly one quoted key per match arm",
                index + 1,
                line.trim()
            );
            assert!(
                !value.trim().is_empty(),
                "locale table `{name}` has an empty value on line {}",
                index + 1
            );
            Some(key)
        })
        .collect()
}

fn table_keys(name: &str, main: &str, git: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for key in source_keys(name, main)
        .into_iter()
        .chain(source_keys(name, git))
    {
        assert!(
            keys.insert(key.to_string()),
            "locale table `{name}` contains duplicate key `{key}`"
        );
    }
    keys
}

fn placeholders(value: &str) -> BTreeSet<String> {
    let bytes = value.as_bytes();
    let mut result = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'{' {
            index += 1;
            continue;
        }
        let doubled = bytes.get(index + 1) == Some(&b'{');
        let start = index + if doubled { 2 } else { 1 };
        let Some(relative_end) = bytes[start..].iter().position(|byte| *byte == b'}') else {
            break;
        };
        let end = start + relative_end;
        if doubled && bytes.get(end + 1) != Some(&b'}') {
            index += 2;
            continue;
        }
        let candidate = &value[start..end];
        if !candidate.is_empty()
            && candidate
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            result.insert(candidate.to_string());
        }
        index = end + if doubled { 2 } else { 1 };
    }
    result
}

#[test]
fn every_locale_has_exactly_the_english_key_set() {
    let all_tables = tables();
    let expected = table_keys(all_tables[0].0, all_tables[0].1, all_tables[0].2);
    assert_eq!(expected.len(), 1674, "update the intentional catalog size");

    for (name, main, git, lookup) in all_tables {
        let actual = table_keys(name, main, git);
        let missing: Vec<_> = expected.difference(&actual).cloned().collect();
        let extra: Vec<_> = actual.difference(&expected).cloned().collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "locale `{name}` differs from English; missing={missing:?}, extra={extra:?}"
        );
        for key in &expected {
            let english = super::en::lookup(key).expect("English key came from its own source");
            assert!(
                lookup(key).is_some_and(|value| english.is_empty() || !value.is_empty()),
                "locale `{name}` has no direct value for `{key}`"
            );
        }
    }
}

#[test]
fn scene_template_catalog_is_complete_in_all_fifteen_locales() {
    for key in [
        "sceneTemplate.title",
        "sceneTemplate.searchPlaceholder",
        "sceneTemplate.empty",
        "sceneTemplate.frames",
        "sceneTemplate.filter.all",
        "sceneTemplate.scene.tutorial",
        "sceneTemplate.scene.comparison",
        "sceneTemplate.scene.carousel",
        "sceneTemplate.scene.slides",
        "sceneTemplate.scene.card",
        "sceneTemplate.item.screenshotTutorial.title",
        "sceneTemplate.item.screenshotTutorial.summary",
        "sceneTemplate.item.knowledgeCarousel.title",
        "sceneTemplate.item.knowledgeCarousel.summary",
        "sceneTemplate.item.beforeAfter.title",
        "sceneTemplate.item.beforeAfter.summary",
        "sceneTemplate.item.slideDeck.title",
        "sceneTemplate.item.slideDeck.summary",
        "sceneTemplate.item.knowledgeCardVertical.title",
        "sceneTemplate.item.knowledgeCardVertical.summary",
        "sceneTemplate.item.knowledgeCardSquare.title",
        "sceneTemplate.item.knowledgeCardSquare.summary",
        "sceneTemplate.item.pitchDeckDark.title",
        "sceneTemplate.item.pitchDeckDark.summary",
        "sceneTemplate.item.lectureDeckLight.title",
        "sceneTemplate.item.lectureDeckLight.summary",
        "sceneTemplate.item.minimalKeynote.title",
        "sceneTemplate.item.minimalKeynote.summary",
        "sceneTemplate.item.gradientTech.title",
        "sceneTemplate.item.gradientTech.summary",
        "fileMenu.newFromTemplate",
    ] {
        for (name, _, _, lookup) in tables() {
            assert!(
                lookup(key).is_some(),
                "locale `{name}` is missing direct Scene Template Center key `{key}`"
            );
        }
    }
}

#[test]
fn collaboration_catalog_is_complete_in_all_fifteen_locales() {
    let all_tables = tables();
    let keys: Vec<_> = table_keys(all_tables[0].0, all_tables[0].1, all_tables[0].2)
        .into_iter()
        .filter(|key| key.starts_with("collab."))
        .collect();
    assert_eq!(
        keys.len(),
        114,
        "update the intentional collaboration key set"
    );
    for key in keys {
        for (name, _, _, lookup) in &all_tables {
            assert!(
                lookup(&key).is_some(),
                "locale `{name}` is missing direct collaboration key `{key}`"
            );
        }
    }
}

#[test]
fn every_translation_preserves_the_english_placeholder_set() {
    let all_tables = tables();
    let expected_keys = table_keys(all_tables[0].0, all_tables[0].1, all_tables[0].2);
    for key in expected_keys {
        let english = super::en::lookup(&key).expect("English key came from its own source");
        let expected = placeholders(english);
        for (name, _, _, lookup) in all_tables {
            let translated =
                lookup(&key).unwrap_or_else(|| panic!("locale `{name}` is missing `{key}`"));
            assert_eq!(
                placeholders(translated),
                expected,
                "locale `{name}` changes placeholders for `{key}`"
            );
        }
    }
}

#[test]
fn previously_missing_callsite_keys_are_catalogued() {
    for key in [
        "rightPanel.interact",
        "widget.title",
        "widget.checked",
        "fill.mesh",
        "fill.shader",
    ] {
        for (name, _, _, lookup) in tables() {
            assert!(
                lookup(key).is_some(),
                "locale `{name}` is missing call-site key `{key}`"
            );
        }
    }
}
