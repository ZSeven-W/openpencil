//! Generation-workspace key coverage across every locale table.

/// Every locale table must carry a DIRECT entry for the workspace
/// keys — `translate`'s EN fallback must not mask a missing
/// translation.
#[test]
fn workspace_keys_exist_in_every_locale_table() {
    let keys = [
        "workspace.export",
        "workspace.professional",
        "workspace.phase.generating",
        "workspace.phase.done",
        "workspace.phase.stopped",
        "workspace.phase.failed",
        "workspace.view.all",
        "workspace.view.singleScreen",
        "workspace.view.singlePage",
        "workspace.view.long",
        "workspace.view.overview",
        "workspace.fit",
        "workspace.present",
        "workspace.retry",
        "workspace.returnEdit",
        "workspace.backToWorkspace",
        "workspace.failed.note",
    ];
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
    for key in keys {
        for (name, lookup) in tables {
            assert!(
                lookup(key).is_some(),
                "locale `{name}` is missing direct workspace key `{key}`"
            );
        }
    }
}
