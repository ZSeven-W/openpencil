//! Desktop pump for the image-node property section: drives the
//! Search popover (Openverse → Wikimedia, TS
//! `server/api/ai/image-search.ts`), the Generate popover
//! (OpenAI / Gemini / Replicate, TS `server/api/ai/image-generate.ts`),
//! and the local-asset existence check behind the warning row (TS
//! `use-image-asset-state.ts` detects it via an <img> onerror; the
//! desktop probes the file system directly).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use jian_ops_schema::node::PenNode;
use op_editor_core::agent_settings::ImageGenProfile;
use op_editor_core::image_panel_state::{
    ImageAssetCheck, ImageAssetStatus, ImageGeneratePhase, ImageSearchHit, ImageSearchSource,
};
use op_host_native::widget_host::WidgetHostNative;

use crate::image_generate_host::run_generate_blocking;
use crate::image_search_session::{
    fetch_image_data_url, fetch_openverse_token, simplify_search_query, OpenverseCredentials,
};

/// TS popover requests `count: 5`.
const SEARCH_RESULT_COUNT: usize = 5;

struct SearchOutcome {
    results: Vec<ImageSearchHit>,
    source: Option<ImageSearchSource>,
}

#[derive(Default)]
pub struct ImagePanelJobs {
    search_spawned: u64,
    search_job: Option<Receiver<SearchOutcome>>,
    generate_spawned: u64,
    generate_job: Option<Receiver<Result<String, String>>>,
    /// `(node_id, src)` the asset check last ran for.
    asset_checked: Option<(String, String)>,
}

