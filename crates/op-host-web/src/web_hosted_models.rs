//! Account-scoped public model projection from the authenticated daemon.
//!
//! Only opaque routes and display metadata cross this boundary. The daemon
//! owns hosted access, endpoint selection, renewal, and provider construction.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use op_editor_core::{
    EditorState, HostedModelCapabilities, HostedModelRoute, HostedProtocol, MAX_HOSTED_MODEL_ROUTES,
};
use serde::Deserialize;
use wasm_bindgen::{closure::Closure, JsCast};

use crate::repaint_ctx::RepaintContext;

const MAX_CATALOG_BYTES: usize = 128 * 1024;
const CATALOG_PATH: &str = "/api/ai/hosted/models";

thread_local! {
    static REQUEST: RefCell<Option<web_sys::XmlHttpRequest>> = const { RefCell::new(None) };
    static GENERATION: Cell<u64> = const { Cell::new(0) };
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CatalogStatus {
    Ready,
    SignedOut,
    Unavailable,
    Loading,
    Failed,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogWire {
    status: CatalogStatus,
    revision: u64,
    routes: Vec<RouteWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RouteWire {
    route_id: String,
    model_id: String,
    display_name: String,
    protocol: ProtocolWire,
    capabilities: CapabilitiesWire,
}

#[derive(Deserialize)]
enum ProtocolWire {
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
    #[serde(rename = "anthropic-messages")]
    AnthropicMessages,
    #[serde(rename = "openai-responses")]
    OpenAiResponses,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilitiesWire {
    image_input: bool,
    tools: bool,
    structured_output: bool,
    reasoning: bool,
}

struct Catalog {
    loading: bool,
    routes: Vec<HostedModelRoute>,
}

#[derive(Debug, PartialEq, Eq)]
enum CatalogError {
    TooLarge,
    Invalid,
}

fn parse_catalog(body: &str) -> Result<Catalog, CatalogError> {
    if body.len() > MAX_CATALOG_BYTES {
        return Err(CatalogError::TooLarge);
    }
    // Deriving directly from the JSON rejects duplicate and unknown fields;
    // parsing through Value first would silently erase duplicate fields.
    let wire: CatalogWire = serde_json::from_str(body).map_err(|_| CatalogError::Invalid)?;
    if wire.routes.len() > MAX_HOSTED_MODEL_ROUTES
        || (wire.status != CatalogStatus::Ready && !wire.routes.is_empty())
        || (wire.status == CatalogStatus::Ready && wire.revision == 0)
    {
        return Err(CatalogError::Invalid);
    }
    let mut ids = HashSet::new();
    let mut routes = Vec::with_capacity(wire.routes.len());
    for route in wire.routes {
        if !ids.insert(route.route_id.clone()) {
            return Err(CatalogError::Invalid);
        }
        let protocol = match route.protocol {
            ProtocolWire::OpenAiCompatible => HostedProtocol::OpenAiCompatible,
            ProtocolWire::AnthropicMessages => HostedProtocol::AnthropicMessages,
            ProtocolWire::OpenAiResponses => HostedProtocol::OpenAiResponses,
        };
        let capabilities = HostedModelCapabilities {
            image_input: route.capabilities.image_input,
            tools: route.capabilities.tools,
            structured_output: route.capabilities.structured_output,
            reasoning: route.capabilities.reasoning,
        };
        routes.push(
            HostedModelRoute::new(
                route.route_id,
                route.model_id,
                route.display_name,
                protocol,
                capabilities,
            )
            .map_err(|_| CatalogError::Invalid)?,
        );
    }
    Ok(Catalog {
        loading: wire.status == CatalogStatus::Loading,
        routes,
    })
}

/// Supersede the old request before aborting it: abort dispatches callbacks.
pub(crate) fn reset() {
    GENERATION.with(|value| value.set(value.get().saturating_add(1)));
    let request = REQUEST.with(|slot| slot.borrow_mut().take());
    if let Some(request) = request {
        let _ = request.abort();
    }
}

fn current_request(generation: u64, epoch: u64) -> bool {
    GENERATION.with(|value| value.get() == generation) && crate::identity_epoch::epoch() == epoch
}

fn apply_catalog(state: &mut EditorState, catalog: Result<Catalog, CatalogError>) -> bool {
    match catalog {
        Ok(catalog) if state.editor_ui.account.is_signed_in() => {
            let loading = catalog.loading;
            if state.replace_hosted_model_catalog(catalog.routes).is_err() {
                state.clear_hosted_model_catalog();
            }
            crate::web_model_catalog::reconcile_models(state);
            loading
        }
        _ => {
            state.clear_hosted_model_catalog();
            crate::web_model_catalog::reconcile_models(state);
            false
        }
    }
}

/// Called only after an accepted daemon auth status has settled UI identity.
pub(crate) fn refresh<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    reset();
    if !inner
        .borrow()
        .host()
        .editor_state()
        .editor_ui
        .account
        .is_signed_in()
    {
        let mut context = inner.borrow_mut();
        let state = context.host_mut().editor_state_mut();
        crate::web_chat::abort_hosted_turn(state);
        apply_catalog(state, Err(CatalogError::Invalid));
        context.host_mut().mark_editor_state_dirty();
        let _ = context.repaint();
        return;
    }
    let generation = GENERATION.with(Cell::get);
    let epoch = crate::identity_epoch::epoch();
    let url = crate::daemon_base::daemon_url(CATALOG_PATH);
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        return;
    };
    if xhr.open_with_async("GET", &url, true).is_err() {
        return;
    }
    crate::live_sync::attach_daemon_headers(&xhr, &url);
    xhr.set_timeout(crate::live_sync::DAEMON_FETCH_TIMEOUT_MS);
    let xhr_cb = xhr.clone();
    let inner_cb = inner.clone();
    let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
        xhr_cb.set_onloadend(None);
        if !current_request(generation, epoch) {
            return;
        }
        REQUEST.with(|slot| slot.borrow_mut().take());
        let catalog = if xhr_cb.status().ok() == Some(200) {
            parse_catalog(&xhr_cb.response_text().ok().flatten().unwrap_or_default())
        } else {
            Err(CatalogError::Invalid)
        };
        let mut context = inner_cb.borrow_mut();
        let state = context.host_mut().editor_state_mut();
        if catalog
            .as_ref()
            .map_or(true, |catalog| catalog.routes.is_empty())
        {
            crate::web_chat::abort_hosted_turn(state);
        }
        let loading = apply_catalog(state, catalog);
        context.host_mut().mark_editor_state_dirty();
        let _ = context.repaint();
        drop(context);
        if loading {
            schedule_retry(&inner_cb, generation, epoch);
        }
    });
    xhr.set_onloadend(Some(onloadend.unchecked_ref()));
    REQUEST.with(|slot| *slot.borrow_mut() = Some(xhr.clone()));
    if xhr.send().is_err() {
        xhr.set_onloadend(None);
        reset();
        // A synchronous send failure has no loadend event to release the
        // one-shot capture. Invoke the detached, superseded callback once.
        let _ = onloadend
            .unchecked_ref::<js_sys::Function>()
            .call0(&wasm_bindgen::JsValue::NULL);
    }
}

fn schedule_retry<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    generation: u64,
    epoch: u64,
) {
    let inner = inner.clone();
    let callback = Closure::<dyn FnMut()>::once_into_js(move || {
        if current_request(generation, epoch) {
            refresh(&inner);
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), 1_000);
    }
}

#[cfg(test)]
#[path = "web_hosted_models_tests.rs"]
mod tests;
