//! Tests for the DeepSeek Harness (dsh) MCP integration — the
//! `cordis.patch.yml` marker-block writer, its refusal paths, and
//! round-trip detection.
//!
//! Split out of `mcp_integrations_tests.rs` so that file (and this one)
//! stay under the repo's 800-line-per-file cap.

use super::*;

fn temp_home(name: &str) -> PathBuf {
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("test");
    let safe_thread_name: String = thread_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let path = std::env::temp_dir().join(format!(
        "openpencil-mcp-{name}-{}-{}",
        std::process::id(),
        safe_thread_name
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("create temp home");
    path
}

fn dsh_patch_path(home: &Path) -> PathBuf {
    home.join(".dsh").join("cordis.patch.yml")
}

/// Index of `cli` in `McpCli::ALL` — the positional `mcp_cli_enabled` /
/// `detect_enabled_clis_*` slot.
fn cli_index(cli: McpCli) -> usize {
    McpCli::ALL
        .iter()
        .position(|candidate| *candidate == cli)
        .expect("CLI is registered in McpCli::ALL")
}

#[test]
fn dsh_config_path_uses_the_dot_dsh_patch_file() {
    let home = Path::new("/test/home");
    assert_eq!(
        config_path(McpCli::Dsh, home, false),
        home.join(".dsh").join("cordis.patch.yml")
    );
}

#[test]
fn dsh_enable_creates_patch_file_with_managed_block() {
    let home = temp_home("dsh-create");
    let path = dsh_patch_path(&home);

    set_cli_enabled_at_home(McpCli::Dsh, true, 4100, &home).expect("install");

    let text = fs::read_to_string(&path).expect("file created");
    assert!(text.contains("# openpencil-mcp-begin"), "{text}");
    assert!(text.contains("# openpencil-mcp-end"), "{text}");
    assert!(text.contains("id: mcp-openpencil"), "{text}");
    assert!(
        text.contains("name: '@deepseek-ai/dsh-mcp-client'"),
        "{text}"
    );
    assert!(text.contains("serverName: openpencil"), "{text}");
    assert!(text.contains("transport: streamable-http"), "{text}");
    assert!(text.contains("url: http://127.0.0.1:4100/mcp"), "{text}");
    assert!(
        detect_enabled_clis_at_home(&home)[cli_index(McpCli::Dsh)],
        "enabled dsh must be detected"
    );

    let _ = fs::remove_dir_all(home);
}

/// The shape `dsh create` seeds under `~/.dsh/profiles/*/cordis.patch.yml`:
/// a `[]` placeholder with surrounding comments.
#[test]
fn dsh_enable_replaces_empty_array_line_and_keeps_comments() {
    let home = temp_home("dsh-empty-array");
    let path = dsh_patch_path(&home);
    fs::create_dir_all(path.parent().expect("parent")).expect("create dsh dir");
    fs::write(&path, "# sample patch layer\n[]\n# trailing comment\n")
        .expect("seed default patch file");

    set_cli_enabled_at_home(McpCli::Dsh, true, 4200, &home).expect("install");

    let text = fs::read_to_string(&path).expect("read installed");
    assert!(text.contains("# sample patch layer"), "{text}");
    assert!(text.contains("# trailing comment"), "{text}");
    assert!(
        !text.lines().any(|line| line.trim() == "[]"),
        "the empty array placeholder must be replaced: {text}"
    );
    assert!(text.contains("url: http://127.0.0.1:4200/mcp"), "{text}");
    assert!(dsh_config_has_openpencil(&path));

    let _ = fs::remove_dir_all(home);
}

#[test]
fn dsh_enable_appends_after_user_entries_verbatim() {
    let home = temp_home("dsh-append");
    let path = dsh_patch_path(&home);
    fs::create_dir_all(path.parent().expect("parent")).expect("create dsh dir");
    let user_entry = "- insert:\n    - id: user-plugin\n      name: '@example/some-plugin'\n      config:\n        key: value\n";
    fs::write(&path, user_entry).expect("seed user entries");

    set_cli_enabled_at_home(McpCli::Dsh, true, 4300, &home).expect("install");

    let text = fs::read_to_string(&path).expect("read installed");
    assert!(
        text.starts_with(user_entry),
        "user entries must stay byte-identical: {text}"
    );
    assert!(
        text.contains("\n\n# openpencil-mcp-begin"),
        "the managed block must be separated by a blank line: {text}"
    );
    assert!(text.contains("url: http://127.0.0.1:4300/mcp"), "{text}");

    let _ = fs::remove_dir_all(home);
}

#[test]
fn dsh_disable_removes_only_the_managed_block() {
    let home = temp_home("dsh-disable");
    let path = dsh_patch_path(&home);
    fs::create_dir_all(path.parent().expect("parent")).expect("create dsh dir");
    let before = "- insert:\n    - id: user-plugin\n      name: '@example/some-plugin'\n      config:\n        key: value\n";
    let managed = "# openpencil-mcp-begin (managed by OpenPencil; do not edit)\n- insert:\n    - id: mcp-openpencil\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: openpencil\n        transport: streamable-http\n        url: http://127.0.0.1:4400/mcp\n# openpencil-mcp-end\n";
    fs::write(&path, format!("{before}\n{managed}")).expect("seed installed config");

    set_cli_enabled_at_home(McpCli::Dsh, false, 4400, &home).expect("uninstall");

    let text = fs::read_to_string(&path).expect("read uninstalled");
    assert!(!text.contains("mcp-openpencil"), "{text}");
    assert!(!text.contains("openpencil-mcp-begin"), "{text}");
    assert!(!text.contains("openpencil-mcp-end"), "{text}");
    assert!(
        text.starts_with(before),
        "user entries must stay byte-identical: {text}"
    );
    assert!(!detect_enabled_clis_at_home(&home)[cli_index(McpCli::Dsh)]);

    let _ = fs::remove_dir_all(home);
}

#[test]
fn dsh_disable_refuses_hand_written_entries_and_keeps_detection() {
    for (name, manual) in [
        (
            "with-id",
            "- insert:\n    - id: mcp-openpencil\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: openpencil\n        transport: streamable-http\n        url: http://127.0.0.1:3000/mcp\n",
        ),
        (
            "server-name-only",
            "- insert:\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: openpencil\n        transport: streamable-http\n        url: http://127.0.0.1:3000/mcp\n",
        ),
    ] {
        let home = temp_home(&format!("dsh-manual-{name}"));
        let path = dsh_patch_path(&home);
        fs::create_dir_all(path.parent().expect("parent")).expect("create dsh dir");
        fs::write(&path, manual).expect("seed hand-written entry");

        let error = set_cli_enabled_at_home(McpCli::Dsh, false, 3000, &home)
            .expect_err("hand-written entries must never be deleted");
        assert!(
            matches!(error, McpConfigError::DshManualEntry { .. }),
            "{error:?}"
        );
        let message = error.to_string();
        assert!(message.contains("manually"), "{message}");
        assert!(message.contains(path.to_str().expect("utf8 path")), "{message}");
        assert_eq!(
            fs::read_to_string(&path).expect("read untouched"),
            manual,
            "the refused disable must not modify the file"
        );
        assert!(
            detect_enabled_clis_at_home(&home)[cli_index(McpCli::Dsh)],
            "a hand-wired dsh must still be detected"
        );

        let _ = fs::remove_dir_all(home);
    }
}

#[test]
fn dsh_lone_marker_is_refused_not_rewritten() {
    let home = temp_home("dsh-lone-marker");
    let path = dsh_patch_path(&home);
    fs::create_dir_all(path.parent().expect("parent")).expect("create dsh dir");
    let input = "# openpencil-mcp-begin (managed by OpenPencil; do not edit)\n- insert:\n    - id: mcp-openpencil\n";
    fs::write(&path, input).expect("seed lone-marker file");

    let error = set_cli_enabled_at_home(McpCli::Dsh, true, 4700, &home)
        .expect_err("a lone marker is a hand-edited block; refuse to guess");
    assert!(
        matches!(error, McpConfigError::DshPatchMarkersMismatched { .. }),
        "{error:?}"
    );
    assert_eq!(fs::read_to_string(&path).expect("read untouched"), input);

    let _ = fs::remove_dir_all(home);
}

#[test]
fn dsh_reenable_rewrites_url_without_duplicating_the_block() {
    let home = temp_home("dsh-reenable");
    let path = dsh_patch_path(&home);

    set_cli_enabled_at_home(McpCli::Dsh, true, 4500, &home).expect("install");
    set_cli_enabled_at_home(McpCli::Dsh, true, 4600, &home).expect("re-enable on new port");

    let text = fs::read_to_string(&path).expect("read re-enabled");
    assert_eq!(text.matches("- insert:").count(), 1, "{text}");
    assert_eq!(text.matches("id: mcp-openpencil").count(), 1, "{text}");
    assert_eq!(
        text.matches("url: http://127.0.0.1:4600/mcp").count(),
        1,
        "{text}"
    );
    assert!(!text.contains("4500"), "stale port must be gone: {text}");

    let _ = fs::remove_dir_all(home);
}
