//! Role-default injection — P2 increment **I2**.
//!
//! Port of the 43 `registerRole` rules in `role-definitions/index.ts` plus
//! `applyDefaults` and `detectThemeFromNode` from `role-resolver.ts`. Given a
//! node's resolved `role` (from [`crate::role_infer`]) this fills in the
//! role's default visual/layout properties — but ONLY fields the model left
//! unset (AI-explicit values always win), exactly like the TS
//! `record[key] === undefined` check.
//!
//! Implementation note: defaults are applied by a JSON round-trip merge
//! (serialize the node → insert absent keys → deserialize). jian's PenNode is
//! `rename_all = "camelCase"` with `#[serde(flatten)]` container/text props, so
//! camelCase keys map straight onto the canonical schema, and "insert if
//! absent" reproduces the TS semantics without hand-mapping every typed field.

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Light,
    Dark,
}

/// Context threaded down the tree walk (port of `RoleContext`).
#[derive(Clone)]
pub struct RoleCtx {
    pub parent_role: Option<String>,
    pub parent_layout: Option<String>,
    pub canvas_width: f64,
    pub theme: Theme,
}

impl RoleCtx {
    /// Entry context for a forest of section roots.
    pub fn root(canvas_width: f64, theme: Theme) -> Self {
        RoleCtx {
            parent_role: None,
            parent_layout: None,
            canvas_width,
            theme,
        }
    }

    fn is_mobile(&self) -> bool {
        self.canvas_width <= 480.0
    }
}

// ── theme-aware style fragments (port of the helpers atop index.ts) ─────────

fn solid(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

fn card_fill(theme: Theme) -> Value {
    solid(if theme == Theme::Dark {
        "#1A1A1A"
    } else {
        "#FFFFFF"
    })
}

fn card_shadow() -> Value {
    json!([
        { "type": "shadow", "offsetX": 0, "offsetY": 1, "blur": 3, "spread": 0, "color": "#0000001A" },
        { "type": "shadow", "offsetX": 0, "offsetY": 1, "blur": 2, "spread": -1, "color": "#0000000F" }
    ])
}

fn input_fill(theme: Theme) -> Value {
    solid(if theme == Theme::Dark {
        "#1A1A1A"
    } else {
        "#F8FAFC"
    })
}

fn input_stroke(theme: Theme) -> Value {
    json!({ "thickness": 1, "fill": solid(if theme == Theme::Dark { "#2A2A2A" } else { "#E2E8F0" }) })
}

fn navbar_fill(theme: Theme) -> Value {
    solid(if theme == Theme::Dark {
        "#111111"
    } else {
        "#FFFFFF"
    })
}

fn navbar_bottom_border(theme: Theme) -> Value {
    json!({ "thickness": [0, 0, 1, 0], "fill": solid(if theme == Theme::Dark { "#1F1F1F" } else { "#E2E8F0" }) })
}

fn divider_fill(theme: Theme) -> Value {
    solid(if theme == Theme::Dark {
        "#2A2A2A"
    } else {
        "#E2E8F0"
    })
}

/// `[vertical, horizontal]` padding tuple as a JSON array.
fn pad2(v: f64, h: f64) -> Value {
    json!([v, h])
}

// ── theme detection (port of detectThemeFromNode) ───────────────────────────

/// Detect light/dark from a node's first solid fill (luminance < 0.3 = dark).
/// Pass the PAGE ROOT — a dark page's card has no fill of its own.
pub fn detect_theme_from_fill(fill: Option<&str>) -> Theme {
    let Some(color) = fill else {
        return Theme::Light;
    };
    let c = color.trim();
    if c.starts_with('$') {
        return Theme::Light; // unresolved ref — can't tell
    }
    let Some((r, g, b)) = parse_hex_rgb(c) else {
        return Theme::Light;
    };
    // sRGB → WCAG relative luminance.
    let lin = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    let y = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
    if y < 0.3 {
        Theme::Dark
    } else {
        Theme::Light
    }
}

fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.strip_prefix('#')?;
    // Reject non-ASCII before byte-slicing below (a multi-byte char could make
    // `h[0..2]` land mid-codepoint and panic).
    if !h.is_ascii() {
        return None;
    }
    let h = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 | 8 => h.to_string(),
        _ => return None,
    };
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some((r, g, b))
}

