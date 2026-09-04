//! `postMessage` bridge protocol codec between the VS Code extension host
//! and the wasm-backed web editor. Inbound parsing is serde_json-based (a
//! hand-rolled scanner is unreliable for arbitrary JSON, and foreign
//! postMessage traffic — e.g. react-devtools — must be ignored, never
//! treated as an error); outbound events are built with `serde_json::json!`
//! and serialized once per call.
//!
//! Message `type` values — inbound: `op-bridge/init`, `op-bridge/theme`,
//! `op-bridge/locale`, `op-bridge/open-document`, `op-bridge/snapshot`,
//! `op-bridge/save-committed`, `op-bridge/resolve-conflict`;
//! outbound: `op-bridge/ready`, `op-bridge/dirty-changed`, `op-bridge/opened`,
//! `op-bridge/snapshot-result`, `op-bridge/snapshot-conflict`,
//! `op-bridge/sync-conflict`, `op-bridge/conflict-resolved`,
//! `op-bridge/listening`. Field names are camelCase (`requestId`,
//! `serverVersion`, `docJson`).

use serde_json::Value;

use crate::{Locale, ThemeMode};

#[derive(Debug, PartialEq)]
pub enum BridgeInbound {
    Init {
        token: String,
        /// The embedding host's stable MCP endpoint (the VS Code
        /// extension's McpProxy URL) — shown by the MCP settings card
        /// instead of the daemon-internal port. Optional: older hosts
        /// don't send it.
        mcp_url: Option<String>,
    },
    Theme {
        color_scheme: ThemeMode,
    },
    Locale {
        locale: Locale,
    },
    OpenDocument {
        json: String,
    },
    Snapshot {
        purpose: String,
        request_id: String,
    },
    SaveCommitted {
        generation: u64,
        revision: u64,
    },
    ResolveConflict {
        mode: ConflictMode,
        request_id: String,
    },
}

#[derive(Debug, PartialEq)]
pub enum ConflictMode {
    UseLocal,
    AcceptRemote,
}

impl BridgeInbound {
    /// None for non-bridge / malformed messages (foreign postMessage traffic
    /// like react-devtools must be ignored, never an error).
    pub fn parse(raw: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(raw).ok()?;
        let ty = value.get("type")?.as_str()?;
        match ty {
            "op-bridge/init" => {
                let token = value.get("token")?.as_str()?.to_string();
                let mcp_url = value
                    .get("mcpUrl")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                Some(BridgeInbound::Init { token, mcp_url })
            }
            "op-bridge/theme" => {
                let color_scheme = color_scheme_from_wire(value.get("colorScheme")?.as_str()?)?;
                Some(BridgeInbound::Theme { color_scheme })
            }
            "op-bridge/locale" => {
                let locale = locale_from_wire(value.get("locale")?.as_str()?)?;
                Some(BridgeInbound::Locale { locale })
            }
            "op-bridge/open-document" => {
                let json = value.get("json")?.as_str()?.to_string();
                Some(BridgeInbound::OpenDocument { json })
            }
            "op-bridge/snapshot" => {
                let purpose = value.get("purpose")?.as_str()?.to_string();
                let request_id = value.get("requestId")?.as_str()?.to_string();
                Some(BridgeInbound::Snapshot {
                    purpose,
                    request_id,
                })
            }
            "op-bridge/save-committed" => {
                let generation = value.get("generation")?.as_u64()?;
                let revision = value.get("revision")?.as_u64()?;
                Some(BridgeInbound::SaveCommitted {
                    generation,
                    revision,
                })
            }
            "op-bridge/resolve-conflict" => {
                let mode = match value.get("mode")?.as_str()? {
                    "use-local" => ConflictMode::UseLocal,
                    "accept-remote" => ConflictMode::AcceptRemote,
                    _ => return None,
                };
                let request_id = value.get("requestId")?.as_str()?.to_string();
                Some(BridgeInbound::ResolveConflict { mode, request_id })
            }
            _ => None,
        }
    }
}

