//! DSH profile discovery, migration, and all-or-nothing configuration tests.

use super::*;
use std::ffi::OsStr;
use std::sync::atomic::{AtomicU64, Ordering};

const FULL_URL: &str = "http://127.0.0.1:4100/mcp";
const LEAN_URL: &str = "http://127.0.0.1:4200/mcp/lean";
const BEGIN: &str = "# openpencil-mcp-begin (managed by OpenPencil; do not edit)";
const END: &str = "# openpencil-mcp-end";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let thread = std::thread::current();
        let name: String = thread
            .name()
            .unwrap_or("test")
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect();
        let path = std::env::temp_dir().join(format!(
            "openpencil-dsh-profile-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create unique fixture");
        Self(path)
    }

    fn profile(&self, name: &str, manifest: &str) -> PathBuf {
        let directory = self.0.join("profiles").join(name);
        fs::create_dir_all(&directory).expect("create profile");
        fs::write(directory.join("package.json"), manifest).expect("write manifest");
        directory.join("cordis.patch.yml")
    }

    fn home_patch(&self) -> PathBuf {
        self.0.join("cordis.patch.yml")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn managed(url: &str) -> String {
    format!(
        "{BEGIN}\n- insert:\n    - id: mcp-openpencil\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: openpencil\n        transport: streamable-http\n        url: {url}\n{END}\n"
    )
}

fn contents(path: &Path) -> String {
    fs::read_to_string(path).expect("read fixture")
}

fn assert_one_endpoint(path: &Path, url: &str) {
    let text = contents(path);
    assert_eq!(text.matches(BEGIN).count(), 1, "{text}");
    assert_eq!(text.matches("id: mcp-openpencil").count(), 1, "{text}");
    assert_eq!(text.matches(&format!("url: {url}")).count(), 1, "{text}");
}

#[test]
fn dsh_root_resolves_explicit_override_without_process_environment() {
    let home = Path::new("/test/home");
    assert_eq!(dsh_root(home, None), home.join(".dsh"));
    assert_eq!(
        dsh_root(home, Some(OsStr::new("/custom/dsh"))),
        PathBuf::from("/custom/dsh")
    );
}

#[test]
fn profile_discovery_accepts_plain_package_objects_and_ignores_other_directories() {
    let fixture = Fixture::new();
    let zeta = fixture.profile("zeta", r#"{"dsh":{"profile":{"bundles":[]}}}"#);
    let alpha = fixture.profile("alpha", "{}");
    fixture.profile("node_modules", "{}");
    fixture.profile("array", "[]");
    fixture.profile("broken", "not JSON");
    fs::create_dir(fixture.0.join("profiles/no-manifest")).expect("unrelated directory");
    fs::write(fixture.0.join("profiles/loose-file"), "{}").expect("unrelated file");

    assert_eq!(
        profile_patch_paths(&fixture.0).expect("discover profiles"),
        vec![alpha, zeta]
    );
}

#[test]
fn no_profiles_uses_supported_home_layer_without_creating_a_profile() {
    let fixture = Fixture::new();
    let path = set_dsh_enabled(&fixture.0, true, FULL_URL).expect("enable fallback");
    assert_eq!(path, fixture.home_patch());
    assert_one_endpoint(&path, FULL_URL);
    assert!(dsh_root_has_openpencil(&fixture.0));
    assert!(!fixture.0.join("profiles").exists());

    set_dsh_enabled(&fixture.0, false, FULL_URL).expect("disable fallback");
    assert_eq!(contents(&path).trim(), "[]");
    assert!(!dsh_root_has_openpencil(&fixture.0));
}

#[test]
fn all_profiles_enable_update_lean_and_disable_without_changing_user_entries() {
    let fixture = Fixture::new();
    let desktop = fixture.profile("desktop", "{}");
    let terminal = fixture.profile("terminal", "{}");
    let user = "# user's plugin\n- insert:\n    - id: my-plugin\n      name: '@example/tool'\n";
    fs::write(&desktop, user).expect("seed user entries");
    fs::write(&terminal, "# terminal notes\n[]\n").expect("seed empty layer");

    let representative = set_dsh_enabled(&fixture.0, true, FULL_URL).expect("enable profiles");
    assert_eq!(representative, desktop);
    assert!(dsh_root_has_openpencil(&fixture.0));
    assert!(!fixture.home_patch().exists());
    for path in [&desktop, &terminal] {
        assert_one_endpoint(path, FULL_URL);
    }

    set_dsh_enabled(&fixture.0, true, LEAN_URL).expect("update port and lean profile");
    for path in [&desktop, &terminal] {
        assert_one_endpoint(path, LEAN_URL);
        assert!(!contents(path).contains(FULL_URL));
    }
    assert!(contents(&desktop).starts_with(user));
    assert!(contents(&terminal).starts_with("# terminal notes\n"));

    set_dsh_enabled(&fixture.0, false, LEAN_URL).expect("disable profiles");
    assert!(!dsh_root_has_openpencil(&fixture.0));
    assert!(contents(&desktop).starts_with(user));
    assert!(contents(&terminal).contains("[]"));
    assert!(!contents(&desktop).contains(BEGIN));
    assert!(!contents(&terminal).contains(BEGIN));
}

#[test]
fn owned_home_block_migrates_once_and_preserves_other_home_entries_exactly() {
    let fixture = Fixture::new();
    let desktop = fixture.profile("desktop", "{}");
    let home = fixture.home_patch();
    let user =
        "# home plugins\n- insert:\n    - id: user-service\n      name: '@example/service'\n";
    fs::write(&home, format!("{user}{}", managed(FULL_URL))).expect("seed legacy layer");
    assert!(
        dsh_root_has_openpencil(&fixture.0),
        "legacy install remains detected"
    );

    set_dsh_enabled(&fixture.0, true, LEAN_URL).expect("migrate");
    assert_one_endpoint(&desktop, LEAN_URL);
    assert_eq!(contents(&home), user);
    set_dsh_enabled(&fixture.0, true, LEAN_URL).expect("repeat migration");
    assert_one_endpoint(&desktop, LEAN_URL);
    assert_eq!(contents(&home), user);
}

#[test]
fn migrating_the_only_home_entry_leaves_a_valid_empty_patch_array() {
    let fixture = Fixture::new();
    fixture.profile("desktop", "{}");
    let home = fixture.home_patch();
    fs::write(&home, format!("# keep this comment\n{}", managed(FULL_URL)))
        .expect("seed legacy layer");

    set_dsh_enabled(&fixture.0, true, FULL_URL).expect("migrate");
    let remaining = contents(&home);
    assert!(
        remaining.starts_with("# keep this comment\n"),
        "{remaining}"
    );
    assert_eq!(
        remaining
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
            .collect::<Vec<_>>(),
        vec!["[]"]
    );
}

#[test]
fn existing_empty_home_layer_is_normalized_without_losing_comments() {
    for original in ["", "# user's home notes\n\n  # keep this indentation\n"] {
        let fixture = Fixture::new();
        let desktop = fixture.profile("desktop", "{}");
        let home = fixture.home_patch();
        fs::write(&home, original).expect("seed existing empty home layer");

        set_dsh_enabled(&fixture.0, true, FULL_URL).expect("enable profile");

        let remaining = contents(&home);
        assert!(remaining.starts_with(original), "{remaining}");
        assert_eq!(
            remaining
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
                .collect::<Vec<_>>(),
            vec!["[]"]
        );
        assert_one_endpoint(&desktop, FULL_URL);
    }
}

#[test]
fn disabling_missing_layers_does_not_create_home_or_profile_patch_files() {
    let fixture = Fixture::new();
    let missing_root = fixture.0.join("not-created");
    set_dsh_enabled(&missing_root, false, FULL_URL).expect("disable absent DSH home");
    assert!(!missing_root.exists());

    set_dsh_enabled(&fixture.0, false, FULL_URL).expect("disable absent fallback patch");
    assert!(!fixture.home_patch().exists());
    assert!(!fixture.0.join("profiles").exists());

    let desktop = fixture.profile("desktop", "{}");
    set_dsh_enabled(&fixture.0, false, FULL_URL).expect("disable absent profile patch");
    assert!(!desktop.exists());
    assert!(!fixture.home_patch().exists());
}

#[test]
fn new_profiles_are_configured_on_the_next_reconciliation() {
    let fixture = Fixture::new();
    let desktop = fixture.profile("desktop", "{}");
    set_dsh_enabled(&fixture.0, true, FULL_URL).expect("initial enable");
    let terminal = fixture.profile("terminal", "{}");

    set_dsh_enabled(&fixture.0, true, LEAN_URL).expect("reconcile new profile");
    assert_one_endpoint(&desktop, LEAN_URL);
    assert_one_endpoint(&terminal, LEAN_URL);
    assert!(!fixture.home_patch().exists());
}

#[test]
fn manual_home_or_profile_entries_refuse_changes_before_any_file_is_written() {
    let manual = "- insert:\n    - id: mcp-openpencil\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: openpencil\n        transport: stdio\n        command: custom-openpencil\n";
    for manual_in_home in [false, true] {
        let fixture = Fixture::new();
        let first = fixture.profile("alpha", "{}");
        let second = fixture.profile("zeta", "{}");
        let home = fixture.home_patch();
        fs::write(&first, managed(FULL_URL)).expect("seed managed profile");
        fs::write(&second, if manual_in_home { "[]\n" } else { manual })
            .expect("seed second profile");
        fs::write(
            &home,
            if manual_in_home {
                manual.to_owned()
            } else {
                managed(FULL_URL)
            },
        )
        .expect("seed home layer");
        let before: Vec<_> = [&first, &second, &home]
            .into_iter()
            .map(|p| (p, contents(p)))
            .collect();

        for enabled in [true, false] {
            let error = set_dsh_enabled(&fixture.0, enabled, LEAN_URL)
                .expect_err("manual entries must not be overwritten or duplicated");
            assert!(
                matches!(error, McpConfigError::DshManualEntry { .. }),
                "{error:?}"
            );
            for (path, text) in &before {
                assert_eq!(contents(path), *text);
            }
            assert!(dsh_root_has_openpencil(&fixture.0));
        }
    }
}

#[test]
fn malformed_markers_in_later_profile_leave_every_layer_unchanged() {
    let malformed = [
        format!("{BEGIN}\n- insert: []\n"),
        format!("{END}\n{BEGIN}\n"),
        format!("{BEGIN}\n{BEGIN}\n{END}\n"),
        format!("{BEGIN}\n{END}\n{END}\n"),
    ];
    for broken in malformed {
        let fixture = Fixture::new();
        let first = fixture.profile("alpha", "{}");
        let second = fixture.profile("zeta", "{}");
        let home = fixture.home_patch();
        fs::write(&first, managed(FULL_URL)).expect("seed first profile");
        fs::write(&second, &broken).expect("seed broken profile");
        fs::write(&home, managed(FULL_URL)).expect("seed legacy home");
        for enabled in [true, false] {
            let error = set_dsh_enabled(&fixture.0, enabled, LEAN_URL)
                .expect_err("all layers must be validated first");
            assert!(
                matches!(error, McpConfigError::DshPatchMarkersMismatched { .. }),
                "{error:?}"
            );
            assert_eq!(contents(&first), managed(FULL_URL));
            assert_eq!(contents(&second), broken);
            assert_eq!(contents(&home), managed(FULL_URL));
        }
    }
}

#[test]
fn unreadable_profile_patch_prevents_partial_migration() {
    let fixture = Fixture::new();
    let first = fixture.profile("alpha", "{}");
    let second = fixture.profile("zeta", "{}");
    let home = fixture.home_patch();
    fs::write(&first, managed(FULL_URL)).expect("seed first profile");
    fs::create_dir(&second).expect("a directory cannot be read as a patch file");
    fs::write(&home, managed(FULL_URL)).expect("seed legacy home");

    assert!(set_dsh_enabled(&fixture.0, true, LEAN_URL).is_err());
    assert_eq!(contents(&first), managed(FULL_URL));
    assert_eq!(contents(&home), managed(FULL_URL));
    assert!(second.is_dir());
}

#[test]
fn unsupported_root_yaml_shapes_refuse_changes_before_migrating_any_layer() {
    for user_patch in [
        "[{insert: [{id: user-plugin, name: '@example/tool'}]}]\n",
        "---\n- insert:\n    - id: user-plugin\n      name: '@example/tool'\n",
    ] {
        let fixture = Fixture::new();
        let first = fixture.profile("alpha", "{}");
        let second = fixture.profile("zeta", "{}");
        let home = fixture.home_patch();
        let original = managed(FULL_URL);
        fs::write(&first, &original).expect("seed first profile");
        fs::write(&second, user_patch).expect("seed unsupported patch shape");
        fs::write(&home, &original).expect("seed legacy home");

        for enabled in [true, false] {
            let error = set_dsh_enabled(&fixture.0, enabled, LEAN_URL)
                .expect_err("unsupported YAML must not be concatenated with a managed block");
            assert!(
                matches!(error, McpConfigError::Parse { ref path, .. } if path == &second),
                "{error:?}"
            );
            assert_eq!(contents(&first), original);
            assert_eq!(contents(&second), user_patch);
            assert_eq!(contents(&home), original);
        }
    }
}

#[test]
fn detection_keeps_valid_profiles_enabled_when_another_manifest_cannot_be_read() {
    let fixture = Fixture::new();
    let broken = fixture.profile("alpha", "{}");
    let valid = fixture.profile("zeta", "{}");
    let manifest = broken
        .parent()
        .expect("profile directory")
        .join("package.json");
    fs::write(&manifest, [0xff, 0xfe]).expect("seed invalid UTF-8 manifest");
    fs::write(&broken, "# unrelated profile\n[]\n").expect("seed unrelated patch");
    let original = managed(FULL_URL);
    fs::write(&valid, &original).expect("seed enabled profile");

    assert!(
        dsh_root_has_openpencil(&fixture.0),
        "one unreadable manifest must not turn off a different configured profile"
    );
    for enabled in [true, false] {
        let error = set_dsh_enabled(&fixture.0, enabled, LEAN_URL)
            .expect_err("mutations still require complete profile discovery");
        assert!(
            matches!(error, McpConfigError::Read { ref path, .. } if path == &manifest),
            "{error:?}"
        );
        assert_eq!(contents(&valid), original);
        assert_eq!(contents(&broken), "# unrelated profile\n[]\n");
        assert_eq!(
            fs::read(&manifest).expect("unchanged manifest"),
            [0xff, 0xfe]
        );
        assert!(!fixture.home_patch().exists());
    }
}

#[test]
fn empty_flow_array_variants_support_install_update_and_disable() {
    for initial in ["[] # keep inline comment\n", "[ ]\n"] {
        let fixture = Fixture::new();
        let desktop = fixture.profile("desktop", "{}");
        fs::write(&desktop, initial).expect("seed empty flow array");

        set_dsh_enabled(&fixture.0, true, FULL_URL).expect("install in empty flow array");
        assert_one_endpoint(&desktop, FULL_URL);
        set_dsh_enabled(&fixture.0, true, LEAN_URL).expect("update installed endpoint");
        assert_one_endpoint(&desktop, LEAN_URL);
        set_dsh_enabled(&fixture.0, false, LEAN_URL).expect("disable installed endpoint");
        let remaining = contents(&desktop);
        if initial.contains("# keep inline comment") {
            assert_eq!(remaining.matches("# keep inline comment").count(), 1);
        }
        assert_eq!(
            remaining
                .lines()
                .map(|line| line.split('#').next().expect("line prefix").trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>(),
            vec!["[]"]
        );
        assert!(!remaining.contains(BEGIN));
        assert!(!dsh_root_has_openpencil(&fixture.0));
    }
}

#[test]
fn failed_write_restores_home_and_profiles_and_removes_newly_created_patch() {
    let fixture = Fixture::new();
    let created = fixture.profile("alpha", "{}");
    let existing = fixture.profile("zeta", "{}");
    let home = fixture.home_patch();
    let original_home = format!("# legacy home\n{}", managed(FULL_URL));
    let original_profile = format!("# existing profile\n{}", managed(FULL_URL));
    fs::write(&home, &original_home).expect("seed legacy home");
    fs::write(&existing, &original_profile).expect("seed existing profile");
    let plan = vec![
        plan_write(home.clone(), false, LEAN_URL)
            .expect("plan home removal")
            .expect("home changes"),
        plan_write(created.clone(), true, LEAN_URL)
            .expect("plan new profile")
            .expect("new file changes"),
        plan_write(existing.clone(), true, LEAN_URL)
            .expect("plan existing profile")
            .expect("existing file changes"),
    ];
    let failure = McpConfigError::Write {
        path: existing.clone(),
        message: "injected write failure".into(),
    };
    let mut writes = 0;
    let error = apply_plan(&plan, |path, bytes| {
        writes += 1;
        atomic_write(path, bytes)?;
        if path == existing.as_path() {
            // Even an error after the destination changed must undo that file.
            Err(failure.clone())
        } else {
            Ok(())
        }
    })
    .expect_err("the injected failure must propagate after rollback");

    assert_eq!(writes, 3);
    assert_eq!(error, failure);
    assert_eq!(contents(&home), original_home);
    assert_eq!(contents(&existing), original_profile);
    assert!(
        !created.exists(),
        "rollback must remove a newly created patch"
    );
}

#[test]
fn unchanged_endpoint_does_not_rewrite_the_watched_profile_file() {
    let fixture = Fixture::new();
    let desktop = fixture.profile("desktop", "{}");
    set_dsh_enabled(&fixture.0, true, FULL_URL).expect("enable profile");
    let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    fs::File::options()
        .write(true)
        .open(&desktop)
        .expect("open fixture")
        .set_times(fs::FileTimes::new().set_modified(old))
        .expect("set old modification time");
    let before = fs::metadata(&desktop)
        .expect("metadata")
        .modified()
        .expect("mtime");

    set_dsh_enabled(&fixture.0, true, FULL_URL).expect("idempotent reconciliation");
    assert_eq!(
        fs::metadata(&desktop)
            .expect("metadata")
            .modified()
            .expect("mtime"),
        before
    );
}

#[cfg(unix)]
#[test]
fn profile_symlinks_do_not_write_to_unrelated_directories() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let desktop = fixture.profile("desktop", "{}");
    let external = fixture.0.join("external");
    fs::create_dir(&external).expect("create unrelated directory");
    fs::write(external.join("package.json"), "{}").expect("write unrelated manifest");
    symlink(&external, fixture.0.join("profiles/linked")).expect("link profile directory");

    assert_eq!(
        profile_patch_paths(&fixture.0).expect("discover"),
        vec![desktop.clone()]
    );
    set_dsh_enabled(&fixture.0, true, FULL_URL).expect("enable real profile");
    assert_one_endpoint(&desktop, FULL_URL);
    assert!(!external.join("cordis.patch.yml").exists());
}

#[cfg(unix)]
#[test]
fn home_and_profile_patch_symlinks_are_refused_before_any_changes() {
    use std::os::unix::fs::symlink;

    for linked_home in [true, false] {
        let fixture = Fixture::new();
        let first = fixture.profile("alpha", "{}");
        let second = fixture.profile("zeta", "{}");
        let home = fixture.home_patch();
        let external = fixture.0.join("outside.patch.yml");
        let original = managed(FULL_URL);
        fs::write(&first, &original).expect("seed first profile");
        fs::write(&external, &original).expect("seed symlink target");
        let linked = if linked_home {
            fs::write(&second, &original).expect("seed second profile");
            &home
        } else {
            fs::write(&home, &original).expect("seed home layer");
            &second
        };
        symlink(&external, linked).expect("link patch file");

        for enabled in [true, false] {
            let error = set_dsh_enabled(&fixture.0, enabled, LEAN_URL)
                .expect_err("symlink patch paths must be refused");
            assert!(
                matches!(error, McpConfigError::Read { ref path, .. } if path == linked),
                "{error:?}"
            );
            assert_eq!(contents(&first), original);
            assert_eq!(contents(&second), original);
            assert_eq!(contents(&home), original);
            assert_eq!(contents(&external), original);
            assert!(
                fs::symlink_metadata(linked)
                    .expect("link metadata")
                    .file_type()
                    .is_symlink(),
                "the symlink itself must remain untouched"
            );
        }
    }
}
