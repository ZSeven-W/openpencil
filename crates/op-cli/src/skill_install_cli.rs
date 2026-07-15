use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const BUNDLE_JSON: &str = include_str!("../assets/skill-bundle.json");
const VERSION_SENTINEL: &str = "__OPENPENCIL_VERSION__";
const REPO: &str = "zseven-w/openpencil-skill";
const REPO_URL: &str = "https://github.com/zseven-w/openpencil-skill.git";
const SKILL_NAME: &str = "openpencil-skill";

#[derive(Debug, Clone)]
struct SkillBundle {
    version: String,
    files: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Claude,
    Codex,
    Cursor,
    Gemini,
    OpenCode,
}

impl Target {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" => Ok(Target::Claude),
            "codex" => Ok(Target::Codex),
            "cursor" => Ok(Target::Cursor),
            "gemini" | "gemini-cli" => Ok(Target::Gemini),
            "opencode" | "open-code" => Ok(Target::OpenCode),
            _ => Err(format!(
                "unknown target {raw:?}; available: claude, codex, cursor, gemini, opencode"
            )),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Target::Claude => "claude",
            Target::Codex => "codex",
            Target::Cursor => "cursor",
            Target::Gemini => "gemini",
            Target::OpenCode => "opencode",
        }
    }
}

pub(crate) fn run_install(target: Option<&str>) -> Result<String, String> {
    run_for_home(Action::Install, target, &home_dir()?)
}

pub(crate) fn run_uninstall(target: Option<&str>) -> Result<String, String> {
    run_for_home(Action::Uninstall, target, &home_dir()?)
}

#[cfg(test)]
pub(crate) fn install_target_at_home(target: &str, home: &Path) -> Result<(), String> {
    let bundle = load_bundle()?;
    install_target(Target::parse(target)?, home, &bundle)
}

#[cfg(test)]
pub(crate) fn uninstall_target_at_home(target: &str, home: &Path) -> Result<(), String> {
    uninstall_target(Target::parse(target)?, home)
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Install,
    Uninstall,
}

