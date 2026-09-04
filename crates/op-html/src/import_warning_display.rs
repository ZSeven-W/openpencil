//! `Display` for [`ImportWarning`].
//!
//! These strings are user-visible in CLI JSON and MCP responses and are
//! asserted on by importer tests. They must stay byte-identical; localize
//! through [`ImportWarning::i18n_key`] instead of editing them here.

use super::import_warning::ImportWarning;
use std::fmt;

impl fmt::Display for ImportWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(text) = self.fixed_message() {
            return formatter.write_str(text);
        }
        match self {
            Self::DomDepthTruncated { max_depth } => {
                write!(
                    formatter,
                    "HTML nesting exceeded {max_depth} levels and deeper content was dropped"
                )
            }
            Self::AtRuleDepthLimit { max_depth } => {
                write!(
                    formatter,
                    "nested at-rule depth limit ({max_depth}) reached; inner rules ignored"
                )
            }
            Self::InvalidLayerName { name } => {
                write!(formatter, "invalid @layer name '{name}' ignored")
            }
            Self::UnsupportedAtStatement { name } => {
                write!(formatter, "unsupported @{name} statement ignored")
            }
            Self::InvalidLayerBlockName { name } => {
                write!(formatter, "invalid @layer block name '{name}' ignored")
            }
            Self::UnsupportedAtBlock { name } => {
                write!(formatter, "unsupported @{name} block ignored")
            }
            Self::WebFontNotDownloaded { family } => {
                write!(formatter, "web font '{family}' declared via @font-face is not downloaded; text falls back to available fonts")
            }
            Self::GridColumnUnsupported { value } => {
                write!(
                    formatter,
                    "CSS grid-column `{value}` is not supported and was auto-placed"
                )
            }
            Self::PropertyNotRepresentable { property } => {
                write!(
                    formatter,
                    "CSS {property} is not representable and was ignored"
                )
            }
            Self::ListStyleTypeUnsupported { value } => {
                write!(
                    formatter,
                    "unsupported CSS list-style-type `{value}` was approximated as a disc"
                )
            }
            Self::ElementPlaceholder { tag } => {
                write!(formatter, "<{tag}> imported as a placeholder")
            }
            Self::InvalidBaseHref { href } => {
                write!(formatter, "invalid <base href> ignored: {href}")
            }
            Self::BaseHrefOutsideOrigin { href } => {
                write!(
                    formatter,
                    "<base href> outside the HTML project origin ignored: {href}"
                )
            }
            Self::ExternalStylesheetSkipped { url } => {
                write!(formatter, "external stylesheet skipped: {url}")
            }
            Self::ImageOutsideOrigin { url } => {
                write!(
                    formatter,
                    "image resource outside the HTML project origin, using placeholder: {url}"
                )
            }
            Self::ImageUnavailable { url } => {
                write!(
                    formatter,
                    "image resource unavailable, using placeholder: {url}"
                )
            }
            Self::CssImportInvalid { prelude } => {
                write!(formatter, "invalid CSS @import ignored: {prelude}")
            }
            Self::CssImportUnresolvable { reference } => {
                write!(
                    formatter,
                    "CSS @import URL could not be resolved or left the project origin: {reference}"
                )
            }
            Self::CssImportCycle { url } => {
                write!(formatter, "CSS @import cycle ignored: {url}")
            }
            Self::CssImportDepthLimit { max_depth, url } => {
                write!(
                    formatter,
                    "CSS @import depth limit ({max_depth}) reached: {url}"
                )
            }
            Self::CssImportUnavailable { url } => {
                write!(formatter, "CSS @import unavailable: {url}")
            }
            Self::MultipleHtmlEntries { count, entry } => {
                write!(
                    formatter,
                    "multiple HTML entries found ({count}); selected {entry}"
                )
            }
            Self::SnapshotTaintedImages { count } => {
                write!(
                    formatter,
                    "{count} images kept as remote URLs (CORS-tainted)"
                )
            }
            Self::SnapshotRejected { reason } => {
                write!(formatter, "{reason}")
            }
            Self::MediaQuery(error) => error.fmt(formatter),
            _ => Ok(()),
        }
    }
}