/// Strictly parse the two color-scheme values accepted from an embedding
/// host. This is public so the web mount can apply the identical contract to
/// its `?theme=` bootstrap hint before the first paint.
pub fn color_scheme_from_wire(raw: &str) -> Option<ThemeMode> {
    match raw {
        "light" => Some(ThemeMode::Light),
        "dark" => Some(ThemeMode::Dark),
        _ => None,
    }
}

/// Strictly parse the two BCP 47 locale values accepted from an embedding host.
pub fn locale_from_wire(raw: &str) -> Option<Locale> {
    match raw {
        "zh-CN" => Some(Locale::ZhCn),
        "en-US" => Some(Locale::EnUs),
        _ => None,
    }
}

pub fn event_ready(generation: u64, revision: u64) -> String {
    serde_json::json!({
        "type": "op-bridge/ready",
        "generation": generation,
        "revision": revision,
    })
    .to_string()
}

/// `op-bridge/listening`: the app-side `message` listener is registered, so
/// the host may (re)send `init`. Posted IMMEDIATELY after the listener
/// installs — before the backend wasm download completes — because
/// `postMessage` to a window without a listener silently drops the message
/// and the host's finite `init` retry burst (~10 s) can expire during a slow
/// mount. The payload is exactly the `type` field: old hosts ignore the
/// unknown type and the app never waits for a reply.
pub fn event_listening() -> String {
    serde_json::json!({
        "type": "op-bridge/listening",
    })
    .to_string()
}

pub fn event_dirty_changed(generation: u64, revision: u64, dirty: bool) -> String {
    serde_json::json!({
        "type": "op-bridge/dirty-changed",
        "generation": generation,
        "revision": revision,
        "dirty": dirty,
    })
    .to_string()
}

pub fn event_opened(generation: u64) -> String {
    serde_json::json!({
        "type": "op-bridge/opened",
        "generation": generation,
    })
    .to_string()
}

pub fn event_snapshot_result(
    request_id: &str,
    doc_json: &str,
    generation: u64,
    revision: u64,
) -> String {
    serde_json::json!({
        "type": "op-bridge/snapshot-result",
        "requestId": request_id,
        "docJson": doc_json,
        "generation": generation,
        "revision": revision,
    })
    .to_string()
}

pub fn event_snapshot_conflict(request_id: &str, server_version: u64) -> String {
    serde_json::json!({
        "type": "op-bridge/snapshot-conflict",
        "requestId": request_id,
        "serverVersion": server_version,
    })
    .to_string()
}

pub fn event_sync_conflict(generation: u64, revision: u64, server_version: u64) -> String {
    serde_json::json!({
        "type": "op-bridge/sync-conflict",
        "generation": generation,
        "revision": revision,
        "serverVersion": server_version,
    })
    .to_string()
}

