//! Component-instance override application — ports the
//! `applyInstanceOverrides` / `mergeSymbolProps` parts of
//! `frame-converter.ts`.
//!
//! Scope note: the direct-GUID resolution strategy (the common case)
//! plus the size-scaling fast path are ported. Figma's virtual-GUID
//! positional fallbacks (strategies 1–3 in the TS) are not — they
//! only apply to files whose override `guidPath`s do not name real
//! subtree nodes, which is rare.

use crate::common::round2;
use crate::figma_types::FigVec2;
use crate::kiwi::FigValue;
use crate::tree::{guid_to_string, TreeNode};
use std::collections::HashMap;

/// Layout keys an instance inherits from its master SYMBOL.
const LAYOUT_KEYS: &[&str] = &[
    "stackMode",
    "stackSpacing",
    "stackPadding",
    "stackHorizontalPadding",
    "stackVerticalPadding",
    "stackPaddingRight",
    "stackPaddingBottom",
    "stackPrimaryAlignItems",
    "stackCounterAlignItems",
    "stackPrimarySizing",
    "stackCounterSizing",
    "stackChildPrimaryGrow",
    "stackChildAlignSelf",
    "frameMaskDisabled",
];

/// Visual keys an instance inherits from its master SYMBOL.
const VISUAL_KEYS: &[&str] = &[
    "fillPaints",
    "strokePaints",
    "strokeWeight",
    "strokeAlign",
    "cornerRadius",
    "rectangleCornerRadiiIndependent",
    "rectangleTopLeftCornerRadius",
    "rectangleTopRightCornerRadius",
    "rectangleBottomLeftCornerRadius",
    "rectangleBottomRightCornerRadius",
];

/// Keys that must never be copied off an override entry onto a node.
const OVERRIDE_SKIP_KEYS: &[&str] = &[
    "guidPath",
    "guid",
    "parentIndex",
    "type",
    "phase",
    "symbolData",
    "derivedSymbolData",
    "componentKey",
    "variableConsumptionMap",
    "parameterConsumptionMap",
    "prototypeInteractions",
    "styleIdForFill",
    "styleIdForStrokeFill",
    "styleIdForText",
    "overrideLevel",
    "componentPropAssignments",
    "proportionsConstrained",
    "fontVersion",
];

/// Copy SYMBOL props onto an instance where the instance lacks them
/// (the instance's own values win).
pub fn merge_symbol_props(instance: &FigValue, symbol: &FigValue) -> FigValue {
    let mut merged = instance.clone();
    for key in LAYOUT_KEYS.iter().chain(VISUAL_KEYS.iter()) {
        if merged.get(key).is_none() {
            if let Some(v) = symbol.get(key) {
                merged.set(key, v.clone());
            }
        }
    }
    merged
}

/// `guidPath.guids` joined into a `/`-separated path key.
fn guid_path_key(entry: &FigValue) -> Option<String> {
    let guids = entry.get("guidPath")?.get_array("guids")?;
    if guids.is_empty() {
        return None;
    }
    let parts: Vec<String> = guids.iter().filter_map(guid_to_string).collect();
    if parts.len() == guids.len() {
        Some(parts.join("/"))
    } else {
        None
    }
}

/// Apply a SYMBOL instance's overrides + derived data onto a clone of
/// the SYMBOL subtree, returning the modified children.
pub fn apply_instance_overrides(
    symbol_node: &TreeNode,
    overrides: Option<&[FigValue]>,
    derived: Option<&[FigValue]>,
    instance_size: Option<FigVec2>,
) -> Vec<TreeNode> {
    let overrides = overrides.unwrap_or(&[]);
    let derived = derived.unwrap_or(&[]);

    // Fast path — nothing to apply: just rescale to the instance size.
    if derived.is_empty() && overrides.is_empty() {
        if let (Some(size), Some(sym_size)) = (
            instance_size,
            symbol_node.figma.get("size").map(FigVec2::from_value),
        ) {
            if let Some(sym_size) = sym_size {
                if sym_size.x != 0.0 && sym_size.y != 0.0 {
                    let sx = size.x / sym_size.x;
                    let sy = size.y / sym_size.y;
                    return crate::common::scale_tree_children(&symbol_node.children, sx, sy);
                }
            }
        }
        return symbol_node.children.clone();
    }

    // Direct-GUID resolution: map every override / derived entry whose
    // single-segment guidPath names a real node in the subtree.
    let mut node_override: HashMap<String, FigValue> = HashMap::new();
    let mut node_derived: HashMap<String, FigValue> = HashMap::new();
    let mut nested_override: HashMap<String, Vec<FigValue>> = HashMap::new();
    let mut nested_derived: HashMap<String, Vec<FigValue>> = HashMap::new();

    for entry in overrides {
        match guid_path_key(entry) {
            Some(key) if !key.contains('/') => {
                node_override.insert(key, entry.clone());
            }
            Some(key) => {
                let head = key.split('/').next().unwrap_or("").to_string();
                if let Some(rest) = strip_first_guid(entry) {
                    nested_override.entry(head).or_default().push(rest);
                }
            }
            None => {}
        }
    }
    for entry in derived {
        match guid_path_key(entry) {
            Some(key) if !key.contains('/') => {
                node_derived.insert(key, entry.clone());
            }
            Some(key) => {
                let head = key.split('/').next().unwrap_or("").to_string();
                if let Some(rest) = strip_first_guid(entry) {
                    nested_derived.entry(head).or_default().push(rest);
                }
            }
            None => {}
        }
    }

    symbol_node
        .children
        .iter()
        .map(|c| {
            apply_to_node(
                c,
                &node_override,
                &node_derived,
                &nested_override,
                &nested_derived,
            )
        })
        .collect()
}

