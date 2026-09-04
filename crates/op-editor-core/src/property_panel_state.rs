//! Property-panel state enums shared by editor core and widget hosts.

/// Which PropertyPanel tab is active — toggled by `Cmd+Shift+C`.
/// Variant order is the tab strip's left-to-right paint order:
/// 设计 (Design) | 交互 (Interact) | 代码 (Code). `Interact` is gated
/// behind the same `agent_settings.experimental_features_enabled`
/// flag as the Widget section / Preview — see `PropertyPanel.show_interact`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyTab {
    Design,
    Interact,
    Code,
}

/// Variants the Fill section's type-selector pill exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillType {
    Solid,
    LinearGradient,
    RadialGradient,
    MeshGradient,
    /// Native SkSL shader fill (v1, render-only). Not offered in the
    /// fill-type picker — arrives via `.op` files or the batch_design
    /// fill passthrough — but carried so a node that already has a
    /// shader fill reports + paints correctly.
    Shader,
    Image,
}

/// Image-fill fit modes exposed by the TS image-fill popover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFillMode {
    Fill,
    Fit,
    Crop,
    Tile,
}

impl ImageFillMode {
    pub const ALL: [Self; 4] = [Self::Fill, Self::Fit, Self::Crop, Self::Tile];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Fill => "image.fill",
            Self::Fit => "image.fitMode",
            Self::Crop => "image.crop",
            Self::Tile => "image.tile",
        }
    }

    pub fn to_schema(self) -> jian_ops_schema::style::ImageFillMode {
        match self {
            Self::Fill => jian_ops_schema::style::ImageFillMode::Fill,
            Self::Fit => jian_ops_schema::style::ImageFillMode::Fit,
            Self::Crop => jian_ops_schema::style::ImageFillMode::Crop,
            Self::Tile => jian_ops_schema::style::ImageFillMode::Tile,
        }
    }

    pub fn from_schema(value: Option<&jian_ops_schema::style::ImageFillMode>) -> Self {
        match value {
            Some(jian_ops_schema::style::ImageFillMode::Fit) => Self::Fit,
            Some(jian_ops_schema::style::ImageFillMode::Crop) => Self::Crop,
            Some(jian_ops_schema::style::ImageFillMode::Tile) => Self::Tile,
            Some(jian_ops_schema::style::ImageFillMode::Fill)
            | Some(jian_ops_schema::style::ImageFillMode::Stretch)
            | None => Self::Fill,
        }
    }

    pub fn to_image_node_schema(self) -> jian_ops_schema::node::image::ImageFitMode {
        match self {
            Self::Fill => jian_ops_schema::node::image::ImageFitMode::Fill,
            Self::Fit => jian_ops_schema::node::image::ImageFitMode::Fit,
            Self::Crop => jian_ops_schema::node::image::ImageFitMode::Crop,
            Self::Tile => jian_ops_schema::node::image::ImageFitMode::Tile,
        }
    }

    pub fn from_image_node_schema(
        value: Option<&jian_ops_schema::node::image::ImageFitMode>,
    ) -> Self {
        match value {
            Some(jian_ops_schema::node::image::ImageFitMode::Fit) => Self::Fit,
            Some(jian_ops_schema::node::image::ImageFitMode::Crop) => Self::Crop,
            Some(jian_ops_schema::node::image::ImageFitMode::Tile) => Self::Tile,
            Some(jian_ops_schema::node::image::ImageFitMode::Fill) | None => Self::Fill,
        }
    }
}

/// Image-fill adjustment sliders. Values are clamped to `[-100, 100]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageAdjustmentField {
    Exposure,
    Contrast,
    Saturation,
    Temperature,
    Tint,
    Highlights,
    Shadows,
}

impl ImageAdjustmentField {
    pub const ALL: [Self; 7] = [
        Self::Exposure,
        Self::Contrast,
        Self::Saturation,
        Self::Temperature,
        Self::Tint,
        Self::Highlights,
        Self::Shadows,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Exposure => "image.exposure",
            Self::Contrast => "image.contrast",
            Self::Saturation => "image.saturation",
            Self::Temperature => "image.temperature",
            Self::Tint => "image.tint",
            Self::Highlights => "image.highlights",
            Self::Shadows => "image.shadows",
        }
    }
}

/// Three flex-layout modes the property panel exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexLayout {
    Free,
    Vertical,
    Horizontal,
}

/// How the padding section shows its inputs — TS `PaddingMode`.
/// `Single` = one value for all four sides; `Axis` = vertical +
/// horizontal; `Individual` = top / right / bottom / left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingEditMode {
    Single,
    Axis,
    Individual,
}

impl PaddingEditMode {
    pub const ALL: [Self; 3] = [Self::Single, Self::Axis, Self::Individual];

    /// i18n key for the gear-popover row label.
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Single => "padding.oneValue",
            Self::Axis => "padding.horizontalVertical",
            Self::Individual => "padding.topRightBottomLeft",
        }
    }

    /// Derive the mode from the four effective padding values — mirrors
    /// the TS `parsePaddingValues` (uniform first, then axis, else
    /// individual).
    pub fn from_values(t: f32, r: f32, b: f32, l: f32) -> Self {
        if t == r && r == b && b == l {
            Self::Single
        } else if t == b && r == l {
            Self::Axis
        } else {
            Self::Individual
        }
    }
}

/// Path boolean ops — TS parity with Paper.js (Ctrl+Alt+U/S/I/X).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

/// Raster export format. State enum ported from shell-core's
/// `widgets/export_dialog::ExportFormat` (the widget render code stays
/// in shell-core).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Jpeg,
    Webp,
    Svg,
    Pdf,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 5] = [
        ExportFormat::Png,
        ExportFormat::Jpeg,
        ExportFormat::Webp,
        ExportFormat::Svg,
        ExportFormat::Pdf,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Png => "PNG",
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Webp => "WEBP",
            ExportFormat::Svg => "SVG",
            ExportFormat::Pdf => "PDF",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Webp => "webp",
            ExportFormat::Svg => "svg",
            ExportFormat::Pdf => "pdf",
        }
    }

    /// Whether the target's shipping renderer contains this encoder.
    ///
    /// The pinned iOS/Android Skia archives omit WebP; desktop builds keep
    /// their existing encoder. PNG/JPEG/SVG/PDF are available everywhere.
    pub fn is_implemented(self) -> bool {
        !matches!(self, ExportFormat::Webp) || !cfg!(any(target_os = "ios", target_os = "android"))
    }
}
