//! Transport-free artifacts for Code panel downloads.

use op_codegen::ai::bundle::{build_structure_bundle, BundleScope};
use op_codegen::ai::stored_zip::build_stored_zip;
use op_editor_core::EditorState;

use crate::codegen::{build_codegen_nodes_json_limited, CodegenNodesJsonError};
use crate::codegen_session::CodegenResult;

/// Hard cap for one generated source or ZIP artifact.
pub const MAX_CODEGEN_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
/// Pre-decode bound for a live bundle's JSON (which may embed base64 images).
pub const MAX_LIVE_BUNDLE_INPUT_BYTES: usize = MAX_CODEGEN_ARTIFACT_BYTES / 4;
/// Bound parse/decode work before duplicate embedded assets are collapsed.
pub const MAX_LIVE_ASSET_REFERENCES: usize = 1_024;

const ZIP_END_OF_CENTRAL_DIRECTORY_BYTES: usize = 22;
const ZIP_ENTRY_FIXED_OVERHEAD_BYTES: usize = 30 + 46;
const MAX_CODEGEN_ZIP_ENTRIES: usize = 4_096;
const MANIFEST_FIXED_UPPER_BOUND_BYTES: usize = 2 * 1024;
const MANIFEST_ASSET_FIXED_UPPER_BOUND_BYTES: usize = 256;

/// Artifact construction failed before a platform save/share surface opened.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CodegenExportError {
    #[error("Code export is too large ({estimated_bytes} bytes; maximum is {max_bytes} bytes)")]
    ArtifactTooLarge {
        estimated_bytes: usize,
        max_bytes: usize,
    },
    #[error("Code export size overflowed while applying the safety limit")]
    SizeOverflow,
    #[error("Code export exceeds the supported ZIP entry limits")]
    ZipEntryLimit,
    #[error("AI bundle input is too large ({input_bytes} bytes; maximum is {max_bytes} bytes)")]
    LiveInputTooLarge {
        input_bytes: usize,
        max_bytes: usize,
    },
    #[error("Could not serialize AI bundle input: {message}")]
    LiveInputSerialization { message: String },
    #[error("AI bundle contains too many embedded asset references ({count}; maximum is {max})")]
    TooManyAssetReferences { count: usize, max: usize },
}

/// A complete file artifact ready for a platform save/share surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodegenArtifact {
    pub file_name: String,
    pub mime_type: &'static str,
    pub bytes: Vec<u8>,
}

/// Build the generated-code Download artifact.
///
/// Results without assets stay a directly editable `component.<ext>` file.
/// Results with assets become a STORED zip containing that component plus
/// every pipeline-produced `assets/<...>` entry.
pub fn generated_artifact(
    result: &CodegenResult,
) -> Result<Option<CodegenArtifact>, CodegenExportError> {
    if result.code.is_empty() {
        return Ok(None);
    }
    if result.assets.is_empty() {
        ensure_artifact_budget(result.code.len())?;
        return Ok(Some(CodegenArtifact {
            file_name: format!("component.{}", result.framework_ext),
            mime_type: "text/plain; charset=utf-8",
            bytes: result.code.as_bytes().to_vec(),
        }));
    }

    let component_name = format!("component.{}", result.framework_ext);
    checked_stored_zip_size(
        std::iter::once((component_name.as_str(), result.code.len())).chain(
            result
                .assets
                .iter()
                .map(|asset| (asset.zip_path.as_str(), asset.bytes.len())),
        ),
    )?;
    // The budget check above deliberately precedes every payload clone.
    let mut files = Vec::with_capacity(1 + result.assets.len());
    files.push((component_name, result.code.as_bytes().to_vec()));
    files.extend(
        result
            .assets
            .iter()
            .map(|asset| (asset.zip_path.clone(), asset.bytes.clone())),
    );
    let bytes = build_stored_zip(&files);
    ensure_artifact_budget(bytes.len())?;
    Ok(Some(CodegenArtifact {
        file_name: "component.zip".into(),
        mime_type: "application/zip",
        bytes,
    }))
}

