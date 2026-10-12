//! Shared polarity prediction for binding and final cleanup.
use super::*;

/// Repair THEME-POLARITY splits in the variable table. A model building a
/// dark design writes its dark values into the ACTIVE theme slot for most
/// variables — but leaves a few (surface-2/3, status-\*-bg) at stock LIGHT
/// values, so a chip resolves #F1F5F9 on a #0A0A0A page (measured: glaring
/// white pills on a dark luxury dashboard). The correct dark value usually
/// EXISTS in the variable's other theme slot; adopt it.
///
/// Polarity contract, judged against the first design root's resolved page
/// background: surface-family variables (`surface|card|panel|chip|bg`) sit on
/// the SAME side as the page; text-family (`text|foreground`) sit OPPOSITE.
/// A variable is only rewritten when its active value clearly violates the
/// contract AND another slot's value clearly satisfies it — everything
/// ambiguous is left alone.
pub(crate) fn fix_theme_variable_polarity(sink: &mut dyn crate::types::DocSink) {
    let fixes = theme_variable_polarity_fixes(sink.state(), None);
    if fixes.is_empty() {
        return;
    }
    let mut variables = std::collections::BTreeMap::new();
    for (name, def) in fixes {
        if let Ok(parsed) =
            serde_json::from_value::<jian_ops_schema::variable::VariableDefinition>(def)
        {
            variables.insert(name, parsed);
        }
    }
    if !variables.is_empty() {
        sink.apply(EditorCommand::SetVariables {
            variables,
            replace: false,
        });
    }
}

pub(crate) fn theme_variable_polarity_fixes(
    state: &EditorState,
    background: Option<&str>,
) -> Vec<(String, Value)> {
    let Some(bg_lum) = background
        .map(str::to_string)
        .or_else(|| {
            state
                .active_children()
                .first()
                .and_then(first_solid_fill_hex)
                .map(str::to_string)
        })
        .and_then(|c| match c.strip_prefix('$') {
            Some(name) => state.resolve_color_variable_hex(name),
            None => Some(c),
        })
        .as_deref()
        .and_then(hex_luminance)
    else {
        return Vec::new();
    };
    let Ok(vars) = serde_json::to_value(state.doc.variables.clone()) else {
        return Vec::new();
    };
    let Some(map) = vars.as_object() else {
        return Vec::new();
    };
    let mut fixes = Vec::new();
    for (name, def) in map {
        if def.get("type").and_then(Value::as_str) != Some("color") {
            continue;
        }
        let lname = name.to_lowercase();
        // `border` / `outline` / `divider` / `input` / `ring` sit with
        // the SURFACE family, not on their own: a hairline lives a step
        // off the page tone, far from the text tone, so the surface
        // thresholds below classify it correctly with no extra tuning.
        // They were missing from this list, which left a dark design's
        // `$--border` resolving to its stock-light slot (measured:
        // 0808-gm-1.op kept `#E2E8F0` on a `#0A0A0A` page — a near-WHITE
        // hairline, which the widget renderer then used as the tab
        // bar's background). The shadcn rename (B1) replaced the
        // `color-*` word set with slot names, so the list carries the
        // new vocabulary (`muted`/`secondary`/`popover`/`sidebar`/
        // `accent` surfaces, `input`/`ring` hairlines, the status
        // colours, `scrim`).
        let surface_like = [
            "surface",
            "card",
            "panel",
            "chip",
            "bg",
            "background",
            "border",
            "outline",
            "divider",
            "muted",
            "secondary",
            "popover",
            "input",
            "ring",
            "sidebar",
            "accent",
            "scrim",
            "success",
            "warning",
            "error",
            "info",
        ]
        .iter()
        .any(|t| lname.contains(t));
        // A `*-foreground` on-color token is a TEXT colour even when its
        // base slot word also appears (`--card-foreground`): foreground
        // wins, mirroring `variable_binding::family_of`.
        let text_like = lname.contains("text") || lname.contains("foreground");
        let family_is_surface = if text_like {
            false
        } else if surface_like {
            true
        } else {
            // neither family — leave alone
            continue;
        };
        let Some(active_hex) = state.resolve_color_variable_hex(name) else {
            continue;
        };
        let Some(active_lum) = hex_luminance(&active_hex) else {
            continue;
        };
        // Every hex present across the variable's theme slots.
        let mut slot_hexes: Vec<String> = Vec::new();
        collect_hex_strings(def.get("value").unwrap_or(&Value::Null), &mut slot_hexes);
        let violated = if family_is_surface {
            (active_lum - bg_lum).abs() > 0.55
        } else {
            (active_lum - bg_lum).abs() < 0.22
        };
        if !violated {
            continue;
        }
        let candidate = slot_hexes
            .iter()
            .filter_map(|h| hex_luminance(h).map(|l| (h, l)))
            .filter(|(h, _)| !h.eq_ignore_ascii_case(&active_hex))
            .filter(|(_, l)| {
                if family_is_surface {
                    (l - bg_lum).abs() < 0.35
                } else {
                    (l - bg_lum).abs() > 0.5
                }
            })
            .min_by(|a, b| {
                let key = |l: f64| {
                    if family_is_surface {
                        (l - bg_lum).abs()
                    } else {
                        -(l - bg_lum).abs()
                    }
                };
                key(a.1).total_cmp(&key(b.1))
            })
            .map(|(h, _)| h.clone());
        if let Some(hex) = candidate {
            // Overwrite the SLOT the resolver actually hit (the entry whose
            // value equals the resolved active hex) — `SetVariableColor`
            // routes through the session's active-theme pin, which a
            // freshly-loaded document does not carry, and would append a
            // themeless entry the resolver never reads.
            let mut def = def.clone();
            if replace_hex_slot(
                def.get_mut("value").unwrap_or(&mut Value::Null),
                &active_hex,
                &hex,
            ) {
                fixes.push((name.clone(), def));
            }
        }
    }
    fixes
}
