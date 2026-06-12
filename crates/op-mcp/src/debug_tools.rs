//! First-party MCP debug tools.
//!
//! `debug_validation_report` restores, on the Rust side, the TS
//! `debug_validation_report` debug tool (`packages/pen-mcp/src/tools/
//! debug-validation-report.ts`). It runs `op_design_lint::detect_all`
//! over the active page of the open document and serializes the result.
//!
//! ## Deliberately narrower than the TS tool (spec §7)
//!
//! The TS tool accepts `filePath` / `pageId` / `rootNodeId` /
//! `categories` / `maxIssues`. This Rust tool ships a no-parameter
//! contract: it reports `detect_all` over the active page of the
//! currently-open document. The per-issue JSON shape (`Issue`) is
//! byte-identical to TS (`#[serde(rename_all = "camelCase")]`), so a
//! client reading individual issues is unaffected.
//!
//! ## Read-only + env-gated
//!
//! The tool emits no `EditorCommand` and mutates nothing — it only
//! runs `detect_all`. It is gated behind `OPENPENCIL_DEBUG_TOOLS=1`
//! (the existing debug-tool isolation env flag); when the flag is
//! unset the tool surfaces a clean `ToolFailed` error at call time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jian_ops_schema::node::PenNode;
use op_design_lint::{detect_all, Issue};
use op_editor_core::EditorState;
use regex::{Regex, RegexBuilder};

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// The env flag that gates the debug tools — matches the isolation
/// flag the TS debug tooling uses (`OPENPENCIL_DEBUG_TOOLS=1`).
const DEBUG_TOOLS_ENV: &str = "OPENPENCIL_DEBUG_TOOLS";

/// Returns whether the debug tools are enabled (`OPENPENCIL_DEBUG_TOOLS=1`).
///
/// Public so the host MCP server can keep the debug tool out of the
/// production catalog entirely — both `rebuild_registry` and
/// `tools/list` consult this, so a client with the flag unset never
/// sees `debug_validation_report` at all (not just `ToolFailed` on call).
pub fn debug_tools_enabled() -> bool {
    // Tests force the gate via a thread-local override instead of mutating
    // process-global env — `setenv` races any concurrent `getenv` (e.g.
    // `temp_dir()` reading `TMPDIR`) in parallel tests, which is UB.
    #[cfg(test)]
    if let Some(forced) = debug_gate_override::get() {
        return forced;
    }
    std::env::var(DEBUG_TOOLS_ENV).is_ok_and(|v| v == "1")
}

/// Test-only thread-local override for [`debug_tools_enabled`]. The debug
/// tool `call()`s run on the test thread, so a thread-local is visible.
#[cfg(test)]
pub(crate) mod debug_gate_override {
    use std::cell::Cell;

    thread_local! {
        static GATE: Cell<Option<bool>> = const { Cell::new(None) };
    }

    pub(crate) fn get() -> Option<bool> {
        GATE.with(|gate| gate.get())
    }

    pub(crate) fn set(value: Option<bool>) {
        GATE.with(|gate| gate.set(value));
    }

    /// RAII guard that forces the gate and resets it on drop, so a
    /// panicking test can't leak the forced value into another test on
    /// the same thread.
    pub(crate) struct Guard;

    impl Guard {
        pub(crate) fn forcing(value: bool) -> Self {
            set(Some(value));
            Guard
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            set(None);
        }
    }
}

/// First-party `debug_validation_report` tool — runs the
/// `op-design-lint` detectors over the active page and returns the
/// `Issue` list. Read-only: snapshots the active page at registration,
/// emits no command.
pub struct DebugValidationReport {
    /// The detected issues, snapshotted at registration time.
    pub issues: Vec<Issue>,
}

pub struct DebugLogsTail {
    log_dir: PathBuf,
}

pub struct DebugScreenshot;

