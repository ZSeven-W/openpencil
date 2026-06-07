//! "Export AI Bundle" packager. Port of
//! `apps/web/src/services/ai/structure-bundle.ts`
//! (`buildAIStructureBundle`). Packages the design structure into the
//! in-memory file set the host later writes as a ZIP: a `manifest.json`,
//! the `views/raw.json` (`asset://` URIs) + `views/sanitized.json`
//! (`./assets/...` paths) split, and one `assets/<...>` entry per asset.
//!
//! Web-only manifest fields are intentionally omitted (noted at the
//! `build_manifest` call): `generatedAt` (needs a clock), the per-node
//! `scope` details (`activePageId` / `selectedIds` / `exportedRootIds` /
//! node counts), the per-view `nodeCount`, and the per-asset `size` /
//! `sha256` / `sourceNodeName` / `sourceKind` / `rawRefs` / `sanitizedRefs`
//! ref index — none of which this leaf packager has the context to fill.

use serde_json::json;

use crate::ai::types::AssetFile;

const MANIFEST_PATH: &str = "manifest.json";
const RAW_VIEW_PATH: &str = "views/raw.json";
const SANITIZED_VIEW_PATH: &str = "views/sanitized.json";
const RAW_ASSET_PREFIX: &str = "asset://";
const ASSETS_BASE_PATH: &str = "./assets/";

/// A built structure bundle ready for the host to write as a ZIP. `files`
/// are (in-zip path, bytes) pairs.
pub struct StructureBundle {
    pub files: Vec<(String, Vec<u8>)>,
}

/// Scope of the bundle. Parity with the TS scope modes
/// (`AIStructureBundleScopeMode = 'selection' | 'page'`).
pub enum BundleScope {
    Selection,
    Page,
}

impl BundleScope {
    /// Wire string matching the TS `mode` values.
    fn as_str(&self) -> &'static str {
        match self {
            BundleScope::Selection => "selection",
            BundleScope::Page => "page",
        }
    }
}

/// Build the bundle: manifest.json + views/raw.json + views/sanitized.json
/// + assets/<...>. Port of `buildAIStructureBundle`.
pub fn build_structure_bundle(
    raw_nodes_json: &str,
    sanitized_nodes_json: &str,
    assets: &[AssetFile],
    scope: BundleScope,
) -> StructureBundle {
    let mut files: Vec<(String, Vec<u8>)> = Vec::with_capacity(3 + assets.len());

    let manifest = build_manifest(&scope, assets);
    // `to_vec_pretty` mirrors the TS `JSON.stringify(value, null, 2)`.
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap_or_default();

    files.push((MANIFEST_PATH.to_string(), manifest_bytes));
    files.push((
        RAW_VIEW_PATH.to_string(),
        raw_nodes_json.as_bytes().to_vec(),
    ));
    files.push((
        SANITIZED_VIEW_PATH.to_string(),
        sanitized_nodes_json.as_bytes().to_vec(),
    ));

    // One entry per asset at its `zip_path`, carrying the asset's bytes —
    // matches the TS `Object.fromEntries(assets.map([zipPath, bytes]))`.
    for asset in assets {
        files.push((asset.zip_path.clone(), asset.bytes.clone()));
    }

    StructureBundle { files }
}

/// Build the manifest.json object. Mirrors the TS manifest field names
/// where this leaf packager has the data: `kind` / `version` /
/// `consumerView` / `scope` / `views.{raw,sanitized}.{path,...}` /
/// `assets[].{id,path,mimeType}`. See the module header for the
/// web-only fields omitted here.
fn build_manifest(scope: &BundleScope, assets: &[AssetFile]) -> serde_json::Value {
    let asset_index: Vec<serde_json::Value> = assets
        .iter()
        .map(|asset| {
            json!({
                "id": asset.id,
                // TS calls this `relativePath`; the task asks for `path`.
                "path": asset.relative_path,
                "mimeType": asset.mime_type,
            })
        })
        .collect();

    json!({
        "kind": "ai-structure-bundle",
        "version": "1",
        "consumerView": "sanitized",
        "scope": scope.as_str(),
        "views": {
            "raw": {
                "path": RAW_VIEW_PATH,
                "assetReferencePrefix": RAW_ASSET_PREFIX,
            },
            "sanitized": {
                "path": SANITIZED_VIEW_PATH,
                "assetBasePath": ASSETS_BASE_PATH,
            },
        },
        "assets": asset_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_contains_manifest_and_both_views() {
        let bundle = build_structure_bundle("[]", "[]", &[], BundleScope::Selection);
        let paths: Vec<&str> = bundle.files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"manifest.json"));
        assert!(paths.contains(&"views/raw.json"));
        assert!(paths.contains(&"views/sanitized.json"));
    }

    #[test]
    fn bundle_includes_asset_files_under_assets_dir() {
        let asset = AssetFile {
            id: "1".into(),
            relative_path: "./assets/image-1.png".into(),
            zip_path: "assets/image-1.png".into(),
            mime_type: "image/png".into(),
            bytes: vec![1, 2, 3],
            source_node_id: "n1".into(),
        };
        let bundle =
            build_structure_bundle("[]", "[]", std::slice::from_ref(&asset), BundleScope::Page);
        assert!(bundle
            .files
            .iter()
            .any(|(p, b)| p == "assets/image-1.png" && b == &vec![1, 2, 3]));
    }

    #[test]
    fn manifest_is_valid_json_with_scope() {
        let bundle = build_structure_bundle("[]", "[]", &[], BundleScope::Page);
        let (_p, manifest_bytes) = bundle
            .files
            .iter()
            .find(|(p, _)| p == "manifest.json")
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(manifest_bytes).unwrap();
        assert_eq!(v["scope"], "page");
    }
}
