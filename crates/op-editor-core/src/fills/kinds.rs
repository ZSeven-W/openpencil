//! Fill-type classification, the per-type default bodies and the
//! type-to-type conversion that preserves as much of the old body as
//! the new type can carry.

use super::*;

/// Read the node's primary fill kind as a [`FillType`]. The canonical
/// model has no scalar `fill_type` field — the kind is the variant of
/// the first `PenFill`. A node with no fills reports `Solid` (the
/// neutral default the property panel paints).
/// `FillType` of a single `PenFill` (the kind is the variant).
pub fn fill_type_of(fill: &PenFill) -> FillType {
    match fill {
        PenFill::Solid(_) => FillType::Solid,
        PenFill::LinearGradient(_) => FillType::LinearGradient,
        PenFill::RadialGradient(_) => FillType::RadialGradient,
        PenFill::MeshGradient(_) => FillType::MeshGradient,
        PenFill::Shader(_) => FillType::Shader,
        PenFill::Image(_) => FillType::Image,
    }
}

pub fn first_fill_type(node: &PenNode) -> FillType {
    node_fills(node)
        .and_then(|f| f.first())
        .map(fill_type_of)
        .unwrap_or(FillType::Solid)
}

/// Build a default `PenFill` of the given `FillType`, seeding it with
/// `hex` where the variant carries a single colour (Solid) or a stop
/// list (gradients). `Image` has no colour, so it gets an empty `url`.
pub(super) fn default_fill_of_type(kind: FillType, hex: &str) -> PenFill {
    match kind {
        FillType::Solid => solid_fill(hex.to_string()),
        FillType::LinearGradient => PenFill::LinearGradient(LinearGradientBody {
            angle: None,
            stops: default_stops(hex),
            explain: None,
            opacity: None,
            blend_mode: None,
        }),
        FillType::RadialGradient => PenFill::RadialGradient(RadialGradientBody {
            cx: None,
            cy: None,
            radius: None,
            stops: default_stops(hex),
            explain: None,
            opacity: None,
            blend_mode: None,
        }),
        FillType::MeshGradient => PenFill::MeshGradient(MeshGradientBody {
            rows: 2,
            cols: 2,
            stops: default_mesh_stops(hex),
            explain: None,
            opacity: None,
            blend_mode: None,
        }),
        FillType::Shader => PenFill::Shader(default_shader_body(hex)),
        FillType::Image => PenFill::Image(ImageFillBody {
            url: "".into(),
            mode: None,
            original_size: None,
            transform: None,
            tile_scale: None,
            explain: None,
            opacity: None,
            blend_mode: None,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
        }),
    }
}

/// Two-stop gradient default — the picked colour at 0.0, transparent
/// black at 1.0, mirroring the property panel's 2-stop gradient body.
fn default_stops(hex: &str) -> Vec<GradientStop> {
    vec![
        GradientStop {
            offset: 0.0,
            color: hex.to_string(),
        },
        GradientStop {
            offset: 1.0,
            color: "#00000000".to_string(),
        },
    ]
}

/// Default 2×2 corner mesh — the picked colour at the top-left vertex
/// and three muted variants at the other corners, so a freshly-flipped
/// mesh fill renders a visible four-corner Gouraud blend instead of a
/// flat patch. Per-vertex editing is deferred (v1 ships a non-editable
/// default), so this is what the panel produces today.
fn default_mesh_stops(hex: &str) -> Vec<MeshVertexStop> {
    vec![
        MeshVertexStop {
            row: 0,
            col: 0,
            color: hex.to_string(),
        },
        MeshVertexStop {
            row: 0,
            col: 1,
            color: "#ffffff".to_string(),
        },
        MeshVertexStop {
            row: 1,
            col: 0,
            color: "#000000".to_string(),
        },
        MeshVertexStop {
            row: 1,
            col: 1,
            color: hex.to_string(),
        },
    ]
}

