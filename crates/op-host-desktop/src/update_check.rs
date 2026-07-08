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

/// Releases list API endpoint. GitHub's `releases/latest` endpoint excludes
/// prereleases, so the desktop updater uses the list feed and picks the first
/// non-draft release itself.
fn latest_release_api() -> String {
    format!("https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/releases?per_page=20")
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

/// Query the releases list and classify the running build against the newest
/// non-draft release, including prereleases. Blocking — only ever called on the
/// probe worker thread.
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
        select_latest_release_tag(&json)
    })
}

/// Select the newest published release tag from GitHub's releases list.
/// GitHub returns newest releases first. Drafts are not visible to users and
/// must not drive update prompts; prereleases are visible and are intentionally
/// allowed for v0.8.0.
fn select_latest_release_tag(json: &serde_json::Value) -> Option<String> {
    json.as_array()?
        .iter()
        .filter(|release| {
            !release
                .get("draft")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
        .find_map(|release| release.get("tag_name")?.as_str().map(str::to_string))
}

/// Download the platform installer for `version` on a worker thread
/// and open it when done (macOS mounts the DMG, Windows launches the
/// NSIS setup, Linux reveals the AppImage after `chmod +x`). Any
/// failure — unknown platform, network error, missing asset — falls
/// back to opening the releases page, so the user always ends up
/// with a path to the update. Mirrors the electron-updater flow
/// minus signature verification (no signing pipeline yet; the
/// artifacts are the rust-release.yml installer matrix).
pub fn download_and_open_installer(version: &str) {
    use std::sync::atomic::{AtomicBool, Ordering};
    // One download at a time — a second "Yes" (or re-check) while a
    // worker is still transferring must not spawn a duplicate writer
    // onto the same temp path.
    static IN_FLIGHT: AtomicBool = AtomicBool::new(false);
    if IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let version = version.to_string();
    std::thread::spawn(move || {
        if !download_and_open_blocking(&version) {
            open_url(&releases_url());
        }
        IN_FLIGHT.store(false, Ordering::SeqCst);
    });
}

/// The release-asset filename rust-release.yml publishes for this
/// platform, or `None` on an unsupported os/arch pair.
fn installer_asset_name(version: &str) -> Option<String> {
    asset_name_for(std::env::consts::OS, std::env::consts::ARCH, version)
}

/// Pure name table — split from [`installer_asset_name`] so tests
/// cover every platform from any host.
fn asset_name_for(os: &str, arch: &str, version: &str) -> Option<String> {
    let arch = match arch {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        _ => return None,
    };
    Some(match os {
        "macos" => format!("OpenPencil-{version}-{arch}-mac.dmg"),
        "windows" => format!("OpenPencil-{version}-{arch}-win-setup.exe"),
        "linux" => format!("OpenPencil-{version}-{arch}-linux.AppImage"),
        _ => return None,
    })
}

/// Blocking download + open. Only ever runs on the worker thread.
fn download_and_open_blocking(version: &str) -> bool {
    let Some(name) = installer_asset_name(version) else {
        return false;
    };
    let url = format!(
        "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/v{version}/{name}"
    );
    let dest = std::env::temp_dir().join(&name);
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return false,
    };
    let downloaded = runtime.block_on(async {
        let client = reqwest::Client::builder()
            // Installers run tens of MB — allow a long transfer.
            .timeout(std::time::Duration::from_secs(600))
            .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .ok()?;
        let mut resp = client.get(&url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        // Stream to disk chunk by chunk — installers run tens of MB
        // and must not be buffered whole in memory.
        let mut file = std::fs::File::create(&dest).ok()?;
        use std::io::Write;
        while let Some(chunk) = resp.chunk().await.ok()? {
            file.write_all(&chunk).ok()?;
        }
        file.flush().ok()?;
        Some(())
    });
    if downloaded.is_none() {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    }
    open_installer_path(&dest)
}

/// Args for `cmd /C start "" "<target>"` with the target double-quoted
/// so cmd doesn't split it at `&` (OAuth / query-string URLs) or at
/// spaces (installer paths). Inside double quotes cmd keeps its
/// metacharacters literal; `"` itself is illegal in URLs and Windows
/// paths but is stripped defensively. `%VAR%` expansion remains
/// possible in pathological inputs — accepted, matches the `open`
/// crate's behaviour.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn windows_start_args(target: &str) -> String {
    format!("/C start \"\" \"{}\"", target.replace('"', "%22"))
}

/// Spawn `cmd /C start "" "<target>"` without flashing a console.
#[cfg(target_os = "windows")]
fn windows_start(target: &str) -> std::io::Result<std::process::Child> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut c = std::process::Command::new("cmd");
    c.raw_arg(windows_start_args(target));
    c.creation_flags(CREATE_NO_WINDOW);
    c.spawn()
}

/// Open the downloaded installer with the platform launcher.
fn open_installer_path(path: &std::path::Path) -> bool {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let result = windows_start(&path.display().to_string());
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(path).spawn();
    result.is_ok()
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
    let result = windows_start(url);
    if let Err(e) = result {
        eprintln!("openpencil-desktop: open_url({url}) failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_start_args_quotes_urls_with_query_params() {
        assert_eq!(
            windows_start_args("https://a.example/auth?b=1&c=2"),
            "/C start \"\" \"https://a.example/auth?b=1&c=2\""
        );
        // Double quotes are illegal in URLs/paths — stripped defensively.
        assert_eq!(windows_start_args("x\"y"), "/C start \"\" \"x%22y\"");
    }

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
    fn releases_api_uses_list_endpoint_so_prereleases_are_visible() {
        assert_eq!(
            latest_release_api(),
            "https://api.github.com/repos/ZSeven-W/openpencil/releases?per_page=20"
        );
    }

    #[test]
    fn release_list_selection_keeps_prerelease_and_skips_drafts() {
        let feed = serde_json::json!([
            {
                "tag_name": "v0.9.0",
                "draft": true,
                "prerelease": false
            },
            {
                "tag_name": "v0.8.0",
                "draft": false,
                "prerelease": true
            },
            {
                "tag_name": "v0.7.5",
                "draft": false,
                "prerelease": false
            }
        ]);

        assert_eq!(select_latest_release_tag(&feed).as_deref(), Some("v0.8.0"));
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

#[cfg(test)]
mod installer_asset_tests {
    use super::asset_name_for;

    #[test]
    fn asset_names_cover_the_release_matrix() {
        assert_eq!(
            asset_name_for("macos", "aarch64", "0.9.0").as_deref(),
            Some("OpenPencil-0.9.0-arm64-mac.dmg")
        );
        assert_eq!(
            asset_name_for("windows", "x86_64", "0.9.0").as_deref(),
            Some("OpenPencil-0.9.0-x64-win-setup.exe")
        );
        assert_eq!(
            asset_name_for("linux", "aarch64", "0.9.0").as_deref(),
            Some("OpenPencil-0.9.0-arm64-linux.AppImage")
        );
        assert_eq!(asset_name_for("freebsd", "x86_64", "0.9.0"), None);
        assert_eq!(asset_name_for("macos", "riscv64", "0.9.0"), None);
    }
}