impl McpTool for DebugValidationReport {
    fn name(&self) -> &str {
        "debug_validation_report"
    }
    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        if !debug_tools_enabled() {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("debug tools are disabled — set {DEBUG_TOOLS_ENV}=1 to enable"),
            );
        }
        // Per-category breakdown — `;`-separated `category|count`
        // records, mirroring `list_node_kinds` / `count_nodes`.
        let mut by_category: BTreeMap<String, u32> = BTreeMap::new();
        for issue in &self.issues {
            let key = serde_json::to_value(issue.category)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            *by_category.entry(key).or_insert(0) += 1;
        }
        let categories: Vec<String> = by_category
            .iter()
            .map(|(cat, count)| format!("{cat}|{count}"))
            .collect();
        // The full issue list — serde keeps the `Issue` wire shape
        // byte-identical to the TS `debug_validation_report` output.
        let issues_json = match serde_json::to_string(&self.issues) {
            Ok(j) => j,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("failed to serialize issues: {e}"),
                )
            }
        };
        let mut out = BTreeMap::new();
        out.insert("count".into(), self.issues.len().to_string());
        out.insert("categories".into(), categories.join(";"));
        out.insert("issues".into(), issues_json);
        ToolOutcome::Ok(out)
    }
}

impl McpTool for DebugLogsTail {
    fn name(&self) -> &str {
        "debug_logs_tail"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if !debug_tools_enabled() {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("debug tools are disabled — set {DEBUG_TOOLS_ENV}=1 to enable"),
            );
        }
        let tail_lines = match parse_tail_lines(args) {
            Ok(v) => v,
            Err(msg) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg),
        };
        let since_ms = match parse_since_ms(args) {
            Ok(v) => v,
            Err(msg) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg),
        };
        let grep = match args.get("grep").map(|raw| Regex::new(raw)) {
            Some(Ok(re)) => Some(re),
            Some(Err(e)) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("grep must be a valid regex: {e}"),
                )
            }
            None => None,
        };
        let Some(path) = find_latest_log(&self.log_dir) else {
            return logs_tail_out(None, Vec::new());
        };
        let lines = read_log_tail(&path, tail_lines, since_ms, grep.as_ref()).unwrap_or_default();
        logs_tail_out(Some(&path), lines)
    }
}

/// Screenshot target — TS `DebugScreenshotParams.target`.
#[derive(Debug, Clone, PartialEq)]
pub enum ScreenshotTarget {
    /// The whole active-page content.
    Root,
    /// One node (+ subtree) by id.
    Node(String),
}

/// Validated `debug_screenshot` arguments, shared between the headless
/// tool (which then reports the honest no-live-canvas error) and the
/// live-GUI MCP server (which routes the request through the raster
/// export pipeline) — so both paths validate byte-identically.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenshotRequest {
    pub target: ScreenshotTarget,
    /// Extra doc-px around the captured bounds (TS default 0).
    pub padding: Option<f64>,
    /// Device-pixel-ratio / render scale (TS default 1 headlessly).
    pub dpr: Option<f64>,
    /// Render budget — TS `Math.min(timeoutMs ?? 15000, 60000)`.
    pub timeout_ms: u64,
}

