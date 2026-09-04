//! One sample value per [`ImportWarning`] variant, for catalog tests.
//!
//! Generated alongside the enum: the exhaustive `match` in
//! `ImportWarning::ident` fails to compile when a variant is added, and
//! this list is what proves the new variant also reaches the tests.

use super::import_warning::ImportWarning;

/// Every variant that carries structured fields.
pub(crate) fn sample_field_variants() -> Vec<ImportWarning> {
    vec![
        ImportWarning::DomDepthTruncated { max_depth: 1 },
        ImportWarning::AtRuleDepthLimit { max_depth: 1 },
        ImportWarning::InvalidLayerName {
            name: "x".to_string(),
        },
        ImportWarning::UnsupportedAtStatement {
            name: "x".to_string(),
        },
        ImportWarning::InvalidLayerBlockName {
            name: "x".to_string(),
        },
        ImportWarning::UnsupportedAtBlock {
            name: "x".to_string(),
        },
        ImportWarning::WebFontNotDownloaded {
            family: "x".to_string(),
        },
        ImportWarning::GridColumnUnsupported {
            value: "x".to_string(),
        },
        ImportWarning::PropertyNotRepresentable {
            property: "x".to_string(),
        },
        ImportWarning::ListStyleTypeUnsupported {
            value: "x".to_string(),
        },
        ImportWarning::ElementPlaceholder {
            tag: "x".to_string(),
        },
        ImportWarning::InvalidBaseHref {
            href: "x".to_string(),
        },
        ImportWarning::BaseHrefOutsideOrigin {
            href: "x".to_string(),
        },
        ImportWarning::ExternalStylesheetSkipped {
            url: "x".to_string(),
        },
        ImportWarning::ImageOutsideOrigin {
            url: "x".to_string(),
        },
        ImportWarning::ImageUnavailable {
            url: "x".to_string(),
        },
        ImportWarning::CssImportInvalid {
            prelude: "x".to_string(),
        },
        ImportWarning::CssImportUnresolvable {
            reference: "x".to_string(),
        },
        ImportWarning::CssImportCycle {
            url: "x".to_string(),
        },
        ImportWarning::CssImportDepthLimit {
            max_depth: 1,
            url: "x".to_string(),
        },
        ImportWarning::CssImportUnavailable {
            url: "x".to_string(),
        },
        ImportWarning::MultipleHtmlEntries {
            count: 1,
            entry: "x".to_string(),
        },
        ImportWarning::SnapshotTaintedImages { count: 1 },
        ImportWarning::SnapshotRejected {
            reason: "x".to_string(),
        },
    ]
}

