//! Desktop-side Iconify loading for the native icon picker.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use op_editor_core::{IconPickerRemoteIcon, IconifyLoadMoreRequest};
use serde::Deserialize;

use crate::asset_fetch_error::AssetFetchError;
use crate::DesktopApp;

/// Shared with the web fetch path (`op-host-web/src/iconify_web.rs`).
const ICONIFY_API: &str = op_editor_core::icon_picker_state::ICONIFY_API_BASE;

pub struct IconifyJob {
    rx: Option<Receiver<IconifyResult>>,
}

struct IconifyResult {
    request: IconifyLoadMoreRequest,
    result: Result<IconifyPage, AssetFetchError>,
}

struct IconifyPage {
    icons: Vec<IconPickerRemoteIcon>,
    total: usize,
    start: usize,
}

#[derive(Deserialize)]
struct SearchResponse {
    icons: Vec<String>,
    total: Option<usize>,
    start: Option<usize>,
}

#[derive(Deserialize)]
struct IconDataResponse {
    icons: HashMap<String, IconData>,
    width: Option<f32>,
    height: Option<f32>,
}

#[derive(Deserialize)]
struct IconData {
    body: String,
    width: Option<f32>,
    height: Option<f32>,
}

impl IconifyJob {
    pub fn spawn(request: IconifyLoadMoreRequest) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = fetch_iconify_page(&request);
            let _ = tx.send(IconifyResult { request, result });
        });
        Self { rx: Some(rx) }
    }

    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    fn poll(&mut self) -> Option<IconifyResult> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(result) => {
                self.rx = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                None
            }
        }
    }
}

impl DesktopApp {
    pub(crate) fn drain_iconify_picker(&mut self) -> bool {
        let mut changed = self.poll_iconify_job();
        if self
            .iconify_job
            .as_ref()
            .is_some_and(IconifyJob::is_pending)
        {
            return changed;
        }
        let request = self
            .host
            .editor_state_mut()
            .editor_ui
            .icon_picker_load_more_request
            .take();
        if let Some(request) = request {
            self.iconify_job = Some(IconifyJob::spawn(request));
            changed = true;
        }
        changed
    }

    fn poll_iconify_job(&mut self) -> bool {
        let Some(job) = self.iconify_job.as_mut() else {
            return false;
        };
        let Some(result) = job.poll() else {
            return false;
        };
        self.iconify_job = None;
        let ui = &mut self.host.editor_state_mut().editor_ui;
        if ui.icon_picker_remote.query != result.request.query {
            return false;
        }
        ui.icon_picker_remote.loading = false;
        match result.result {
            Ok(page) => {
                if page.start == 0 {
                    ui.icon_picker_remote.icons.clear();
                }
                for icon in page.icons {
                    if !ui
                        .icon_picker_remote
                        .icons
                        .iter()
                        .any(|i| i.collection == icon.collection && i.name == icon.name)
                    {
                        ui.icon_picker_remote.icons.push(icon);
                    }
                }
                ui.icon_picker_remote.total = page.total;
                ui.icon_picker_remote.next_start = page.start + result.request.limit;
                ui.icon_picker_remote.error = None;
            }
            Err(err) => {
                // `icon_picker_remote.error` is an `Option<String>` field on
                // `op-editor-core`, a crate this pass does not own.
                ui.icon_picker_remote.error = Some(err.to_string());
            }
        }
        self.host.mark_editor_state_dirty();
        true
    }
}

fn fetch_iconify_page(request: &IconifyLoadMoreRequest) -> Result<IconifyPage, AssetFetchError> {
    // Bridge through the shared runtime instead of building a private one:
    // a nested `Runtime::new` aborts with "Cannot start a runtime from within
    // a runtime" the moment this helper is reached from a tokio worker.
    op_host_services::chat_runtime::block_on_anywhere(async {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(15))
            .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| AssetFetchError::HttpClient(e.to_string()))?;
        let search_url = format!(
            "{ICONIFY_API}/search?query={}&limit={}&start={}",
            encode_component(&request.query),
            request.limit,
            request.start
        );
        let search: SearchResponse = client
            .get(search_url)
            .send()
            .await
            .map_err(|e| AssetFetchError::Request(e.to_string()))?
            .error_for_status()
            .map_err(|e| AssetFetchError::Request(e.to_string()))?
            .json()
            .await
            .map_err(|e| AssetFetchError::Request(e.to_string()))?;
        let grouped = group_icon_ids(&search.icons);
        let mut loaded = HashMap::new();
        for (collection, names) in grouped {
            let url = format!(
                "{ICONIFY_API}/{}.json?icons={}",
                encode_component(&collection),
                names
                    .iter()
                    .map(|name| encode_component(name))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let data: IconDataResponse = client
                .get(url)
                .send()
                .await
                .map_err(|e| AssetFetchError::Request(e.to_string()))?
                .error_for_status()
                .map_err(|e| AssetFetchError::Request(e.to_string()))?
                .json()
                .await
                .map_err(|e| AssetFetchError::Request(e.to_string()))?;
            for (name, icon) in data.icons {
                let w = icon.width.or(data.width).unwrap_or(24.0);
                let h = icon.height.or(data.height).unwrap_or(24.0);
                if let Some(parsed) =
                    op_editor_ui::widgets::icon_catalog::parse_iconify_body(&icon.body, w, h)
                {
                    let style = match parsed.style {
                        op_editor_ui::widgets::icon_catalog::IconRenderStyle::Stroke => "stroke",
                        op_editor_ui::widgets::icon_catalog::IconRenderStyle::Fill => "fill",
                    };
                    loaded.insert(
                        format!("{collection}:{name}"),
                        IconPickerRemoteIcon {
                            collection: collection.clone(),
                            name,
                            width: parsed.width,
                            height: parsed.height,
                            style: style.to_string(),
                            d: parsed.d,
                        },
                    );
                }
            }
        }
        let icons = search
            .icons
            .into_iter()
            .filter_map(|id| loaded.remove(&id))
            .collect();
        Ok(IconifyPage {
            icons,
            total: search.total.unwrap_or(0),
            start: search.start.unwrap_or(request.start),
        })
    })
}

fn group_icon_ids(ids: &[String]) -> HashMap<String, Vec<String>> {
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for id in ids {
        if let Some((collection, name)) = id.split_once(':') {
            grouped
                .entry(collection.to_string())
                .or_default()
                .push(name.to_string());
        }
    }
    grouped
}

fn encode_component(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
