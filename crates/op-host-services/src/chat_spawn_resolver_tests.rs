use super::*;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let serial = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "openpencil-chat-spawn-resolver-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create resolver test directory");
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn resolver_prefers_path_and_preserves_the_selected_path() {
    let temp = TestDir::new();
    let path_dir = temp.0.join("path-bin");
    let fallback_dir = temp.0.join("fallback-bin");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&fallback_dir).unwrap();
    let path_cli = path_dir.join("codex");
    let fallback_cli = fallback_dir.join("codex");
    fs::write(&path_cli, "").unwrap();
    fs::write(&fallback_cli, "").unwrap();
    let path_env = std::env::join_paths([&path_dir]).unwrap();

    assert_eq!(
        resolve_binary_from("codex", &path_env, &[fallback_cli]),
        Some(path_cli)
    );
}

#[cfg(unix)]
#[test]
fn resolver_does_not_canonicalize_a_fallback_symlink() {
    use std::os::unix::fs::symlink;

    let temp = TestDir::new();
    let target = temp.0.join("real-codex");
    let link = temp.0.join("codex");
    fs::write(&target, "").unwrap();
    symlink(&target, &link).unwrap();

    assert_eq!(
        resolve_binary_from("codex", OsStr::new(""), std::slice::from_ref(&link)),
        Some(link)
    );
}

#[cfg(unix)]
#[test]
fn resolver_keeps_the_full_unix_fallback_set() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let candidates = well_known_install_paths("codex");
    let expected = [
        ".local/bin",
        ".bun/bin",
        ".volta/bin",
        ".local/share/mise/shims",
        ".asdf/shims",
        "Library/pnpm",
        ".pnpm-global/bin",
        ".cargo/bin",
        ".opencode/bin",
        ".npm-global/bin",
        "node_modules/.bin",
        ".yarn/bin",
    ];
    for rel in expected {
        let candidate = home.join(rel).join("codex");
        assert!(
            candidates.contains(&candidate),
            "missing fallback candidate {candidate:?}"
        );
    }
    assert_eq!(candidates.first(), Some(&home.join(".local/bin/codex")));
    assert!(candidates.contains(&PathBuf::from("/usr/local/bin/codex")));
    assert!(candidates.contains(&PathBuf::from("/opt/homebrew/bin/codex")));
}

#[test]
fn runtime_path_leads_with_selected_parent_and_keeps_other_order() {
    let root = std::env::temp_dir().join("openpencil-runtime-path-test");
    let wrong_node = root.join("wrong-node");
    let selected = root.join("selected");
    let tail = root.join("tail");
    let base = std::env::join_paths([&wrong_node, &selected, &tail, &selected]).unwrap();

    let actual: Vec<_> =
        std::env::split_paths(&runtime_path_for(&selected.join("codex"), &base)).collect();
    assert_eq!(actual, vec![selected, wrong_node, tail]);
}

#[test]
fn runtime_path_for_bare_binary_keeps_base_unchanged() {
    let base = std::env::join_paths([std::env::temp_dir().join("bin")]).unwrap();
    assert_eq!(runtime_path_for(Path::new("codex"), &base), base);
}
