//! Lossless TOML editing and crash-safe config writes for MCP integrations.

use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use toml_edit::{value, DocumentMut, InlineTable, Item, Table, Value};

const SERVER_NAME: &str = "openpencil";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(crate) struct FileSnapshot {
    bytes: Option<Vec<u8>>,
    permissions: Option<Permissions>,
}

impl FileSnapshot {
    pub(crate) fn capture(path: &Path) -> Result<Self, String> {
        match fs::read(path) {
            Ok(bytes) => {
                let permissions = fs::metadata(path)
                    .map(|metadata| metadata.permissions())
                    .map_err(|error| format!("metadata {}: {error}", path.display()))?;
                Ok(Self {
                    bytes: Some(bytes),
                    permissions: Some(permissions),
                })
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self {
                bytes: None,
                permissions: None,
            }),
            Err(error) => Err(format!("read {}: {error}", path.display())),
        }
    }

    pub(crate) fn restore(&self, path: &Path) -> Result<(), String> {
        match &self.bytes {
            Some(bytes) => atomic_write_with_permissions(path, bytes, self.permissions.clone()),
            None => match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!("remove {}: {error}", path.display())),
            },
        }
    }
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    atomic_write_with_permissions(path, bytes, permissions)
}

fn atomic_write_with_permissions(
    path: &Path,
    bytes: &[u8],
    permissions: Option<Permissions>,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;

    let temp_path = unique_temp_path(path, parent);
    let result = (|| {
        let mut temp = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .map_err(|error| format!("create {}: {error}", temp_path.display()))?;
        temp.write_all(bytes)
            .map_err(|error| format!("write {}: {error}", temp_path.display()))?;
        temp.sync_all()
            .map_err(|error| format!("sync {}: {error}", temp_path.display()))?;
        drop(temp);

        if let Some(permissions) = permissions {
            fs::set_permissions(&temp_path, permissions)
                .map_err(|error| format!("permissions {}: {error}", temp_path.display()))?;
        }
        replace_file(&temp_path, path)?;
        sync_parent(parent);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(not(windows))]
fn replace_file(temp_path: &Path, path: &Path) -> Result<(), String> {
    fs::rename(temp_path, path).map_err(|error| {
        format!(
            "replace {} with {}: {error}",
            path.display(),
            temp_path.display()
        )
    })
}

/// Windows `std::fs::rename` cannot replace an existing destination. Use the
/// OS replace primitives instead; unlike remove-then-rename, neither path
/// leaves a window where the user's config file is absent.
#[cfg(windows)]
fn replace_file(temp_path: &Path, path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    const REPLACEFILE_IGNORE_MERGE_ERRORS: u32 = 0x2;
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "Kernel32")]
    #[allow(non_snake_case)]
    extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let temp = wide(temp_path);
    let destination = wide(path);
    // SAFETY: both buffers are NUL-terminated and live for the duration of the
    // calls. The optional pointer parameters are documented as nullable.
    let replaced = unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            temp.as_ptr(),
            ptr::null(),
            REPLACEFILE_IGNORE_MERGE_ERRORS,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if replaced != 0 {
        return Ok(());
    }

    // ReplaceFileW requires the destination to exist. MoveFileExW covers a
    // first write and races where the destination appeared/disappeared, while
    // still asking Windows for an atomic replacement and write-through flush.
    let moved = unsafe {
        MoveFileExW(
            temp.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved != 0 {
        Ok(())
    } else {
        Err(format!(
            "replace {} with {}: {}",
            path.display(),
            temp_path.display(),
            io::Error::last_os_error()
        ))
    }
}

fn unique_temp_path(path: &Path, parent: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".{file_name}.openpencil-tmp-{}-{sequence}",
        std::process::id()
    ))
}

fn sync_parent(parent: &Path) {
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
}

pub(crate) fn update_grok_config(
    path: &Path,
    enabled: bool,
    server_url: &str,
) -> Result<(), String> {
    if !enabled && !path.exists() {
        return Ok(());
    }
    let existing = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("read {}: {error}", path.display())),
    };
    let mut document = existing
        .parse::<DocumentMut>()
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    edit_grok_server(&mut document, enabled, server_url)?;
    atomic_write(path, document.to_string().as_bytes())
}

pub(crate) fn grok_config_has_openpencil(input: &str) -> bool {
    let Ok(document) = input.parse::<DocumentMut>() else {
        return false;
    };
    let Some(servers) = document
        .as_table()
        .get("mcp_servers")
        .and_then(Item::as_table_like)
    else {
        return false;
    };
    let Some(server) = servers.get(SERVER_NAME).and_then(Item::as_table_like) else {
        return false;
    };
    if server.get("enabled").and_then(Item::as_bool) == Some(false) {
        return false;
    }
    server
        .get("url")
        .and_then(Item::as_str)
        .map(is_usable_http_url)
        .unwrap_or(false)
}