/// CJK detection for typography roles (Han / Kana / Hangul ranges).
fn has_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c as u32,
            0x4E00..=0x9FFF   // CJK Unified Ideographs
            | 0x3400..=0x4DBF // CJK Extension A
            | 0x3040..=0x30FF // Hiragana + Katakana
            | 0xAC00..=0xD7AF // Hangul syllables
            | 0xF900..=0xFAFF // CJK Compatibility Ideographs
        )
    })
}

// ── the 43 role rules (port of registerRole(...) in index.ts) ───────────────

fn node_number(node_json: &Value, key: &str) -> Option<f64> {
    node_json.get(key).and_then(Value::as_f64)
}

fn node_layout(node_json: &Value) -> Option<&str> {
    node_json.get("layout").and_then(Value::as_str)
}

fn node_text(node_json: &Value) -> String {
    node_json
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// Return the role's default property object, or `None` for an unknown role.
fn role_defaults(role: &str, node_json: &Value, ctx: &RoleCtx) -> Option<Value> {
    let mobile = ctx.is_mobile();
    let parent_layout = ctx.parent_layout.as_deref();
    let parent_role = ctx.parent_role.as_deref();
    let theme = ctx.theme;

    let v = match role {
        // ── Layout ──
        "section" => json!({
            "layout": "vertical", "width": "fill_container", "height": "fit_content",
            "gap": 24, "padding": if mobile { pad2(40.0, 16.0) } else { pad2(60.0, 80.0) },
            "alignItems": "center"
        }),
        "row" => {
            json!({ "layout": "horizontal", "width": "fill_container", "gap": 16, "alignItems": "center" })
        }
        "column" => json!({ "layout": "vertical", "width": "fill_container", "gap": 16 }),
        "centered-content" => json!({
            "layout": "vertical", "width": if mobile { json!("fill_container") } else { json!(1080) },
            "gap": 24, "alignItems": "center"
        }),
        "form-group" => json!({ "layout": "vertical", "width": "fill_container", "gap": 16 }),
        "spacer" => json!({ "width": "fill_container", "height": 40 }),
        "divider" => {
            let vertical = node_json
                .get("name")
                .and_then(Value::as_str)
                .map(|n| n.to_lowercase().contains("vertical"))
                .unwrap_or(false);
            if vertical {
                json!({ "width": 1, "height": "fill_container", "layout": "none", "fill": divider_fill(theme) })
            } else {
                json!({ "width": "fill_container", "height": 1, "layout": "none", "fill": divider_fill(theme) })
            }
        }

        // ── Navigation ──
        "navbar" => json!({
            "layout": "horizontal", "width": "fill_container",
            "height": if mobile { 56 } else { 72 },
            "padding": if mobile { pad2(0.0, 16.0) } else { pad2(0.0, 80.0) },
            "alignItems": "center", "justifyContent": "space_between",
            "fill": navbar_fill(theme), "stroke": navbar_bottom_border(theme)
        }),
        "nav-links" => json!({ "layout": "horizontal", "gap": 24, "alignItems": "center" }),
        "nav-link" => json!({ "textGrowth": "auto", "lineHeight": 1.2 }),

        // ── Interactive ──
        "button" => button_defaults(node_json, parent_role),
        "icon-button" => json!({
            "width": 44, "height": 44, "layout": "horizontal",
            "justifyContent": "center", "alignItems": "center", "cornerRadius": 8
        }),
        "badge" => json!({
            "layout": "horizontal", "padding": json!([6, 12]), "gap": 4,
            "alignItems": "center", "justifyContent": "center", "cornerRadius": 999,
            "fill": solid("#DBEAFE")
        }),
        "tag" => json!({
            "layout": "horizontal", "padding": json!([4, 10]), "gap": 4,
            "alignItems": "center", "justifyContent": "center", "cornerRadius": 6
        }),
        "pill" => json!({
            "layout": "horizontal", "padding": json!([6, 14]), "gap": 6,
            "alignItems": "center", "justifyContent": "center", "cornerRadius": 999
        }),
        "input" => {
            let mut m = json!({
                "height": 48, "layout": "horizontal", "padding": json!([12, 16]),
                "alignItems": "center", "cornerRadius": 8,
                "fill": input_fill(theme), "stroke": input_stroke(theme)
            });
            if parent_layout == Some("vertical") {
                m["width"] = json!("fill_container");
            }
            m
        }
        "form-input" => json!({
            "width": "fill_container", "height": 48, "layout": "horizontal",
            "padding": json!([12, 16]), "alignItems": "center", "cornerRadius": 8,
            "fill": input_fill(theme), "stroke": input_stroke(theme)
        }),
        "search-bar" => {
            if matches!(
                parent_role,
                Some("bottom-tab-bar") | Some("tab-bar") | Some("tab-row")
            ) {
                json!({})
            } else {
                json!({
                    "layout": "horizontal", "height": 44, "padding": json!([10, 16]), "gap": 8,
                    "alignItems": "center", "cornerRadius": 22,
                    "fill": input_fill(theme), "stroke": input_stroke(theme)
                })
            }
        }

        // ── Display / cards ──
        "card" => card_like(
            theme,
            parent_layout,
            json!({ "gap": 12, "cornerRadius": 12, "clipContent": true }),
        ),
        "stat-card" => card_like(
            theme,
            parent_layout,
            json!({ "gap": 8, "padding": json!([24, 24]), "cornerRadius": 12 }),
        ),
        "pricing-card" => card_like(
            theme,
            parent_layout,
            json!({ "gap": 16, "padding": json!([32, 24]), "cornerRadius": 16, "clipContent": true }),
        ),
        "image-card" => json!({
            "layout": "vertical", "gap": 0, "cornerRadius": 12, "clipContent": true, "effects": card_shadow()
        }),
        "feature-card" => card_like(
            theme,
            parent_layout,
            json!({ "gap": 12, "padding": json!([24, 24]), "cornerRadius": 12 }),
        ),
        "testimonial" => json!({
            "layout": "vertical", "gap": 16, "padding": json!([24, 24]), "cornerRadius": 12,
            "fill": card_fill(theme), "effects": card_shadow()
        }),

        // ── Content ──
        "hero" => json!({
            "layout": "vertical", "width": "fill_container", "height": "fit_content",
            "padding": if mobile { pad2(40.0, 16.0) } else { pad2(80.0, 80.0) },
            "gap": 24, "alignItems": "center"
        }),
        "feature-grid" => {
            json!({ "layout": "horizontal", "width": "fill_container", "gap": 24, "alignItems": "start" })
        }
        "cta-section" => json!({
            "layout": "vertical", "width": "fill_container", "height": "fit_content",
            "padding": if mobile { pad2(40.0, 16.0) } else { pad2(60.0, 80.0) },
            "gap": 20, "alignItems": "center"
        }),
        "footer" => json!({
            "layout": "vertical", "width": "fill_container", "height": "fit_content",
            "padding": if mobile { pad2(32.0, 16.0) } else { pad2(48.0, 80.0) }, "gap": 24
        }),
        "stats-section" => json!({
            "layout": "horizontal", "width": "fill_container", "height": "fit_content",
            "padding": if mobile { pad2(32.0, 16.0) } else { pad2(48.0, 80.0) },
            "gap": 32, "justifyContent": "center", "alignItems": "center"
        }),

        // ── Media ──
        "phone-mockup" => {
            json!({ "width": 280, "height": 560, "cornerRadius": 32, "layout": "none" })
        }
        "screenshot-frame" => json!({ "cornerRadius": 12, "clipContent": true }),
        "avatar" => {
            let size = node_number(node_json, "width")
                .filter(|w| *w > 0.0)
                .unwrap_or(48.0);
            json!({ "width": size, "height": size, "cornerRadius": (size / 2.0).round(), "clipContent": true })
        }
        "icon" => {
            if matches!(node_json.get("type").and_then(Value::as_str), Some("frame")) {
                json!({ "width": 24, "height": 24, "layout": "horizontal", "alignItems": "center", "justifyContent": "center" })
            } else {
                json!({ "width": 24, "height": 24 })
            }
        }

        // ── Typography (CJK-aware) ──
        "heading" => {
            let cjk = has_cjk(&node_text(node_json));
            let mut m = json!({
                "lineHeight": if cjk { 1.35 } else { 1.2 },
                "letterSpacing": if cjk { 0.0 } else { -0.5 },
                "textGrowth": if parent_layout == Some("vertical") { "fixed-width" } else { "auto" }
            });
            if parent_layout == Some("vertical") {
                m["width"] = json!("fill_container");
            }
            m
        }
        "subheading" => {
            let cjk = has_cjk(&node_text(node_json));
            json!({ "lineHeight": if cjk { 1.4 } else { 1.3 }, "textGrowth": "fixed-width", "width": "fill_container" })
        }
        "body-text" => {
            let cjk = has_cjk(&node_text(node_json));
            json!({ "lineHeight": if cjk { 1.6 } else { 1.5 }, "textGrowth": "fixed-width", "width": "fill_container" })
        }
        "caption" => {
            let cjk = has_cjk(&node_text(node_json));
            json!({ "lineHeight": if cjk { 1.4 } else { 1.3 }, "textGrowth": "auto" })
        }
        "label" => {
            json!({ "lineHeight": 1.2, "textGrowth": "auto", "textAlignVertical": "middle" })
        }

        // ── Table ──
        "table" => {
            json!({ "layout": "vertical", "width": "fill_container", "gap": 0, "clipContent": true })
        }
        "table-row" => {
            json!({ "layout": "horizontal", "width": "fill_container", "alignItems": "center", "padding": json!([12, 16]) })
        }
        "table-header" => json!({
            "layout": "horizontal", "width": "fill_container", "alignItems": "center",
            "padding": json!([12, 16]), "fill": input_fill(theme)
        }),
        "table-cell" => json!({ "width": "fill_container" }),

        _ => return None,
    };
    Some(v)
}

/// Card-family default builder. `parent_layout == "horizontal"` adds
/// fill/fill sizing so cards in a row stretch evenly. Port of the shared
/// branch in card/stat-card/pricing-card/feature-card.
fn card_like(theme: Theme, parent_layout: Option<&str>, extra: Value) -> Value {
    let mut m = json!({
        "layout": "vertical", "fill": card_fill(theme), "effects": card_shadow()
    });
    if parent_layout == Some("horizontal") {
        m["width"] = json!("fill_container");
        m["height"] = json!("fill_container");
    }
    if let (Some(obj), Some(ex)) = (m.as_object_mut(), extra.as_object()) {
        for (k, val) in ex {
            obj.insert(k.clone(), val.clone());
        }
    }
    m
}

/// `button` role — branches by parent role and the node's own size. Port of
/// the `registerRole('button', ...)` body.
fn button_defaults(node_json: &Value, parent_role: Option<&str>) -> Value {
    if parent_role == Some("navbar") {
        return json!({
            "padding": json!([8, 16]), "height": 36, "layout": "horizontal", "gap": 8,
            "alignItems": "center", "justifyContent": "center", "cornerRadius": 8
        });
    }
    if parent_role == Some("form-group") {
        return json!({
            "width": "fill_container", "height": 48, "layout": "horizontal", "gap": 8,
            "padding": json!([12, 24]), "alignItems": "center", "justifyContent": "center", "cornerRadius": 10
        });
    }
    // Bottom-tab cell: tight padding, no fill/cornerRadius.
    if matches!(
        parent_role,
        Some("bottom-tab-bar") | Some("tab-bar") | Some("tab-row")
    ) && node_layout(node_json) == Some("vertical")
    {
        return json!({
            "gap": 4, "padding": json!([6, 4]), "alignItems": "center", "justifyContent": "center"
        });
    }
    // Avatar / icon-button shape: small square frame.
    let w = node_number(node_json, "width");
    let h = node_number(node_json, "height");
    if let (Some(w), Some(h)) = (w, h) {
        if w <= 60.0 && h <= 60.0 && w < 48.0 {
            return json!({
                "layout": "horizontal", "gap": 8, "alignItems": "center", "justifyContent": "center"
            });
        }
    }
    json!({
        "padding": json!([12, 24]), "height": 44, "layout": "horizontal", "gap": 8,
        "alignItems": "center", "justifyContent": "center", "cornerRadius": 8
    })
}

/// Apply a role's defaults to `node`, setting only keys the node doesn't
/// already have (AI-explicit wins). JSON round-trip; on any (de)serialize
/// failure the node is left untouched.
pub fn apply_role_defaults(node: &mut PenNode, role: &str, ctx: &RoleCtx) {
    let Ok(mut v) = serde_json::to_value(&*node) else {
        return;
    };
    let Some(defaults) = role_defaults(role, &v, ctx) else {
        return;
    };
    let (Some(obj), Some(def_obj)) = (v.as_object_mut(), defaults.as_object()) else {
        return;
    };
    let mut changed = false;
    for (k, val) in def_obj {
        let absent = obj.get(k).map(Value::is_null).unwrap_or(true);
        if absent {
            obj.insert(k.clone(), val.clone());
            changed = true;
        }
    }
    if changed {
        if let Ok(new_node) = serde_json::from_value::<PenNode>(v) {
            *node = new_node;
        }
    }
}

/// The layout string a node declares (for the child context's `parentLayout`).
pub fn node_layout_string(node: &PenNode) -> Option<String> {
    let v = serde_json::to_value(node).ok()?;
    v.get("layout").and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
#[path = "role_defaults_tests.rs"]
mod tests;
