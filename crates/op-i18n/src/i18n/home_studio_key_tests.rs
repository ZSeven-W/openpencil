//! Studio Home key coverage across every locale table.

/// Every locale table must carry a DIRECT entry for the Studio Home
/// keys — `translate`'s EN fallback must not mask a missing translation.
#[test]
fn studio_home_keys_exist_in_every_locale_table() {
    const TASKS: [&str; 7] = [
        "app",
        "web",
        "presentation",
        "knowledge",
        "tutorial",
        "infographic",
        "poster",
    ];
    let mut keys: Vec<String> = Vec::new();
    for task in TASKS {
        for field in [
            "name",
            "summary",
            "label",
            "placeholder",
            "example",
            "exampleTitle",
            "exampleDesc",
            "examplePages",
        ] {
            keys.push(format!("home.task.{task}.{field}"));
        }
    }
    for extra in [
        "home.task.app.desktopPlaceholder",
        "home.task.app.desktopExample",
        "home.task.app.desktopTitle",
        "home.task.app.desktopDesc",
        "home.task.app.desktopPages",
        "home.task.infographic.flowPlaceholder",
        "home.task.infographic.flowExample",
        "home.task.infographic.flowTitle",
        "home.task.infographic.flowDesc",
        "home.task.infographic.flowPages",
        "home.task.infographic.comparisonPlaceholder",
        "home.task.infographic.comparisonExample",
        "home.task.infographic.comparisonTitle",
        "home.task.infographic.comparisonDesc",
        "home.task.infographic.comparisonPages",
        "home.welcome.titleLead",
        "home.welcome.titleMarked",
        "home.welcome.sub",
        "home.topbar.context",
        "home.topbar.openFile",
        "home.topbar.professional",
        "home.tabs.more",
        "home.tabs.moreCaption",
        "home.segment.mobile",
        "home.segment.desktop",
        "home.segment.wide",
        "home.segment.classic",
        "home.segment.infoData",
        "home.segment.infoFlow",
        "home.segment.infoCompare",
        "home.tools.screenshot",
        "home.tools.link",
        "home.tools.figma",
        "home.tools.soon",
        "home.submit.start",
        "home.submit.connect",
        "home.preview.eyebrow",
        "home.preview.sticker",
        "home.preview.use",
        "home.replace.title",
        "home.replace.keep",
        "home.replace.use",
        "home.explore.title",
        "home.explore.sub",
        "home.explore.view",
        "home.explore.knowledgeDesc",
        "home.explore.tutorialDesc",
        "home.explore.posterDesc",
        "home.recent.title",
        "home.recent.empty",
        "home.recent.newCanvas",
    ] {
        keys.push(extra.to_string());
    }
    type Lookup = fn(&str) -> Option<&'static str>;
    let tables: [(&str, Lookup); 15] = [
        ("en", super::en::lookup),
        ("zh_cn", super::zh_cn::lookup),
        ("zh_tw", super::zh_tw::lookup),
        ("ja", super::ja::lookup),
        ("ko", super::ko::lookup),
        ("fr", super::fr::lookup),
        ("es", super::es::lookup),
        ("de", super::de::lookup),
        ("pt", super::pt::lookup),
        ("ru", super::ru::lookup),
        ("hi", super::hi::lookup),
        ("tr", super::tr::lookup),
        ("th", super::th::lookup),
        ("vi", super::vi::lookup),
        ("id", super::id::lookup),
    ];
    for key in &keys {
        for (name, lookup) in tables {
            assert!(
                lookup(key).is_some(),
                "locale `{name}` is missing direct Studio Home key `{key}`"
            );
        }
    }
}