impl ImagePanelJobs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a worker is in flight (keeps the winit loop waking).
    pub fn is_pending(&self) -> bool {
        self.search_job.is_some() || self.generate_job.is_some()
    }

    /// Per-frame pump: refresh the asset check, spawn newly requested
    /// search / generate jobs, land finished ones. Returns `true`
    /// when editor state changed (a repaint is due).
    pub fn pump(&mut self, host: &mut WidgetHostNative, current_path: &Option<PathBuf>) -> bool {
        let mut changed = false;
        changed |= self.refresh_asset_check(host, current_path);
        changed |= self.spawn_search_if_requested(host);
        changed |= self.poll_search(host);
        changed |= self.spawn_generate_if_requested(host);
        changed |= self.poll_generate(host);
        changed
    }

    // --- Local-asset check -------------------------------------------

    fn refresh_asset_check(
        &mut self,
        host: &mut WidgetHostNative,
        current_path: &Option<PathBuf>,
    ) -> bool {
        let Some((node_id, src)) = selected_image_src(host) else {
            // No image selection — drop any stale check so the next
            // image selection re-probes.
            if host
                .editor_state()
                .editor_ui
                .image_panel
                .asset_check
                .is_some()
                || self.asset_checked.is_some()
            {
                self.asset_checked = None;
                host.editor_state_mut().editor_ui.image_panel.asset_check = None;
                return false;
            }
            return false;
        };
        if self.asset_checked.as_ref() == Some(&(node_id.clone(), src.clone())) {
            return false;
        }
        let status = asset_status(&src, current_path.as_deref());
        self.asset_checked = Some((node_id.clone(), src.clone()));
        host.editor_state_mut().editor_ui.image_panel.asset_check = Some(ImageAssetCheck {
            node_id,
            src,
            status,
        });
        true
    }

    // --- Search --------------------------------------------------------

    fn spawn_search_if_requested(&mut self, host: &mut WidgetHostNative) -> bool {
        let state = host.editor_state();
        let panel = &state.editor_ui.image_panel;
        if !panel.search_loading || panel.search_epoch == self.search_spawned {
            return false;
        }
        self.search_spawned = panel.search_epoch;
        let query = panel.search_query.clone();
        let credentials = OpenverseCredentials::from_state(state);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(run_search_blocking(&query, credentials.as_ref()));
        });
        self.search_job = Some(rx);
        false
    }

    fn poll_search(&mut self, host: &mut WidgetHostNative) -> bool {
        let Some(rx) = self.search_job.as_ref() else {
            return false;
        };
        let outcome = match rx.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => SearchOutcome {
                results: Vec::new(),
                source: None,
            },
        };
        self.search_job = None;
        let panel = &mut host.editor_state_mut().editor_ui.image_panel;
        panel.search_loading = false;
        // Land the results only if the popover is still open (a
        // dismissed popover discards the late response).
        if panel.search_open {
            panel.search_results = outcome.results;
            panel.search_source = outcome.source;
        }
        host.mark_editor_state_dirty();
        true
    }

    // --- Generate --------------------------------------------------------

    fn spawn_generate_if_requested(&mut self, host: &mut WidgetHostNative) -> bool {
        let state = host.editor_state();
        let panel = &state.editor_ui.image_panel;
        if panel.generate_phase != ImageGeneratePhase::Loading
            || panel.generate_epoch == self.generate_spawned
        {
            return false;
        }
        self.generate_spawned = panel.generate_epoch;
        let prompt = panel.generate_prompt.clone();
        let Some(profile) = active_image_gen_profile(state).cloned() else {
            let panel = &mut host.editor_state_mut().editor_ui.image_panel;
            panel.generate_phase = ImageGeneratePhase::Error;
            panel.generate_error = "Image generation not configured".to_string();
            return true;
        };
        let (width, height) = selected_image_dimensions(host);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(run_generate_blocking(&prompt, &profile, width, height));
        });
        self.generate_job = Some(rx);
        false
    }

    fn poll_generate(&mut self, host: &mut WidgetHostNative) -> bool {
        let Some(rx) = self.generate_job.as_ref() else {
            return false;
        };
        let outcome = match rx.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err("image generation worker vanished".to_string()),
        };
        self.generate_job = None;
        let panel = &mut host.editor_state_mut().editor_ui.image_panel;
        if panel.generate_open && panel.generate_phase == ImageGeneratePhase::Loading {
            match outcome {
                Ok(url) => {
                    panel.generate_preview = Some(Arc::new(url));
                    panel.generate_phase = ImageGeneratePhase::Preview;
                }
                Err(message) => {
                    // TS truncates the surfaced message to 200 chars.
                    panel.generate_error = message.chars().take(200).collect();
                    panel.generate_phase = ImageGeneratePhase::Error;
                }
            }
        }
        host.mark_editor_state_dirty();
        true
    }
}

fn selected_image_src(host: &WidgetHostNative) -> Option<(String, String)> {
    match host.editor_state().selected_node() {
        Some(PenNode::Image(image)) => Some((image.base.id.clone(), image.src.to_string())),
        _ => None,
    }
}

