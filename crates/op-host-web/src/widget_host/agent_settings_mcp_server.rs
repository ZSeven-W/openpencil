//! Browser-side MCP server bridge for the Rust web host.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::widget_host) struct McpServerRequest {
    action: &'static str,
    port: u16,
}

pub(in crate::widget_host) fn mcp_server_request(
    was_running: bool,
    was_port: u16,
    is_running: bool,
    is_port: u16,
) -> Option<McpServerRequest> {
    match (was_running, is_running, was_port != is_port) {
        (false, true, _) => Some(McpServerRequest {
            action: "start",
            port: is_port,
        }),
        (true, false, _) => Some(McpServerRequest {
            action: "stop",
            port: was_port,
        }),
        (true, true, true) => Some(McpServerRequest {
            action: "start",
            port: is_port,
        }),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
pub(in crate::widget_host) fn request_mcp_server_update(request: McpServerRequest) {
    use js_sys::{Array, Function, Object, Reflect};
    use wasm_bindgen::{JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(fetch_value) = Reflect::get(window.as_ref(), &JsValue::from_str("fetch")) else {
        return;
    };
    let Ok(fetch) = fetch_value.dyn_into::<Function>() else {
        return;
    };

    let headers = Object::new();
    let _ = Reflect::set(
        &headers,
        &JsValue::from_str("Content-Type"),
        &JsValue::from_str("application/json"),
    );

    let init = Object::new();
    let _ = Reflect::set(
        &init,
        &JsValue::from_str("method"),
        &JsValue::from_str("POST"),
    );
    let _ = Reflect::set(&init, &JsValue::from_str("headers"), &headers);
    let body = format!(
        r#"{{"action":"{}","port":{}}}"#,
        request.action, request.port
    );
    let _ = Reflect::set(&init, &JsValue::from_str("body"), &JsValue::from_str(&body));

    let args = Array::new();
    args.push(&JsValue::from_str("/api/mcp/server"));
    args.push(&init);
    let _ = fetch.apply(window.as_ref(), &args);
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::widget_host) fn request_mcp_server_update(_request: McpServerRequest) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_request_tracks_start_stop_and_port_restart() {
        assert_eq!(
            mcp_server_request(false, 3100, true, 3100),
            Some(McpServerRequest {
                action: "start",
                port: 3100,
            })
        );
        assert_eq!(
            mcp_server_request(true, 3100, false, 3100),
            Some(McpServerRequest {
                action: "stop",
                port: 3100,
            })
        );
        assert_eq!(
            mcp_server_request(true, 3100, true, 3200),
            Some(McpServerRequest {
                action: "start",
                port: 3200,
            })
        );
        assert_eq!(mcp_server_request(false, 3100, false, 3200), None);
    }
}
