//! Web drain for the image-node property section: Search and Generate
//! popovers.
//!
//! The dispatch layer (`widget_host/image_panel_dispatch.rs`) only flips
//! `search_loading` / `generate_phase` and bumps the matching epoch; this
//! module is the network half the desktop hosts in `image_panel_host.rs`.
//! Instead of dialing Openverse / the gen provider directly (CORS +
//! credential exposure), it POSTs to the daemon's `/api/ai/image/*` routes
//! and lands the JSON reply back into `image_panel` state.
//!
//! Mirrors the desktop pump's guarantees: one in-flight job per epoch
//! counter, and a reply is discarded when the popover was dismissed or the
//! epoch moved on (a newer request owns the state).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;
use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider};
use op_editor_core::image_panel_state::{ImageGeneratePhase, ImageSearchHit, ImageSearchSource};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::repaint_ctx::RepaintContext;

/// Search worst case on the daemon: credentialed Openverse (token + list,
/// twice), both providers' thumbnail batches, and two Wikimedia attempts —
/// ~128 s of 8 s-capped requests back-to-back. 180 s clears it with margin;
/// the daemon answers as soon as the ladder actually resolves.
const SEARCH_TIMEOUT_MS: u32 = 180_000;
/// Generate worst case: Replicate polls up to 120 s, then downloads.
const GENERATE_TIMEOUT_MS: u32 = 300_000;

thread_local! {
    /// Epochs whose jobs have already been spawned (desktop
    /// `ImagePanelJobs::{search_spawned, generate_spawned}`).
    static SEARCH_SPAWNED: Cell<u64> = const { Cell::new(0) };
    static GENERATE_SPAWNED: Cell<u64> = const { Cell::new(0) };
}

/// Drain both image-panel jobs. Called from the DOM listeners after their
/// `inner` borrow is released, like the other web drains.
pub(crate) fn drain_image_jobs<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    drain_search(inner);
    drain_generate(inner);
}

fn drain_search<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let request = {
        let Ok(b) = inner.try_borrow() else {
            return;
        };
        let state = b.host().editor_state();
        let panel = &state.editor_ui.image_panel;
        if !panel.search_loading || panel.search_epoch == SEARCH_SPAWNED.with(Cell::get) {
            None
        } else {
            let settings = &state.editor_ui.agent_settings;
            let mut body = serde_json::json!({ "query": panel.search_query.text() });
            let client_id = settings.openverse_client_id.trim();
            let client_secret = settings.openverse_client_secret.trim();
            if !client_id.is_empty() && !client_secret.is_empty() {
                body["openverse"] = serde_json::json!({
                    "client_id": client_id,
                    "client_secret": client_secret,
                });
            }
            Some((panel.search_epoch, body.to_string()))
        }
    };
    let Some((epoch, body)) = request else {
        return;
    };
    SEARCH_SPAWNED.with(|slot| slot.set(epoch));
    let url = crate::daemon_base::daemon_url("/api/ai/image/search");
    let inner_cb = inner.clone();
    let started = post_json_with_timeout(
        &url,
        &body,
        SEARCH_TIMEOUT_MS,
        Rc::new(move |status, text| {
            let Ok(mut b) = inner_cb.try_borrow_mut() else {
                return;
            };
            let panel = &mut b.host_mut().editor_state_mut().editor_ui.image_panel;
            // A newer epoch owns the panel, or the popover was closed
            // mid-flight (`close_popovers` clears `search_loading`; a
            // reopen keeps the same epoch, so the epoch check alone would
            // let a dismissed search repopulate the fresh popover).
            if panel.search_epoch != epoch || !panel.search_loading {
                return;
            }
            panel.search_loading = false;
            // Land the results only if the popover is still open (a
            // dismissed popover discards the late response).
            if panel.search_open {
                let (results, source) = parse_search_reply(status, &text);
                panel.search_results = results;
                panel.search_source = source;
            }
            b.host_mut().mark_editor_state_dirty();
            drop(b);
            crate::repaint_coalescer::request();
        }),
    );
    if !started {
        // The XHR never left the browser — release the spinner with an
        // empty answer instead of loading forever.
        settle_failed_search(inner, epoch);
    }
}

fn settle_failed_search<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, epoch: u64) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    let panel = &mut b.host_mut().editor_state_mut().editor_ui.image_panel;
    if panel.search_epoch == epoch && panel.search_loading {
        panel.search_loading = false;
        panel.search_results = Vec::new();
        panel.search_source = None;
        b.host_mut().mark_editor_state_dirty();
    }
}

