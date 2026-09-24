//! `op brand:extract` — a website or screenshot → brand kit.
//!
//! Two routes, same split as `import:html`:
//! - **Live** (default): the running editor's `brand_extract` MCP tool does
//!   the work (it owns the SSRF-screened fetcher); `--apply` also writes the
//!   kit into the open document as one undo step.
//! - **Offline** (`--out kit.optheme`, local image / saved HTML only): the
//!   kit is extracted here and written as a theme-preset file the Variables
//!   panel imports. No network, no editor needed.

use std::path::Path;

use super::{flag_value, pair, push_file_path, required_pos, tool_call, Command, Flags};
use crate::cli_error::CliError;
use crate::path_args::resolve_file_path_arg;

const USAGE: &str =
    "Usage: op brand:extract <url|screenshot.png|page.html> [--apply] [--out kit.optheme] [--file doc.op]";

fn is_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

fn is_html_path(source: &str) -> bool {
    Path::new(source)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
}

pub(super) fn map_brand_extract(
    positionals: &[String],
    flags: &Flags,
) -> Result<Command, CliError> {
    let source = required_pos(positionals, 1, USAGE)?;
    let apply = flags.contains_key("apply");
    if let Some(out_path) = flag_value(flags, "out") {
        if is_url(&source) {
            return Err(CliError::usage(
                "--out requires a local screenshot or HTML file; URL extraction needs a running editor",
            ));
        }
        if apply {
            return Err(CliError::usage(
                "--apply writes into the running editor and cannot be combined with --out",
            ));
        }
        return Ok(Command::BrandExtract {
            source_path: source,
            out_path,
        });
    }
    if is_html_path(&source) {
        return Err(CliError::usage(
            "a saved HTML page is read offline: pass --out kit.optheme",
        ));
    }
    let mut pairs = if is_url(&source) {
        vec![pair("url", source)]
    } else {
        vec![pair("imagePath", resolve_file_path_arg(&source))]
    };
    if apply {
        pairs.push(pair("apply", "true"));
    }
    push_file_path(&mut pairs, flags);
    tool_call("brand_extract", pairs)
}

/// Offline extraction → `.optheme` file; prints the kit report.
pub(super) fn run_brand_extract(source_path: &str, out_path: &str) -> Result<String, CliError> {
    let bytes = std::fs::read(source_path)
        .map_err(|error| CliError::Io(format!("read {source_path:?}: {error}")))?;
    let kit = if is_html_path(source_path) {
        let html = op_html::html_encoding::decode_html_bytes(&bytes);
        let stem = Path::new(source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("brand");
        op_brand::extract_from_html(&html, &[], stem)
    } else {
        op_brand::extract_from_image(&bytes, source_path)
            .map_err(|error| CliError::Document(error.to_string()))?
    };
    let preset = op_editor_core::theme_presets::preset_file_to_json(
        &kit.name,
        &kit.themes(),
        &kit.variables(),
    );
    std::fs::write(out_path, preset)
        .map_err(|error| CliError::Io(format!("write {out_path:?}: {error}")))?;
    let mut report = kit.to_json();
    report["out"] = serde_json::Value::String(out_path.to_string());
    Ok(report.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn flags(pairs: &[(&str, Option<&str>)]) -> Flags {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.map(str::to_string)))
            .collect::<BTreeMap<_, _>>()
    }

    fn pos(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_url_routes_to_the_live_tool() {
        let cmd = map_brand_extract(
            &pos(&["brand:extract", "https://brew.example"]),
            &flags(&[("apply", None)]),
        )
        .unwrap();
        assert_eq!(
            cmd,
            Command::ToolCall {
                tool: "brand_extract".into(),
                args: vec![
                    ("url".into(), "https://brew.example".into()),
                    ("apply".into(), "true".into()),
                ],
            }
        );
    }

    #[test]
    fn a_screenshot_path_is_made_absolute() {
        let Command::ToolCall { args, .. } =
            map_brand_extract(&pos(&["brand:extract", "shot.png"]), &flags(&[])).unwrap()
        else {
            panic!("tool call");
        };
        assert_eq!(args[0].0, "imagePath");
        assert!(Path::new(&args[0].1).is_absolute());
    }

    #[test]
    fn out_is_offline_and_refuses_urls_and_apply() {
        assert_eq!(
            map_brand_extract(
                &pos(&["brand:extract", "shot.png"]),
                &flags(&[("out", Some("kit.optheme"))])
            )
            .unwrap(),
            Command::BrandExtract {
                source_path: "shot.png".into(),
                out_path: "kit.optheme".into(),
            }
        );
        assert!(map_brand_extract(
            &pos(&["brand:extract", "https://brew.example"]),
            &flags(&[("out", Some("k.optheme"))])
        )
        .is_err());
        assert!(map_brand_extract(
            &pos(&["brand:extract", "shot.png"]),
            &flags(&[("out", Some("k.optheme")), ("apply", None)])
        )
        .is_err());
        assert!(map_brand_extract(&pos(&["brand:extract", "page.html"]), &flags(&[])).is_err());
        assert!(map_brand_extract(&pos(&["brand:extract"]), &flags(&[])).is_err());
    }

    #[test]
    fn offline_html_writes_an_importable_theme_preset() {
        let dir = std::env::temp_dir().join(format!("op-brand-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let html = dir.join("ember.html");
        std::fs::write(
            &html,
            "<style>body{background:#FFF8F1;color:#2A1A10} .btn{background:#D9480F;color:#fff}</style><a class=btn>x</a>",
        )
        .unwrap();
        let out = dir.join("ember.optheme");
        let report = run_brand_extract(html.to_str().unwrap(), out.to_str().unwrap()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(json["light"]["--primary"], "#D9480F");
        let preset = std::fs::read_to_string(&out).unwrap();
        let (name, themes, variables) =
            op_editor_core::theme_presets::parse_preset_file(&preset).expect("valid .optheme");
        assert_eq!(name, "ember");
        assert!(themes.contains_key("Mode"));
        assert!(variables.contains_key("--primary"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
