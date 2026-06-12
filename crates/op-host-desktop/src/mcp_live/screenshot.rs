//! `debug_screenshot` wire glue for the live MCP server.
//!
//! Intercepts the `tools/call` BEFORE the generic registry dispatch
//! (whose headless `DebugScreenshot` tool can only report the honest
//! no-live-canvas error), validates the arguments with the SAME
//! `op_mcp::parse_screenshot_args` the headless tool uses, hands the
//! request to a fulfiller (the UI-thread raster capture in
//! production; a scripted closure in tests), and serializes the TS
//! `debug-screenshot.ts` result shape: an `image` content block
//! (base64 PNG) plus a pretty-printed `text` metadata block.

use op_mcp::{RequestId, ScreenshotRequest, ScreenshotTarget, ToolErrorCode, ToolResponse};
use serde_json::{json, Value};

use crate::export::screenshot::{CaptureSpec, ScreenshotPng};

/// If `body` is a `tools/call` for `debug_screenshot` AND the debug
/// gate is open, produce the full JSON-RPC response via `fulfill`.
/// `None` ⇒ not a screenshot call (or gate closed) — the caller falls
/// through to the generic dispatch.
pub(crate) fn maybe_serve<F>(body: &str, debug_enabled: bool, fulfill: F) -> Option<String>
where
    F: FnOnce(ScreenshotRequest) -> Result<ScreenshotPng, String>,
{
    let call = op_mcp::parse_tool_call(body.trim())?;
    if call.tool != "debug_screenshot" || !debug_enabled {
        return None;
    }
    let response = match op_mcp::parse_screenshot_args(&call.arguments) {
        Err((code, message)) => error_response(&call.id, code, &message),
        Ok(req) => match fulfill(req.clone()) {
            Ok(shot) => success_response(&call.id, &req, &shot),
            // TS `debug-screenshot.ts` throws "Renderer reported
            // failure: …"; the server envelope turns it into an
            // isError text block.
            Err(message) => error_response(
                &call.id,
                ToolErrorCode::ToolFailed,
                &format!("Renderer reported failure: {message}"),
            ),
        },
    };
    Some(response)
}

/// Convert validated wire args into the renderer-side capture spec.
pub(crate) fn capture_spec(req: &ScreenshotRequest) -> CaptureSpec {
    CaptureSpec {
        node_id: match &req.target {
            ScreenshotTarget::Root => None,
            ScreenshotTarget::Node(id) => Some(id.clone()),
        },
        // TS `SkiaEngine.captureRegion` defaults: padding 0, dpr 1
        // (no `window.devicePixelRatio` headlessly).
        padding: req.padding.unwrap_or(0.0) as f32,
        scale: req.dpr.unwrap_or(1.0) as f32,
    }
}

/// TS `handleScreenshot` result: `[{type:image,data,mimeType},
/// {type:text,text:JSON.stringify({target,nodeId,actualBounds,dpr},
/// null, 2)}]` — absent fields dropped, exactly like JSON.stringify.
fn success_response(id: &RequestId, req: &ScreenshotRequest, shot: &ScreenshotPng) -> String {
    let mut meta = serde_json::Map::new();
    let target = match &req.target {
        ScreenshotTarget::Root => "root",
        ScreenshotTarget::Node(_) => "node",
    };
    meta.insert("target".into(), json!(target));
    if let ScreenshotTarget::Node(node_id) = &req.target {
        meta.insert("nodeId".into(), json!(node_id));
    }
    if let Some((x, y, w, h)) = shot.actual_bounds {
        meta.insert(
            "actualBounds".into(),
            json!({ "x": x, "y": y, "w": w, "h": h }),
        );
    }
    if let Some(dpr) = req.dpr {
        meta.insert("dpr".into(), json!(dpr));
    }
    let text = serde_json::to_string_pretty(&Value::Object(meta)).unwrap_or_default();
    let content = json!([
        { "type": "image", "data": shot.png_base64, "mimeType": "image/png" },
        { "type": "text", "text": text },
    ]);
    format!(
        r#"{{"jsonrpc":"2.0","id":{},"result":{{"content":{}}}}}"#,
        id_raw(id),
        content
    )
}

/// Tool-level failure — the MCP `tools/call` isError envelope (same
/// serializer the generic dispatch path uses).
fn error_response(id: &RequestId, code: ToolErrorCode, message: &str) -> String {
    op_mcp::json_serializer::tool_response_to_json(&ToolResponse::Err {
        id: id.clone(),
        code,
        message: message.to_string(),
    })
}

