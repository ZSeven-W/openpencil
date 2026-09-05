//! Legacy Pencil/TypeScript JSON repairs for the canonical loader.
//!
//! The schema loader calls this on its own parsed JSON tree before typed
//! conversion, avoiding the former parse-normalize-serialize-reparse cycle.

/// Repair known Pencil/TypeScript leniency gaps in the schema loader's own
/// JSON tree. Mutating in place is important: serializing this value and then
/// asking jian-ops-schema to parse it again used to build two full DOMs for
/// every canonical document.
pub(super) fn normalize_legacy_value(value: &mut serde_json::Value) -> bool {
    let mut changed = super::normalize_video_alias(value);
    let repair_legacy_figma_child_order = is_legacy_figma_child_order_candidate(value)
        && auto_layout_order_evidence(value).strongly_indicates_flow_order();
    // Pencil design-kit `.pen` files reference `$--radius-*` number variables
    // from `cornerRadius`; the canonical `CornerRadius` enum can only hold a
    // number / `[f64; 4]`, so collect the document's number-variable table up
    // front to resolve those refs to concrete radii during the walk.
    let radius_vars = collect_number_variables(value);
    normalize_node_value(
        value,
        &radius_vars,
        repair_legacy_figma_child_order,
        &mut changed,
    );
    changed
}