/// Parse + validate `debug_screenshot` args (TS `debug-screenshot.ts`
/// contract). Errors carry the typed code + the exact TS messages.
pub fn parse_screenshot_args(
    args: &BTreeMap<String, String>,
) -> Result<ScreenshotRequest, (ToolErrorCode, String)> {
    let Some(target) = args.get("target") else {
        return Err((ToolErrorCode::MissingArgument, "target is required".into()));
    };
    let target = match target.as_str() {
        "node" => match args
            .get("nodeId")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            Some(node_id) => ScreenshotTarget::Node(node_id.to_string()),
            None => {
                return Err((
                    ToolErrorCode::InvalidArgument,
                    "target=\"node\" requires nodeId".into(),
                ));
            }
        },
        "root" => ScreenshotTarget::Root,
        other => {
            return Err((
                ToolErrorCode::InvalidArgument,
                format!("target must be node or root, got {other:?}"),
            ));
        }
    };
    let padding = parse_f64_arg(args, "padding")?;
    let dpr = parse_f64_arg(args, "dpr")?;
    let timeout_ms = match args.get("timeoutMs").or_else(|| args.get("timeout_ms")) {
        None => 15_000,
        Some(raw) => raw
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| (v.max(0.0) as u64).min(60_000))
            .ok_or_else(|| {
                (
                    ToolErrorCode::InvalidArgument,
                    format!("timeoutMs must be a number, got {raw:?}"),
                )
            })?,
    };
    Ok(ScreenshotRequest {
        target,
        padding,
        dpr,
        timeout_ms,
    })
}

fn parse_f64_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<f64>, (ToolErrorCode, String)> {
    match args.get(key) {
        None => Ok(None),
        Some(raw) => raw
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(Some)
            .ok_or_else(|| {
                (
                    ToolErrorCode::InvalidArgument,
                    format!("{key} must be a number, got {raw:?}"),
                )
            }),
    }
}

impl McpTool for DebugScreenshot {
    fn name(&self) -> &str {
        "debug_screenshot"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        if !debug_tools_enabled() {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("debug tools are disabled — set {DEBUG_TOOLS_ENV}=1 to enable"),
            );
        }
        match parse_screenshot_args(args) {
            // Headless / file-backed MCP has no renderer — same honest
            // error TS standalone mode reports. The live-GUI server
            // intercepts `debug_screenshot` BEFORE this tool dispatches
            // and fulfills it via the raster export pipeline.
            Ok(_) => ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                "No live canvas available. Ensure an Electron window or /editor tab is running."
                    .into(),
            ),
            Err((code, message)) => ToolOutcome::Err(code, message),
        }
    }
}

/// Snapshot the active page + run `op_design_lint::detect_all` over
/// every top-level node of the active page.
///
/// `detect_all` walks a single root `PenNode`; a page can hold several
/// top-level nodes, so we run the detectors once per top-level node and
/// concatenate — the same multi-root discipline `apply_fixes` uses.
pub fn debug_validation_report_snapshot(state: &EditorState) -> DebugValidationReport {
    let roots: &[PenNode] = state.active_children();
    let mut issues = Vec::new();
    for root in roots {
        issues.extend(detect_all(root, &state.doc));
    }
    DebugValidationReport { issues }
}

pub fn debug_logs_tail_snapshot() -> DebugLogsTail {
    debug_logs_tail_snapshot_for_log_dir(default_log_dir())
}

pub fn debug_logs_tail_snapshot_for_log_dir(log_dir: PathBuf) -> DebugLogsTail {
    DebugLogsTail { log_dir }
}

pub fn debug_screenshot_snapshot() -> DebugScreenshot {
    DebugScreenshot
}

fn default_log_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openpencil")
        .join("logs")
}

fn find_latest_log(dir: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_server_log_name(&name) {
            candidates.push(name.to_string());
        }
    }
    candidates.sort();
    candidates.pop().map(|name| dir.join(name))
}

fn is_server_log_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == "server-YYYY-MM-DD.log".len()
        && name.starts_with("server-")
        && name.ends_with(".log")
        && bytes[7..11].iter().all(u8::is_ascii_digit)
        && bytes[11] == b'-'
        && bytes[12..14].iter().all(u8::is_ascii_digit)
        && bytes[14] == b'-'
        && bytes[15..17].iter().all(u8::is_ascii_digit)
}