fn id_raw(id: &RequestId) -> String {
    match id {
        RequestId::Str(s) => Value::String(s.clone()).to_string(),
        RequestId::Num(n) => n.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_body(arguments: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{{"name":"debug_screenshot","arguments":{arguments}}}}}"#
        )
    }

    fn shot(bounds: Option<(f32, f32, f32, f32)>) -> ScreenshotPng {
        ScreenshotPng {
            png_base64: "QUJD".into(),
            actual_bounds: bounds,
        }
    }

    #[test]
    fn non_screenshot_calls_and_closed_gate_fall_through() {
        let other = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_selection","arguments":{}}}"#;
        assert!(maybe_serve(other, true, |_| Ok(shot(None))).is_none());
        // Gate closed → None even for a real screenshot call, so the
        // generic dispatch reports UnknownTool like the headless
        // registry that never registered the tool.
        assert!(
            maybe_serve(&call_body(r#"{"target":"root"}"#), false, |_| Ok(shot(
                None
            )))
            .is_none()
        );
    }

    #[test]
    fn root_capture_returns_an_image_content_block_with_metadata_text() {
        let response = maybe_serve(&call_body(r#"{"target":"root"}"#), true, |req| {
            assert_eq!(req.target, ScreenshotTarget::Root);
            assert_eq!(req.timeout_ms, 15_000);
            Ok(shot(None))
        })
        .expect("intercepted");
        let v: Value = serde_json::from_str(&response).expect("valid JSON-RPC");
        assert_eq!(v["id"], 7);
        let content = v["result"]["content"].as_array().expect("content");
        assert_eq!(content.len(), 2, "{response}");
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["data"], "QUJD");
        assert_eq!(content[0]["mimeType"], "image/png");
        assert_eq!(content[1]["type"], "text");
        let meta: Value =
            serde_json::from_str(content[1]["text"].as_str().expect("text")).expect("meta json");
        assert_eq!(meta["target"], "root");
        assert!(meta.get("nodeId").is_none(), "{meta}");
        assert!(meta.get("actualBounds").is_none(), "{meta}");
        assert!(v["result"].get("isError").is_none(), "{response}");
    }

    #[test]
    fn node_capture_threads_bounds_node_id_and_dpr_into_the_text_block() {
        let body = call_body(r#"{"target":"node","nodeId":"n9","dpr":"2","padding":"4"}"#);
        let response = maybe_serve(&body, true, |req| {
            assert_eq!(req.target, ScreenshotTarget::Node("n9".into()));
            let spec = capture_spec(&req);
            assert_eq!(spec.node_id.as_deref(), Some("n9"));
            assert_eq!(spec.padding, 4.0);
            assert_eq!(spec.scale, 2.0);
            Ok(shot(Some((100.0, 50.0, 40.0, 30.0))))
        })
        .expect("intercepted");
        let v: Value = serde_json::from_str(&response).expect("valid JSON-RPC");
        let meta: Value =
            serde_json::from_str(v["result"]["content"][1]["text"].as_str().expect("text"))
                .expect("meta json");
        assert_eq!(meta["target"], "node");
        assert_eq!(meta["nodeId"], "n9");
        assert_eq!(meta["dpr"], 2.0);
        assert_eq!(
            meta["actualBounds"],
            serde_json::json!({ "x": 100.0, "y": 50.0, "w": 40.0, "h": 30.0 })
        );
    }

    #[test]
    fn invalid_arguments_report_the_shared_validation_error() {
        let response = maybe_serve(&call_body(r#"{"target":"node"}"#), true, |_| {
            panic!("validation failures must never reach the fulfiller")
        })
        .expect("intercepted");
        let v: Value = serde_json::from_str(&response).expect("valid JSON-RPC");
        assert_eq!(v["result"]["isError"], true, "{response}");
        assert_eq!(
            v["result"]["content"][0]["text"],
            "Error: target=\"node\" requires nodeId"
        );
    }

    #[test]
    fn fulfiller_failures_surface_as_renderer_errors() {
        let response = maybe_serve(&call_body(r#"{"target":"root"}"#), true, |_| {
            Err("timed out waiting for UI screenshot ack".into())
        })
        .expect("intercepted");
        let v: Value = serde_json::from_str(&response).expect("valid JSON-RPC");
        assert_eq!(v["result"]["isError"], true);
        assert_eq!(
            v["result"]["content"][0]["text"],
            "Error: Renderer reported failure: timed out waiting for UI screenshot ack"
        );
    }
}
