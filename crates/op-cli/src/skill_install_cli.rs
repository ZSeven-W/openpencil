use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::skill_install_cline;
use crate::skill_install_error::{BundleError, FsAction, SkillInstallError};

type Result<T> = std::result::Result<T, SkillInstallError>;

const BUNDLE_JSON: &str = include_str!("../assets/skill-bundle.json");
pub(crate) const VERSION_SENTINEL: &str = "__OPENPENCIL_VERSION__";
// Top-level bundle version plus four embedded plugin/package manifest versions.
const EXPECTED_VERSION_SENTINEL_COUNT: usize = 5;
const REPO: &str = "zseven-w/openpencil-skill";
pub(crate) const SKILL_NAME: &str = "openpencil-skill";

#[derive(Debug, Clone)]
pub(crate) struct SkillBundle {
    pub(crate) version: String,
    pub(crate) files: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Claude,
    Codex,
    Cursor,
    OpenCode,
    Cline,
}

impl Target {
    fn parse(raw: &str) -> Result<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" => Ok(Target::Claude),
            "codex" => Ok(Target::Codex),
            "cursor" => Ok(Target::Cursor),
            "opencode" | "open-code" => Ok(Target::OpenCode),
            "cline" | "cline-cli" => Ok(Target::Cline),
            _ => Err(SkillInstallError::UnknownTarget(raw.to_string())),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Target::Claude => "claude",
            Target::Codex => "codex",
            Target::Cursor => "cursor",
            Target::OpenCode => "opencode",
            Target::Cline => "cline",
        }
    }
}

pub(crate) fn run_install(target: Option<&str>) -> Result<String> {
    run_for_home(Action::Install, target, &home_dir()?)
}

pub(crate) fn run_uninstall(target: Option<&str>) -> Result<String> {
    run_for_home(Action::Uninstall, target, &home_dir()?)
}

#[cfg(test)]
pub(crate) fn install_target_at_home(target: &str, home: &Path) -> Result<()> {
    let bundle = load_bundle()?;
    install_target(Target::parse(target)?, home, &bundle)
}

#[cfg(test)]
pub(crate) fn uninstall_target_at_home(target: &str, home: &Path) -> Result<()> {
    uninstall_target(Target::parse(target)?, home)
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Install,
    Uninstall,
}

fn run_for_home(action: Action, target: Option<&str>, home: &Path) -> Result<String> {
    let targets = resolve_targets(target, action, home)?;
    let bundle = load_bundle()?;
    let mut results = Vec::new();
    for target in targets {
        let result = match action {
            Action::Install => install_target(target, home, &bundle),
            Action::Uninstall => uninstall_target(target, home),
        };
        results.push(match result {
            Ok(()) => json!({ "target": target.key(), "ok": true }),
            Err(error) => {
                json!({ "target": target.key(), "ok": false, "error": error.to_string() })
            }
        });
    }
    Ok(json!({
        "ok": results.iter().all(|r| r["ok"].as_bool() == Some(true)),
        "action": match action { Action::Install => "install", Action::Uninstall => "uninstall" },
        "skill": SKILL_NAME,
        "version": bundle.version,
        "targets": results,
    })
    .to_string())
}

fn resolve_targets(target: Option<&str>, action: Action, home: &Path) -> Result<Vec<Target>> {
    if let Some(target) = target {
        return Ok(vec![Target::parse(target)?]);
    }
    let detected = detect_targets(home);
    if detected.is_empty() && matches!(action, Action::Install) {
        return Err(SkillInstallError::NoTargetsDetected);
    }
    Ok(detected)
}

fn detect_targets(home: &Path) -> Vec<Target> {
    let mut targets = Vec::new();
    if command_exists("claude") {
        targets.push(Target::Claude);
    }
    if command_exists("codex") {
        targets.push(Target::Codex);
    }
    if home.join(".cursor").exists() {
        targets.push(Target::Cursor);
    }
    if command_exists("opencode") {
        targets.push(Target::OpenCode);
    }
    // The CLI ships as an npm shim; the VS Code extension has no binary but
    // shares the same `~/.cline/skills` scan root, so an existing root counts.
    if command_exists("cline") || cline_root(home, env::var_os("CLINE_DIR")).exists() {
        targets.push(Target::Cline);
    }
    targets
}