fn selected_image_dimensions(host: &WidgetHostNative) -> (Option<f64>, Option<f64>) {
    use jian_ops_schema::sizing::SizingBehavior;
    match host.editor_state().selected_node() {
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

pub(crate) fn active_image_gen_profile(
    state: &op_editor_core::EditorState,
) -> Option<&ImageGenProfile> {
    let settings = &state.editor_ui.agent_settings;
    settings
        .image_gen_profiles
        .iter()
        .find(|p| Some(&p.id) == settings.active_image_gen_profile_id.as_ref())
        .or_else(|| settings.image_gen_profiles.first())
        .filter(|p| !p.api_key.trim().is_empty())
}

// --- Asset status (TS resolveRuntimeAssetSource + warning gating) ----

pub(crate) fn asset_status(src: &str, document_path: Option<&Path>) -> ImageAssetStatus {
    let trimmed = src.trim();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.is_empty()
        || lower.starts_with("data:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
        || lower.starts_with("blob:")
    {
        return ImageAssetStatus::Ok;
    }
    let normalized = normalize_path(trimmed);
    let decoded = normalized
        .strip_prefix("file://")
        .map(|rest| rest.trim_start_matches('/').to_string())
        .map(|rest| {
            // file:///Users/x → /Users/x; file:///C:/x → C:/x.
            if rest.len() >= 2 && rest.as_bytes()[1] == b':' {
                rest
            } else {
                format!("/{rest}")
            }
        })
        .unwrap_or(normalized);
    if is_absolute_path(&decoded) {
        return if Path::new(&decoded).exists() {
            ImageAssetStatus::Ok
        } else {
            ImageAssetStatus::Missing
        };
    }
    let Some(doc) = document_path else {
        return ImageAssetStatus::Unresolved;
    };
    let base = doc.parent().unwrap_or_else(|| Path::new("."));
    if base.join(&decoded).exists() {
        ImageAssetStatus::Ok
    } else {
        ImageAssetStatus::Missing
    }
}

fn is_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("\\\\")
        || (value.len() >= 3 && value.as_bytes()[1] == b':' && value.as_bytes()[2] == b'/')
}

fn normalize_path(value: &str) -> String {
    value.replace('\\', "/")
}

// --- Search backend (TS image-search.ts) -----------------------------

fn run_search_blocking(query: &str, credentials: Option<&OpenverseCredentials>) -> SearchOutcome {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return SearchOutcome {
            results: Vec::new(),
            source: None,
        };
    };
    runtime.block_on(run_search(query, credentials))
}

async fn run_search(query: &str, credentials: Option<&OpenverseCredentials>) -> SearchOutcome {
    let empty = SearchOutcome {
        results: Vec::new(),
        source: None,
    };
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return empty;
    };
    // Simplify verbose prompts into keywords (TS simplifySearchQuery).
    let query = simplify_search_query(query);

    // Openverse first; a zero-result answer retries with the first
    // two keywords before falling through to Wikimedia (TS:429 /
    // zero-results fallback ladder).
    let mut hits = fetch_openverse_list(&client, &query, credentials).await;
    if hits.as_ref().is_some_and(Vec::is_empty) {
        if let Some(truncated) = two_keyword_retry(&query) {
            if let Some(retry) = fetch_openverse_list(&client, &truncated, credentials).await {
                if !retry.is_empty() {
                    hits = Some(retry);
                }
            }
        }
    }
    if let Some(urls) = hits.filter(|h| !h.is_empty()) {
        let results = materialize_thumbs(&client, urls).await;
        if !results.is_empty() {
            return SearchOutcome {
                results,
                source: Some(ImageSearchSource::Openverse),
            };
        }
    }
    let mut wiki = fetch_wikimedia_list(&client, &query).await;
    if wiki.is_empty() {
        if let Some(truncated) = two_keyword_retry(&query) {
            wiki = fetch_wikimedia_list(&client, &truncated).await;
        }
    }
    let results = materialize_thumbs(&client, wiki).await;
    let source = (!results.is_empty()).then_some(ImageSearchSource::Wikimedia);
    SearchOutcome { results, source }
}

fn two_keyword_retry(query: &str) -> Option<String> {
    let words: Vec<&str> = query.split_whitespace().filter(|w| !w.is_empty()).collect();
    (words.len() > 2).then(|| words[..2].join(" "))
}

struct RawHit {
    id: String,
    thumb_url: String,
    attribution: String,
}

/// `None` = request-level failure (429 / network), `Some([])` = the
/// catalogue answered with zero hits (TS distinguishes the two).
async fn fetch_openverse_list(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&OpenverseCredentials>,
) -> Option<Vec<RawHit>> {
    let url = reqwest::Url::parse_with_params(
        "https://api.openverse.org/v1/images/",
        &[
            ("q", query),
            ("page_size", &SEARCH_RESULT_COUNT.to_string()),
        ],
    )
    .ok()?;
    let mut request = client.get(url);
    if let Some(credentials) = credentials {
        if let Some(token) = fetch_openverse_token(client, credentials).await {
            request = request.bearer_auth(token);
        }
    }
    let resp = request.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    let results = json.get("results")?.as_array()?;
    Some(
        results
            .iter()
            .filter_map(|r| {
                let thumb = r
                    .get("thumbnail")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| r.get("url").and_then(serde_json::Value::as_str))?;
                let license = format!(
                    "{} {}",
                    r.get("license")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(""),
                    r.get("license_version")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(""),
                );
                Some(RawHit {
                    id: r
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    thumb_url: thumb.to_string(),
                    attribution: r
                        .get("attribution")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                        .unwrap_or_else(|| license.trim().to_string()),
                })
            })
            .take(SEARCH_RESULT_COUNT)
            .collect(),
    )
}