impl ImportWarning {
    /// The message of every variant that carries no structured field.
    fn fixed_message(&self) -> Option<&'static str> {
        let text = match self {
            Self::EmptyInput => "no importable content: input HTML is empty",
            Self::EmptyBody => "no importable content: input HTML produced an empty body",
            Self::NodeLimitTruncated => "node limit reached (40000), remaining content dropped",
            Self::NodeLimitMapping => "node limit reached while mapping HTML",
            Self::NodeLimitInlineRow => "node limit reached while creating an inline formatting row",
            Self::NodeLimitPseudo => "generated pseudo-elements omitted because the node limit was reached",
            Self::UnterminatedRule => "unterminated CSS rule ignored",
            Self::MarkerRulesUnsupported => "CSS ::marker rules are not supported; list markers use the item's own text style",
            Self::NestingUnsupported => "CSS nesting is not supported; nested style rules ignored",
            Self::MediaWithoutViewport => "@media rules ignored because no viewport was provided",
            Self::UnsupportedContainerBlock => "unsupported @container block ignored",
            Self::PercentageAbsoluteOffsetInferred => "percentage absolute offsets resolved against the inferred containing block",
            Self::PercentageRelativeOffsetInferred => "percentage position:relative offsets resolved against the inferred containing block",
            Self::AspectRatioNoDefiniteAxis => "CSS aspect-ratio ignored: neither axis has a definite size",
            Self::AspectRatioIndefiniteContainer => "CSS aspect-ratio ignored: the containing block has no definite width to derive the height from",
            Self::PositionStickyIgnored => "CSS position:sticky has no static canvas equivalent and was ignored",
            Self::GridTracksApproximated => "unsupported CSS grid tracks approximated as a vertical frame",
            Self::FloatIgnored => "CSS float ignored during structured HTML import",
            Self::MixBlendModeNoNodeEquivalent => "CSS mix-blend-mode has no node-level equivalent; fill blending is used where possible",
            Self::OverflowScrollClipped => "CSS overflow: auto / scroll is imported as a clip, so content that would be scrollable is not reachable",
            Self::NegativeMarginsIgnored => "negative CSS margins are not representable and were ignored",
            Self::MarginsOnVisualBoxIgnored => "CSS margins on visual boxes cannot be represented without changing the box and were ignored",
            Self::InlineMarginWrappingApproximated => "a non-atomic inline with CSS margins was boxed and may no longer wrap across lines",
            Self::ContentBoxPercentageApproximated => "content-box percentage sizing cannot include padding exactly and was approximated",
            Self::GridEmptyCellsPacked => "CSS grid explicit start lines left empty cells; items were packed without spacers",
            Self::GridSpanReflowed => "CSS grid explicit start line could not host its span and the item was re-flowed",
            Self::GridRowsNodeLimit => "CSS grid row wrappers omitted because the node limit was reached",
            Self::GridTrackWidthsUnresolved => "CSS grid track widths could not be resolved (auto-fit / auto-fill), so spanning items keep their own width and under-fill the row",
            Self::GridTemplateAreasIgnored => "CSS grid-template-areas placement is not imported",
            Self::GridRowPlacementIgnored => "CSS grid-row placement is not imported; rows are chunked evenly",
            Self::BlockAutoMarginsIgnored => "CSS block-axis auto margins were not emulated during import",
            Self::AutoMarginNodeLimit => "CSS auto-margin alignment dropped because the node limit was reached",
            Self::FlowOffsetNoDefiniteSize => "CSS in-flow offset dropped: the element has no definite size to reserve in flow",
            Self::FlowOffsetNodeLimit => "CSS in-flow offset dropped because the node limit was reached",
            Self::FlowOffsetApproximated => "CSS in-flow offsets (position:relative insets, transform translation) are approximated: the node shifts inside a fixed-size wrapper so surrounding flow keeps the un-shifted box",
            Self::FlowOffsetNoWrapper => "CSS in-flow offset (position:relative or a transform translation) was dropped because this box cannot host an offset wrapper",
            Self::FlexWrapColumnNotEmulated => "flex-wrap on a column flex container was not emulated; children stay in one column (per-child auto margins under this container stay unemulated)",
            Self::FlexWrapReversePlain => "flex-wrap:wrap-reverse imported as plain wrap; row order is unchanged",
            Self::FlexWrapIndefiniteWidth => "flex-wrap ignored: this container has no definite width, so rows cannot be measured (per-child auto margins under this container stay unemulated)",
            Self::FlexAlignContentIgnored => "CSS align-content on a wrapping flex container was not imported",
            Self::FlexWrapIndeterminateChildren => "flex-wrap ignored: child main-axis sizes are indeterminate, so rows cannot be measured (per-child auto margins under this container stay unemulated)",
            Self::FlexWrapNodeLimit => "flex-wrap rows omitted because the node limit was reached (per-child auto margins under this container stay unemulated)",
            Self::TransformUnsupportedSyntax => "unsupported CSS transform syntax ignored",
            Self::TransformUnsupportedFunction => "unsupported CSS transform function ignored (3D and matrix3d are dropped)",
            Self::TransformPercentageTranslationDropped => "CSS percentage transform translation dropped on an axis with no definite size",
            Self::TransformNonFiniteMatrix => "CSS transform produced a non-finite matrix and was ignored",
            Self::TransformSkewDropped => "CSS transform skew has no node equivalent and was dropped",
            Self::TransformDegenerateScale => "CSS transform collapses the element to a zero or non-finite scale; it was imported at full size",
            Self::TransformMirroringAbsolute => "CSS transform mirroring was imported as its absolute scale",
            Self::TransformOriginZIgnored => "CSS transform-origin Z offset ignored during 2D import",
            Self::TransformScaleNotBaked => "CSS transform scale could not be baked into this node's size",
            Self::TransformScaleBaked => "CSS transform scale baked into node size; font sizes and descendant sizes are unchanged",
            Self::TransformScaleAutoSizeIgnored => "CSS transform scale ignored on an auto-sized element",
            Self::BackgroundRepeatApproximated => "directional or spaced CSS background repeat approximated as tile",
            Self::BackgroundTileSizeIgnored => "explicit CSS background tile size is not representable and was ignored",
            Self::BackgroundSizeAutoBox => "CSS background-size on an element with an automatic width or height needs a used box this importer does not resolve and was approximated as a cover fill",
            Self::BackgroundSizeNeedsIntrinsicSize => "CSS background-size with an automatic or unresolvable axis needs the image's intrinsic size and was approximated as a cover fill",
            Self::BackgroundPositionUnsupported => "unsupported CSS background-position was ignored",
            Self::BackgroundImageUrlEmpty => "empty CSS background image URL was ignored",
            Self::ConicGradientIgnored => "conic gradients are not representable and were ignored",
            Self::BackgroundImageLayerUnsupported => "unsupported CSS background-image layer was ignored",
            Self::BackgroundColorUnresolved => "unresolved CSS background color was ignored",
            Self::BackgroundPositionDropped => "CSS background-position is not representable and was ignored",
            Self::BorderColorsApproximated => "per-side border colors approximated using the first opaque color",
            Self::BorderStylesApproximated => "mixed per-side border styles approximated using the first style",
            Self::BorderStyleComplex => "complex CSS border style approximated as a solid stroke",
            Self::BorderStyleUnsupported => "unsupported CSS border style approximated as a solid stroke",
            Self::BorderRadiusElliptical => "elliptical CSS border radii approximated using horizontal radii",
            Self::BorderRadiusUnsupported => "unsupported CSS border radius was ignored",
            Self::BoxShadowLayerUnsupported => "unsupported CSS box-shadow layer was ignored",
            Self::GradientInterpolationIgnored => "CSS gradient color interpolation method was ignored",
            Self::LinearGradientDirectionUnsupported => "unsupported CSS linear-gradient direction was ignored",
            Self::GradientColorHintsIgnored => "CSS gradient color hints are not representable and were ignored",
            Self::GradientColorStopUnsupported => "unsupported CSS gradient color stop was ignored",
            Self::GradientTooFewStops => "CSS gradient with fewer than two usable stops was ignored",
            Self::GradientRepeatingApproximated => "repeating CSS gradient approximated using one gradient cycle",
            Self::GradientStopsClamped => "out-of-range CSS gradient stops were clamped to the fill bounds",
            Self::BlurRadiusUnsupported => "unsupported CSS blur radius was ignored",
            Self::FilterDropShadowUnsupported => "unsupported CSS filter drop-shadow was ignored",
            Self::FilterFunctionUnsupported => "unsupported CSS filter function was ignored",
            Self::BackdropFilterUnsupported => "unsupported CSS backdrop-filter function was ignored",
            Self::BackgroundBlendModeUnsupported => "unsupported CSS background-blend-mode was ignored",
            Self::MixBlendModeOnFills => "CSS mix-blend-mode approximated on individual fills",
            Self::MixBlendModeUnsupported => "unsupported CSS mix-blend-mode was ignored",
            Self::GradientBackgroundSizeIgnored => "CSS background-size on gradients is not representable and was ignored",
            Self::RadialGradientPositionUnsupported => "unsupported CSS radial-gradient position was ignored",
            Self::RadialGradientElliptical => "elliptical CSS radial-gradient approximated as circular",
            Self::RadialGradientExtentApproximated => "CSS radial-gradient extent keyword was approximated",
            Self::RadialGradientSizeUnsupported => "unsupported CSS radial-gradient size was ignored",
            Self::TextShadowLayerUnsupported => "unsupported CSS text-shadow layer was ignored",
            Self::TextShadowExtraLayersIgnored => "CSS text-shadow layers after the first were ignored",
            Self::TextShadowOnInlineIgnored => "CSS text-shadow on an inline element was ignored; the shadow is a property of the whole text node",
            Self::ListStyleImageIgnored => "CSS list-style-image is not imported; the item's text marker is used instead",
            Self::ListMarkerPositionOutsideApproximated => "CSS list markers are inlined before the item text, so a `list-style-position: outside` hanging marker is approximated",
            Self::ObjectFitScaleDown => "CSS object-fit:scale-down approximated as contain",
            Self::ObjectFitNoneIgnored => "CSS object-fit:none has no image-node equivalent and was ignored",
            Self::ObjectPositionIgnored => "CSS object-position has no image-node equivalent; the image remains centered",
            Self::ImageMixBlendModeUnsupported => "unsupported CSS mix-blend-mode was ignored on an image",
            Self::ImageIntrinsicAxisUnresolved => "image intrinsic aspect ratio could not resolve the missing axis because the authored size is dynamic or its containing block is indefinite",
            Self::InlineSvgPlaceholder => "inline <svg> imported as image placeholder",
            Self::InputTypeFallback => "unsupported input type imported as a text input",
            Self::PictureUndecodableTypes => "<picture> only offered source types this importer cannot decode; the first matching source was imported anyway",
            Self::TableRowspanIgnored => "HTML rowspan is not imported; the cell occupies a single row",
            Self::TableRowGroupsUnflattened => "CSS un-flattened a table's row groups, so its cells no longer share one column grid and each row's widths are approximated",
            Self::TableIndefiniteWidthApproximated => "a CSS table without a definite width was measured against its containing block, so its column widths are approximate",
            Self::SnapshotTruncated => "browser snapshot was truncated during extraction",
            Self::SnapshotNodeLimit => "node limit reached (40000), remaining snapshot content dropped",
            Self::SnapshotInvalidRect => "snapshot node with missing or invalid rect was skipped",
            Self::SnapshotUnknownKind => "snapshot node with unknown kind was skipped",
            Self::SnapshotUnsupportedTransform => "unsupported snapshot transform ignored (only matrix rotation imported)",
            _ => return None,
        };
        Some(text)
    }
}