fn command_exists(name: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path_var).any(|dir| {
        let candidate = dir.join(name);
        candidate.is_file() || executable_shim_exists(&candidate)
    })
}

#[cfg(windows)]
fn executable_shim_exists(candidate: &Path) -> bool {
    // npm-installed CLIs (Cline among them) land on PATH as `.cmd` / `.ps1`
    // shims rather than `.exe` binaries.
    ["exe", "cmd", "bat"]
        .iter()
        .any(|ext| candidate.with_extension(ext).is_file())
}

#[cfg(not(windows))]
fn executable_shim_exists(candidate: &Path) -> bool {
    candidate.with_extension("exe").is_file()
}

fn install_target(target: Target, home: &Path, bundle: &SkillBundle) -> Result<()> {
    match target {
        Target::Claude => install_claude(home, bundle),
        Target::Codex => install_codex(home, bundle),
        Target::Cursor => write_bundle_to(&home.join(".cursor/plugins").join(SKILL_NAME), bundle),
        Target::OpenCode => install_opencode(home, bundle),
        Target::Cline => {
            skill_install_cline::install(&cline_root(home, env::var_os("CLINE_DIR")), bundle)
        }
    }
}

fn uninstall_target(target: Target, home: &Path) -> Result<()> {
    match target {
        Target::Claude => uninstall_claude(home),
        Target::Codex => uninstall_codex(home),
        Target::Cursor => remove_path(&home.join(".cursor/plugins").join(SKILL_NAME)),
        Target::OpenCode => uninstall_opencode(home),
        Target::Cline => {
            skill_install_cline::uninstall(&cline_root(home, env::var_os("CLINE_DIR")))
        }
    }
}

/// Cline's global root: `CLINE_DIR` when set (and non-blank), else
/// `~/.cline` — the same precedence as Cline's `resolveClineDir()`.
pub(crate) fn cline_root(home: &Path, cline_dir: Option<std::ffi::OsString>) -> PathBuf {
    cline_dir
        .filter(|dir| !dir.to_string_lossy().trim().is_empty())
        .map(|dir| PathBuf::from(dir.to_string_lossy().trim()))
        .unwrap_or_else(|| home.join(".cline"))
}

fn install_claude(home: &Path, bundle: &SkillBundle) -> Result<()> {
    let cache_dir = home
        .join(".claude/plugins/cache")
        .join(SKILL_NAME)
        .join(SKILL_NAME)
        .join(&bundle.version);
    write_bundle_to(&cache_dir, bundle)?;

    let registry_path = home.join(".claude/plugins/installed_plugins.json");
    let mut registry = read_json_object(&registry_path)?;
    registry
        .entry("version")
        .or_insert_with(|| Value::Number(2.into()));
    let plugins = object_entry(&mut registry, "plugins")?;
    plugins.insert(
        format!("{SKILL_NAME}@{SKILL_NAME}"),
        json!([{
            "scope": "user",
            "installPath": cache_dir.display().to_string(),
            "version": bundle.version,
            "installedAt": timestamp_string(),
            "lastUpdated": timestamp_string(),
        }]),
    );
    write_json_object(&registry_path, &registry)?;

    let marketplace_path = home.join(".claude/plugins/known_marketplaces.json");
    let mut marketplaces = read_json_object(&marketplace_path)?;
    marketplaces.entry(SKILL_NAME).or_insert_with(|| {
        json!({
            "source": { "source": "github", "repo": REPO },
            "installLocation": home.join(".claude/plugins/marketplaces").join(SKILL_NAME).display().to_string(),
            "lastUpdated": timestamp_string(),
        })
    });
    write_json_object(&marketplace_path, &marketplaces)
}

