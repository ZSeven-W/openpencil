//! Clean-stdout local ACP test agent.
//!
//! This is an example target because Cargo compiles examples during ordinary
//! package tests without inserting libtest's human-readable progress output
//! into the process's stdout. The desktop local-process E2E launches it and
//! therefore exercises the production stdio transport with strict ndJSON.

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const FIXTURE_ENV: &str = "OPENPENCIL_LOCAL_ACP_E2E_FIXTURE";
const FIXTURE_AGENT_NAME: &str = "OpenPencil Local ACP Fixture";
const FIXTURE_AGENT_VERSION: &str = "1.0";
const FIXTURE_SESSION_ID: &str = "fixture-session";
const FIXTURE_PROMPT: &str = "LOCAL_ACP_E2E_7C1: reply with the fixture greeting.";
const FIXTURE_REPLY: &str = "Hello from the real local ACP subprocess.";

fn main() {
    if std::env::var_os(FIXTURE_ENV).is_none() {
        return;
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut initialized = false;
    let mut session_open = false;

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        let Ok(frame) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = frame.get("id").cloned().unwrap_or(Value::Null);
        let method = frame
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = frame.get("params").cloned().unwrap_or(Value::Null);

        match method {
            "initialize" => {
                let valid = params.get("protocolVersion").and_then(Value::as_u64) == Some(1)
                    && params.pointer("/clientInfo/name").and_then(Value::as_str)
                        == Some("openpencil");
                if !valid {
                    write_rpc_error(&mut stdout, id, "invalid initialize payload");
                    continue;
                }
                initialized = true;
                write_frame(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": 1,
                            "agentCapabilities": {
                                "mcpCapabilities": { "http": true }
                            },
                            "agentInfo": {
                                "name": FIXTURE_AGENT_NAME,
                                "version": FIXTURE_AGENT_VERSION
                            }
                        }
                    }),
                );
            }
            "session/new" => {
                let servers = params.get("mcpServers").and_then(Value::as_array);
                let valid_base = initialized
                    && params
                        .get("cwd")
                        .and_then(Value::as_str)
                        .is_some_and(|cwd| !cwd.is_empty());
                let valid = match servers {
                    // Connect-time probing deliberately opens an empty session
                    // after initialize to prove the agent is actually usable.
                    Some(servers) if servers.is_empty() => valid_base,
                    Some(servers) => valid_base && valid_canvas_session(&params, servers),
                    None => false,
                };
                if !valid {
                    write_rpc_error(&mut stdout, id, "invalid session/new payload");
                    continue;
                }
                session_open = true;
                write_frame(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "sessionId": FIXTURE_SESSION_ID }
                    }),
                );
            }
            "session/prompt" => {
                let prompt_text = params
                    .get("prompt")
                    .and_then(Value::as_array)
                    .and_then(|blocks| blocks.first())
                    .and_then(|block| block.get("text"))
                    .and_then(Value::as_str);
                let valid = initialized
                    && session_open
                    && params.get("sessionId").and_then(Value::as_str) == Some(FIXTURE_SESSION_ID)
                    && prompt_text.is_some_and(|prompt| prompt.contains(FIXTURE_PROMPT));
                if !valid {
                    write_rpc_error(&mut stdout, id, "invalid session/prompt payload");
                    continue;
                }
                write_frame(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "method": "session/update",
                        "params": {
                            "sessionId": FIXTURE_SESSION_ID,
                            "update": {
                                "sessionUpdate": "agent_message_chunk",
                                "content": { "type": "text", "text": FIXTURE_REPLY }
                            }
                        }
                    }),
                );
                write_frame(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "stopReason": "end_turn" }
                    }),
                );
                break;
            }
            _ => write_rpc_error(&mut stdout, id, "unsupported method"),
        }
    }
}

fn valid_canvas_session(params: &Value, servers: &[Value]) -> bool {
    let Some(server) = servers.first() else {
        return false;
    };
    server.get("name").and_then(Value::as_str) == Some("openpencil")
        && server.get("type").and_then(Value::as_str) == Some("http")
        && server.get("url").and_then(Value::as_str) == Some("http://127.0.0.1:4123/mcp")
        && server
            .get("headers")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        && params
            .pointer("/_meta/systemPrompt")
            .and_then(Value::as_str)
            .is_some_and(|prompt| prompt.contains("mcp__openpencil__"))
}

fn write_rpc_error(stdout: &mut dyn Write, id: Value, message: &str) {
    write_frame(
        stdout,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32_000, "message": message }
        }),
    );
}

fn write_frame(stdout: &mut dyn Write, frame: Value) {
    serde_json::to_writer(&mut *stdout, &frame).expect("serialize ACP fixture frame");
    stdout.write_all(b"\n").expect("write ACP fixture newline");
    stdout.flush().expect("flush ACP fixture frame");
}
