//! Intent fingerprints for the desktop image-search session's stale-result
//! guard.
//!
//! `search_intent_key` / `canonical_search_intent_query` moved here from the
//! `image_search_session.rs` spine; `intent_fingerprint` /
//! current-intent snapshots moved here from the (since-relocated)
//! `targets.rs`. The snapshot depends on
//! the desktop crate's `image_panel_host` gen-profile lookup, which is why
//! this file stays in the desktop crate while the slot predicates themselves
//! now live in the shared `op-image-enrich` crate.

use std::collections::{HashMap, HashSet};

use op_editor_core::agent_settings::ImageGenProfile;
use op_editor_core::EditorState;
use op_image_enrich::{
    collect_targets, collect_targets_with_scene, ImageAspectRatio, ImageJudgeView,
    ImageRequestMode, ImageSearchTarget,
};

/// Resolve the same generation profile for dispatch and stale-result checks.
/// Explicit generation ignores the chat toggle; only Auto follows it.
pub(super) fn generation_profile_for_request(
    mode: ImageRequestMode,
    profile: Option<&ImageGenProfile>,
    auto_gen_active: bool,
) -> Option<&ImageGenProfile> {
    match mode {
        ImageRequestMode::Generate => profile,
        ImageRequestMode::Auto if auto_gen_active => profile,
        ImageRequestMode::Auto | ImageRequestMode::Search => None,
    }
}

pub(crate) fn intent_fingerprint(
    target: &ImageSearchTarget,
    profile: Option<&ImageGenProfile>,
) -> String {
    let generate = target.mode == ImageRequestMode::Generate
        || (target.mode == ImageRequestMode::Auto && profile.is_some());
    if generate {
        let (profile_id, model) = profile
            .map(|profile| (profile.id.as_str(), profile.model.as_str()))
            .unwrap_or(("unconfigured", "unconfigured"));
        format!(
            "generate|{profile_id}|{model}|{}|{:?}|{:?}",
            target
                .prompt
                .as_deref()
                .filter(|prompt| !prompt.trim().is_empty())
                .unwrap_or(target.query.as_str())
                .trim(),
            target.width.map(f64::to_bits),
            target.height.map(f64::to_bits)
        )
    } else {
        let key = search_intent_key(
            &target.query,
            target.aspect_ratio,
            target.prompt.as_deref(),
            target.judge_view,
        );
        let mut fingerprint = format!("search|{}|{:?}", key.query, key.aspect_ratio);
        if let Some(prompt) = key.prompt {
            fingerprint.push_str(&format!("|prompt={prompt:?}"));
        }
        if let Some(view) = key.judge_view {
            fingerprint.push_str(&format!("|view={view:?}"));
        }
        fingerprint
    }
}

pub(super) struct ImageIntentSnapshot {
    pub(super) fingerprint: String,
    pub(super) judge_view: Option<ImageJudgeView>,
}

pub(super) fn current_intent_snapshot(
    state: &EditorState,
    scene: Option<&op_editor_ui::layout_scene::LayoutScene>,
) -> HashMap<String, ImageIntentSnapshot> {
    let profile = crate::image_panel_host::active_image_gen_profile(state);
    let auto_gen_active = state.editor_ui.agent_settings.image_gen_active();
    let targets = match scene {
        Some(scene) => collect_targets_with_scene(state, &HashSet::new(), scene),
        None => collect_targets(state, &HashSet::new()),
    };
    targets
        .into_iter()
        .map(|target| {
            let profile = generation_profile_for_request(target.mode, profile, auto_gen_active);
            let fingerprint = intent_fingerprint(&target, profile);
            (
                target.node_id.as_str().to_string(),
                ImageIntentSnapshot {
                    fingerprint,
                    judge_view: target.judge_view,
                },
            )
        })
        .collect()
}

/// Memo key for the authored stock-search intent.
///
/// This deliberately does NOT use `simplify_search_query`: that function is a
/// lossy provider adapter (it drops words such as `album` / `cover` and caps
/// the request at four keywords). Those transformations are useful for a
/// photo corpus, but they must not make two distinct authored subjects share a
/// cached image or make the stale-result guard treat a changed intent as the
/// same intent. Case, punctuation, and repeated whitespace are canonicalized;
/// every authored query word remains part of identity. The rich prompt is also
/// part of identity because the judge and photo contract consume it even when
/// retrieval uses the same short query. Preserve prompt wording and punctuation;
/// only trim its edges, treating blank prompts like the query-only legacy path.
/// Aspect remains part of intent so a square cover never reuses a wide hero.
/// The actual view also participates: two wide slots can show different crops,
/// and a photo judged under cover cannot stand in for a contain view.
pub(crate) fn search_intent_key(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    prompt: Option<&str>,
    judge_view: Option<ImageJudgeView>,
) -> super::SearchIntentKey {
    super::SearchIntentKey {
        query: canonical_search_intent_query(query),
        aspect_ratio,
        prompt: prompt
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string),
        judge_view: judge_view.map(|view| view.cache_key()),
    }
}

fn canonical_search_intent_query(query: &str) -> String {
    let mut canonical = String::with_capacity(query.len());
    let mut pending_separator = false;
    for character in query.trim().to_lowercase().chars() {
        if character.is_alphanumeric() {
            if pending_separator && !canonical.is_empty() {
                canonical.push(' ');
            }
            canonical.push(character);
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    if canonical.is_empty() {
        query.trim().to_lowercase()
    } else {
        canonical
    }
}