fn uninstall_claude(home: &Path) -> Result<()> {
    remove_path(&home.join(".claude/plugins/cache").join(SKILL_NAME))?;
    let registry_path = home.join(".claude/plugins/installed_plugins.json");
    if registry_path.exists() {
        let mut registry = read_json_object(&registry_path)?;
        if let Some(plugins) = registry.get_mut("plugins").and_then(Value::as_object_mut) {
            plugins.remove(&format!("{SKILL_NAME}@{SKILL_NAME}"));
        }
        write_json_object(&registry_path, &registry)?;
    }
    Ok(())
}

fn install_codex(home: &Path, bundle: &SkillBundle) -> Result<()> {
    let clone_dir = home.join(".codex").join(SKILL_NAME);
    write_bundle_to(&clone_dir, bundle)?;

    let skills_dir = home.join(".agents/skills");
    fs::create_dir_all(&skills_dir)
        .map_err(|e| SkillInstallError::fs(FsAction::Create, &skills_dir, e))?;
    let link_path = skills_dir.join(SKILL_NAME);
    let link_target = clone_dir.join("skills");
    if fs::symlink_metadata(&link_path).is_err() {
        link_or_copy_dir(&link_target, &link_path)?;
    }
    Ok(())
}

fn uninstall_codex(home: &Path) -> Result<()> {
    remove_path(&home.join(".agents/skills").join(SKILL_NAME))?;
    remove_path(&home.join(".codex").join(SKILL_NAME))
}

fn install_opencode(home: &Path, bundle: &SkillBundle) -> Result<()> {
    // opencode discovers skills by scanning its config directory for
    // `{skill,skills}/**/SKILL.md` (packages/opencode/src/skill/index.ts).
    // A `plugin` array entry does NOT work for skills: the npm/git package
    // is installed but never scanned, and this bundle has no JS entrypoint,
    // so the earlier plugin-entry approach delivered nothing. Mirror the
    // codex layout instead: bundle beside the config, skills dir linked in.
    let bundle_dir = home.join(".config/opencode").join(SKILL_NAME);
    write_bundle_to(&bundle_dir, bundle)?;

    let skills_dir = home.join(".config/opencode/skills");
    fs::create_dir_all(&skills_dir)
        .map_err(|e| SkillInstallError::fs(FsAction::Create, &skills_dir, e))?;
    let link_path = skills_dir.join(SKILL_NAME);
    let link_target = bundle_dir.join("skills");
    // The discovery entry is owned by this installer: recreate it on every
    // install so a stale symlink, plain file, or outdated copied directory
    // can't shadow the freshly written bundle.
    remove_path(&link_path)?;
    link_or_copy_dir(&link_target, &link_path)?;
    // Prune the legacy no-op plugin entry from configs written by older
    // versions of this installer.
    prune_opencode_plugin_entry(home)
}

fn uninstall_opencode(home: &Path) -> Result<()> {
    remove_path(&home.join(".config/opencode/skills").join(SKILL_NAME))?;
    remove_path(&home.join(".config/opencode").join(SKILL_NAME))?;
    prune_opencode_plugin_entry(home)
}

/// Remove the legacy `openpencil-skill@git+…` plugin entry (older installers
/// wrote it; opencode installs the package but never loads anything from it).
fn prune_opencode_plugin_entry(home: &Path) -> Result<()> {
    let config_path = home.join(".config/opencode/opencode.json");
    if !config_path.exists() {
        return Ok(());
    }
    let mut config = read_json_object(&config_path)?;
    let plugins = array_entry(&mut config, "plugin");
    plugins.retain(|value| !value.as_str().is_some_and(|p| p.contains(SKILL_NAME)));
    write_json_object(&config_path, &config)
}

fn render_bundle_template(
    template: &str,
    version: &str,
) -> std::result::Result<String, BundleError> {
    let sentinel_count = template.matches(VERSION_SENTINEL).count();
    if sentinel_count != EXPECTED_VERSION_SENTINEL_COUNT {
        return Err(BundleError::SentinelCount {
            expected: EXPECTED_VERSION_SENTINEL_COUNT,
            found: sentinel_count,
        });
    }
    let rendered = template.replace(VERSION_SENTINEL, version);
    if rendered.contains(VERSION_SENTINEL) {
        return Err(BundleError::SentinelRemains);
    }
    Ok(rendered)
}