async fn fetch_wikimedia_list(client: &reqwest::Client, query: &str) -> Vec<RawHit> {
    let Ok(url) = reqwest::Url::parse_with_params(
        "https://commons.wikimedia.org/w/api.php",
        &[
            ("action", "query"),
            ("generator", "search"),
            ("gsrsearch", query),
            ("gsrnamespace", "6"),
            ("gsrlimit", &SEARCH_RESULT_COUNT.to_string()),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime|extmetadata"),
            ("iiurlwidth", "800"),
            ("format", "json"),
            ("origin", "*"),
        ],
    ) else {
        return Vec::new();
    };
    let Ok(resp) = client.get(url).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(json) = resp.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    let Some(pages) = json
        .get("query")
        .and_then(|q| q.get("pages"))
        .and_then(serde_json::Value::as_object)
    else {
        return Vec::new();
    };
    pages
        .values()
        .filter_map(|page| {
            let info = page.get("imageinfo")?.as_array()?.first()?;
            let thumb = info
                .get("thumburl")
                .and_then(serde_json::Value::as_str)
                .or_else(|| info.get("url").and_then(serde_json::Value::as_str))?;
            Some(RawHit {
                id: page
                    .get("pageid")
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                thumb_url: thumb.to_string(),
                attribution: info
                    .get("extmetadata")
                    .and_then(|m| m.get("LicenseShortName"))
                    .and_then(|l| l.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .take(SEARCH_RESULT_COUNT)
        .collect()
}

/// Download each hit's thumbnail into a `data:` URL the wasm-clean
/// panel painter can render (and write into `ImageNode.src` on
/// select). Hits whose thumbnails fail to download are dropped.
async fn materialize_thumbs(client: &reqwest::Client, hits: Vec<RawHit>) -> Vec<ImageSearchHit> {
    let mut out = Vec::with_capacity(hits.len());
    for hit in hits {
        if let Some(data_url) = fetch_image_data_url(client, &hit.thumb_url).await {
            out.push(ImageSearchHit {
                id: hit.id,
                thumb_data_url: Arc::new(data_url),
                attribution: hit.attribution,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_status_classifies_like_the_ts_resolver() {
        // Remote / data URLs never warn.
        assert_eq!(
            asset_status("data:image/png;base64,AA==", None),
            ImageAssetStatus::Ok
        );
        assert_eq!(asset_status("https://x/y.png", None), ImageAssetStatus::Ok);
        // Relative path with no document path → unresolved.
        assert_eq!(asset_status("./a.png", None), ImageAssetStatus::Unresolved);
        // Absolute path that doesn't exist → missing.
        assert_eq!(
            asset_status("/definitely/not/here/x.png", None),
            ImageAssetStatus::Missing
        );
        // Absolute path that exists → ok.
        let dir = std::env::temp_dir();
        let file = dir.join(format!("op-image-panel-test-{}.png", std::process::id()));
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(
            asset_status(&file.display().to_string(), None),
            ImageAssetStatus::Ok
        );
        // Relative path resolved against the document dir.
        let doc = dir.join("doc.op");
        assert_eq!(
            asset_status(
                file.file_name().unwrap().to_str().unwrap(),
                Some(doc.as_path())
            ),
            ImageAssetStatus::Ok
        );
        assert_eq!(
            asset_status("nope-not-here.png", Some(doc.as_path())),
            ImageAssetStatus::Missing
        );
        let _ = std::fs::remove_file(&file);
    }
}