fn read_log_tail(
    path: &Path,
    tail_lines: usize,
    since_ms: Option<i64>,
    grep: Option<&Regex>,
) -> std::io::Result<Vec<String>> {
    let sensitive =
        RegexBuilder::new(r"ANTHROPIC_API_KEY=|Authorization:\s*Bearer|api[_-]?key\s*[:=]")
            .case_insensitive(true)
            .build()
            .expect("sensitive log regex");
    let raw = std::fs::read_to_string(path)?;
    let mut filtered = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        if sensitive.is_match(line) {
            continue;
        }
        if since_ms.is_some_and(|since| {
            parse_iso_timestamp_ms(line).is_some_and(|line_ms| line_ms < since)
        }) {
            continue;
        }
        if grep.is_some_and(|re| !re.is_match(line)) {
            continue;
        }
        filtered.push(line.to_string());
    }
    let start = filtered.len().saturating_sub(tail_lines);
    Ok(filtered[start..].to_vec())
}

fn logs_tail_out(path: Option<&Path>, lines: Vec<String>) -> ToolOutcome {
    let lines_json = match serde_json::to_string(&lines) {
        Ok(json) => json,
        Err(e) => {
            return ToolOutcome::Err(
                ToolErrorCode::Internal,
                format!("failed to serialize log lines: {e}"),
            )
        }
    };
    let mut out = BTreeMap::new();
    out.insert("source".into(), "server".into());
    out.insert(
        "path".into(),
        path.map(|p| p.display().to_string()).unwrap_or_default(),
    );
    out.insert("totalLines".into(), lines.len().to_string());
    out.insert("lines".into(), lines_json);
    ToolOutcome::Ok(out)
}

fn parse_tail_lines(args: &BTreeMap<String, String>) -> Result<usize, String> {
    let Some(raw) = args.get("tailLines").or_else(|| args.get("tail_lines")) else {
        return Ok(100);
    };
    let parsed = raw
        .parse::<usize>()
        .map_err(|_| format!("tailLines must be a non-negative integer, got {raw:?}"))?;
    Ok(parsed.min(500))
}

fn parse_since_ms(args: &BTreeMap<String, String>) -> Result<Option<i64>, String> {
    let Some(raw) = args.get("sinceMs").or_else(|| args.get("since_ms")) else {
        return Ok(None);
    };
    raw.parse::<i64>()
        .map(Some)
        .map_err(|_| format!("sinceMs must be an integer unix-ms timestamp, got {raw:?}"))
}

