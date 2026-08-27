//! Desktop entry for the image Generate popover.
//!
//! The provider backends (OpenAI / custom OpenAI-compatible, Gemini
//! inline-image, Replicate / Atlas prediction polling — a port of the TS
//! `server/api/ai/image-generate.ts`) are single-sourced in
//! `op_host_services::web_image_generate`; this residual owns only the
//! desktop-specific parts: the desktop user-agent client, the skia
//! down-scale pass, and the offline `data:` embedding via the desktop
//! image fetcher.

use std::time::Duration;

use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider};
use op_host_services::web_image_generate::{
    generate_atlas, generate_gemini, generate_openai, generate_replicate, ImageGenerateError,
};

use crate::image_search_session::fetch_image_data_url;

// --- Generate backend (TS image-generate.ts) -------------------------

/// Run one generation for the image-panel worker.
///
/// Reports the shared [`ImageGenerateError`] rather than a desktop-local
/// type: every failure this can produce originates in
/// `op_host_services::web_image_generate`'s provider calls, which the web
/// daemon's route reports the same way. `image_panel_host.rs` re-labels it
/// into its own `AssetFetchError::Generate` at the channel boundary, and
/// `Display` reproduces the strings this used to build by hand, so the
/// popover's error row is unchanged.
pub(crate) fn run_generate_blocking(
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, ImageGenerateError> {
    // Runtime-aware bridge: this is called from the image-panel worker thread
    // today, but a private current-thread runtime here would abort with
    // "Cannot start a runtime from within a runtime" the moment the caller
    // moves onto a tokio worker. See `chat_runtime::block_on_anywhere`.
    op_host_services::chat_runtime::block_on_anywhere(run_generate(prompt, profile, width, height))
}

async fn run_generate(
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<String, ImageGenerateError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| ImageGenerateError::ClientBuild {
            message: e.to_string(),
        })?;
    let url = match profile.provider {
        ImageGenProvider::OpenAi | ImageGenProvider::Custom => {
            generate_openai(&client, prompt, profile, width, height).await?
        }
        ImageGenProvider::Gemini => {
            generate_gemini(&client, prompt, profile, width, height).await?
        }
        ImageGenProvider::Replicate => {
            generate_replicate(&client, prompt, profile, width, height).await?
        }
        ImageGenProvider::Atlas => generate_atlas(&client, prompt, profile, width, height).await?,
    };
    if url.starts_with("data:") {
        // Inline base64 (Gemini / OpenAI b64_json) → shrink an oversized
        // render before it enters the document, same as the file-pick path.
        return Ok(crate::image_downscale::maybe_downscale_data_url(&url).unwrap_or(url));
    }
    // Remote URL → embed as a data URL so the preview paints and the
    // applied src stays renderable offline (matches the search path).
    fetch_image_data_url(&client, &url)
        .await
        .ok_or(ImageGenerateError::DownloadFailed)
}
