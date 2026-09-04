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

use crate::asset_fetch_error::AssetFetchError;
use crate::image_generate_host::run_generate_blocking;
use crate::image_search_session::{fetch_image_data_url, OpenverseCredentials};

struct SearchOutcome {
    results: Vec<ImageSearchHit>,
    source: Option<ImageSearchSource>,
}

#[derive(Default)]
pub struct ImagePanelJobs {
    search_spawned: u64,
    search_job: Option<Receiver<SearchOutcome>>,
    generate_spawned: u64,
    generate_job: Option<Receiver<Result<String, AssetFetchError>>>,
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
        let query = panel.search_query.text().to_owned();
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
        let prompt = panel.generate_prompt.text().to_owned();
        let Some(profile) = active_image_gen_profile(state).cloned() else {
            let panel = &mut host.editor_state_mut().editor_ui.image_panel;
            panel.generate_phase = ImageGeneratePhase::Error;
            panel.generate_error = "Image generation not configured".to_string();
            return true;
        };
        let (width, height) = selected_image_dimensions(host);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            // `run_generate_blocking` reports its provider's own message;
            // carry it so the panel's error text is unchanged.
            let _ = tx.send(
                run_generate_blocking(&prompt, &profile, width, height)
                    .map_err(|error| AssetFetchError::Generate(error.to_string())),
            );
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
            Err(TryRecvError::Disconnected) => Err(AssetFetchError::GenerateWorkerVanished),
        };
        self.generate_job = None;
        let panel = &mut host.editor_state_mut().editor_ui.image_panel;
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
    state
        .editor_ui
        .agent_settings
        .active_image_gen_profile()
        .filter(|p| !p.api_key.trim().is_empty())
}

// --- Asset status (TS resolveRuntimeAssetSource + warning gating) ----
// Divergence from TS: a local path that resolves on THIS machine is
// `LinkedLocal`, not folded into `Ok` — see `ImageAssetStatus::LinkedLocal`.

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
            ImageAssetStatus::LinkedLocal
        } else {
            ImageAssetStatus::Missing
        };
    }
    let Some(doc) = document_path else {
        return ImageAssetStatus::Unresolved;
    };
    let base = doc.parent().unwrap_or_else(|| Path::new("."));
    if base.join(&decoded).exists() {
        ImageAssetStatus::LinkedLocal
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
//
// The provider ladder (Openverse → two-keyword retry → Wikimedia, result
// parsing, thumbnail materialization) is single-sourced in
// `op_host_services::web_image_search`. The desktop supplies its own
// client (desktop user-agent) and its own thumbnail fetcher so each
// downloaded thumb still goes through the skia down-scale pass before it
// becomes a `data:` URL the panel painter renders (and writes into
// `ImageNode.src` on select).

fn run_search_blocking(query: &str, credentials: Option<&OpenverseCredentials>) -> SearchOutcome {
    // Runtime-aware bridge (see `image_generate_host::run_generate_blocking`):
    // a private current-thread runtime here would abort the process if this
    // sync entry point were ever reached from a tokio worker.
    op_host_services::chat_runtime::block_on_anywhere(run_search(query, credentials))
}

async fn run_search(query: &str, credentials: Option<&OpenverseCredentials>) -> SearchOutcome {
    let Ok(client) = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return SearchOutcome {
            results: Vec::new(),
            source: None,
        };
    };
    let outcome = op_host_services::web_image_search::run_search_with_fetcher(
        &client,
        query,
        credentials.map(OpenverseCredentials::as_web),
        |url: String| {
            let client = client.clone();
            async move { fetch_image_data_url(&client, &url).await }
        },
    )
    .await;
    SearchOutcome {
        results: outcome
            .results
            .into_iter()
            .map(|hit| ImageSearchHit {
                id: hit.id,
                thumb_data_url: Arc::new(hit.thumb_data_url),
                attribution: hit.attribution,
            })
            .collect(),
        source: outcome.source.map(|source| match source {
            "openverse" => ImageSearchSource::Openverse,
            _ => ImageSearchSource::Wikimedia,
        }),
    }
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
        // Absolute path that exists → LinkedLocal, not Ok — it resolves
        // HERE, but it's still a pointer, not portable content (see
        // `ImageAssetStatus::LinkedLocal`'s doc).
        let dir = std::env::temp_dir();
        let file = dir.join(format!("op-image-panel-test-{}.png", std::process::id()));
        std::fs::write(&file, b"x").unwrap();
        assert_eq!(
            asset_status(&file.display().to_string(), None),
            ImageAssetStatus::LinkedLocal
        );
        // Relative path resolved against the document dir — same
        // LinkedLocal treatment.
        let doc = dir.join("doc.op");
        assert_eq!(
            asset_status(
                file.file_name().unwrap().to_str().unwrap(),
                Some(doc.as_path())
            ),
            ImageAssetStatus::LinkedLocal
        );
        assert_eq!(
            asset_status("nope-not-here.png", Some(doc.as_path())),
            ImageAssetStatus::Missing
        );
        let _ = std::fs::remove_file(&file);
    }
}