fn parse_iso_timestamp_ms(line: &str) -> Option<i64> {
    if line.len() < 20 || line.as_bytes().get(19) != Some(&b'Z') && !line[19..].contains('Z') {
        return None;
    }
    let year = line.get(0..4)?.parse::<i32>().ok()?;
    let month = line.get(5..7)?.parse::<u32>().ok()?;
    let day = line.get(8..10)?.parse::<u32>().ok()?;
    let hour = line.get(11..13)?.parse::<i64>().ok()?;
    let minute = line.get(14..16)?.parse::<i64>().ok()?;
    let second = line.get(17..19)?.parse::<i64>().ok()?;
    let millis = if line.as_bytes().get(19) == Some(&b'.') {
        let z = line[20..].find('Z')? + 20;
        let mut frac = line.get(20..z)?.chars().take(3).collect::<String>();
        while frac.len() < 3 {
            frac.push('0');
        }
        frac.parse::<i64>().ok()?
    } else {
        0
    };
    let days = days_from_civil(year, month, day);
    Some((((days * 24 + hour) * 60 + minute) * 60 + second) * 1000 + millis)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month as i32 + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{frame, state_with};
    use jian_ops_schema::node::PenNodeBase;
    use serde_json::Value;

    /// Build a frame whose `base.rotation` is set — triggers the
    /// `unexpected-rotation` detector (UI nodes rarely tilt on purpose).
    fn rotated_frame() -> PenNode {
        let mut node = frame("f1", "tilted", 0.0, 0.0, 100.0, 100.0, vec![]);
        if let PenNode::Frame(f) = &mut node {
            f.base = PenNodeBase {
                rotation: Some(12.0),
                ..f.base.clone()
            };
        }
        node
    }

    /// One test covers the gate + the report shape: the three cases all
    /// flip the same process-wide env var, so they MUST run as a single
    /// `#[test]` — separate tests would race under cargo's parallel
    /// runner. The snapshot (`detect_all`) is env-independent; only the
    /// `call()` gate reads `OPENPENCIL_DEBUG_TOOLS`.
    #[test]
    fn debug_tools_respect_the_env_gate_and_match_ts_debug_behaviour() {
        let bad = debug_validation_report_snapshot(&state_with(vec![rotated_frame()]));
        let clean_frame = frame("f1", "clean", 0.0, 0.0, 100.0, 100.0, vec![]);
        let clean = debug_validation_report_snapshot(&state_with(vec![clean_frame]));
        let empty = BTreeMap::new();
        let log_dir = temp_dir("logs");
        std::fs::create_dir_all(&log_dir).expect("log dir");
        let log_path = log_dir.join("server-2024-01-02.log");
        std::fs::write(
            &log_path,
            "2024-01-02T00:00:00.000Z skip old\n\
             2024-01-02T00:00:02.000Z keep alpha\n\
             Authorization: Bearer secret\n\
             no timestamp keep beta\n\
             2024-01-02T00:00:03.000Z error gamma\n",
        )
        .expect("log file");
        let logs = debug_logs_tail_snapshot_for_log_dir(log_dir.clone());
        let screenshot = debug_screenshot_snapshot();

        // Gate closed — the tool rejects regardless of document state.
        // Forced via a thread-local (no process-global env mutation); the
        // guard resets the gate on drop, even if an assertion panics.
        let _gate = debug_gate_override::Guard::forcing(false);
        match bad.call(&empty) {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::ToolFailed);
                assert!(msg.contains(DEBUG_TOOLS_ENV));
            }
            other => panic!("expected ToolFailed when gate is closed, got {other:?}"),
        }
        match logs.call(&empty) {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::ToolFailed);
                assert!(msg.contains(DEBUG_TOOLS_ENV));
            }
            other => panic!("expected ToolFailed for logs when gate is closed, got {other:?}"),
        }

        // Gate open — a known-bad document reports its issue.
        debug_gate_override::set(Some(true));
        match bad.call(&empty) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("count"), Some(&"1".to_string()));
                // Per-category breakdown carries the rotation detector.
                assert_eq!(
                    out.get("categories"),
                    Some(&"unexpected-rotation|1".to_string())
                );
                // The serialized issue list round-trips back to `Issue`.
                let issues_json = out.get("issues").expect("issues field");
                let issues: Vec<Issue> = serde_json::from_str(issues_json).unwrap();
                assert_eq!(issues.len(), 1);
                assert_eq!(issues[0].node_id, "f1");
            }
            other => panic!("expected Ok for the bad document, got {other:?}"),
        }

        // Gate open — a clean document reports zero issues.
        match clean.call(&empty) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("count"), Some(&"0".to_string()));
                assert_eq!(out.get("categories"), Some(&String::new()));
                assert_eq!(out.get("issues"), Some(&"[]".to_string()));
            }
            other => panic!("expected Ok for the clean document, got {other:?}"),
        }

        let mut log_args = BTreeMap::new();
        log_args.insert("tailLines".into(), "2".into());
        log_args.insert("sinceMs".into(), "1704153601000".into());
        log_args.insert("grep".into(), "keep|error".into());
        match logs.call(&log_args) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("source"), Some(&"server".to_string()));
                assert_eq!(out.get("path"), Some(&log_path.display().to_string()));
                assert_eq!(out.get("totalLines"), Some(&"2".to_string()));
                let lines: Value =
                    serde_json::from_str(out.get("lines").expect("lines json")).unwrap();
                assert_eq!(
                    lines,
                    serde_json::json!([
                        "no timestamp keep beta",
                        "2024-01-02T00:00:03.000Z error gamma"
                    ])
                );
            }
            other => panic!("expected filtered log tail, got {other:?}"),
        }

        let missing_logs = debug_logs_tail_snapshot_for_log_dir(log_dir.join("missing"));
        match missing_logs.call(&empty) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("source"), Some(&"server".to_string()));
                assert_eq!(out.get("path"), Some(&String::new()));
                assert_eq!(out.get("totalLines"), Some(&"0".to_string()));
                assert_eq!(out.get("lines"), Some(&"[]".to_string()));
            }
            other => panic!("expected empty log tail, got {other:?}"),
        }

        let mut bad_screenshot_args = BTreeMap::new();
        bad_screenshot_args.insert("target".into(), "node".into());
        match screenshot.call(&bad_screenshot_args) {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument);
                assert_eq!(msg, "target=\"node\" requires nodeId");
            }
            other => panic!("expected nodeId validation error, got {other:?}"),
        }
        let mut root_screenshot_args = BTreeMap::new();
        root_screenshot_args.insert("target".into(), "root".into());
        match screenshot.call(&root_screenshot_args) {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::ToolFailed);
                assert_eq!(
                    msg,
                    "No live canvas available. Ensure an Electron window or /editor tab is running."
                );
            }
            other => panic!("expected live-canvas screenshot error, got {other:?}"),
        }

        // `_gate` resets the override to None on drop.
        let _ = std::fs::remove_dir_all(&log_dir);
    }

    #[test]
    fn parse_screenshot_args_mirrors_the_ts_contract() {
        // Missing target.
        let err = parse_screenshot_args(&BTreeMap::new()).unwrap_err();
        assert_eq!(
            err,
            (ToolErrorCode::MissingArgument, "target is required".into())
        );
        // target=node without nodeId.
        let mut args = BTreeMap::new();
        args.insert("target".into(), "node".into());
        let err = parse_screenshot_args(&args).unwrap_err();
        assert_eq!(
            err,
            (
                ToolErrorCode::InvalidArgument,
                "target=\"node\" requires nodeId".into()
            )
        );
        // Unknown target.
        args.insert("target".into(), "page".into());
        let err = parse_screenshot_args(&args).unwrap_err();
        assert_eq!(err.0, ToolErrorCode::InvalidArgument);
        assert!(err.1.contains("target must be node or root"), "{}", err.1);
        // Full node request with TS timeout cap (min(x, 60000)).
        args.insert("target".into(), "node".into());
        args.insert("nodeId".into(), "n7".into());
        args.insert("padding".into(), "8".into());
        args.insert("dpr".into(), "2".into());
        args.insert("timeoutMs".into(), "90000".into());
        let req = parse_screenshot_args(&args).expect("valid request");
        assert_eq!(req.target, ScreenshotTarget::Node("n7".into()));
        assert_eq!(req.padding, Some(8.0));
        assert_eq!(req.dpr, Some(2.0));
        assert_eq!(req.timeout_ms, 60_000);
        // Defaults: TS `timeoutMs ?? 15000`, padding/dpr absent.
        let mut root = BTreeMap::new();
        root.insert("target".into(), "root".into());
        let req = parse_screenshot_args(&root).expect("root request");
        assert_eq!(req.target, ScreenshotTarget::Root);
        assert_eq!(req.timeout_ms, 15_000);
        assert_eq!(req.padding, None);
        assert_eq!(req.dpr, None);
        // Non-numeric scalar opts reject with a typed error.
        root.insert("dpr".into(), "huge".into());
        let err = parse_screenshot_args(&root).unwrap_err();
        assert_eq!(err.0, ToolErrorCode::InvalidArgument);
        assert!(err.1.contains("dpr must be a number"), "{}", err.1);
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "openpencil-debug-tools-{label}-{}-{nanos}",
            std::process::id()
        ))
    }
}