fn load_bundle() -> Result<SkillBundle> {
    let rendered = render_bundle_template(BUNDLE_JSON, env!("CARGO_PKG_VERSION"))?;
    let value: Value =
        serde_json::from_str(&rendered).map_err(|e| BundleError::Parse(e.to_string()))?;
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .ok_or(BundleError::MissingField("version"))?
        .to_string();
    let files_obj = value
        .get("files")
        .and_then(Value::as_object)
        .ok_or(BundleError::MissingField("files"))?;
    if files_obj.is_empty() {
        return Err(BundleError::Empty.into());
    }
    let mut files = Vec::new();
    for (path, content) in files_obj {
        let content = content
            .as_str()
            .ok_or_else(|| BundleError::FileNotString(path.clone()))?;
        files.push((path.clone(), content.to_string()));
    }
    Ok(SkillBundle { version, files })
}

pub(crate) fn write_bundle_to(dest: &Path, bundle: &SkillBundle) -> Result<()> {
    fs::create_dir_all(dest).map_err(|e| SkillInstallError::fs(FsAction::Create, dest, e))?;
    for (relative, content) in &bundle.files {
        let path = dest.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SkillInstallError::fs(FsAction::Create, parent, e))?;
        }
        fs::write(&path, content).map_err(|e| SkillInstallError::fs(FsAction::Write, &path, e))?;
    }
    Ok(())
}

pub(crate) fn link_or_copy_dir(target: &Path, link_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)
            .or_else(|_| copy_dir_recursive(target, link_path))
            .map_err(|e| SkillInstallError::link(link_path, target, e))
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link_path)
            .or_else(|_| copy_dir_recursive(target, link_path))
            .map_err(|e| SkillInstallError::link(link_path, target, e))
    }
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

pub(crate) fn remove_path(path: &Path) -> Result<()> {
    // Only a missing entry is "nothing to remove"; any other metadata error
    // (permissions, I/O) must propagate — treating it as absence would let a
    // later create step fail with a misleading error, or silently keep a
    // stale entry in place.
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(SkillInstallError::fs(FsAction::Inspect, path, e)),
    };
    if metadata.file_type().is_symlink() {
        remove_symlink_path(path)
    } else if metadata.is_file() {
        fs::remove_file(path).map_err(|e| SkillInstallError::fs(FsAction::Remove, path, e))
    } else {
        fs::remove_dir_all(path).map_err(|e| SkillInstallError::fs(FsAction::Remove, path, e))
    }
}

#[cfg(windows)]
fn remove_symlink_path(path: &Path) -> Result<()> {
    // Windows removes directory symlinks via remove_dir and file symlinks via
    // remove_file. `is_dir()` follows the target, so a DANGLING directory
    // symlink reports false — try both forms instead of classifying.
    fs::remove_dir(path)
        .or_else(|_| fs::remove_file(path))
        .map_err(|e| SkillInstallError::fs(FsAction::Remove, path, e))
}

#[cfg(not(windows))]
fn remove_symlink_path(path: &Path) -> Result<()> {
    fs::remove_file(path).map_err(|e| SkillInstallError::fs(FsAction::Remove, path, e))
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(e) => return Err(SkillInstallError::fs(FsAction::Read, path, e)),
    };
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value =
        serde_json::from_str(&text).map_err(|e| SkillInstallError::fs(FsAction::Parse, path, e))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| SkillInstallError::NotAJsonObject(path.to_path_buf()))
}

fn write_json_object(path: &Path, root: &Map<String, Value>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| SkillInstallError::fs(FsAction::Create, parent, e))?;
    }
    let text = serde_json::to_string_pretty(root)
        .map_err(|e| SkillInstallError::fs(FsAction::Serialize, path, e))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|e| SkillInstallError::fs(FsAction::Write, path, e))
}