fn edit_grok_server(
    document: &mut DocumentMut,
    enabled: bool,
    server_url: &str,
) -> Result<(), String> {
    let root = document.as_table_mut();
    if !root.contains_key("mcp_servers") {
        if !enabled {
            return Ok(());
        }
        root.insert("mcp_servers", Item::Table(Table::new()));
    }
    let servers_item = root
        .get_mut("mcp_servers")
        .ok_or_else(|| "mcp_servers is missing".to_string())?;
    let inline = servers_item.is_inline_table();
    let servers = servers_item
        .as_table_like_mut()
        .ok_or_else(|| "mcp_servers must be a table".to_string())?;

    if enabled {
        let server = if inline {
            let mut table = InlineTable::new();
            table.insert("url", Value::from(server_url));
            Item::Value(Value::InlineTable(table))
        } else {
            let mut table = Table::new();
            table.insert("url", value(server_url));
            Item::Table(table)
        };
        servers.insert(SERVER_NAME, server);
    } else {
        servers.remove(SERVER_NAME);
    }
    Ok(())
}

fn is_usable_http_url(input: &str) -> bool {
    reqwest::Url::parse(input)
        .map(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "openpencil-mcp-io-{name}-{}-{}",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp directory");
        path.join("config.toml")
    }

    #[test]
    fn grok_inline_table_round_trip_preserves_comments_and_other_servers() {
        let path = temp_path("inline");
        let input = "# keep this comment\nmcp_servers = { openpencil = { url = \"http://old/mcp\", headers = { Authorization = \"stale\" } }, other = { url = \"https://example.com/mcp\" } } # keep tail\n";
        fs::write(&path, input).expect("seed config");

        update_grok_config(&path, true, "http://127.0.0.1:3400/mcp").expect("enable");
        let enabled = fs::read_to_string(&path).expect("read enabled config");
        assert!(grok_config_has_openpencil(&enabled), "{enabled}");
        assert!(enabled.contains("# keep this comment"), "{enabled}");
        assert!(enabled.contains("# keep tail"), "{enabled}");
        assert!(enabled.contains("other"), "{enabled}");
        assert!(!enabled.contains("stale"), "{enabled}");

        update_grok_config(&path, false, "http://unused").expect("disable");
        let disabled = fs::read_to_string(&path).expect("read disabled config");
        assert!(!grok_config_has_openpencil(&disabled), "{disabled}");
        assert!(disabled.contains("other"), "{disabled}");
        assert!(disabled.contains("# keep this comment"), "{disabled}");
        let _ = fs::remove_dir_all(path.parent().expect("parent"));
    }

    #[test]
    fn atomic_write_replaces_an_existing_file_without_temp_debris() {
        let path = temp_path("atomic-replace");
        fs::write(&path, b"old").expect("seed config");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, Permissions::from_mode(0o600)).expect("set permissions");
        }
        atomic_write(&path, b"new").expect("replace config");
        assert_eq!(fs::read(&path).expect("read config"), b"new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
                0o600
            );
        }
        let parent = path.parent().expect("parent");
        let names: Vec<_> = fs::read_dir(parent)
            .expect("read parent")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(names, vec![path.file_name().expect("file name")]);
        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn grok_dotted_and_normal_tables_stay_valid() {
        for (name, input) in [
            (
                "dotted",
                "mcp_servers.openpencil.url = \"http://old/mcp\"\nmcp_servers.other.url = \"https://example.com/mcp\"\n",
            ),
            (
                "normal",
                "[mcp_servers.openpencil]\nurl = \"http://old/mcp\"\n\n[mcp_servers.other]\nurl = \"https://example.com/mcp\"\n",
            ),
        ] {
            let path = temp_path(name);
            fs::write(&path, input).expect("seed config");
            update_grok_config(&path, true, "http://127.0.0.1:3401/mcp").expect("enable");
            let text = fs::read_to_string(&path).expect("read config");
            text.parse::<DocumentMut>().expect("valid TOML");
            assert!(grok_config_has_openpencil(&text), "{text}");
            assert!(text.contains("other"), "{text}");
            let _ = fs::remove_dir_all(path.parent().expect("parent"));
        }
    }

    #[test]
    fn grok_detection_rejects_missing_or_invalid_urls() {
        for input in [
            "[mcp_servers.openpencil]\nurl = \"not-a-url\"\n",
            "mcp_servers = { openpencil = { url = \"file:///tmp/mcp\" } }\n",
            "[mcp_servers.openpencil]\ncommand = \"mcp-server\"\n",
            "[mcp_servers.openpencil]\nurl = \"https://example.com/mcp\"\nenabled = false\n",
        ] {
            assert!(!grok_config_has_openpencil(input), "{input}");
        }
        assert!(grok_config_has_openpencil(
            "mcp_servers = { openpencil = { url = \"https://example.com/mcp\" } }\n"
        ));
    }
}
