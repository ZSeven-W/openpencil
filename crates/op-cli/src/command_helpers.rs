use crate::path_args::resolve_file_path_arg;
use crate::{Command, Flags};

pub(crate) fn flag_value(flags: &Flags, key: &str) -> Option<String> {
    flags.get(key).and_then(Clone::clone)
}

pub(crate) fn push_file_path(pairs: &mut Vec<(String, String)>, flags: &Flags) {
    if let Some(file) = flag_value(flags, "file") {
        pairs.push(pair("filePath", resolve_file_path_arg(&file)));
    }
}

pub(crate) fn pair(key: impl Into<String>, value: impl Into<String>) -> (String, String) {
    (key.into(), value.into())
}

pub(crate) fn tool_call_with_file(tool: &str, flags: &Flags) -> Result<Command, String> {
    let mut pairs = Vec::new();
    push_file_path(&mut pairs, flags);
    tool_call(tool, pairs)
}

pub(crate) fn tool_call(tool: &str, args: Vec<(String, String)>) -> Result<Command, String> {
    Ok(Command::ToolCall {
        tool: tool.to_string(),
        args,
    })
}

pub(crate) fn version_json() -> String {
    format!(r#"{{"version":"{}"}}"#, env!("CARGO_PKG_VERSION"))
}
