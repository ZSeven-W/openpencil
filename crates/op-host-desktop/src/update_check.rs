//! Background auto-update check.
//!
//! The TS Electron app ships a full `electron-updater` pipeline
//! (download + signature-verify + self-install). The Rust desktop
//! host has no signed-installer pipeline yet, so this module does
//! the part that is safe and useful without one: a background probe
//! of the GitHub releases API that compares the running build to
//! the latest published release and surfaces the result.
//!
//! Flow: [`UpdateProbe::spawn`] runs one HTTPS request on a worker
//! thread (never the UI thread); the runner drains the outcome with
//! [`UpdateProbe::poll`] and writes it into
//! `EditorUiState::update_status` (rendered by the settings System
//! tab). When a newer release is found the runner offers to open
//! the download page via [`open_url`].
//!
//! Every failure mode (offline, rate-limited, malformed JSON)
//! degrades to `UpdateStatus::Error` — the probe never panics and
//! never blocks shutdown.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_editor_core::UpdateStatus;

/// GitHub publish target — mirrors the TS app's `constants.ts`
/// (`GITHUB_OWNER` / `GITHUB_REPO`), the same repo `electron-updater`
/// reads releases from.
const GITHUB_OWNER: &str = "ZSeven-W";
const GITHUB_REPO: &str = "openpencil";

/// Releases page a found-update notice opens in the browser.
pub fn releases_url() -> String {
    format!("https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases")
}

/// `releases/latest` API endpoint.
fn latest_release_api() -> String {
    format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases/latest")
}

/// Background release-API probe. One request per `spawn`; the
/// runner re-spawns to re-check (e.g. from the "Check for Updates"
/// menu item).
pub struct UpdateProbe {
    rx: Option<Receiver<UpdateStatus>>,
}

impl UpdateProbe {
    /// Startup probe constructor that honors the persisted
    /// auto-update preference.
    pub fn for_auto_check(auto_update_enabled: bool) -> Self {
        if auto_update_enabled {
            Self::spawn()
        } else {
            Self::idle()
        }
    }

    /// Disabled auto-check state. Manual checks still call
    /// [`UpdateProbe::spawn`] directly.
    pub fn idle() -> Self {
        Self { rx: None }
    }

    /// Spawn the probe worker. Returns immediately; the request
    /// runs on its own thread.
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(check_latest_release());
        });
        Self { rx: Some(rx) }
    }

    /// Whether the probe worker is still running (result not yet
    /// drained). The runner uses this to keep waking the event loop
    /// so a result that lands while the app is idle is still drained.
    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// Drain the probe result if it has landed. Returns `Some` once
    /// (when the worker resolves), `None` while still in flight or
    /// after it has already been drained — so the runner can write
    /// the new status and act on a found update exactly once.
    pub fn poll(&mut self) -> Option<UpdateStatus> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(status) => {
                self.rx = None;
                Some(status)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                None
            }
        }
    }
}

/// Query `releases/latest` and classify the running build against
/// it. Blocking — only ever called on the probe worker thread.
fn check_latest_release() -> UpdateStatus {
    let Some(tag) = fetch_latest_tag() else {
        return UpdateStatus::Error;
    };
    let latest = tag.trim().trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");
    if is_newer(latest, current) {
        UpdateStatus::Available {
            version: latest.to_string(),
        }
    } else {
        UpdateStatus::UpToDate
    }
}

/// Run the HTTPS request. `reqwest` is async-only in this
/// workspace (no `blocking` feature), so the worker spins up a
/// single-threaded tokio runtime for the one call. `None` on any
/// transport / parse failure.
fn fetch_latest_tag() -> Option<String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            // GitHub's API rejects requests without a User-Agent.
            .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok()?;
        let resp = client
            .get(latest_release_api())
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let json: serde_json::Value = resp.json().await.ok()?;
        json.get("tag_name")?.as_str().map(str::to_string)
    })
}

/// Whether `latest` is a strictly newer version than `current`.
/// Both are dotted numeric strings (`0.8.0`); a pre-release suffix
/// (`-beta.1`) is dropped before the numeric compare so a stable
/// release of the same core version doesn't read as an update.
fn is_newer(latest: &str, current: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.split('-')
            .next()
            .unwrap_or(v)
            .split('.')
            .map(|n| n.trim().parse::<u64>().unwrap_or(0))
            .collect()
    }
    let l = parts(latest);
    let c = parts(current);
    let len = l.len().max(c.len());
    for i in 0..len {
        let lv = l.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if lv != cv {
            return lv > cv;
        }
    }
    false
}

/// Open `url` in the user's default browser. Best-effort: a missing
/// opener tool just logs. Platform openers — `open` (macOS),
/// `xdg-open` (Linux), `cmd /c start` (Windows).
pub fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    if let Err(e) = result {
        eprintln!("openpencil-desktop: open_url({url}) failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_compares_dotted_numbers() {
        assert!(is_newer("0.9.0", "0.8.0"));
        assert!(is_newer("0.8.1", "0.8.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.8.0", "0.8.0"));
        assert!(!is_newer("0.7.9", "0.8.0"));
    }

    #[test]
    fn is_newer_handles_uneven_segment_counts() {
        // A shorter version pads missing trailing segments with 0.
        assert!(is_newer("0.8.0.1", "0.8.0"));
        assert!(!is_newer("0.8", "0.8.0"));
        assert!(is_newer("0.9", "0.8.5"));
    }

    #[test]
    fn is_newer_drops_a_prerelease_suffix() {
        // A stable release equal to the running core version is not
        // "newer"; the `-beta` suffix must not skew the compare.
        assert!(!is_newer("0.8.0-beta.1", "0.8.0"));
        assert!(is_newer("0.9.0-rc.1", "0.8.0"));
    }

    #[test]
    fn releases_url_points_at_the_publish_repo() {
        assert_eq!(
            releases_url(),
            "https://github.com/ZSeven-W/openpencil/releases"
        );
    }

    #[test]
    fn idle_probe_is_not_pending() {
        let probe = UpdateProbe::idle();

        assert!(!probe.is_pending());
    }

    #[test]
    fn disabled_auto_check_creates_idle_probe() {
        let probe = UpdateProbe::for_auto_check(false);

        assert!(!probe.is_pending());
    }
}