/// Detect the short-lived Figma Preserve writer format that omitted the
/// geometry-mode bit and serialized auto-layout children in flow order.
/// Every condition is required so ordinary documents and explicit false are
/// byte-semantically untouched by the order repair.
fn is_legacy_figma_child_order_candidate(root: &serde_json::Value) -> bool {
    use serde_json::Value;

    let Some(meta) = root.get("editorMeta").and_then(Value::as_object) else {
        return false;
    };
    if meta.contains_key("preserveAuthoredGeometry")
        || meta.contains_key("preserve_authored_geometry")
    {
        return false;
    }
    let Some(page_id) = root
        .get("pages")
        .and_then(Value::as_array)
        .and_then(|pages| pages.first())
        .and_then(|page| page.get("id"))
        .and_then(Value::as_str)
    else {
        return false;
    };
    page_id.strip_prefix("figma-page-").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

#[derive(Default)]
struct ChildOrderEvidence {
    flow_order_pairs: usize,
    paint_order_pairs: usize,
}

impl ChildOrderEvidence {
    fn strongly_indicates_flow_order(&self) -> bool {
        const MIN_FLOW_PAIRS: usize = 3;

        self.flow_order_pairs >= MIN_FLOW_PAIRS
            && self.flow_order_pairs >= self.paint_order_pairs.saturating_mul(2)
    }
}

/// Sample authored coordinates without mutating the parsed tree. The broken
/// writer stored auto-layout flow children in ascending axis order; canonical
/// paint order is descending. Absolute-positioned children carry constraints
/// and do not provide flow-order evidence, so pairs touching one are skipped.
fn auto_layout_order_evidence(root: &serde_json::Value) -> ChildOrderEvidence {
    use serde_json::Value;

    let mut evidence = ChildOrderEvidence::default();
    let mut stack = vec![root];
    while let Some(value) = stack.pop() {
        match value {
            Value::Object(map) => {
                let axis = match (
                    map.get("type").and_then(Value::as_str),
                    map.get("layout").and_then(Value::as_str),
                ) {
                    (Some("frame"), Some("vertical")) => Some("y"),
                    (Some("frame"), Some("horizontal")) => Some("x"),
                    _ => None,
                };
                if let (Some(axis), Some(children)) =
                    (axis, map.get("children").and_then(Value::as_array))
                {
                    for pair in children.windows(2) {
                        let Some(first) = pair[0].as_object() else {
                            continue;
                        };
                        let Some(second) = pair[1].as_object() else {
                            continue;
                        };
                        if first.contains_key("constraints") || second.contains_key("constraints") {
                            continue;
                        }
                        let Some(first_axis) = first.get(axis).and_then(Value::as_f64) else {
                            continue;
                        };
                        let Some(second_axis) = second.get(axis).and_then(Value::as_f64) else {
                            continue;
                        };
                        const AXIS_EPSILON: f64 = 0.01;
                        let delta = second_axis - first_axis;
                        if delta > AXIS_EPSILON {
                            evidence.flow_order_pairs += 1;
                        } else if delta < -AXIS_EPSILON {
                            evidence.paint_order_pairs += 1;
                        }
                    }
                }
                stack.extend(map.values());
            }
            Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    evidence
}

/// Harvest `variables.<name> = { type: "number", value: N | [{value, theme}] }`
/// into a `$<name>` → first-concrete-number map. Used to resolve
/// `cornerRadius: "$--radius-pill"` legacy refs (Pencil design kits) into the
/// number the canonical `CornerRadius` enum can actually hold. Theme-keyed
/// arrays collapse to their first entry's value — corner radius is rarely
/// theme-varied, and a constant fallback beats refusing the whole file.
fn collect_number_variables(root: &serde_json::Value) -> std::collections::HashMap<String, f64> {
    use serde_json::Value;
    let mut out = std::collections::HashMap::new();
    let Some(vars) = root.get("variables").and_then(Value::as_object) else {
        return out;
    };
    for (name, def) in vars {
        if def.get("type").and_then(Value::as_str) != Some("number") {
            continue;
        }
        let resolved = match def.get("value") {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::Array(entries)) => entries
                .first()
                .and_then(|e| e.get("value"))
                .and_then(Value::as_f64),
            _ => None,
        };
        if let Some(v) = resolved {
            out.insert(format!("${name}"), v);
        }
    }
    out
}

/// Iterative (explicit stack, not recursion — deep trees must not blow the
/// stack) walk that repairs known legacy `.pen` / `.op` shapes the strict
/// canonical schema rejects but the TS runtime tolerated:
///
/// - PenNode `fill` written as a bare color/`$ref` string → wrap into the
///   `Vec<PenFill>` jian expects.
/// - Image node missing the required `src`.
/// - Pencil legacy `type: "icon"` → canonical `icon_font`.
/// - A `stroke` written as a bare color string → wrap into a `PenStroke`.
/// - A stroke object's `fill` string (no `type` key on the stroke object) →
///   wrap into `Vec<PenFill>` (same shape as node fill).
/// - `cornerRadius` written as a `$ref` string (or array of them) → resolved
///   to the concrete number(s) the `CornerRadius` enum can hold.
/// - The narrowly identified intermediate Figma Preserve format's auto-layout
///   frame children → reverse from legacy flow order to canonical paint order.
///
/// Type-less objects whose `fill` is legitimately a `String`
/// (`StyledTextSegment`) are left untouched — the wrap is gated on the object
/// being a PenNode (`type` key) or a stroke (`thickness` key).
fn normalize_node_value(
    root: &mut serde_json::Value,
    number_vars: &std::collections::HashMap<String, f64>,
    repair_legacy_figma_child_order: bool,
    changed: &mut bool,
) {
    use serde_json::Value;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        match node {
            Value::Object(map) => {
                let is_pen_node = map.contains_key("type");
                if is_pen_node {
                    // Copy the kind decisions to owned bools so the mutating
                    // inserts below don't conflict with the borrow on `map`.
                    let kind = map.get("type").and_then(Value::as_str);
                    let is_image = kind == Some("image");
                    let is_icon = kind == Some("icon");
                    let is_prompt = kind == Some("prompt");
                    let is_path = kind == Some("path");
                    let reverse_auto_layout_children = repair_legacy_figma_child_order
                        && kind == Some("frame")
                        && matches!(
                            map.get("layout").and_then(Value::as_str),
                            Some("horizontal" | "vertical")
                        );
                    if reverse_auto_layout_children {
                        if let Some(Value::Array(children)) = map.get_mut("children") {
                            if children.len() > 1 {
                                children.reverse();
                                *changed = true;
                            }
                        }
                    }
                    if is_image && !map.contains_key("src") {
                        map.insert("src".to_string(), Value::String(String::new()));
                        *changed = true;
                    }
                    // Pencil's `prompt` node is an AI-annotation card with no
                    // canonical equivalent → degrade to a `text` node. Its
                    // `content` field already matches `TextNode.content`; just
                    // drop the editor-only `model` field.
                    if is_prompt {
                        map.insert("type".to_string(), Value::String("text".to_string()));
                        map.remove("model");
                        *changed = true;
                    }
                    // Pencil's legacy `icon` node is jian's `icon_font`: the
                    // glyph lives under `icon` (→ `iconFontName`) and the font
                    // family under `library` (→ `iconFontFamily`).
                    if is_icon {
                        map.insert("type".to_string(), Value::String("icon_font".to_string()));
                        if let Some(glyph) = map.remove("icon") {
                            map.insert("iconFontName".to_string(), glyph);
                        }
                        if let Some(family) = map.remove("library") {
                            map.insert("iconFontFamily".to_string(), family);
                        }
                        *changed = true;
                    }
                    // Pencil stores a `path` node's flattened outline as an SVG
                    // string under `geometry`; jian's `PathNode` reads it from
                    // `d`. Without this remap the geometry is dropped and the
                    // path renders empty — status-bar icons, logos, and any
                    // vector art authored as a path vanish.
                    if is_path {
                        if let Some(geom) = map.remove("geometry") {
                            if geom.is_string() && !map.contains_key("d") {
                                map.insert("d".to_string(), geom);
                            }
                            *changed = true;
                        }
                    }
                    // A PenNode `fill` written as a bare string / single object /
                    // legacy `type:"color"` array → canonical `Vec<PenFill>`.
                    normalize_fill(map, changed);
                    // `cornerRadius` as a `$ref` string / array of them →
                    // resolve to the number(s) the `CornerRadius` enum holds.
                    normalize_corner_radius(map, number_vars, changed);
                    // A `stroke` written as a bare color string → wrap into a
                    // minimal `PenStroke { thickness: 1, fill: [...] }`.
                    if let Some(color) = map.get("stroke").and_then(|s| match s {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    }) {
                        map.insert(
                            "stroke".to_string(),
                            serde_json::json!({
                                "thickness": 1,
                                "fill": [{ "type": "solid", "color": color }],
                            }),
                        );
                        *changed = true;
                    }
                } else if map.contains_key("thickness") {
                    // Stroke object (has `thickness`, no `type`): its `fill`
                    // is the same normalization target as a node's.
                    normalize_fill(map, changed);
                }
                // Spacing fields (`padding` / `gap` / `margin`) may be numeric
                // arrays with an embedded `$ref` string (`[8, "$spacing/3"]`).
                // The `Padding` enum's array arms are all-number, so resolve any
                // ref element to its number. A bare-string padding (`"$x"`) is
                // left alone — the `Expression(String)` arm accepts it.
                for key in ["padding", "gap", "margin"] {
                    resolve_spacing_ref_array(map, key, number_vars, changed);
                }
                stack.extend(map.values_mut());
            }
            Value::Array(items) => stack.extend(items.iter_mut()),
            _ => {}
        }
    }
}

/// Normalize a `fill` into the canonical `Vec<PenFill>` jian expects. Handles
/// the legacy Pencil shapes the strict schema rejects:
/// - bare color/`$ref` string (`"#1A1D2E"` / `"$--sidebar-border"`)
/// - a single fill *object* (`{ "type": "color", "color": "...", "enabled": false }`)
///   rather than a one-element array
/// - the `type: "color"` discriminant (jian's solid arm is `"solid"`)
/// - an `enabled: false` flag → the fill is dropped (disabled in the source)
///
/// No-op when `fill` is absent or already a clean canonical array.
fn normalize_fill(map: &mut serde_json::Map<String, serde_json::Value>, changed: &mut bool) {
    use serde_json::Value;
    let Some(fill) = map.get("fill") else {
        return;
    };
    match fill {
        Value::String(s) => {
            let color = s.clone();
            map.insert(
                "fill".to_string(),
                serde_json::json!([{ "type": "solid", "color": color }]),
            );
            *changed = true;
        }
        Value::Object(_) => {
            // A single fill written as an object → array of (maybe) one,
            // after legacy-discriminant + enabled normalization.
            let mut one = fill.clone();
            let mut dummy = false;
            let arr: Vec<Value> = match normalize_fill_entry(&mut one, &mut dummy) {
                Some(v) => vec![v],
                None => vec![],
            };
            map.insert("fill".to_string(), Value::Array(arr));
            *changed = true;
        }
        Value::Array(items) => {
            // Already an array — but its elements may still carry the legacy
            // `type: "color"` discriminant or an `enabled: false` flag.
            let mut local_changed = false;
            let mut out = Vec::with_capacity(items.len());
            for item in items.clone() {
                let mut entry = item;
                if let Some(v) = normalize_fill_entry(&mut entry, &mut local_changed) {
                    out.push(v);
                }
            }
            if local_changed {
                map.insert("fill".to_string(), Value::Array(out));
                *changed = true;
            }
        }
        _ => {}
    }
}

/// Normalize one PenFill value. Returns `None` when the fill is explicitly
/// `enabled: false` (a disabled fill is dropped from the array). Rewrites the
/// legacy `type: "color"` discriminant to `"solid"` and strips the non-schema
/// `enabled` key. Sets `changed` when it touches anything.
fn normalize_fill_entry(
    entry: &mut serde_json::Value,
    changed: &mut bool,
) -> Option<serde_json::Value> {
    use serde_json::Value;
    let Value::Object(obj) = entry else {
        return Some(entry.clone());
    };
    if obj.get("enabled") == Some(&Value::Bool(false)) {
        *changed = true;
        return None;
    }
    if obj.remove("enabled").is_some() {
        *changed = true;
    }
    match obj.get("type").and_then(Value::as_str) {
        Some("color") => {
            obj.insert("type".to_string(), Value::String("solid".to_string()));
            *changed = true;
        }
        Some("gradient") => {
            normalize_gradient_fill(obj, changed);
        }
        _ => {}
    }
    Some(Value::Object(obj.clone()))
}

/// Rewrite Pencil's generic `type: "gradient"` fill into the canonical
/// `linear_gradient` / `radial_gradient` PenFill. Source shape:
/// `{ type: "gradient", gradientType: "linear"|"radial", rotation, colors: [{ color, position }], size }`.
/// Canonical shape: `{ type: "linear_gradient", angle, stops: [{ offset, color }] }`.
fn normalize_gradient_fill(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    changed: &mut bool,
) {
    use serde_json::Value;
    let radial = obj.get("gradientType").and_then(Value::as_str) == Some("radial");
    obj.insert(
        "type".to_string(),
        Value::String(
            if radial {
                "radial_gradient"
            } else {
                "linear_gradient"
            }
            .to_string(),
        ),
    );
    obj.remove("gradientType");
    // `rotation` (degrees) → `angle`; only for the linear arm (radial ignores it).
    if let Some(rot) = obj.remove("rotation") {
        if !radial {
            obj.insert("angle".to_string(), rot);
        }
    }
    obj.remove("size");
    // `colors: [{ color, position }]` → `stops: [{ offset, color }]`.
    if let Some(Value::Array(colors)) = obj.remove("colors") {
        let stops: Vec<Value> = colors
            .into_iter()
            .filter_map(|c| {
                let o = c.as_object()?;
                let color = o.get("color").and_then(Value::as_str)?.to_string();
                let offset = o.get("position").and_then(Value::as_f64).unwrap_or(0.0);
                Some(serde_json::json!({ "offset": offset, "color": color }))
            })
            .collect();
        obj.insert("stops".to_string(), Value::Array(stops));
    } else if !obj.contains_key("stops") {
        // A gradient with no stops is illegal; seed an empty array so the
        // canonical loader gets a valid (if degenerate) gradient body.
        obj.insert("stops".to_string(), Value::Array(vec![]));
    }
    *changed = true;
}

/// Resolve `$ref` strings embedded in a numeric spacing array
/// (`padding`/`gap`/`margin`: `[8, "$spacing/3"]`) to their concrete numbers.
/// No-op unless the value is an array containing at least one `$ref` string —
/// a bare-string spacing is left for the schema's `Expression` arm.
fn resolve_spacing_ref_array(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    number_vars: &std::collections::HashMap<String, f64>,
    changed: &mut bool,
) {
    use serde_json::Value;
    let Some(Value::Array(items)) = map.get(key) else {
        return;
    };
    if !items.iter().any(Value::is_string) {
        return;
    }
    let resolved: Vec<Value> = items
        .iter()
        .map(|v| match v {
            Value::String(s) => {
                serde_json::json!(number_vars.get(s).copied().unwrap_or(0.0))
            }
            other => other.clone(),
        })
        .collect();
    map.insert(key.to_string(), Value::Array(resolved));
    *changed = true;
}

/// Resolve a `cornerRadius` that is a `$ref` string, or an array of
/// `$ref`/number entries, into the number / `[f64; 4]` the canonical
/// `CornerRadius` enum accepts. A ref with no matching number variable falls
/// back to `0.0` so the file still loads (an unresolved radius is cosmetic).
fn normalize_corner_radius(
    map: &mut serde_json::Map<String, serde_json::Value>,
    number_vars: &std::collections::HashMap<String, f64>,
    changed: &mut bool,
) {
    use serde_json::Value;
    let resolve_scalar = |v: &Value| -> Option<f64> {
        match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => Some(number_vars.get(s).copied().unwrap_or(0.0)),
            _ => None,
        }
    };
    match map.get("cornerRadius") {
        Some(Value::String(s)) => {
            let n = number_vars.get(s).copied().unwrap_or(0.0);
            map.insert("cornerRadius".to_string(), serde_json::json!(n));
            *changed = true;
        }
        Some(Value::Array(entries)) => {
            // Only rewrite if at least one entry is a `$ref` string (a plain
            // numeric `[f64; 4]` is already valid; leave it alone).
            if entries.iter().any(|e| e.is_string()) {
                let nums: Vec<f64> = entries.iter().filter_map(&resolve_scalar).collect();
                // The enum's array arm is exactly `[f64; 4]`. Pad/truncate so a
                // 1- or 2-value Pencil shorthand still lands in a legal shape.
                let four = match nums.len() {
                    0 => [0.0; 4],
                    1 => [nums[0]; 4],
                    n if n >= 4 => [nums[0], nums[1], nums[2], nums[3]],
                    _ => {
                        let mut a = [0.0; 4];
                        for (i, v) in nums.iter().enumerate() {
                            a[i] = *v;
                        }
                        a
                    }
                };
                map.insert("cornerRadius".to_string(), serde_json::json!(four));
                *changed = true;
            }
        }
        _ => {}
    }
}
