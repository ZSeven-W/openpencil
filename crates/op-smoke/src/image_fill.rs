//! Post-loop image-fill step for the headless agentic loop.
//!
//! After the design loop completes and before the `.op` is persisted, optionally
//! resolve pending image-search queries to real URLs from Openverse. Gated by
//! `OPENPENCIL_SMOKE_FILL_IMAGES=1` (default off).

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use op_editor_core::EditorState;
use op_host_services::image_relevance_judge::OpenAiCompatVisionJudge;

/// Parsed config for the image-fill post-loop step.
#[derive(Debug, Clone)]
pub struct ImageFillConfig {
    /// Enable the step (OPENPENCIL_SMOKE_FILL_IMAGES=1).
    pub enabled: bool,
    /// Max images to fill (default 40; OPENPENCIL_SMOKE_IMAGE_LIMIT).
    pub limit: usize,
    /// Milliseconds to wait between requests (default 250; OPENPENCIL_SMOKE_IMAGE_DELAY_MS).
    pub delay_ms: u64,
}

impl ImageFillConfig {
    /// Parse the config from environment variables.
    pub fn from_env() -> Self {
        let enabled = std::env::var("OPENPENCIL_SMOKE_FILL_IMAGES")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("on"))
            .unwrap_or(false);
        let limit = std::env::var("OPENPENCIL_SMOKE_IMAGE_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(40);
        let delay_ms = std::env::var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(250);
        Self {
            enabled,
            limit,
            delay_ms,
        }
    }
}

/// Resolve the visual relevance judge for the fill step. The env-configured
/// OpenAI-compatible judge (all three `OPENPENCIL_IMAGE_JUDGE_*` vars present)
/// wins; otherwise the fill keeps the legacy one-result ladder — which is the
/// `NoJudge` behavior and stays byte-identical with the pre-judge output.
fn resolve_judge() -> Option<OpenAiCompatVisionJudge> {
    OpenAiCompatVisionJudge::from_env()
}