fn object_entry<'a>(
    root: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    let entry = root
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    entry
        .as_object_mut()
        .ok_or_else(|| SkillInstallError::NotAnObject(key.to_string()))
}

fn array_entry<'a>(root: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    let entry = root
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    entry.as_array_mut().expect("array value")
}

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or(SkillInstallError::HomeUnavailable)
}

fn timestamp_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        load_bundle, render_bundle_template, BUNDLE_JSON, EXPECTED_VERSION_SENTINEL_COUNT,
        VERSION_SENTINEL,
    };
    use serde_json::Value;

    #[test]
    fn embedded_bundle_renders_cargo_version_everywhere() {
        let bundle = load_bundle().expect("embedded skill bundle should load");
        let cargo_version = env!("CARGO_PKG_VERSION");

        assert_eq!(bundle.version, cargo_version);
        for (path, content) in &bundle.files {
            assert!(
                !content.contains(VERSION_SENTINEL),
                "rendered bundle file {path:?} still contains the version sentinel"
            );
        }

        for path in [
            ".claude-plugin/plugin.json",
            ".cursor-plugin/plugin.json",
            "package.json",
        ] {
            let content = bundle
                .files
                .iter()
                .find_map(|(candidate, content)| (candidate == path).then_some(content))
                .unwrap_or_else(|| panic!("embedded bundle missing {path}"));
            let manifest: Value = serde_json::from_str(content)
                .unwrap_or_else(|error| panic!("parse embedded {path}: {error}"));
            assert_eq!(
                manifest.get("version").and_then(Value::as_str),
                Some(cargo_version),
                "embedded {path} version differs from Cargo"
            );
        }

        let marketplace_content = bundle
            .files
            .iter()
            .find_map(|(path, content)| {
                (path == ".claude-plugin/marketplace.json").then_some(content)
            })
            .expect("embedded bundle missing .claude-plugin/marketplace.json");
        let marketplace: Value = serde_json::from_str(marketplace_content)
            .expect("parse embedded .claude-plugin/marketplace.json");
        let plugins = marketplace
            .get("plugins")
            .and_then(Value::as_array)
            .expect("embedded marketplace plugins must be an array");
        assert!(!plugins.is_empty(), "embedded marketplace has no plugins");
        for plugin in plugins {
            assert_eq!(
                plugin.get("version").and_then(Value::as_str),
                Some(cargo_version),
                "embedded marketplace plugin version differs from Cargo"
            );
        }
    }

    #[test]
    fn bundle_renderer_rejects_missing_version_sentinels() {
        let error = render_bundle_template("{}", env!("CARGO_PKG_VERSION"))
            .expect_err("template without version sentinels should fail")
            .to_string();
        let expected = format!("expected {EXPECTED_VERSION_SENTINEL_COUNT}");

        assert!(error.contains(&expected), "unexpected error: {error}");
        assert!(error.contains("found 0"), "unexpected error: {error}");
    }

    #[test]
    fn bundle_renderer_rejects_partially_templated_versions() {
        let partial_template = BUNDLE_JSON.replacen(VERSION_SENTINEL, env!("CARGO_PKG_VERSION"), 1);
        let error = render_bundle_template(&partial_template, env!("CARGO_PKG_VERSION"))
            .expect_err("partially rendered template should fail")
            .to_string();
        let expected = format!("expected {EXPECTED_VERSION_SENTINEL_COUNT}");
        let found = format!("found {}", EXPECTED_VERSION_SENTINEL_COUNT - 1);

        assert!(error.contains(&expected), "unexpected error: {error}");
        assert!(error.contains(&found), "unexpected error: {error}");
    }

    #[test]
    fn gemini_cli_is_not_an_install_target() {
        let error = super::Target::parse("gemini-cli")
            .expect_err("retired Gemini CLI integration must stay unavailable")
            .to_string();

        assert!(
            error.contains("unknown target"),
            "unexpected error: {error}"
        );
        assert!(
            !error.contains("available: gemini"),
            "unexpected error: {error}"
        );
    }
}
