//! Property-panel state enums shared by editor core and widget hosts.

/// Which PropertyPanel tab is active — toggled by `Cmd+Shift+C`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyTab {
    Design,
    Code,
}

/// Variants the Fill section's type-selector pill exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillType {
    Solid,
    LinearGradient,
    RadialGradient,
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

    /// Whether the format has a working export backend.
    pub fn is_implemented(self) -> bool {
        true
    }
}