/// Build an AI structure bundle from the live selection or active page.
///
/// This deliberately does not use the targets or assets captured by a prior
/// generation: the Export Bundle action describes what is selected when the
/// user presses it and works before any code has been generated.
pub fn live_bundle_artifact(
    state: &EditorState,
) -> Result<Option<CodegenArtifact>, CodegenExportError> {
    let Some(raw_nodes_json) = build_codegen_nodes_json_limited(state, MAX_LIVE_BUNDLE_INPUT_BYTES)
        .map_err(codegen_nodes_json_error)?
    else {
        return Ok(None);
    };
    // Raw JSON is itself a bundle entry. Reject an oversized embedded data URL
    // before decoding it into an additional asset allocation.
    preflight_live_raw_input(&raw_nodes_json)?;
    let scope = if state.selection.is_empty() {
        BundleScope::Page
    } else {
        BundleScope::Selection
    };
    let (sanitized_nodes_json, assets) =
        op_codegen::ai::assets::extract_codegen_assets(&raw_nodes_json);
    let manifest_upper_bound = manifest_size_upper_bound(&assets)?;
    checked_stored_zip_size(
        [
            ("manifest.json", manifest_upper_bound),
            ("views/raw.json", raw_nodes_json.len()),
            ("views/sanitized.json", sanitized_nodes_json.len()),
        ]
        .into_iter()
        .chain(
            assets
                .iter()
                .map(|asset| (asset.zip_path.as_str(), asset.bytes.len())),
        ),
    )?;
    // Only clone raw/sanitized/asset payloads into bundle files after the
    // conservative final ZIP size has passed the hard cap.
    let bundle = build_structure_bundle(&raw_nodes_json, &sanitized_nodes_json, &assets, scope);
    let bytes = build_stored_zip(&bundle.files);
    ensure_artifact_budget(bytes.len())?;
    Ok(Some(CodegenArtifact {
        file_name: "bundle.zip".into(),
        mime_type: "application/zip",
        bytes,
    }))
}

fn codegen_nodes_json_error(error: CodegenNodesJsonError) -> CodegenExportError {
    match error {
        CodegenNodesJsonError::TooLarge {
            input_bytes,
            max_bytes,
        } => CodegenExportError::LiveInputTooLarge {
            input_bytes,
            max_bytes,
        },
        CodegenNodesJsonError::Serialization { message } => {
            CodegenExportError::LiveInputSerialization { message }
        }
    }
}

fn ensure_artifact_budget(estimated_bytes: usize) -> Result<(), CodegenExportError> {
    if estimated_bytes > MAX_CODEGEN_ARTIFACT_BYTES {
        return Err(CodegenExportError::ArtifactTooLarge {
            estimated_bytes,
            max_bytes: MAX_CODEGEN_ARTIFACT_BYTES,
        });
    }
    Ok(())
}

fn checked_stored_zip_size<'a>(
    entries: impl IntoIterator<Item = (&'a str, usize)>,
) -> Result<usize, CodegenExportError> {
    let mut total = ZIP_END_OF_CENTRAL_DIRECTORY_BYTES;
    let mut entry_count = 0usize;
    for (name, data_len) in entries {
        entry_count = entry_count
            .checked_add(1)
            .ok_or(CodegenExportError::SizeOverflow)?;
        if entry_count > MAX_CODEGEN_ZIP_ENTRIES
            || entry_count > u16::MAX as usize
            || name.len() > u16::MAX as usize
            || data_len > u32::MAX as usize
        {
            return Err(CodegenExportError::ZipEntryLimit);
        }
        let name_bytes = name
            .len()
            .checked_mul(2)
            .ok_or(CodegenExportError::SizeOverflow)?;
        let entry_bytes = ZIP_ENTRY_FIXED_OVERHEAD_BYTES
            .checked_add(name_bytes)
            .and_then(|bytes| bytes.checked_add(data_len))
            .ok_or(CodegenExportError::SizeOverflow)?;
        total = total
            .checked_add(entry_bytes)
            .ok_or(CodegenExportError::SizeOverflow)?;
        ensure_artifact_budget(total)?;
    }
    Ok(total)
}

fn preflight_live_raw_input(raw_nodes_json: &str) -> Result<(), CodegenExportError> {
    if raw_nodes_json.len() > MAX_LIVE_BUNDLE_INPUT_BYTES {
        return Err(CodegenExportError::LiveInputTooLarge {
            input_bytes: raw_nodes_json.len(),
            max_bytes: MAX_LIVE_BUNDLE_INPUT_BYTES,
        });
    }
    let reference_count = raw_nodes_json
        .match_indices("data:")
        .take(MAX_LIVE_ASSET_REFERENCES + 1)
        .count();
    if reference_count > MAX_LIVE_ASSET_REFERENCES {
        return Err(CodegenExportError::TooManyAssetReferences {
            count: reference_count,
            max: MAX_LIVE_ASSET_REFERENCES,
        });
    }
    checked_stored_zip_size(std::iter::once(("views/raw.json", raw_nodes_json.len())))?;
    Ok(())
}