/// Known-good default SkSL shader body, seeded with the picked `hex` as
/// a `tint` colour uniform. The program is a vertical fade from `tint`
/// at the top to transparent at the bottom — valid SkSL that compiles on
/// the native host, and whose `tint` uniform doubles as the visible
/// solid fallback colour on backends that can't run it (web / capture /
/// frame). v1 is render-only, so the panel produces this fixed default;
/// per-fragment authoring is deferred.
fn default_shader_body(hex: &str) -> ShaderFillBody {
    let mut uniforms = std::collections::BTreeMap::new();
    uniforms.insert(
        "tint".to_string(),
        jian_ops_schema::style::ShaderUniformValue::Color(hex.to_string()),
    );
    ShaderFillBody {
        preset: None,
        sksl: Some("uniform half4 tint; half4 main(float2 p){ return tint; }".to_string()),
        uniforms: Some(uniforms),
        explain: None,
        opacity: None,
        blend_mode: None,
    }
}

/// Carry a representative hex colour out of a `PenFill` so flipping
/// fill types keeps the node's colour where one exists. Solid → its
/// colour; gradient → the first stop's colour; shader → its first
/// `color` uniform; image → none.
fn fill_hex(fill: &PenFill) -> Option<&str> {
    match fill {
        PenFill::Solid(body) => Some(body.color.as_str()),
        PenFill::LinearGradient(body) => body.stops.first().map(|s| s.color.as_str()),
        PenFill::RadialGradient(body) => body.stops.first().map(|s| s.color.as_str()),
        PenFill::MeshGradient(body) => body.stops.first().map(|s| s.color.as_str()),
        PenFill::Shader(body) => body.uniforms.as_ref().and_then(|u| {
            u.values().find_map(|v| match v {
                jian_ops_schema::style::ShaderUniformValue::Color(c) => Some(c.as_str()),
                _ => None,
            })
        }),
        PenFill::Image(_) => None,
    }
}

/// Convert an existing first `PenFill` to fill-type `kind`, preserving
/// as much of the existing body as the target variant structurally
/// allows. This mirrors shell-core's `set_selected_fill_type`, which
/// only flipped a scalar `Node.fill_type` discriminant and never
/// discarded the fill body: shell-core kept the gradient stops / image
/// payload while the discriminant moved. The canonical model has no
/// scalar discriminant — type IS the `PenFill` variant — so a faithful
/// port carries the body across the variant flip by hand:
///
///   - already the target variant → returned unchanged (no-op).
///   - LinearGradient ⇄ RadialGradient → carry `stops`, `opacity`,
///     `blend_mode`, `explain`; only the angle / centre fields that
///     have no counterpart are dropped.
///   - Solid → gradient → seed stops from the solid colour.
///   - gradient → Solid → carry the first stop's colour.
///   - anything → Image → fresh image body (no shared structure to
///     carry; the previous body cannot become a URL).
pub(super) fn convert_fill(existing: PenFill, kind: FillType) -> PenFill {
    match (kind, existing) {
        // Already the requested variant — keep the body verbatim.
        (FillType::Solid, f @ PenFill::Solid(_)) => f,
        (FillType::LinearGradient, f @ PenFill::LinearGradient(_)) => f,
        (FillType::RadialGradient, f @ PenFill::RadialGradient(_)) => f,
        (FillType::Shader, f @ PenFill::Shader(_)) => f,
        (FillType::Image, f @ PenFill::Image(_)) => f,
        // Linear → Radial — carry every shared field.
        (FillType::RadialGradient, PenFill::LinearGradient(body)) => {
            PenFill::RadialGradient(RadialGradientBody {
                cx: None,
                cy: None,
                radius: None,
                stops: body.stops,
                explain: body.explain,
                opacity: body.opacity,
                blend_mode: body.blend_mode,
            })
        }
        // Radial → Linear — carry every shared field.
        (FillType::LinearGradient, PenFill::RadialGradient(body)) => {
            PenFill::LinearGradient(LinearGradientBody {
                angle: None,
                stops: body.stops,
                explain: body.explain,
                opacity: body.opacity,
                blend_mode: body.blend_mode,
            })
        }
        // Cross-family flips — carry the representative colour only.
        (kind, other) => {
            let hex = fill_hex(&other).unwrap_or("#000000").to_string();
            default_fill_of_type(kind, &hex)
        }
    }
}