fn run_for_home(action: Action, target: Option<&str>, home: &Path) -> Result<String, String> {
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
            Err(error) => json!({ "target": target.key(), "ok": false, "error": error }),
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

fn resolve_targets(
    target: Option<&str>,
    action: Action,
    home: &Path,
) -> Result<Vec<Target>, String> {
    if let Some(target) = target {
        return Ok(vec![Target::parse(target)?]);
    }
    let detected = detect_targets(home);
    if detected.is_empty() && matches!(action, Action::Install) {
        return Err(
            "no supported AI coding agents detected; pass --target claude|codex|cursor|gemini|opencode"
                .into(),
        );
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
    if command_exists("gemini") {
        targets.push(Target::Gemini);
    }
    if command_exists("opencode") {
        targets.push(Target::OpenCode);
    }
    targets
}

fn command_exists(name: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path_var).any(|dir| {
        let candidate = dir.join(name);
        candidate.is_file() || candidate.with_extension("exe").is_file()
    })
}

fn install_target(target: Target, home: &Path, bundle: &SkillBundle) -> Result<(), String> {
    match target {
        Target::Claude => install_claude(home, bundle),
        Target::Codex => install_codex(home, bundle),
        Target::Cursor => write_bundle_to(&home.join(".cursor/plugins").join(SKILL_NAME), bundle),
        Target::Gemini => {
            write_bundle_to(&home.join(".gemini/extensions").join(SKILL_NAME), bundle)
        }
        Target::OpenCode => install_opencode(home),
    }
}

fn uninstall_target(target: Target, home: &Path) -> Result<(), String> {
    match target {
        Target::Claude => uninstall_claude(home),
        Target::Codex => uninstall_codex(home),
        Target::Cursor => remove_path(&home.join(".cursor/plugins").join(SKILL_NAME)),
        Target::Gemini => remove_path(&home.join(".gemini/extensions").join(SKILL_NAME)),
        Target::OpenCode => uninstall_opencode(home),
    }
}

fn install_claude(home: &Path, bundle: &SkillBundle) -> Result<(), String> {
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

fn uninstall_claude(home: &Path) -> Result<(), String> {
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

fn install_codex(home: &Path, bundle: &SkillBundle) -> Result<(), String> {
    let clone_dir = home.join(".codex").join(SKILL_NAME);
    write_bundle_to(&clone_dir, bundle)?;

    let skills_dir = home.join(".agents/skills");
    fs::create_dir_all(&skills_dir).map_err(|e| format!("create {}: {e}", skills_dir.display()))?;
    let link_path = skills_dir.join(SKILL_NAME);
    let link_target = clone_dir.join("skills");
    if fs::symlink_metadata(&link_path).is_err() {
        link_or_copy_dir(&link_target, &link_path)?;
    }
    Ok(())
}

fn uninstall_codex(home: &Path) -> Result<(), String> {
    remove_path(&home.join(".agents/skills").join(SKILL_NAME))?;
    remove_path(&home.join(".codex").join(SKILL_NAME))
}

fn install_opencode(home: &Path) -> Result<(), String> {
    let config_path = home.join(".config/opencode/opencode.json");
    let mut config = read_json_object(&config_path)?;
    let plugin_entry = format!("{SKILL_NAME}@git+{REPO_URL}");
    let plugins = array_entry(&mut config, "plugin");
    if !plugins
        .iter()
        .any(|value| value.as_str().is_some_and(|p| p.contains(SKILL_NAME)))
    {
        plugins.push(Value::String(plugin_entry));
    }
    write_json_object(&config_path, &config)
}

fn uninstall_opencode(home: &Path) -> Result<(), String> {
    let config_path = home.join(".config/opencode/opencode.json");
    if !config_path.exists() {
        return Ok(());
    }
    let mut config = read_json_object(&config_path)?;
    let plugins = array_entry(&mut config, "plugin");
    plugins.retain(|value| !value.as_str().is_some_and(|p| p.contains(SKILL_NAME)));
    write_json_object(&config_path, &config)
}

fn load_bundle() -> Result<SkillBundle, String> {
    if !BUNDLE_JSON.contains(VERSION_SENTINEL) {
        return Err(format!(
            "embedded skill bundle template missing version sentinel {VERSION_SENTINEL:?}"
        ));
    }
    let rendered = BUNDLE_JSON.replace(VERSION_SENTINEL, env!("CARGO_PKG_VERSION"));
    if rendered.contains(VERSION_SENTINEL) {
        return Err(
            "embedded skill bundle still contains the version sentinel after rendering".into(),
        );
    }
    let value: Value =
        serde_json::from_str(&rendered).map_err(|e| format!("parse skill bundle: {e}"))?;
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .ok_or("skill bundle missing version")?
        .to_string();
    let files_obj = value
        .get("files")
        .and_then(Value::as_object)
        .ok_or("skill bundle missing files")?;
    if files_obj.is_empty() {
        return Err("embedded skill bundle is empty".into());
    }
    let mut files = Vec::new();
    for (path, content) in files_obj {
        let content = content
            .as_str()
            .ok_or_else(|| format!("bundle file {path:?} is not a string"))?;
        files.push((path.clone(), content.to_string()));
    }
    Ok(SkillBundle { version, files })
}

#[cfg(test)]
mod tests {
    use super::load_bundle;
    use serde_json::Value;

    #[test]
    fn embedded_bundle_renders_cargo_version_everywhere() {
        let bundle = load_bundle().expect("embedded skill bundle should load");
        let cargo_version = env!("CARGO_PKG_VERSION");

        assert_eq!(bundle.version, cargo_version);
        for (path, content) in &bundle.files {
            assert!(
                !content.contains("__OPENPENCIL_VERSION__"),
                "rendered bundle file {path:?} still contains the version sentinel"
            );
        }

        for path in [
            ".claude-plugin/plugin.json",
            ".cursor-plugin/plugin.json",
            "package.json",
            "gemini-extension.json",
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
}

fn write_bundle_to(dest: &Path, bundle: &SkillBundle) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("create {}: {e}", dest.display()))?;
    for (relative, content) in &bundle.files {
        let path = dest.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        fs::write(&path, content).map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}

fn link_or_copy_dir(target: &Path, link_path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)
            .or_else(|_| copy_dir_recursive(target, link_path))
            .map_err(|e| format!("link {} -> {}: {e}", link_path.display(), target.display()))
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link_path)
            .or_else(|_| copy_dir_recursive(target, link_path))
            .map_err(|e| format!("link {} -> {}: {e}", link_path.display(), target.display()))
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

fn remove_path(path: &Path) -> Result<(), String> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() {
        remove_symlink_path(path)
    } else if metadata.is_file() {
        fs::remove_file(path).map_err(|e| format!("remove {}: {e}", path.display()))
    } else {
        fs::remove_dir_all(path).map_err(|e| format!("remove {}: {e}", path.display()))
    }
}

#[cfg(windows)]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir(path).map_err(|e| format!("remove {}: {e}", path.display()))
    } else {
        fs::remove_file(path).map_err(|e| format!("remove {}: {e}", path.display()))
    }
}

#[cfg(not(windows))]
fn remove_symlink_path(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| format!("remove {}: {e}", path.display()))
}

fn read_json_object(path: &Path) -> Result<Map<String, Value>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    if text.trim().is_empty() {
        return Ok(Map::new());
    }
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{} must contain a JSON object", path.display()))
}

fn write_json_object(path: &Path, root: &Map<String, Value>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(root)
        .map_err(|e| format!("serialize {}: {e}", path.display()))?;
    fs::write(path, format!("{text}\n")).map_err(|e| format!("write {}: {e}", path.display()))
}

fn object_entry<'a>(
    root: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    let entry = root
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    entry
        .as_object_mut()
        .ok_or_else(|| format!("{key} is not an object"))
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

fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "home directory not available".to_string())
}

fn timestamp_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
