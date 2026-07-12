use std::path::Path;

use base64::Engine as _;
use serde_json::Value;

use crate::command_helpers::flag_value;
use crate::mcp_http_cli::{post, tool_call_body};
use crate::{Command, Flags};

pub(crate) fn map_export(flags: &Flags) -> Result<Command, String> {
    let item_id = flag_value(flags, "item");
    let selection = flags.contains_key("selection");
    if item_id.is_some() && selection {
        return Err("--item and --selection cannot be used together".into());
    }
    let format_flag = flag_value(flags, "format");
    let formats_flag = flag_value(flags, "formats");
    if let (Some(format), Some(formats)) = (&format_flag, &formats_flag) {
        if format != formats {
            return Err("--format and --formats cannot specify different values".into());
        }
    }
    let format = format_flag.or(formats_flag).unwrap_or_else(|| "png".into());
    if !matches!(format.as_str(), "png" | "jpeg" | "jpg" | "webp" | "pdf") {
        return Err(format!("unsupported export format {format:?}"));
    }
    let output = flag_value(flags, "output").ok_or("--output is required")?;
    let scale = flag_value(flags, "scale");
    if let Some(value) = &scale {
        value
            .parse::<f32>()
            .map_err(|_| format!("--scale must be a number, got {value:?}"))?;
    }
    Ok(Command::Export {
        item_id,
        selection,
        output,
        format,
        scale,
    })
}

pub(crate) fn run_export(
    port: u16,
    item_id: Option<&str>,
    output: &str,
    format: &str,
    scale: Option<&str>,
) -> Result<String, String> {
    let mut arguments = serde_json::Map::new();
    if let Some(item_id) = item_id {
        arguments.insert("itemId".into(), Value::String(item_id.into()));
    }
    arguments.insert("format".into(), Value::String(format.into()));
    if let Some(scale) = scale {
        let scale = scale
            .parse::<f64>()
            .map_err(|_| format!("--scale must be a number, got {scale:?}"))?;
        arguments.insert("scale".into(), Value::from(scale));
    }
    let response = post(
        port,
        &tool_call_body("export_item", &Value::Object(arguments).to_string()),
    )?;
    write_export_response(&response, Path::new(output))
}

pub(crate) fn write_export_response(response: &str, output: &Path) -> Result<String, String> {
    let value: Value = serde_json::from_str(response)
        .map_err(|error| format!("export_item returned invalid JSON: {error}"))?;
    let encoded = value
        .get("bytes_base64")
        .and_then(Value::as_str)
        .ok_or("export_item response is missing bytes_base64")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("export_item returned invalid Base64: {error}"))?;
    std::fs::write(output, bytes)
        .map_err(|error| format!("cannot write export to {}: {error}", output.display()))?;

    Ok(serde_json::json!({
        "output": output.to_string_lossy(),
        "itemId": value.get("itemId").and_then(Value::as_str).unwrap_or(""),
        "itemType": value.get("itemType").and_then(Value::as_str).unwrap_or(""),
        "format": value.get("format").and_then(Value::as_str).unwrap_or(""),
    })
    .to_string())
}