/// Build a copy of `entry` with the first `guidPath` segment dropped.
fn strip_first_guid(entry: &FigValue) -> Option<FigValue> {
    let guids = entry.get("guidPath")?.get_array("guids")?;
    if guids.len() < 2 {
        return None;
    }
    let mut copy = entry.clone();
    let rest: Vec<FigValue> = guids[1..].to_vec();
    let mut path = FigValue::Object(Vec::new());
    path.set("guids", FigValue::Array(rest));
    copy.set("guidPath", path);
    Some(copy)
}

/// Recursively clone the subtree, applying derived data + overrides to
/// each node keyed by its guid.
fn apply_to_node(
    node: &TreeNode,
    node_override: &HashMap<String, FigValue>,
    node_derived: &HashMap<String, FigValue>,
    nested_override: &HashMap<String, Vec<FigValue>>,
    nested_derived: &HashMap<String, Vec<FigValue>>,
) -> TreeNode {
    let key = node
        .figma
        .get("guid")
        .and_then(guid_to_string)
        .unwrap_or_default();
    let d = node_derived.get(&key);
    let ov = node_override.get(&key);
    let nested_ov = nested_override.get(&key);
    let nested_d = nested_derived.get(&key);

    if d.is_none() && ov.is_none() && nested_ov.is_none() && nested_d.is_none() {
        return TreeNode {
            figma: node.figma.clone(),
            children: node
                .children
                .iter()
                .map(|c| {
                    apply_to_node(
                        c,
                        node_override,
                        node_derived,
                        nested_override,
                        nested_derived,
                    )
                })
                .collect(),
        };
    }

    let mut figma = node.figma.clone();

    // Derived data — scale stroke weight before overwriting size.
    if let Some(d) = d {
        if let (Some(dsize), Some(nsize)) = (
            d.get("size").and_then(FigVec2::from_value),
            node.figma.get("size").and_then(FigVec2::from_value),
        ) {
            if let Some(sw) = figma.get_f64("strokeWeight") {
                if nsize.x != 0.0 && nsize.y != 0.0 {
                    let scale = (dsize.x / nsize.x).min(dsize.y / nsize.y);
                    if scale < 0.99 {
                        figma.set("strokeWeight", FigValue::Float(round2(sw * scale) as f32));
                    }
                }
            }
        }
        if let Some(size) = d.get("size") {
            figma.set("size", size.clone());
        }
        if let Some(t) = d.get("transform") {
            figma.set("transform", t.clone());
        }
        if let Some(fs) = d.get("fontSize") {
            figma.set("fontSize", fs.clone());
        }
        if let Some(dtd) = d.get("derivedTextData") {
            if dtd.get("characters").is_some() {
                figma.set("textData", dtd.clone());
            }
        }
    }

    // Override props — copy every non-blacklisted, present key.
    if let Some(ov) = ov {
        if let FigValue::Object(pairs) = ov {
            for (k, v) in pairs {
                if !OVERRIDE_SKIP_KEYS.contains(&k.as_str()) && !matches!(v, FigValue::Null) {
                    figma.set(k, v.clone());
                }
            }
        }
    }

    // Forward nested entries into nested INSTANCE nodes.
    let is_instance =
        figma.get_str("type") == Some("INSTANCE") || figma.get("symbolData").is_some();
    if is_instance {
        if let Some(nested) = nested_ov {
            let mut existing: Vec<FigValue> = figma
                .get("symbolData")
                .and_then(|s| s.get_array("symbolOverrides"))
                .map(|a| a.to_vec())
                .unwrap_or_default();
            existing.extend(nested.clone());
            let mut symbol_data = figma
                .get("symbolData")
                .cloned()
                .unwrap_or(FigValue::Object(Vec::new()));
            symbol_data.set("symbolOverrides", FigValue::Array(existing));
            figma.set("symbolData", symbol_data);
        }
        if let Some(nested) = nested_d {
            figma.set("derivedSymbolData", FigValue::Array(nested.clone()));
        }
    }

    TreeNode {
        figma,
        children: node
            .children
            .iter()
            .map(|c| {
                apply_to_node(
                    c,
                    node_override,
                    node_derived,
                    nested_override,
                    nested_derived,
                )
            })
            .collect(),
    }
}
