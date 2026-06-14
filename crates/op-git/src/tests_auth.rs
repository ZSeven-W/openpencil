//! Credential + SSH-key store tests: key generate / list / load /
//! delete / import, name-validation hardening, and the JSON-backed
//! `AuthStore` (set / get / replace / remove / persistence / perms).

use crate::tests::{ssh_keygen_available, unique_temp_dir};
use crate::{AuthStore, Credential, SshKeyStore};

#[test]
fn ssh_key_generate_list_load_and_delete() {
    if !ssh_keygen_available() {
        return;
    }
    let dir = unique_temp_dir("ssh");
    let store = SshKeyStore::at(&dir);
    assert!(store.list().expect("list empty").is_empty());

    let key = store.generate("id_test", "op@test").expect("generate");
    assert_eq!(key.name, "id_test");
    assert!(key.public_key.starts_with("ssh-ed25519 "));
    assert!(key.public_key.contains("op@test"));
    assert!(key.private_path.is_file());

    let listed = store.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], key);

    // Re-generating an existing key name is refused (no overwrite).
    assert!(store.generate("id_test", "x").is_err());

    store.delete("id_test").expect("delete");
    assert!(store.list().expect("list after delete").is_empty());
    // Deleting a missing key is a tolerated no-op.
    store.delete("id_test").expect("delete is idempotent");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn auth_store_set_get_replace_remove_and_persist() {
    let dir = unique_temp_dir("auth");
    let path = dir.join("git-auth.json");
    let store = AuthStore::at(&path);

    // Empty store.
    assert!(store.hosts().expect("hosts").is_empty());
    assert!(store.get("github.com").expect("get").is_none());

    // Store an HTTPS credential.
    let https = Credential::Https {
        username: "octocat".to_string(),
        token: "ghp_secret".to_string(),
    };
    store.set("github.com", https.clone()).expect("set https");
    assert_eq!(store.get("github.com").expect("get"), Some(https));

    // A second host with an SSH credential; both must survive a
    // reload from a fresh handle on the same file.
    store
        .set(
            "git.example.com",
            Credential::Ssh {
                key_name: "id_work".to_string(),
            },
        )
        .expect("set ssh");
    let reloaded = AuthStore::at(&path);
    assert_eq!(
        reloaded.hosts().expect("hosts"),
        vec!["git.example.com".to_string(), "github.com".to_string()]
    );

    // Re-setting a host replaces its credential.
    store
        .set(
            "github.com",
            Credential::Ssh {
                key_name: "id_personal".to_string(),
            },
        )
        .expect("replace");
    assert_eq!(
        store.get("github.com").expect("get"),
        Some(Credential::Ssh {
            key_name: "id_personal".to_string()
        })
    );

    // Remove — and removing an absent host is a tolerated no-op.
    store.remove("github.com").expect("remove");
    assert!(store.get("github.com").expect("get").is_none());
    store.remove("github.com").expect("remove is idempotent");
    assert_eq!(
        store.hosts().expect("hosts"),
        vec!["git.example.com".to_string()]
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn auth_store_file_is_created_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = unique_temp_dir("auth-perms");
    let path = dir.join("git-auth.json");
    let store = AuthStore::at(&path);
    store
        .set(
            "host",
            Credential::Ssh {
                key_name: "k".to_string(),
            },
        )
        .expect("set");
    // The store holds secrets — it must never be world-readable,
    // not even momentarily during the write.
    let mode = std::fs::metadata(&path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "credential store must be owner-only (0600)");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn auth_store_at_accepts_non_utf8_file_names() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = unique_temp_dir("auth-non-utf8");
    let path = dir.join(OsString::from_vec(b"git-auth-\xff.json".to_vec()));
    let store = AuthStore::at(&path);

    store
        .set(
            "host",
            Credential::Ssh {
                key_name: "id_non_utf8".to_string(),
            },
        )
        .expect("set non-utf8 path");

    let reloaded = AuthStore::at(&path);
    assert_eq!(
        reloaded.get("host").expect("get non-utf8 path"),
        Some(Credential::Ssh {
            key_name: "id_non_utf8".to_string()
        })
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ssh_key_store_rejects_directory_escaping_names() {
    // No `ssh-keygen` needed: name validation runs first, before any
    // filesystem work or subprocess.
    let store = SshKeyStore::at(unique_temp_dir("ssh-reject"));
    for bad in ["..", ".", "", "../escape", "sub/key", "/abs/key", "a\\b"] {
        assert!(
            store.generate(bad, "c").is_err(),
            "generate must reject `{bad}`"
        );
        assert!(store.load(bad).is_err(), "load must reject `{bad}`");
        assert!(store.delete(bad).is_err(), "delete must reject `{bad}`");
        assert!(
            store
                .import(std::path::Path::new("/nonexistent"), bad)
                .is_err(),
            "import must reject `{bad}`"
        );
    }
}

#[test]
fn ssh_key_import_copies_the_pair() {
    if !ssh_keygen_available() {
        return;
    }
    // Generate a key in a "source" store, then import it into a
    // separate store under a new name.
    let src_dir = unique_temp_dir("ssh-src");
    let src = SshKeyStore::at(&src_dir);
    let original = src.generate("orig", "src@test").expect("generate");

    let dst_dir = unique_temp_dir("ssh-dst");
    let dst = SshKeyStore::at(&dst_dir);
    let imported = dst
        .import(&original.private_path, "imported")
        .expect("import");

    assert_eq!(imported.name, "imported");
    // The public key is carried over verbatim.
    assert_eq!(imported.public_key, original.public_key);
    assert!(dst_dir.join("imported").is_file());
    assert!(dst_dir.join("imported.pub").is_file());

    for dir in [src_dir, dst_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}
