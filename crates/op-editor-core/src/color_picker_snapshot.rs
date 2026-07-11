//! Variable / fill / effect hex-extraction helpers for the colour
//! picker's history-change check — extracted from `color_picker.rs`
//! to keep that file under the 800-line cap. `pub(crate)` so the
//! picker state machine in `color_picker.rs` can reach them; nothing
//! outside the crate uses these.

/// Reduce a resolved variable scalar to a `#rrggbb` hex string, if it
/// is a `Str` scalar. Used by the variable-mode `close_color_picker`
/// change check.
pub(crate) fn scalar_as_hex(s: &jian_ops_schema::variable::VariableScalar) -> Option<String> {
    match s {
        jian_ops_schema::variable::VariableScalar::Str(hex) => Some(hex.clone()),
        _ => None,
    }
}

/// Scalar shown in one variant column: exact `(axis, value)` match →
/// untagged (`theme: None`) entry → first entry. Same fallback chain
/// as the panel grid (TS `variable-row.tsx getValueForTheme`).
pub(crate) fn variant_scalar<'a>(
    value: &'a jian_ops_schema::variable::VariableValue,
    axis: &str,
    theme_value: &str,
) -> Option<&'a jian_ops_schema::variable::VariableScalar> {
    use jian_ops_schema::variable::VariableValue;
    match value {
        VariableValue::Scalar(s) => Some(s),
        VariableValue::Themed(entries) => entries
            .iter()
            .find(|entry| {
                entry
                    .theme
                    .as_ref()
                    .and_then(|theme| theme.get(axis))
                    .is_some_and(|v| v == theme_value)
            })
            .or_else(|| entries.iter().find(|entry| entry.theme.is_none()))
            .or_else(|| entries.first())
            .map(|entry| &entry.value),
    }
}

/// Re-attach an alpha (0..=1) to a `#RRGGBB` hex. When the alpha
/// would round to fully opaque the 6-char form is preserved so the
/// canonical schema stays compact.
pub(crate) fn splice_alpha(hex: &str, alpha: f32) -> String {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a == 255 {
        hex.to_string()
    } else {
        format!("{}{:02X}", hex, a)
    }
}

/// Read the colour hex of the Shadow effect at `index` on `node`.
/// `None` when the node has no effects, the index is out of range,
/// or the effect isn't a Shadow.
pub(crate) fn effect_color_hex(
    node: &jian_ops_schema::node::PenNode,
    index: usize,
) -> Option<String> {
    use jian_ops_schema::style::PenEffect;
    let effects = crate::fills::node_effects(node);
    match effects.get(index)? {
        PenEffect::Shadow(s) => Some(s.color.clone()),
        _ => None,
    }
}

/// Read one stop's hex from the node's primary gradient body.
/// `None` when the first fill isn't a gradient or `index` is out of
/// range — the same gating `set_primary_gradient_stop_hex` applies
/// on the write path.
pub(crate) fn gradient_stop_hex(
    node: &jian_ops_schema::node::PenNode,
    index: usize,
) -> Option<String> {
    use jian_ops_schema::style::PenFill;
    let fills = crate::fills::node_fills(node)?;
    let first = fills.first()?;
    let stops = match first {
        PenFill::LinearGradient(b) => &b.stops,
        PenFill::RadialGradient(b) => &b.stops,
        _ => return None,
    };
    stops.get(index).map(|s| s.color.clone())
}

/// Resolve a Color variable's hex scalar from a history snapshot's
/// `doc` under the supplied active-theme selection. The snapshot's
/// `EditorSnapshot` carries the full `PenDocument` (variables
/// included) but not the transient `ui.variables.active_theme`, so the
/// caller threads the live active theme in — it is stable across the
/// short-lived picker session.
pub(crate) fn snapshot_variable_hex(
    snap: &crate::history::EditorSnapshot,
    name: &str,
    active_theme: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    let def = snap.doc.variables()?.get(name)?;
    if !matches!(def.kind, jian_ops_schema::variable::VariableKind::Color) {
        return None;
    }
    let scalar = resolve_snapshot_value(&def.value, active_theme)?;
    scalar_as_hex(scalar)
}

/// Resolve a `VariableValue` under `active_theme` — picks a `Scalar`
/// directly, or a `Themed` list's subset-matching entry, falling back
/// to the `theme: None` default. Mirrors `variables.rs::resolve_value`.
fn resolve_snapshot_value<'a>(
    value: &'a jian_ops_schema::variable::VariableValue,
    active_theme: &std::collections::BTreeMap<String, String>,
) -> Option<&'a jian_ops_schema::variable::VariableScalar> {
    use jian_ops_schema::variable::VariableValue;
    match value {
        VariableValue::Scalar(s) => Some(s),
        VariableValue::Themed(entries) => {
            for e in entries {
                if let Some(t) = &e.theme {
                    if t.iter().all(|(k, v)| active_theme.get(k) == Some(v)) {
                        return Some(&e.value);
                    }
                }
            }
            entries.iter().find(|e| e.theme.is_none()).map(|e| &e.value)
        }
    }
}

/// Find a node by id on the active page inside a history snapshot —
/// mirrors [`crate::state::EditorState::active_children`] + `find_node`
/// but reads from the snapshot's shared (`Arc`) document. The snapshot
/// document no longer exposes a `&[PenNode]` slice (its top-level nodes
/// are `Arc<PenNode>`), so the lookup is folded into one call.
pub(crate) fn snapshot_find_node<'a>(
    snap: &'a crate::history::EditorSnapshot,
    id: &crate::node_id::NodeId,
) -> Option<&'a jian_ops_schema::node::PenNode> {
    snap.doc.snapshot_find_node(snap.active_page_index, id)
}