/// Run the image-fill post-loop step on the provided state.
///
/// Collects image-search targets (nodes with a query but empty `src`), fetches
/// URLs sequentially with a delay, applies each, and logs the results. Never
/// fails the run — a target whose search returns nothing is logged and skipped.
/// Must be called AFTER the agent loop finishes but BEFORE the `.op` is saved.
pub fn fill_images(state: &mut EditorState, config: &ImageFillConfig, dump: bool) {
    if !config.enabled {
        return;
    }

    let judge = resolve_judge();
    eprintln!(
        "[SMOKE] image judge: {}",
        if judge.is_some() { "env" } else { "none" }
    );

    let mut targets = op_image_enrich::collect_targets(state, &HashSet::new());
    if targets.is_empty() {
        if dump {
            eprintln!("[SMOKE] image fill skipped: no target(s) found");
        }
        return;
    }

    let cap = config.limit.min(targets.len());
    targets.truncate(cap);

    let used_urls = Mutex::new(HashSet::new());
    let mut filled = 0usize;
    let total = targets.len();

    if dump {
        eprintln!(
            "[SMOKE] image fill starting: {} target(s) (limit={}, delay={}ms)",
            total, config.limit, config.delay_ms
        );
    }

    for target in targets {
        if dump {
            eprintln!(
                "[SMOKE] image search: node_id={} query={:?}",
                target.node_id, target.query
            );
        }

        let url = match &judge {
            Some(judge) => {
                // Same intent rule as the desktop session: the authored image
                // prompt wins, the search query is the fallback.
                let intent = target
                    .prompt
                    .as_deref()
                    .filter(|prompt| !prompt.trim().is_empty())
                    .unwrap_or(target.query.as_str());
                op_image_enrich::net::fetch::fetch_first_image_url_blocking_with_judge(
                    &target.query,
                    target.aspect_ratio,
                    None,
                    &used_urls,
                    judge,
                    intent,
                )
            }
            // No configured judge deliberately uses the old one-result path;
            // this is the NoJudge behavior and keeps default output bytes
            // unchanged.
            None => op_image_enrich::net::fetch::fetch_first_image_url_blocking(
                &target.query,
                target.aspect_ratio,
                None,
                &used_urls,
            ),
        };
        if let Some(url) = url {
            if op_image_enrich::apply_result(state, &target.node_id, &url) {
                if dump {
                    eprintln!(
                        "[SMOKE] image applied: node_id={} url={url}",
                        target.node_id
                    );
                }
                filled += 1;
            } else if dump {
                eprintln!(
                    "[SMOKE] image apply failed: node_id={} (collab gate or node gone)",
                    target.node_id
                );
            }
        } else if dump {
            eprintln!(
                "[SMOKE] image search returned nothing: query={:?}",
                target.query
            );
        }

        if filled < total {
            std::thread::sleep(Duration::from_millis(config.delay_ms));
        }
    }

    let remaining_targets = op_image_enrich::collect_targets(state, &HashSet::new());
    let remaining = remaining_targets.len();

    eprintln!(
        "[SMOKE] image fill done: filled {filled}/{total} target(s), {remaining} unfilled remain"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn config_defaults_are_correct() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");
        let config = ImageFillConfig::from_env();
        assert_eq!(config.limit, 40);
        assert_eq!(config.delay_ms, 250);
        assert!(!config.enabled);
    }

    #[test]
    fn config_enabled_by_env() {
        let _guard = TEST_LOCK.lock();
        std::env::set_var("OPENPENCIL_SMOKE_FILL_IMAGES", "1");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");
        let config = ImageFillConfig::from_env();
        assert!(config.enabled);
    }

    #[test]
    fn config_parses_limit_from_env() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::set_var("OPENPENCIL_SMOKE_IMAGE_LIMIT", "100");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");
        let config = ImageFillConfig::from_env();
        assert_eq!(config.limit, 100);
    }

    #[test]
    fn config_parses_delay_from_env() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::set_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS", "500");
        let config = ImageFillConfig::from_env();
        assert_eq!(config.delay_ms, 500);
    }

    #[test]
    fn config_parses_truthy_values() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");

        std::env::set_var("OPENPENCIL_SMOKE_FILL_IMAGES", "true");
        let config = ImageFillConfig::from_env();
        assert!(config.enabled);

        std::env::set_var("OPENPENCIL_SMOKE_FILL_IMAGES", "on");
        let config = ImageFillConfig::from_env();
        assert!(config.enabled);
    }

    #[test]
    fn config_ignores_invalid_limit() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::set_var("OPENPENCIL_SMOKE_IMAGE_LIMIT", "not_a_number");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");
        let config = ImageFillConfig::from_env();
        assert_eq!(config.limit, 40);
    }

    #[test]
    fn config_ignores_invalid_delay() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::set_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS", "xyz");
        let config = ImageFillConfig::from_env();
        assert_eq!(config.delay_ms, 250);
    }

    // The fill entry delegates the search itself to op-image-enrich's public
    // fetch fns, so a scripted judge cannot be injected at the op-smoke layer
    // (the re-ranking policy is covered by op-image-enrich's own judge tests).
    // What op-smoke owns is judge RESOLUTION, so these tests pin its tri-state.

    fn clear_judge_env() {
        std::env::remove_var("OPENPENCIL_IMAGE_JUDGE_BASE_URL");
        std::env::remove_var("OPENPENCIL_IMAGE_JUDGE_API_KEY");
        std::env::remove_var("OPENPENCIL_IMAGE_JUDGE_MODEL");
    }

    #[test]
    fn judge_resolves_env_when_all_three_vars_present() {
        let _guard = TEST_LOCK.lock();
        clear_judge_env();
        std::env::set_var(
            "OPENPENCIL_IMAGE_JUDGE_BASE_URL",
            "https://judge.example.com/v1",
        );
        std::env::set_var("OPENPENCIL_IMAGE_JUDGE_API_KEY", "test-key");
        std::env::set_var("OPENPENCIL_IMAGE_JUDGE_MODEL", "test-model");
        let judge = resolve_judge();
        clear_judge_env();
        assert!(judge.is_some());
    }

    #[test]
    fn judge_falls_back_to_none_when_any_var_is_missing() {
        let _guard = TEST_LOCK.lock();
        clear_judge_env();
        assert!(resolve_judge().is_none());

        std::env::set_var(
            "OPENPENCIL_IMAGE_JUDGE_BASE_URL",
            "https://judge.example.com/v1",
        );
        std::env::set_var("OPENPENCIL_IMAGE_JUDGE_MODEL", "test-model");
        // API key missing.
        assert!(resolve_judge().is_none());

        std::env::set_var("OPENPENCIL_IMAGE_JUDGE_API_KEY", "test-key");
        std::env::remove_var("OPENPENCIL_IMAGE_JUDGE_MODEL");
        // Model missing.
        assert!(resolve_judge().is_none());

        clear_judge_env();
    }

    #[test]
    fn judge_ignores_blank_env_values() {
        let _guard = TEST_LOCK.lock();
        clear_judge_env();
        std::env::set_var("OPENPENCIL_IMAGE_JUDGE_BASE_URL", "   ");
        std::env::set_var("OPENPENCIL_IMAGE_JUDGE_API_KEY", "test-key");
        std::env::set_var("OPENPENCIL_IMAGE_JUDGE_MODEL", "test-model");
        let judge = resolve_judge();
        clear_judge_env();
        assert!(judge.is_none());
    }
}
