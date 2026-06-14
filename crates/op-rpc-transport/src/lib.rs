//! Shared JSON-RPC envelopes plus a small local TCP HTTP transport.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub const MCP_PATH: &str = "/mcp";
pub const POST_TIMEOUT: Duration = Duration::from_secs(60);
pub const PING_TIMEOUT: Duration = Duration::from_secs(2);
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub params: Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: &str, params: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn method_not_found(id: Value, method: &str) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("method '{method}' not supported"),
                data: None,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, PartialEq)]
pub enum Inbound {
    Response {
        id: u64,
        result: Option<Value>,
        error: Option<JsonRpcError>,
    },
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
    Unknown,
}

pub fn classify_inbound(value: &Value) -> Inbound {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return Inbound::Unknown,
    };
    let has_method = obj.contains_key("method");
    let id = obj.get("id");
    match (has_method, id) {
        (true, Some(id)) if !id.is_null() => Inbound::Request {
            id: id.clone(),
            method: obj["method"].as_str().unwrap_or_default().to_string(),
            params: obj.get("params").cloned().unwrap_or(Value::Null),
        },
        (true, _) => Inbound::Notification {
            method: obj["method"].as_str().unwrap_or_default().to_string(),
            params: obj.get("params").cloned().unwrap_or(Value::Null),
        },
        (false, Some(id)) => match id.as_u64() {
            Some(id) => Inbound::Response {
                id,
                result: obj.get("result").cloned(),
                error: obj
                    .get("error")
                    .and_then(|e| serde_json::from_value(e.clone()).ok()),
            },
            None => Inbound::Unknown,
        },
        _ => Inbound::Unknown,
    }
}

pub fn correlate(value: &Value, expected_id: u64) -> Option<Result<Value, JsonRpcError>> {
    match classify_inbound(value) {
        Inbound::Response { id, result, error } if id == expected_id => Some(match error {
            Some(error) => Err(error),
            None => Ok(result.unwrap_or(Value::Null)),
        }),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpReply {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpJsonRpc {
    addr: SocketAddr,
    path: String,
}

impl TcpJsonRpc {
    pub fn localhost(port: u16, path: &str) -> Self {
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
            path: path.to_string(),
        }
    }

    pub fn local_mcp(port: u16) -> Self {
        Self::localhost(port, MCP_PATH)
    }

    pub fn http_post_request(&self, body: &str) -> String {
        format!(
            "POST {} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.path,
            body.len(),
            body
        )
    }

    pub fn http_get_request(&self, path: &str) -> String {
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
    }

    pub fn post_raw(&self, body: &str, timeout: Duration) -> Result<HttpReply, String> {
        self.send(&self.http_post_request(body), timeout)
    }

    pub fn get_raw(&self, path: &str, timeout: Duration) -> Result<HttpReply, String> {
        self.send(&self.http_get_request(path), timeout)
    }

    fn send(&self, request: &str, timeout: Duration) -> Result<HttpReply, String> {
        let mut stream = TcpStream::connect_timeout(&self.addr, timeout).map_err(|e| {
            format!(
                "cannot reach the editor on {}: {e}\n\
                 start the OpenPencil MCP server and point clients at http://{}/mcp",
                self.addr, self.addr
            )
        })?;
        stream.set_read_timeout(Some(timeout)).ok();
        stream.set_write_timeout(Some(timeout)).ok();
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("http write: {e}"))?;
        stream.flush().ok();
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|e| format!("http read: {e}"))?;
        Ok(split_http_response(&response))
    }
}

fn split_http_response(response: &str) -> HttpReply {
    let status = parse_status_code(response).unwrap_or(0);
    let body = match response.split_once("\r\n\r\n") {
        Some((_, body)) => body.trim().to_string(),
        None => response.trim().to_string(),
    };
    HttpReply { status, body }
}

fn parse_status_code(response: &str) -> Option<u16> {
    let mut parts = response.lines().next()?.split_whitespace();
    let _http_version = parts.next()?;
    parts.next()?.parse::<u16>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn json_rpc_request_response_and_error_frames_are_canonical() {
        let req = JsonRpcRequest::new(7, "tools/list", Value::Null);
        let req_value = serde_json::to_value(&req).unwrap();
        assert_eq!(req_value["jsonrpc"], "2.0");
        assert_eq!(req_value["id"], 7);
        assert!(req_value.get("params").is_none());

        let ok = JsonRpcResponse::ok(json!(7), json!({ "pong": true }));
        let ok_value = serde_json::to_value(&ok).unwrap();
        assert_eq!(ok_value["result"]["pong"], true);
        assert!(ok_value.get("error").is_none());

        let err = JsonRpcResponse::method_not_found(json!("abc"), "missing");
        let err_value = serde_json::to_value(&err).unwrap();
        assert_eq!(err_value["id"], "abc");
        assert_eq!(err_value["error"]["code"], -32601);
    }

    #[test]
    fn classify_inbound_distinguishes_response_request_and_notification() {
        assert!(matches!(
            classify_inbound(&json!({"jsonrpc":"2.0","id":7,"result":{"ok":true}})),
            Inbound::Response { id: 7, .. }
        ));
        assert!(matches!(
            classify_inbound(
                &json!({"jsonrpc":"2.0","id":"abc","method":"client/request","params":{}})
            ),
            Inbound::Request { .. }
        ));
        assert!(matches!(
            classify_inbound(&json!({"jsonrpc":"2.0","method":"session/update","params":{}})),
            Inbound::Notification { .. }
        ));
        assert!(matches!(classify_inbound(&json!(false)), Inbound::Unknown));
    }

    #[test]
    fn correlate_returns_only_the_matching_response() {
        let response = json!({"jsonrpc":"2.0","id":42,"result":{"ok":true}});
        assert_eq!(correlate(&response, 42).unwrap().unwrap()["ok"], true);
        assert!(correlate(&response, 43).is_none());

        let error = json!({"jsonrpc":"2.0","id":42,"error":{"code":-32000,"message":"boom"}});
        let err = correlate(&error, 42).unwrap().unwrap_err();
        assert_eq!(err.code, -32000);
        assert_eq!(err.message, "boom");
    }

    #[test]
    fn tcp_http_request_targets_mcp_path() {
        let request = TcpJsonRpc::localhost(3100, MCP_PATH).http_post_request("{}");
        assert!(request.starts_with("POST /mcp HTTP/1.1\r\n"));
        assert!(request.contains("Host: 127.0.0.1\r\n"));
        assert!(request.contains("Content-Length: 2\r\n"));
    }
}
