//! Code-to-design conversion command aliases.

use std::fs;

use super::{flag_value, pair, tool_call, Command, Flags};

pub(crate) fn map_design_conversion(
    positionals: &[String],
    flags: &Flags,
) -> Result<Command, String> {
    match positionals[0].as_str() {
        "design:upsert-vars" => map_upsert_variables(flags),
        "design:upsert-component" => map_upsert_component(flags),
        "design:upsert-screen" => map_upsert_screen(flags),
        "design:status" => map_status(flags),
        "design:lint" => map_lint(flags),
        _ => unreachable!("caller guards design conversion command set"),
    }
}

fn map_upsert_variables(flags: &Flags) -> Result<Command, String> {
    let key = required_flag(
        flags,
        "key",
        "Usage: op design:upsert-vars --key <key> --file vars.json",
    )?;
    let variables = read_payload_file(
        flags,
        "Usage: op design:upsert-vars --key <key> --file vars.json",
    )?;
    let mut args = vec![pair("key", key), pair("variables", variables)];
    push_source_fields(&mut args, flags);
    tool_call("upsert_variables", args)
}

fn map_upsert_component(flags: &Flags) -> Result<Command, String> {
    let key = required_flag(
        flags,
        "key",
        "Usage: op design:upsert-component --key <key> --name <name> --file node.json",
    )?;
    let name = required_flag(
        flags,
        "name",
        "Usage: op design:upsert-component --key <key> --name <name> --file node.json",
    )?;
    let node_json = read_payload_file(
        flags,
        "Usage: op design:upsert-component --key <key> --name <name> --file node.json",
    )?;
    let mut args = vec![
        pair("key", key),
        pair("name", name),
        pair("node_json", node_json),
    ];
    push_source_fields(&mut args, flags);
    tool_call("upsert_component", args)
}

fn map_upsert_screen(flags: &Flags) -> Result<Command, String> {
    let key = required_flag(
        flags,
        "key",
        "Usage: op design:upsert-screen --key <key> --file node.json",
    )?;
    let node_json = read_payload_file(
        flags,
        "Usage: op design:upsert-screen --key <key> --file node.json",
    )?;
    let mut args = vec![pair("key", key), pair("node_json", node_json)];
    push_source_fields(&mut args, flags);
    tool_call("upsert_screen", args)
}

fn map_status(flags: &Flags) -> Result<Command, String> {
    let mut args = Vec::new();
    if let Some(kind) = flag_value(flags, "kind") {
        if !matches!(kind.as_str(), "token" | "component" | "screen") {
            return Err("--kind must be token, component, or screen".into());
        }
        args.push(pair("kind", kind));
    }
    tool_call("conversion_status", args)
}

fn map_lint(flags: &Flags) -> Result<Command, String> {
    let mut args = Vec::new();
    if let Some(node_id) = flag_value(flags, "node") {
        args.push(pair("nodeId", node_id));
    }
    tool_call("lint_document", args)
}

fn required_flag(flags: &Flags, name: &str, usage: &str) -> Result<String, String> {
    flag_value(flags, name).ok_or_else(|| usage.to_string())
}

fn read_payload_file(flags: &Flags, usage: &str) -> Result<String, String> {
    let path = required_flag(flags, "file", usage)?;
    fs::read_to_string(&path)
        .map(|contents| contents.trim().to_string())
        .map_err(|e| format!("cannot read --file {path:?}: {e}"))
}

fn push_source_fields(args: &mut Vec<(String, String)>, flags: &Flags) {
    if let Some(source) = flag_value(flags, "source") {
        args.push(pair("sourcePath", source));
    }
    if let Some(hash) = flag_value(flags, "hash") {
        args.push(pair("sourceHash", hash));
    }
}
