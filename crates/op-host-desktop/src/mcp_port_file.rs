//! Writes `~/.openpencil/.op-mcp-port` so the Rust `op` CLI can discover
//! this running editor's live MCP server.
//!
//! This is a SEPARATE file from the TS app's `~/.openpencil/.port` because
//! the two hosts speak different wire protocols: the Rust editor serves
//! JSON-RPC at `/mcp`, while the TS app serves REST at `/api/mcp/*`. To make
//! the two cross-discoverable (transport-unification, step 4 of that plan),
//! the record now carries an explicit `transport` field (`"json-rpc"` here;
//! the TS writer publishes `"rest"`), so a CLI that finds the *other*
//! stack's file can read `transport` and speak the right protocol instead
//! of mis-routing. The content is JSON
//! `{port, pid, writerPid, token, timestamp, transport}`; the CLI reads
//! `port`/`pid`/`token`/`transport` and always confirms identity with a
//! `ping` (matching `token`) before trusting it or terminating the pid. We
//! only ever remove a file we ourselves wrote (`writerPid` guard) so a
//! second instance can't delete the live instance's discovery file.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const PORT_FILE_DIR: &str = ".openpencil";
const PORT_FILE_NAME: &str = ".op-mcp-port";

fn port_file_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(PORT_FILE_DIR).join(PORT_FILE_NAME))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Publish the live MCP server on `port` (with identity `token`) under
/// `~/.openpencil/.op-mcp-port`. Best-effort: failures are non-fatal
/// (discovery falls back to the fixed default port on the CLI side).
pub(crate) fn write(port: u16, token: &str) {
    let Some(path) = port_file_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let pid = std::process::id();
    let body = port_record_json(port, pid, token, now_millis());
    let _ = std::fs::write(&path, body);
}

/// Build the discovery-file JSON record. Pure (no I/O / no env) so it's
/// unit-testable. `transport: "json-rpc"` advertises this host's wire
/// protocol for cross-stack discovery (the TS writer publishes `"rest"`).
pub(crate) fn port_record_json(port: u16, pid: u32, token: &str, timestamp: u64) -> String {
    serde_json::json!({
        "port": port,
        "pid": pid,
        "writerPid": pid,
        "token": token,
        "timestamp": timestamp,
        "transport": "json-rpc",
    })
    .to_string()
}

/// Remove `~/.openpencil/.op-mcp-port`, but only if we wrote it (matching
/// `writerPid`). A missing or foreign file is left untouched.
pub(crate) fn remove() {
    let Some(path) = port_file_path() else {
        return;
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return;
    };
    let writer_pid = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get("writerPid").and_then(serde_json::Value::as_u64));
    if writer_pid == Some(u64::from(std::process::id())) {
        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_record_advertises_json_rpc_transport() {
        let record = port_record_json(3100, 4242, "tok-1", 1_700_000_000_000);
        let value: serde_json::Value = serde_json::from_str(&record).expect("valid json");
        assert_eq!(value["port"], 3100);
        assert_eq!(value["pid"], 4242);
        assert_eq!(value["writerPid"], 4242);
        assert_eq!(value["token"], "tok-1");
        assert_eq!(value["timestamp"], 1_700_000_000_000u64);
        // The cross-stack discovery marker: a CLI reading this file knows to
        // speak JSON-RPC (vs the TS app's `.port` which carries "rest").
        assert_eq!(value["transport"], "json-rpc");
    }
}