pub fn event_conflict_resolved(request_id: &str) -> String {
    serde_json::json!({
        "type": "op-bridge/conflict-resolved",
        "requestId": request_id,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inbound_messages() {
        assert_eq!(
            BridgeInbound::parse(r#"{"type":"op-bridge/init","token":"t0k"}"#),
            Some(BridgeInbound::Init {
                token: "t0k".into(),
                mcp_url: None
            })
        );
        assert_eq!(
            BridgeInbound::parse(
                r#"{"type":"op-bridge/init","token":"t0k","mcpUrl":"http://127.0.0.1:9/mcp"}"#
            ),
            Some(BridgeInbound::Init {
                token: "t0k".into(),
                mcp_url: Some("http://127.0.0.1:9/mcp".into())
            })
        );
        assert_eq!(
            BridgeInbound::parse(
                r#"{"type":"op-bridge/save-committed","generation":3,"revision":41}"#
            ),
            Some(BridgeInbound::SaveCommitted {
                generation: 3,
                revision: 41
            })
        );
        assert_eq!(
            BridgeInbound::parse(r#"{"type":"op-bridge/theme","colorScheme":"light"}"#),
            Some(BridgeInbound::Theme {
                color_scheme: ThemeMode::Light
            })
        );
        assert_eq!(
            BridgeInbound::parse(r#"{"type":"op-bridge/theme","colorScheme":"dark"}"#),
            Some(BridgeInbound::Theme {
                color_scheme: ThemeMode::Dark
            })
        );
        assert_eq!(
            BridgeInbound::parse(
                r#"{"type":"op-bridge/resolve-conflict","mode":"use-local","requestId":"r1"}"#
            ),
            Some(BridgeInbound::ResolveConflict {
                mode: ConflictMode::UseLocal,
                request_id: "r1".into()
            })
        );
        assert_eq!(
            BridgeInbound::parse(r#"{"type":"op-bridge/locale","locale":"zh-CN"}"#),
            Some(BridgeInbound::Locale {
                locale: Locale::ZhCn
            })
        );
        assert_eq!(
            BridgeInbound::parse(r#"{"type":"op-bridge/locale","locale":"en-US"}"#),
            Some(BridgeInbound::Locale {
                locale: Locale::EnUs
            })
        );
        assert_eq!(BridgeInbound::parse(r#"{"type":"react-devtools"}"#), None);
        assert_eq!(BridgeInbound::parse("not json"), None);
    }

    #[test]
    fn theme_requires_an_exact_supported_color_scheme() {
        for raw in [
            r#"{"type":"op-bridge/theme"}"#,
            r#"{"type":"op-bridge/theme","colorScheme":null}"#,
            r#"{"type":"op-bridge/theme","colorScheme":1}"#,
            r#"{"type":"op-bridge/theme","colorScheme":"Light"}"#,
            r#"{"type":"op-bridge/theme","colorScheme":"system"}"#,
            r#"{"type":"op-bridge/theme","colorScheme":""}"#,
        ] {
            assert_eq!(BridgeInbound::parse(raw), None, "{raw}");
        }
    }

    #[test]
    fn locale_requires_an_exact_supported_value() {
        for raw in [
            r#"{"type":"op-bridge/locale"}"#,
            r#"{"type":"op-bridge/locale","locale":null}"#,
            r#"{"type":"op-bridge/locale","locale":1}"#,
            r#"{"type":"op-bridge/locale","locale":"zh"}"#,
            r#"{"type":"op-bridge/locale","locale":"en"}"#,
            r#"{"type":"op-bridge/locale","locale":"EN"}"#,
            r#"{"type":"op-bridge/locale","locale":""}"#,
        ] {
            assert_eq!(BridgeInbound::parse(raw), None, "{raw}");
        }
    }

    #[test]
    fn snapshot_result_json_escapes_doc_payload() {
        let ev = event_snapshot_result("r1", r#"{"a":"b \" c"}"#, 2, 17);
        let v: serde_json::Value = serde_json::from_str(&ev).unwrap();
        assert_eq!(v["type"], "op-bridge/snapshot-result");
        assert_eq!(v["docJson"], r#"{"a":"b \" c"}"#);
        assert_eq!(v["generation"], 2);
    }

    #[test]
    fn listening_payload_is_exactly_the_type_field() {
        let ev = event_listening();
        let v: serde_json::Value = serde_json::from_str(&ev).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "op-bridge/listening" }));
        assert_eq!(v.as_object().map(|o| o.len()), Some(1));
    }

    #[test]
    fn listening_is_outbound_only_and_ignored_when_echoed() {
        // The app's own `listening` post is not inbound traffic: a host that
        // echoes it back must not be treated as a bridge message.
        assert_eq!(
            BridgeInbound::parse(r#"{"type":"op-bridge/listening"}"#),
            None
        );
    }
}
