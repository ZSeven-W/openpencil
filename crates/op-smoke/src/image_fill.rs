//! Post-loop image-fill step for the headless agentic loop.
//!
//! After the design loop completes and before the final `.op` is persisted, optionally
//! resolve pending image-search queries to real URLs from Openverse. Gated by
//! `OPENPENCIL_SMOKE_FILL_IMAGES=1` (default off).

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::smoke_post_error::PostGenerationError;
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
    /// Optional whole-stage budget (OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS).
    pub budget: Option<Duration>,
}

impl ImageFillConfig {
    /// Parse the config from environment variables.
    pub fn from_env() -> Result<Self, PostGenerationError> {
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
        let budget = parse_budget(
            std::env::var("OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS")
                .ok()
                .as_deref(),
        )?;
        Ok(Self {
            enabled,
            limit,
            delay_ms,
            budget,
        })
    }
}

fn parse_budget(raw: Option<&str>) -> Result<Option<Duration>, PostGenerationError> {
    let Some(raw) = raw else { return Ok(None) };
    let seconds = raw.trim().parse::<u64>().ok().filter(|value| *value > 0);
    let budget = seconds
        .map(Duration::from_secs)
        .filter(|duration| Instant::now().checked_add(*duration).is_some());
    budget
        .map(Some)
        .ok_or(PostGenerationError::InvalidImageBudget)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImageFillReport {
    pub total: usize,
    pub selected: usize,
    pub attempted: usize,
    pub unattempted: usize,
    pub filled: usize,
    pub no_hit: usize,
    pub deadline_limited: usize,
    pub unfilled: usize,
    pub budget_exhausted: bool,
}

impl ImageFillReport {
    pub fn deadline_incomplete(&self) -> bool {
        self.budget_exhausted && self.unfilled > 0
    }

    fn log(&self) {
        eprintln!(
            "[SMOKE_IMAGE_RESULT] {}",
            serde_json::json!({
                "total": self.total,
                "selected": self.selected,
                "attempted": self.attempted,
                "unattempted": self.unattempted,
                "filled": self.filled,
                "no_hit": self.no_hit,
                "deadline_limited": self.deadline_limited,
                "unfilled": self.unfilled,
                "budget_exhausted": self.budget_exhausted,
            })
        );
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
/// treats a completed no-hit search as a fatal error. An explicit budget that
/// expires with unfinished slots is reported separately to the caller.
pub fn fill_images(
    state: &mut EditorState,
    config: &ImageFillConfig,
    dump: bool,
) -> ImageFillReport {
    if !config.enabled {
        return ImageFillReport::default();
    }

    // The whole-stage budget includes judge setup and target-layout discovery;
    // cold initialization must not silently grant extra network time.
    let deadline = config.budget.map(|budget| Instant::now() + budget);
    let judge = resolve_judge();
    eprintln!(
        "[SMOKE] image judge: {}",
        if judge.is_some() { "env" } else { "none" }
    );

    fill_images_with_fetch(
        state,
        config,
        dump,
        deadline,
        Instant::now,
        |target, used_urls, deadline| {
            let intent = target
                .prompt
                .as_deref()
                .filter(|prompt| !prompt.trim().is_empty())
                .unwrap_or(target.query.as_str());
            use op_image_enrich::net::{fetch, resolve};
            match &judge {
                Some(judge) => fetch::fetch_first_image_resolved_blocking_with_judge_for_view(
                    &target.query,
                    target.aspect_ratio,
                    None,
                    used_urls,
                    judge,
                    intent,
                    deadline,
                    target.judge_view,
                ),
                None => resolve::fetch_first_image_resolved_blocking_with_deadline(
                    &target.query,
                    intent,
                    target.aspect_ratio,
                    None,
                    used_urls,
                    deadline,
                ),
            }
        },
    )
}

fn fill_images_with_fetch(
    state: &mut EditorState,
    config: &ImageFillConfig,
    dump: bool,
    deadline: Option<Instant>,
    mut now: impl FnMut() -> Instant,
    mut fetch: impl FnMut(
        &op_image_enrich::ImageSearchTarget,
        &Mutex<HashSet<String>>,
        Option<Instant>,
    ) -> Option<op_image_enrich::net::fetch::ResolvedImage>,
) -> ImageFillReport {
    let mut targets = op_image_enrich::collect_targets(state, &HashSet::new());
    let mut report = ImageFillReport {
        total: targets.len(),
        ..ImageFillReport::default()
    };
    if targets.is_empty() {
        if dump {
            eprintln!("[SMOKE] image fill skipped: no target(s) found");
        }
        report.log();
        return report;
    }

    let cap = config.limit.min(targets.len());
    targets.truncate(cap);

    let used_urls = Mutex::new(HashSet::new());
    let total = targets.len();
    report.selected = total;

    if dump {
        eprintln!(
            "[SMOKE] image fill starting: {} target(s) (limit={}, delay={}ms)",
            total, config.limit, config.delay_ms
        );
    }

    for target in targets {
        if deadline.is_some_and(|end| now() >= end) {
            report.budget_exhausted = true;
            break;
        }
        if dump {
            eprintln!(
                "[SMOKE] image search: node_id={} query={:?}",
                target.node_id, target.query
            );
        }

        report.attempted += 1;
        let hit = fetch(&target, &used_urls, deadline);
        // The shared unjudged ladder reserves 250 ms for delivery/save; count
        // a no-hit inside that margin as deadline-limited as well.
        if hit.is_none() {
            if deadline.is_some_and(|end| {
                end.saturating_duration_since(now()) <= Duration::from_millis(250)
            }) {
                report.budget_exhausted = true;
                report.deadline_limited += 1;
            } else {
                report.no_hit += 1;
            }
        }
        // Shipped nodes keep only `op-image:<hash>`; this line is the only
        // record of which catalogue hit filled the slot.
        eprintln!(
            "{}",
            image_provenance_line(target.node_id.as_str(), &target.query, hit.as_ref())
        );
        let url = hit.map(|hit| hit.src);
        let resolved = url
            .as_deref()
            .unwrap_or(op_image_enrich::SEARCH_FAILED_PLACEHOLDER_SRC);
        if op_image_enrich::apply_result(state, &target.node_id, resolved) {
            // Match the desktop's late-image callback: contrast cannot be
            // established while the original photo slot is still empty.
            let repaired = op_orchestrator::repair_photo_caption_for_image(state, &target.node_id);
            if repaired > 0 {
                eprintln!(
                    "[SMOKE] photo contrast: {} repair(s) after {}",
                    repaired, target.node_id
                );
            }
            if let Some(url) = url {
                if dump {
                    eprintln!(
                        "[SMOKE] image applied: node_id={} url={url}",
                        target.node_id
                    );
                }
                report.filled += 1;
            } else if dump {
                eprintln!(
                    "[SMOKE] image search returned nothing: query={:?}; fallback tile applied",
                    target.query
                );
            }
        } else if dump {
            eprintln!(
                "[SMOKE] image apply failed: node_id={} (collab gate or node gone)",
                target.node_id
            );
        }

        if report.budget_exhausted {
            break;
        }
        if report.filled < total {
            let delay = Duration::from_millis(config.delay_ms);
            let delay =
                deadline.map_or(delay, |end| delay.min(end.saturating_duration_since(now())));
            std::thread::sleep(delay);
        }
    }

    let remaining_targets = op_image_enrich::collect_targets(state, &HashSet::new());
    let remaining = remaining_targets.len();
    report.unfilled = report.total.saturating_sub(report.filled);
    report.unattempted = report.total.saturating_sub(report.attempted);

    eprintln!(
        "[SMOKE] image fill done: filled {}/{total} target(s), {remaining} unresolved target(s), budget_exhausted={}",
        report.filled, report.budget_exhausted
    );
    report.log();
    report
}

/// One-line, URL-free record of the hit that filled an image slot:
/// `[IMAGE] <node id> query="…" -> <provider> "<hit title>" (via "…")`.
fn image_provenance_line(
    node_id: &str,
    query: &str,
    hit: Option<&op_image_enrich::net::fetch::ResolvedImage>,
) -> String {
    let query = op_image_enrich::net::search_trace::short_title(query, 60);
    match hit {
        Some(hit) => format!("[IMAGE] {node_id} query=\"{query}\" -> {}", hit.describe()),
        None => format!("[IMAGE] {node_id} query=\"{query}\" -> none (fallback tile)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::PenNode;
    use op_editor_core::{EditorState, NodeId};
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn failed_search_result_becomes_a_fallback_tile() {
        let mut state = EditorState::new();
        let node: PenNode = serde_json::from_value(serde_json::json!({
            "type": "image",
            "id": "smoke-slot",
            "src": "",
            "imageSearchQuery": "coffee cup",
            "width": 56,
            "height": 56
        }))
        .expect("image fixture");
        state.active_children_mut().push(node);

        assert!(op_image_enrich::apply_result(
            &mut state,
            &NodeId::new("smoke-slot"),
            op_image_enrich::SEARCH_FAILED_PLACEHOLDER_SRC
        ));
        let node =
            op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("smoke-slot"))
                .expect("fallback survives");
        assert!(op_image_enrich::is_image_fallback(node));
    }

    #[test]
    fn provenance_line_names_provider_and_hit_title_without_urls() {
        let hit = op_image_enrich::net::fetch::ResolvedImage {
            attribution: String::new(),
            original: None,
            src: "data:image/jpeg;base64,AAAA".to_string(),
            provider: "openverse",
            provider_query: "serum oil bottle".to_string(),
            title: "Dropper bottle, blank retro label".to_string(),
        };
        assert_eq!(
            image_provenance_line("n246", "serum oil bottle", Some(&hit)),
            "[IMAGE] n246 query=\"serum oil bottle\" -> openverse \"Dropper bottle, blank retro label\" (via \"serum oil bottle\")"
        );
        assert_eq!(
            image_provenance_line("n257", "cleanser tube white", None),
            "[IMAGE] n257 query=\"cleanser tube white\" -> none (fallback tile)"
        );
    }

    #[test]
    fn config_defaults_are_correct() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");
        let config = ImageFillConfig::from_env().unwrap();
        assert_eq!(config.limit, 40);
        assert_eq!(config.delay_ms, 250);
        assert!(!config.enabled);
        assert_eq!(config.budget, None);
    }

    #[test]
    fn budget_is_opt_in_and_invalid_values_fail_closed() {
        assert_eq!(parse_budget(None).unwrap(), None);
        assert_eq!(
            parse_budget(Some("120")).unwrap(),
            Some(Duration::from_secs(120))
        );
        for invalid in ["", "0", "-1", "abc", "1.5", "18446744073709551615"] {
            assert!(parse_budget(Some(invalid)).is_err(), "{invalid}");
        }
    }

    fn image_slots() -> EditorState {
        let mut state = EditorState::new();
        for id in ["slot-1", "slot-2"] {
            state.active_children_mut().push(
                serde_json::from_value(serde_json::json!({
                    "type": "image", "id": id, "src": "", "imageSearchQuery": "coffee cup",
                    "width": 56, "height": 56
                }))
                .unwrap(),
            );
        }
        state
    }

    fn test_config() -> ImageFillConfig {
        ImageFillConfig {
            enabled: true,
            limit: 40,
            delay_ms: 0,
            budget: None,
        }
    }

    #[test]
    fn expired_budget_starts_no_fetch_and_reports_all_unfilled() {
        let report = fill_images_with_fetch(
            &mut image_slots(),
            &test_config(),
            false,
            Some(Instant::now()),
            Instant::now,
            |_, _, _| panic!("expired stage must not start a request"),
        );
        assert_eq!(report.attempted, 0);
        assert_eq!(report.unattempted, 2);
        assert_eq!(report.no_hit, 0);
        assert_eq!(report.unfilled, 2);
        assert!(report.deadline_incomplete());
    }

    #[test]
    fn deadline_preserves_completed_image_and_stops_the_next_slot() {
        let mut state = image_slots();
        let clock = std::cell::Cell::new(Instant::now());
        let deadline = clock.get() + Duration::from_secs(10);
        let report = fill_images_with_fetch(
            &mut state,
            &test_config(),
            false,
            Some(deadline),
            || clock.get(),
            |_, _, passed_deadline| {
                assert_eq!(passed_deadline, Some(deadline));
                clock.set(deadline);
                Some(op_image_enrich::net::fetch::ResolvedImage {
                    src: "https://images.example/coffee.jpg".into(),
                    provider: "test",
                    provider_query: "coffee cup".into(),
                    title: "Coffee".into(),
                    attribution: String::new(),
                    original: None,
                })
            },
        );
        assert_eq!(report.attempted, 1);
        assert_eq!(report.filled, 1);
        assert_eq!(report.unfilled, 1);
        assert_eq!(report.unattempted, 1);
        assert_eq!(report.no_hit, 0);
        assert!(report.deadline_incomplete());
        let PenNode::Image(image) = &state.active_children()[0] else {
            panic!("image survived")
        };
        assert_eq!(image.src, "https://images.example/coffee.jpg");
    }

    #[test]
    fn completed_no_hit_search_is_visible_without_deadline_failure() {
        // Layout/font initialization uses real time, while this test concerns
        // completed no-hit searches with an unexpired stage deadline.
        let now = Instant::now();
        let deadline = now + Duration::from_secs(10);
        let report = fill_images_with_fetch(
            &mut image_slots(),
            &test_config(),
            false,
            Some(deadline),
            || now,
            |_, _, passed_deadline| {
                assert_eq!(passed_deadline, Some(deadline));
                None
            },
        );
        assert_eq!(report.attempted, 2);
        assert_eq!(report.filled, 0);
        assert_eq!(report.no_hit, 2);
        assert_eq!(report.deadline_limited, 0);
        assert_eq!(report.unattempted, 0);
        assert_eq!(report.unfilled, 2);
        assert!(!report.deadline_incomplete());
    }

    #[test]
    fn deadline_limited_search_is_separate_from_no_hit_and_unattempted() {
        let clock = std::cell::Cell::new(Instant::now());
        let deadline = clock.get() + Duration::from_secs(10);
        let report = fill_images_with_fetch(
            &mut image_slots(),
            &test_config(),
            false,
            Some(deadline),
            || clock.get(),
            |_, _, _| {
                clock.set(deadline - Duration::from_millis(250));
                None
            },
        );
        assert_eq!(report.attempted, 1);
        assert_eq!(report.deadline_limited, 1);
        assert_eq!(report.no_hit, 0);
        assert_eq!(report.unattempted, 1);
        assert!(report.deadline_incomplete());
    }

    #[test]
    fn config_enabled_by_env() {
        let _guard = TEST_LOCK.lock();
        std::env::set_var("OPENPENCIL_SMOKE_FILL_IMAGES", "1");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");
        let config = ImageFillConfig::from_env().unwrap();
        assert!(config.enabled);
    }

    #[test]
    fn config_parses_limit_from_env() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS");
        std::env::set_var("OPENPENCIL_SMOKE_IMAGE_LIMIT", "100");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");
        let config = ImageFillConfig::from_env().unwrap();
        assert_eq!(config.limit, 100);
    }

    #[test]
    fn config_parses_delay_from_env() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::set_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS", "500");
        let config = ImageFillConfig::from_env().unwrap();
        assert_eq!(config.delay_ms, 500);
    }

    #[test]
    fn config_parses_truthy_values() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");

        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS");
        std::env::set_var("OPENPENCIL_SMOKE_FILL_IMAGES", "true");
        let config = ImageFillConfig::from_env().unwrap();
        assert!(config.enabled);

        std::env::set_var("OPENPENCIL_SMOKE_FILL_IMAGES", "on");
        let config = ImageFillConfig::from_env().unwrap();
        assert!(config.enabled);
    }

    #[test]
    fn config_ignores_invalid_limit() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS");
        std::env::set_var("OPENPENCIL_SMOKE_IMAGE_LIMIT", "not_a_number");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS");
        let config = ImageFillConfig::from_env().unwrap();
        assert_eq!(config.limit, 40);
    }

    #[test]
    fn config_ignores_invalid_delay() {
        let _guard = TEST_LOCK.lock();
        std::env::remove_var("OPENPENCIL_SMOKE_FILL_IMAGES");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_BUDGET_SECS");
        std::env::remove_var("OPENPENCIL_SMOKE_IMAGE_LIMIT");
        std::env::set_var("OPENPENCIL_SMOKE_IMAGE_DELAY_MS", "xyz");
        let config = ImageFillConfig::from_env().unwrap();
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
