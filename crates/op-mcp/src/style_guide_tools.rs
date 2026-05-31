//! TS-compatible style-guide MCP tools.

use std::collections::BTreeMap;

use op_ai_skills::style_guide::{
    select_style_guide, style_guide_registry, Platform, SelectOptions, STYLE_GUIDE_TAGS,
};

use super::{McpTool, ToolErrorCode, ToolOutcome};

pub struct GetStyleGuideTags;

impl McpTool for GetStyleGuideTags {
    fn name(&self) -> &str {
        "get_style_guide_tags"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let tags = match serde_json::to_string(STYLE_GUIDE_TAGS) {
            Ok(json) => json,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize style guide tags failed: {e}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("tags".into(), tags);
        out.insert("count".into(), STYLE_GUIDE_TAGS.len().to_string());
        ToolOutcome::Ok(out)
    }
}

pub struct GetStyleGuide;

impl McpTool for GetStyleGuide {
    fn name(&self) -> &str {
        "get_style_guide"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let tags = match parse_tags(args.get("tags").map(String::as_str)) {
            Ok(tags) => tags,
            Err(msg) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg),
        };
        let platform = args
            .get("platform")
            .filter(|s| !s.is_empty())
            .map(|s| Platform::from_str(s));
        let opts = SelectOptions {
            tags,
            name: args.get("name").filter(|s| !s.is_empty()).cloned(),
            platform,
        };
        let Some(guide) = select_style_guide(style_guide_registry(), &opts) else {
            let mut out = BTreeMap::new();
            out.insert(
                "error".into(),
                "No matching style guide found. Try different tags or list available names.".into(),
            );
            return ToolOutcome::Ok(out);
        };
        let tags_json = match serde_json::to_string(&guide.tags) {
            Ok(json) => json,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize style guide tags failed: {e}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("name".into(), guide.name.clone());
        out.insert("tags".into(), tags_json);
        out.insert("platform".into(), guide.platform.as_str().into());
        out.insert("content".into(), guide.content.clone());
        ToolOutcome::Ok(out)
    }
}

pub fn get_style_guide_tags_snapshot() -> GetStyleGuideTags {
    GetStyleGuideTags
}

pub fn get_style_guide_snapshot() -> GetStyleGuide {
    GetStyleGuide
}

fn parse_tags(raw: Option<&str>) -> Result<Vec<String>, String> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Vec::new());
    };
    if raw.starts_with('[') {
        return serde_json::from_str::<Vec<String>>(raw)
            .map_err(|e| format!("tags must be a string array: {e}"));
    }
    Ok(raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}
