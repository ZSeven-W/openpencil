//! `op page ...`, `op vars:set` / `themes:set`, and `op theme:*` command
//! mappers — split out of `main.rs` per the 800-line-cap convention. Pure
//! arg → `Command` mapping; the shared helpers stay in `main.rs`.

use crate::{flag_value, pair, push_file_path, required_pos, resolve_arg, tool_call};
use crate::{Command, Flags};

pub(crate) fn map_page(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let sub = positionals
        .get(1)
        .map(String::as_str)
        .ok_or("Usage: op page list|add|remove|rename|reorder|duplicate ...")?;
    match sub {
        "list" => {
            let mut pairs = Vec::new();
            push_file_path(&mut pairs, flags);
            tool_call("list_pages", pairs)
        }
        "add" => {
            let name = flag_value(flags, "name").or_else(|| positionals.get(2).cloned());
            let mut pairs = Vec::new();
            if let Some(name) = name {
                pairs.push(pair("name", name));
            }
            push_file_path(&mut pairs, flags);
            tool_call("add_page", pairs)
        }
        "remove" | "delete" => {
            let page_id = required_pos(positionals, 2, "Usage: op page remove <page-id>")?;
            let mut pairs = vec![pair("pageId", page_id)];
            push_file_path(&mut pairs, flags);
            tool_call("remove_page", pairs)
        }
        "rename" => {
            let page_id = required_pos(positionals, 2, "Usage: op page rename <page-id> <name>")?;
            let name = required_pos(positionals, 3, "Usage: op page rename <page-id> <name>")?;
            let mut pairs = vec![pair("pageId", page_id), pair("name", name)];
            push_file_path(&mut pairs, flags);
            tool_call("rename_page", pairs)
        }
        "reorder" => {
            let page_id = required_pos(positionals, 2, "Usage: op page reorder <page-id> <index>")?;
            let index = required_pos(positionals, 3, "Usage: op page reorder <page-id> <index>")?;
            let mut pairs = vec![pair("pageId", page_id), pair("index", index)];
            push_file_path(&mut pairs, flags);
            tool_call("reorder_page", pairs)
        }
        "duplicate" => {
            let page_id = required_pos(positionals, 2, "Usage: op page duplicate <page-id>")?;
            let mut pairs = vec![pair("pageId", page_id)];
            if let Some(name) = flag_value(flags, "name") {
                pairs.push(pair("name", name));
            }
            push_file_path(&mut pairs, flags);
            tool_call("duplicate_page", pairs)
        }
        _ => Err(format!("unknown page subcommand {sub:?}")),
    }
}

pub(crate) fn map_vars_set(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let raw = resolve_arg(positionals.get(1).map(String::as_str))?;
    let mut pairs = vec![pair("variables", raw)];
    if flags.contains_key("replace") {
        pairs.push(pair("replace", "true"));
    }
    push_file_path(&mut pairs, flags);
    tool_call("set_variables", pairs)
}

pub(crate) fn map_themes_set(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let raw = resolve_arg(positionals.get(1).map(String::as_str))?;
    let mut pairs = vec![pair("themes", raw)];
    if flags.contains_key("replace") {
        pairs.push(pair("replace", "true"));
    }
    push_file_path(&mut pairs, flags);
    tool_call("set_themes", pairs)
}

pub(crate) fn map_theme_save(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let preset_path = required_pos(positionals, 1, "Usage: op theme:save <file.optheme>")?;
    let mut pairs = vec![pair("presetPath", preset_path)];
    if let Some(name) = flag_value(flags, "name") {
        pairs.push(pair("name", name));
    }
    push_file_path(&mut pairs, flags);
    tool_call("save_theme_preset", pairs)
}

pub(crate) fn map_theme_load(positionals: &[String], flags: &Flags) -> Result<Command, String> {
    let preset_path = required_pos(positionals, 1, "Usage: op theme:load <file.optheme>")?;
    let mut pairs = vec![pair("presetPath", preset_path)];
    push_file_path(&mut pairs, flags);
    tool_call("load_theme_preset", pairs)
}

pub(crate) fn map_theme_list(positionals: &[String]) -> Result<Command, String> {
    let directory = required_pos(positionals, 1, "Usage: op theme:list <directory>")?;
    tool_call("list_theme_presets", vec![pair("directory", directory)])
}