fn manifest_size_upper_bound(
    assets: &[op_codegen::ai::types::AssetFile],
) -> Result<usize, CodegenExportError> {
    let mut total = MANIFEST_FIXED_UPPER_BOUND_BYTES;
    for asset in assets {
        let escaped_text_bytes = asset
            .id
            .len()
            .checked_add(asset.relative_path.len())
            .and_then(|bytes| bytes.checked_add(asset.mime_type.len()))
            .and_then(|bytes| bytes.checked_mul(6))
            .ok_or(CodegenExportError::SizeOverflow)?;
        total = total
            .checked_add(MANIFEST_ASSET_FIXED_UPPER_BOUND_BYTES)
            .and_then(|bytes| bytes.checked_add(escaped_text_bytes))
            .ok_or(CodegenExportError::SizeOverflow)?;
        ensure_artifact_budget(total)?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use jian_ops_schema::node::{ContainerProps, PenNode, PenNodeBase, RectangleNode};
    use jian_ops_schema::sizing::SizingBehavior;
    use op_codegen::ai::types::AssetFile;

    use super::*;

    fn asset(zip_path: &str, bytes: &[u8]) -> AssetFile {
        AssetFile {
            id: "a".into(),
            relative_path: format!("./{zip_path}"),
            zip_path: zip_path.into(),
            mime_type: "image/png".into(),
            bytes: bytes.to_vec(),
            source_node_id: "n1".into(),
        }
    }

    fn rect_node(id: &str) -> PenNode {
        PenNode::Rectangle(RectangleNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: Some(id.to_string()),
                x: Some(0.0),
                y: Some(0.0),
                ..Default::default()
            },
            container: ContainerProps {
                width: Some(SizingBehavior::Number(10.0)),
                height: Some(SizingBehavior::Number(10.0)),
                ..Default::default()
            },
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    fn entry_names(bytes: &[u8]) -> Vec<String> {
        let archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).expect("valid zip");
        archive.file_names().map(str::to_string).collect()
    }

    fn entry_bytes(bytes: &[u8], name: &str) -> Vec<u8> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).expect("valid zip");
        let mut entry = archive.by_name(name).expect("zip entry");
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents).expect("read zip entry");
        contents
    }

    #[test]
    fn empty_result_has_no_download_artifact() {
        assert!(generated_artifact(&CodegenResult::default())
            .expect("within export budget")
            .is_none());
    }

    #[test]
    fn generated_code_without_assets_stays_a_plain_file() {
        let artifact = generated_artifact(&CodegenResult {
            code: "export default function X(){}".into(),
            framework_ext: "tsx".into(),
            assets: Vec::new(),
        })
        .expect("within export budget")
        .expect("artifact");
        assert_eq!(artifact.file_name, "component.tsx");
        assert_eq!(artifact.mime_type, "text/plain; charset=utf-8");
        assert_eq!(artifact.bytes, b"export default function X(){}");
    }

    #[test]
    fn every_framework_gets_its_directly_editable_component_suffix() {
        use op_editor_core::codegen::Framework;

        let expected = [
            (Framework::React, "tsx"),
            (Framework::Vue, "vue"),
            (Framework::Svelte, "svelte"),
            (Framework::Html, "html"),
            (Framework::Flutter, "dart"),
            (Framework::SwiftUi, "swift"),
            (Framework::Compose, "kt"),
            (Framework::ReactNative, "tsx"),
        ];
        for (framework, extension) in expected {
            assert_eq!(crate::codegen::framework_ext(framework), extension);
            let artifact = generated_artifact(&CodegenResult {
                code: "source".into(),
                framework_ext: extension.into(),
                assets: Vec::new(),
            })
            .expect("within export budget")
            .expect("artifact");
            assert_eq!(artifact.file_name, format!("component.{extension}"));
            assert_eq!(artifact.mime_type, "text/plain; charset=utf-8");
        }
    }

    #[test]
    fn generated_code_with_assets_is_a_stored_zip() {
        let artifact = generated_artifact(&CodegenResult {
            code: "code".into(),
            framework_ext: "vue".into(),
            assets: vec![
                asset("assets/img-1.png", &[1, 2, 3]),
                asset("assets/img-2.png", &[4, 5, 6]),
            ],
        })
        .expect("within export budget")
        .expect("artifact");
        assert_eq!(artifact.file_name, "component.zip");
        assert_eq!(artifact.mime_type, "application/zip");
        assert_eq!(
            entry_names(&artifact.bytes),
            ["component.vue", "assets/img-1.png", "assets/img-2.png"]
        );
        assert_eq!(entry_bytes(&artifact.bytes, "assets/img-1.png"), [1, 2, 3]);
    }

    #[test]
    fn live_bundle_uses_selection_and_works_before_generation() {
        let mut state = EditorState::new();
        state.doc.children = vec![rect_node("n1"), rect_node("n2")];
        state.set_single_selection(op_editor_core::NodeId::new("n1"));

        let artifact = live_bundle_artifact(&state)
            .expect("within export budget")
            .expect("artifact");
        assert_eq!(artifact.file_name, "bundle.zip");
        assert_eq!(artifact.mime_type, "application/zip");
        let raw = String::from_utf8(entry_bytes(&artifact.bytes, "views/raw.json")).unwrap();
        assert!(raw.contains("n1"));
        assert!(!raw.contains("n2"));
        let manifest = String::from_utf8(entry_bytes(&artifact.bytes, "manifest.json")).unwrap();
        assert!(manifest.contains("\"scope\": \"selection\""));
    }

    #[test]
    fn live_bundle_falls_back_to_page_and_empty_page_has_none() {
        let mut state = EditorState::new();
        assert!(live_bundle_artifact(&state)
            .expect("within export budget")
            .is_none());

        state.doc.children = vec![rect_node("n1"), rect_node("n2")];
        let artifact = live_bundle_artifact(&state)
            .expect("within export budget")
            .expect("artifact");
        let raw = String::from_utf8(entry_bytes(&artifact.bytes, "views/raw.json")).unwrap();
        assert!(raw.contains("n1") && raw.contains("n2"));
        let manifest = String::from_utf8(entry_bytes(&artifact.bytes, "manifest.json")).unwrap();
        assert!(manifest.contains("\"scope\": \"page\""));
    }

    #[test]
    fn plain_source_budget_accepts_limit_and_rejects_next_byte() {
        assert_eq!(ensure_artifact_budget(MAX_CODEGEN_ARTIFACT_BYTES), Ok(()));
        assert_eq!(
            ensure_artifact_budget(MAX_CODEGEN_ARTIFACT_BYTES + 1),
            Err(CodegenExportError::ArtifactTooLarge {
                estimated_bytes: MAX_CODEGEN_ARTIFACT_BYTES + 1,
                max_bytes: MAX_CODEGEN_ARTIFACT_BYTES,
            })
        );
    }

    #[test]
    fn stored_zip_budget_counts_headers_names_and_boundary_byte() {
        let name = "component.tsx";
        let overhead =
            ZIP_END_OF_CENTRAL_DIRECTORY_BYTES + ZIP_ENTRY_FIXED_OVERHEAD_BYTES + name.len() * 2;
        let at_limit = MAX_CODEGEN_ARTIFACT_BYTES - overhead;
        assert_eq!(
            checked_stored_zip_size(std::iter::once((name, at_limit))),
            Ok(MAX_CODEGEN_ARTIFACT_BYTES)
        );
        assert_eq!(
            checked_stored_zip_size(std::iter::once((name, at_limit + 1))),
            Err(CodegenExportError::ArtifactTooLarge {
                estimated_bytes: MAX_CODEGEN_ARTIFACT_BYTES + 1,
                max_bytes: MAX_CODEGEN_ARTIFACT_BYTES,
            })
        );
    }

    #[test]
    fn live_raw_preflight_rejects_oversize_before_decode() {
        let raw = "x".repeat(MAX_LIVE_BUNDLE_INPUT_BYTES + 1);
        assert_eq!(
            preflight_live_raw_input(&raw),
            Err(CodegenExportError::LiveInputTooLarge {
                input_bytes: MAX_LIVE_BUNDLE_INPUT_BYTES + 1,
                max_bytes: MAX_LIVE_BUNDLE_INPUT_BYTES,
            })
        );
    }

    #[test]
    fn live_bundle_stops_while_serializing_an_oversized_document() {
        let mut node = rect_node("n1");
        let PenNode::Rectangle(rectangle) = &mut node else {
            unreachable!("rect fixture")
        };
        rectangle.base.name = Some("x".repeat(MAX_LIVE_BUNDLE_INPUT_BYTES + 1));
        let mut state = EditorState::new();
        state.doc.children = vec![node];

        let error = live_bundle_artifact(&state).expect_err("live JSON exceeds preflight cap");
        assert!(matches!(
            error,
            CodegenExportError::LiveInputTooLarge {
                input_bytes,
                max_bytes: MAX_LIVE_BUNDLE_INPUT_BYTES,
            } if input_bytes > MAX_LIVE_BUNDLE_INPUT_BYTES
        ));
    }

    #[test]
    fn live_raw_preflight_bounds_asset_references_before_decode() {
        let raw = "data:,".repeat(MAX_LIVE_ASSET_REFERENCES + 1);
        assert_eq!(
            preflight_live_raw_input(&raw),
            Err(CodegenExportError::TooManyAssetReferences {
                count: MAX_LIVE_ASSET_REFERENCES + 1,
                max: MAX_LIVE_ASSET_REFERENCES,
            })
        );
    }

    #[test]
    fn stored_zip_budget_bounds_entry_count_without_payload_allocations() {
        assert_eq!(
            checked_stored_zip_size(std::iter::repeat_n(
                ("assets/a.png", 0),
                MAX_CODEGEN_ZIP_ENTRIES + 1
            )),
            Err(CodegenExportError::ZipEntryLimit)
        );
    }
}