/// Every variant with no structured field.
pub(crate) fn sample_fixed_variants() -> Vec<ImportWarning> {
    vec![
        ImportWarning::EmptyInput,
        ImportWarning::EmptyBody,
        ImportWarning::NodeLimitTruncated,
        ImportWarning::NodeLimitMapping,
        ImportWarning::NodeLimitInlineRow,
        ImportWarning::NodeLimitPseudo,
        ImportWarning::UnterminatedRule,
        ImportWarning::MarkerRulesUnsupported,
        ImportWarning::NestingUnsupported,
        ImportWarning::MediaWithoutViewport,
        ImportWarning::UnsupportedContainerBlock,
        ImportWarning::PercentageAbsoluteOffsetInferred,
        ImportWarning::PercentageRelativeOffsetInferred,
        ImportWarning::AspectRatioNoDefiniteAxis,
        ImportWarning::AspectRatioIndefiniteContainer,
        ImportWarning::PositionStickyIgnored,
        ImportWarning::GridTracksApproximated,
        ImportWarning::FloatIgnored,
        ImportWarning::MixBlendModeNoNodeEquivalent,
        ImportWarning::OverflowScrollClipped,
        ImportWarning::NegativeMarginsIgnored,
        ImportWarning::MarginsOnVisualBoxIgnored,
        ImportWarning::InlineMarginWrappingApproximated,
        ImportWarning::ContentBoxPercentageApproximated,
        ImportWarning::GridEmptyCellsPacked,
        ImportWarning::GridSpanReflowed,
        ImportWarning::GridRowsNodeLimit,
        ImportWarning::GridTrackWidthsUnresolved,
        ImportWarning::GridTemplateAreasIgnored,
        ImportWarning::GridRowPlacementIgnored,
        ImportWarning::BlockAutoMarginsIgnored,
        ImportWarning::AutoMarginNodeLimit,
        ImportWarning::FlowOffsetNoDefiniteSize,
        ImportWarning::FlowOffsetNodeLimit,
        ImportWarning::FlowOffsetApproximated,
        ImportWarning::FlowOffsetNoWrapper,
        ImportWarning::FlexWrapColumnNotEmulated,
        ImportWarning::FlexWrapReversePlain,
        ImportWarning::FlexWrapIndefiniteWidth,
        ImportWarning::FlexAlignContentIgnored,
        ImportWarning::FlexWrapIndeterminateChildren,
        ImportWarning::FlexWrapNodeLimit,
        ImportWarning::TransformUnsupportedSyntax,
        ImportWarning::TransformUnsupportedFunction,
        ImportWarning::TransformPercentageTranslationDropped,
        ImportWarning::TransformNonFiniteMatrix,
        ImportWarning::TransformSkewDropped,
        ImportWarning::TransformDegenerateScale,
        ImportWarning::TransformMirroringAbsolute,
        ImportWarning::TransformOriginZIgnored,
        ImportWarning::TransformScaleNotBaked,
        ImportWarning::TransformScaleBaked,
        ImportWarning::TransformScaleAutoSizeIgnored,
        ImportWarning::BackgroundRepeatApproximated,
        ImportWarning::BackgroundTileSizeIgnored,
        ImportWarning::BackgroundSizeAutoBox,
        ImportWarning::BackgroundSizeNeedsIntrinsicSize,
        ImportWarning::BackgroundPositionUnsupported,
        ImportWarning::BackgroundImageUrlEmpty,
        ImportWarning::ConicGradientIgnored,
        ImportWarning::BackgroundImageLayerUnsupported,
        ImportWarning::BackgroundColorUnresolved,
        ImportWarning::BackgroundPositionDropped,
        ImportWarning::BorderColorsApproximated,
        ImportWarning::BorderStylesApproximated,
        ImportWarning::BorderStyleComplex,
        ImportWarning::BorderStyleUnsupported,
        ImportWarning::BorderRadiusElliptical,
        ImportWarning::BorderRadiusUnsupported,
        ImportWarning::BoxShadowLayerUnsupported,
        ImportWarning::GradientInterpolationIgnored,
        ImportWarning::LinearGradientDirectionUnsupported,
        ImportWarning::GradientColorHintsIgnored,
        ImportWarning::GradientColorStopUnsupported,
        ImportWarning::GradientTooFewStops,
        ImportWarning::GradientRepeatingApproximated,
        ImportWarning::GradientStopsClamped,
        ImportWarning::BlurRadiusUnsupported,
        ImportWarning::FilterDropShadowUnsupported,
        ImportWarning::FilterFunctionUnsupported,
        ImportWarning::BackdropFilterUnsupported,
        ImportWarning::BackgroundBlendModeUnsupported,
        ImportWarning::MixBlendModeOnFills,
        ImportWarning::MixBlendModeUnsupported,
        ImportWarning::GradientBackgroundSizeIgnored,
        ImportWarning::RadialGradientPositionUnsupported,
        ImportWarning::RadialGradientElliptical,
        ImportWarning::RadialGradientExtentApproximated,
        ImportWarning::RadialGradientSizeUnsupported,
        ImportWarning::TextShadowLayerUnsupported,
        ImportWarning::TextShadowExtraLayersIgnored,
        ImportWarning::TextShadowOnInlineIgnored,
        ImportWarning::ListStyleImageIgnored,
        ImportWarning::ListMarkerPositionOutsideApproximated,
        ImportWarning::ObjectFitScaleDown,
        ImportWarning::ObjectFitNoneIgnored,
        ImportWarning::ObjectPositionIgnored,
        ImportWarning::ImageMixBlendModeUnsupported,
        ImportWarning::ImageIntrinsicAxisUnresolved,
        ImportWarning::InlineSvgPlaceholder,
        ImportWarning::InputTypeFallback,
        ImportWarning::PictureUndecodableTypes,
        ImportWarning::TableRowspanIgnored,
        ImportWarning::TableRowGroupsUnflattened,
        ImportWarning::TableIndefiniteWidthApproximated,
        ImportWarning::SnapshotTruncated,
        ImportWarning::SnapshotNodeLimit,
        ImportWarning::SnapshotInvalidRect,
        ImportWarning::SnapshotUnknownKind,
        ImportWarning::SnapshotUnsupportedTransform,
    ]
}
