//! TS-compatible `design.md` MCP tools.

use std::collections::BTreeMap;

use op_editor_core::{
    extract_design_md_from_document, generate_design_md, parse_design_md, EditorCommand,
    EditorState,
};

use super::{McpTool, ToolErrorCode, ToolOutcome};

pub struct GetDesignMd {
    spec: Option<op_editor_core::DesignMdSpec>,
    extracted: op_editor_core::DesignMdSpec,
}

pub struct SetDesignMd {
    extracted: op_editor_core::DesignMdSpec,
}

pub struct ExportDesignMd {
    spec: Option<op_editor_core::DesignMdSpec>,
    extracted: op_editor_core::DesignMdSpec,
}

impl McpTool for GetDesignMd {
    fn name(&self) -> &str {
        "get_design_md"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let mut out = BTreeMap::new();
        let selected = self
            .spec
            .as_ref()
            .or_else(|| spec_has_content(&self.extracted).then_some(&self.extracted));
        let Some(spec) = selected else {
            out.insert("hasDesignMd".into(), "false".into());
            return ToolOutcome::Ok(out);
        };
        match spec_json(spec) {
            Ok(json) => {
                out.insert("hasDesignMd".into(), "true".into());
                out.insert("spec".into(), json);
                out.insert("markdown".into(), generate_design_md(spec));
                ToolOutcome::Ok(out)
            }
            Err(msg) => ToolOutcome::Err(ToolErrorCode::Internal, msg),
        }
    }
}

impl McpTool for SetDesignMd {
    fn name(&self) -> &str {
        "set_design_md"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let auto_extract = match parse_bool_arg(args, "autoExtract") {
            Ok(v) => v,
            Err(msg) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg),
        };
        let spec = if auto_extract {
            self.extracted.clone()
        } else if let Some(markdown) = args.get("markdown").filter(|s| !s.is_empty()) {
            parse_design_md(markdown)
        } else {
            let mut out = BTreeMap::new();
            out.insert("success".into(), "false".into());
            return ToolOutcome::Ok(out);
        };
        let spec_json = match spec_json(&spec) {
            Ok(json) => json,
            Err(msg) => return ToolOutcome::Err(ToolErrorCode::Internal, msg),
        };
        let mut out = BTreeMap::new();
        out.insert("success".into(), "true".into());
        out.insert("wrote".into(), "true".into());
        out.insert("spec".into(), spec_json);
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetDesignMd {
                spec: Box::new(spec),
            },
        )
    }
}

impl McpTool for ExportDesignMd {
    fn name(&self) -> &str {
        "export_design_md"
    }

    fn call(&self, _args: &BTreeMap<String, String>) -> ToolOutcome {
        let spec = self.spec.as_ref().unwrap_or(&self.extracted);
        let mut out = BTreeMap::new();
        out.insert("markdown".into(), generate_design_md(spec));
        ToolOutcome::Ok(out)
    }
}

pub fn get_design_md_snapshot(state: &EditorState) -> GetDesignMd {
    GetDesignMd {
        spec: state.doc.design_md.clone(),
        extracted: extract_design_md_from_document(&state.doc),
    }
}

pub fn set_design_md_snapshot(state: &EditorState) -> SetDesignMd {
    SetDesignMd {
        extracted: extract_design_md_from_document(&state.doc),
    }
}

pub fn export_design_md_snapshot(state: &EditorState) -> ExportDesignMd {
    ExportDesignMd {
        spec: state.doc.design_md.clone(),
        extracted: extract_design_md_from_document(&state.doc),
    }
}

fn spec_has_content(spec: &op_editor_core::DesignMdSpec) -> bool {
    spec.color_palette.as_ref().is_some_and(|c| !c.is_empty())
        || spec
            .typography
            .as_ref()
            .and_then(|t| t.font_family.as_ref())
            .is_some_and(|s| !s.is_empty())
        || spec.visual_theme.as_ref().is_some_and(|s| !s.is_empty())
}

fn spec_json(spec: &op_editor_core::DesignMdSpec) -> Result<String, String> {
    serde_json::to_string(spec).map_err(|e| format!("serialize design.md spec failed: {e}"))
}

fn parse_bool_arg(args: &BTreeMap<String, String>, key: &str) -> Result<bool, String> {
    let Some(raw) = args.get(key).or_else(|| args.get("auto_extract")) else {
        return Ok(false);
    };
    match raw.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{key} must be true or false, got {raw:?}")),
    }
}