pub(crate) fn parse_search_reply(
    status: u16,
    text: &str,
) -> (Vec<ImageSearchHit>, Option<ImageSearchSource>) {
    if status != 200 {
        return (Vec::new(), None);
    }
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else {
        return (Vec::new(), None);
    };
    let results = json
        .get("results")
        .and_then(serde_json::Value::as_array)
        .map(|results| {
            results
                .iter()
                .filter_map(|hit| {
                    let thumb = hit
                        .get("thumb_data_url")
                        .and_then(serde_json::Value::as_str)?;
                    if thumb.is_empty() {
                        return None;
                    }
                    Some(ImageSearchHit {
                        id: hit
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        thumb_data_url: Arc::new(thumb.to_string()),
                        attribution: hit
                            .get("attribution")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let source = match json.get("source").and_then(serde_json::Value::as_str) {
        Some("openverse") => Some(ImageSearchSource::Openverse),
        Some("wikimedia") => Some(ImageSearchSource::Wikimedia),
        _ => None,
    };
    (results, source)
}

fn drain_generate<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let request = {
        let Ok(b) = inner.try_borrow() else {
            return;
        };
        let state = b.host().editor_state();
        let panel = &state.editor_ui.image_panel;
        if panel.generate_phase != ImageGeneratePhase::Loading
            || panel.generate_epoch == GENERATE_SPAWNED.with(Cell::get)
        {
            None
        } else {
            Some((
                panel.generate_epoch,
                generate_body(state, panel.generate_prompt.text()),
            ))
        }
    };
    let Some((epoch, body)) = request else {
        return;
    };
    GENERATE_SPAWNED.with(|slot| slot.set(epoch));
    let Some(body) = body else {
        // Dispatch guards `configured`, but settings can change between the
        // press and this drain — surface the same error the desktop does.
        settle_generate(inner, epoch, Err(ImageGenerateError::NotConfigured));
        return;
    };
    let url = format!(
        "{}/api/ai/image/generate",
        crate::daemon_base::daemon_base()
    );
    let inner_cb = inner.clone();
    let started = post_json_with_timeout(
        &url,
        &body,
        GENERATE_TIMEOUT_MS,
        Rc::new(move |status, text| {
            settle_generate(&inner_cb, epoch, parse_generate_reply(status, &text));
        }),
    );
    if !started {
        settle_generate(inner, epoch, Err(ImageGenerateError::RequestFailed));
    }
}

/// Build the generate request body from the browser-held profile. `None`
/// when no configured profile exists.
fn generate_body(state: &op_editor_core::EditorState, prompt: &str) -> Option<String> {
    let settings = &state.editor_ui.agent_settings;
    let profile = settings
        .active_image_gen_profile()
        .filter(|p| !p.api_key.trim().is_empty())?;
    let (width, height) = selected_image_dimensions(state);
    let mut body = serde_json::json!({
        "prompt": prompt,
        "profile": profile_json(profile),
    });
    if let Some(w) = width {
        body["width"] = serde_json::json!(w);
    }
    if let Some(h) = height {
        body["height"] = serde_json::json!(h);
    }
    Some(body.to_string())
}

fn profile_json(profile: &ImageGenProfile) -> serde_json::Value {
    let provider = match profile.provider {
        ImageGenProvider::OpenAi => "openai",
        ImageGenProvider::Gemini => "gemini",
        ImageGenProvider::Replicate => "replicate",
        ImageGenProvider::Atlas => "atlas",
        ImageGenProvider::Custom => "custom",
    };
    let mut json = serde_json::json!({
        "provider": provider,
        "model": profile.model,
        "api_key": profile.api_key,
    });
    if let Some(base) = profile.base_url.as_deref().filter(|b| !b.trim().is_empty()) {
        json["base_url"] = serde_json::json!(base);
    }
    json
}

fn selected_image_dimensions(state: &op_editor_core::EditorState) -> (Option<f64>, Option<f64>) {
    match state.selected_node() {
        Some(PenNode::Image(image)) => {
            let num = |s: &Option<SizingBehavior>| match s {
                Some(SizingBehavior::Number(px)) => Some(*px),
                _ => None,
            };
            (num(&image.width), num(&image.height))
        }
        _ => (None, None),
    }
}

/// A failed image-generation turn.
///
/// `Display` reproduces the ad-hoc `String` messages this enum replaced byte
/// for byte. `Remote` is the daemon/provider text shown verbatim in the
/// popover, so it stays a payload string rather than being re-derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ImageGenerateError {
    /// No image-generation profile is configured.
    NotConfigured,
    /// The request could not be started at all.
    RequestFailed,
    /// The reply was unusable; only the HTTP status is known.
    Failed(u16),
    /// The daemon/provider reported an error message.
    Remote(String),
}

impl std::fmt::Display for ImageGenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageGenerateError::NotConfigured => write!(f, "Image generation not configured"),
            ImageGenerateError::RequestFailed => write!(f, "image generation request failed"),
            ImageGenerateError::Failed(status) => {
                write!(f, "image generation failed ({status})")
            }
            ImageGenerateError::Remote(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ImageGenerateError {}

impl From<ImageGenerateError> for String {
    fn from(error: ImageGenerateError) -> String {
        error.to_string()
    }
}

pub(crate) fn parse_generate_reply(status: u16, text: &str) -> Result<String, ImageGenerateError> {
    let json: serde_json::Value =
        serde_json::from_str(text).map_err(|_| ImageGenerateError::Failed(status))?;
    if status == 200 {
        if let Some(url) = json
            .get("url")
            .and_then(serde_json::Value::as_str)
            .filter(|u| !u.is_empty())
        {
            return Ok(url.to_string());
        }
    }
    Err(json
        .get("error")
        .and_then(serde_json::Value::as_str)
        .filter(|e| !e.is_empty())
        .map(|message| ImageGenerateError::Remote(message.to_string()))
        .unwrap_or(ImageGenerateError::Failed(status)))
}

fn settle_generate<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    epoch: u64,
    outcome: Result<String, ImageGenerateError>,
) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    let panel = &mut b.host_mut().editor_state_mut().editor_ui.image_panel;
    if panel.generate_epoch != epoch {
        return;
    }
    if panel.generate_open && panel.generate_phase == ImageGeneratePhase::Loading {
        match outcome {
            Ok(url) => {
                panel.generate_preview = Some(Arc::new(url));
                panel.generate_phase = ImageGeneratePhase::Preview;
            }
            Err(error) => {
                // TS truncates the surfaced message to 200 chars.
                panel.generate_error = error.to_string().chars().take(200).collect();
                panel.generate_phase = ImageGeneratePhase::Error;
            }
        }
    } else if panel.generate_phase == ImageGeneratePhase::Loading {
        // Popover dismissed mid-flight: release the phase so a reopen
        // doesn't look stuck.
        panel.generate_phase = ImageGeneratePhase::Idle;
    }
    b.host_mut().mark_editor_state_dirty();
    drop(b);
    crate::repaint_coalescer::request();
}

/// `live_sync::post_json_with_status` with a caller-chosen timeout — the
/// shared helper's 15 s cap is tuned for doc-sync writes and would abort a
/// legitimate search ladder / Replicate poll.
fn post_json_with_timeout(
    url: &str,
    body: &str,
    timeout_ms: u32,
    on_response: Rc<dyn Fn(u16, String)>,
) -> bool {
    let Ok(xhr) = web_sys::XmlHttpRequest::new() else {
        return false;
    };
    // Same reason as the AI transport: this helper builds its own XHR for a
    // caller-chosen timeout and so bypasses the `live_sync` stamping.
    let url = &crate::daemon_base::with_tenant_param(url);
    if xhr.open_with_async("POST", url, true).is_err() {
        return false;
    }
    crate::live_sync::attach_daemon_headers(&xhr, url);
    xhr.set_timeout(timeout_ms);
    let _ = xhr.set_request_header("Content-Type", "application/json");
    let xhr_for_load = xhr.clone();
    let onloadend = Closure::<dyn FnMut()>::once_into_js(move || {
        let status = xhr_for_load.status().unwrap_or(0);
        let text = xhr_for_load
            .response_text()
            .ok()
            .flatten()
            .unwrap_or_default();
        on_response(status, text);
    });
    xhr.set_onloadend(Some(onloadend.unchecked_ref()));
    xhr.send_with_opt_str(Some(body)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_search_reply_maps_hits_and_source() {
        let (results, source) = parse_search_reply(
            200,
            r#"{"ok":true,"source":"openverse","results":[
                {"id":"a","thumb_data_url":"data:image/png;base64,AA==","attribution":"By A"},
                {"id":"b","thumb_data_url":""},
                {"id":"c"}
            ]}"#,
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
        assert_eq!(*results[0].thumb_data_url, "data:image/png;base64,AA==");
        assert_eq!(source, Some(ImageSearchSource::Openverse));

        let (results, source) =
            parse_search_reply(200, r#"{"ok":true,"results":[],"source":null}"#);
        assert!(results.is_empty());
        assert_eq!(source, None);

        // Non-200 / garbage → empty, no crash.
        assert!(parse_search_reply(500, "boom").0.is_empty());
        assert!(parse_search_reply(200, "not json").0.is_empty());
    }

    #[test]
    fn parse_generate_reply_maps_ok_error_and_garbage() {
        assert_eq!(
            parse_generate_reply(200, r#"{"ok":true,"url":"data:image/png;base64,AA=="}"#),
            Ok("data:image/png;base64,AA==".to_string())
        );
        assert_eq!(
            parse_generate_reply(502, r#"{"ok":false,"error":"quota exceeded"}"#)
                .map_err(|error| error.to_string()),
            Err("quota exceeded".to_string())
        );
        // 200 without a url still errors (defensive against a half-shaped reply).
        assert_eq!(
            parse_generate_reply(200, r#"{"ok":true}"#).map_err(|error| error.to_string()),
            Err("image generation failed (200)".to_string())
        );
        assert_eq!(
            parse_generate_reply(0, "").map_err(|error| error.to_string()),
            Err("image generation failed (0)".to_string())
        );
    }
}
