//! Typed, machine-codeable diagnostics for the HTML importer.
//!
//! Every degradation the importer reports is a variant here. `Display`
//! reproduces the historical warning text byte-for-byte, so CLI JSON and
//! MCP output keep their exact wording, while [`ImportWarning::code`]
//! gives machines a stable identifier and [`ImportWarning::i18n_key`]
//! points at the localized panel copy.
//!
//! The `Display` bodies live in `import_warning_display.rs`.

use crate::css::MediaQueryError;

/// One non-fatal degradation recorded while importing HTML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportWarning {
    // --- content: Whole-document content that never made it into the tree.
    /// `content.empty_input` — no importable content: input HTML is empty
    EmptyInput,
    /// `content.empty_body` — no importable content: input HTML produced an empty body
    EmptyBody,
    /// `content.dom_depth_truncated` — HTML nesting exceeded {max_depth} levels and deeper content was dropped
    DomDepthTruncated { max_depth: usize },
    /// `content.node_limit_truncated` — node limit reached (40000), remaining content dropped
    NodeLimitTruncated,
    /// `content.node_limit_mapping` — node limit reached while mapping HTML
    NodeLimitMapping,
    /// `content.node_limit_inline_row` — node limit reached while creating an inline formatting row
    NodeLimitInlineRow,
    /// `content.node_limit_pseudo` — generated pseudo-elements omitted because the node limit was reached
    NodeLimitPseudo,
    // --- css: CSS parsing and at-rule handling.
    /// `css.at_rule_depth_limit` — nested at-rule depth limit ({max_depth}) reached; inner rules
    ///   ignored
    AtRuleDepthLimit { max_depth: usize },
    /// `css.unterminated_rule` — unterminated CSS rule ignored
    UnterminatedRule,
    /// `css.marker_rules_unsupported` — CSS ::marker rules are not supported; list markers use the
    ///   item's own text style
    MarkerRulesUnsupported,
    /// `css.nesting_unsupported` — CSS nesting is not supported; nested style rules ignored
    NestingUnsupported,
    /// `css.invalid_layer_name` — invalid @layer name '{name}' ignored
    InvalidLayerName { name: String },
    /// `css.unsupported_statement` — unsupported @{name} statement ignored
    UnsupportedAtStatement { name: String },
    /// `css.media_without_viewport` — @media rules ignored because no viewport was provided
    MediaWithoutViewport,
    /// `css.invalid_layer_block_name` — invalid @layer block name '{name}' ignored
    InvalidLayerBlockName { name: String },
    /// `css.unsupported_container_block` — unsupported @container block ignored
    UnsupportedContainerBlock,
    /// `css.unsupported_block` — unsupported @{name} block ignored
    UnsupportedAtBlock { name: String },
    /// `css.media_*` — an `@media` query the evaluator could not read.
    MediaQuery(MediaQueryError),
    // --- font: `@font-face` web fonts.
    /// `font.web_font_not_downloaded` — web font '{family}' declared via @font-face is not
    ///   downloaded; text falls back to available fonts
    WebFontNotDownloaded { family: String },
    // --- layout: Box, flex, grid and offset layout degradations.
    /// `layout.percentage_absolute_offset_inferred` — percentage absolute offsets resolved against the inferred containing block
    PercentageAbsoluteOffsetInferred,
    /// `layout.percentage_relative_offset_inferred` — percentage position:relative offsets resolved against the inferred containing block
    PercentageRelativeOffsetInferred,
    /// `layout.aspect_ratio_no_definite_axis` — CSS aspect-ratio ignored: neither axis has a definite size
    AspectRatioNoDefiniteAxis,
    /// `layout.aspect_ratio_indefinite_container` — CSS aspect-ratio ignored: the containing block has no definite width to derive the height from
    AspectRatioIndefiniteContainer,
    /// `layout.position_sticky_ignored` — CSS position:sticky has no static canvas equivalent and was ignored
    PositionStickyIgnored,
    /// `layout.grid_tracks_approximated` — unsupported CSS grid tracks approximated as a vertical frame
    GridTracksApproximated,
    /// `layout.float_ignored` — CSS float ignored during structured HTML import
    FloatIgnored,
    /// `layout.mix_blend_mode_no_node_equivalent` — CSS mix-blend-mode has no node-level
    ///   equivalent; fill blending is used where possible
    MixBlendModeNoNodeEquivalent,
    /// `layout.overflow_scroll_clipped` — CSS overflow: auto / scroll is imported as a clip, so content that would be scrollable is not reachable
    OverflowScrollClipped,
    /// `layout.negative_margins_ignored` — negative CSS margins are not representable and were ignored
    NegativeMarginsIgnored,
    /// `layout.margins_on_visual_box_ignored` — CSS margins on visual boxes cannot be represented without changing the box and were ignored
    MarginsOnVisualBoxIgnored,
    /// `layout.inline_margin_wrapping_approximated` — a non-atomic inline with
    ///   margins was boxed and may no longer wrap across lines
    InlineMarginWrappingApproximated,
    /// `layout.content_box_percentage_approximated` — content-box percentage sizing cannot include padding exactly and was approximated
    ContentBoxPercentageApproximated,
    /// `layout.grid_empty_cells_packed` — CSS grid explicit start lines left empty cells; items
    ///   were packed without spacers
    GridEmptyCellsPacked,
    /// `layout.grid_span_reflowed` — CSS grid explicit start line could not host its span and the item was re-flowed
    GridSpanReflowed,
    /// `layout.grid_rows_node_limit` — CSS grid row wrappers omitted because the node limit was reached
    GridRowsNodeLimit,
    /// `layout.grid_track_widths_unresolved` — CSS grid track widths could not be resolved (auto-fit / auto-fill), so spanning items keep their own width and under-fill the row
    GridTrackWidthsUnresolved,
    /// `layout.grid_template_areas_ignored` — CSS grid-template-areas placement is not imported
    GridTemplateAreasIgnored,
    /// `layout.grid_row_placement_ignored` — CSS grid-row placement is not imported; rows are
    ///   chunked evenly
    GridRowPlacementIgnored,
    /// `layout.grid_column_unsupported` — CSS grid-column `{value}` is not supported and was auto-placed
    GridColumnUnsupported { value: String },
    /// `layout.block_auto_margins_ignored` — CSS block-axis auto margins were not emulated during import
    BlockAutoMarginsIgnored,
    /// `layout.auto_margin_node_limit` — CSS auto-margin alignment dropped because the node limit was reached
    AutoMarginNodeLimit,
    /// `layout.flow_offset_no_definite_size` — CSS in-flow offset dropped: the element has no definite size to reserve in flow
    FlowOffsetNoDefiniteSize,
    /// `layout.flow_offset_node_limit` — CSS in-flow offset dropped because the node limit was reached
    FlowOffsetNodeLimit,
    /// `layout.flow_offset_approximated` — CSS in-flow offsets (position:relative insets, transform translation) are approximated: the node shifts inside a fixed-size wrapper so surrounding flow keeps the un-shifted box
    FlowOffsetApproximated,
    /// `layout.flow_offset_no_wrapper` — CSS in-flow offset (position:relative or a transform translation) was dropped because this box cannot host an offset wrapper
    FlowOffsetNoWrapper,
    /// `layout.flex_wrap_column_not_emulated` — flex-wrap on a column flex container was not
    ///   emulated; children stay in one column (per-child auto margins under this container stay
    ///   unemulated)
    FlexWrapColumnNotEmulated,
    /// `layout.flex_wrap_reverse_plain` — flex-wrap:wrap-reverse imported as plain wrap; row order
    ///   is unchanged
    FlexWrapReversePlain,
    /// `layout.flex_wrap_indefinite_width` — flex-wrap ignored: this container has no definite width, so rows cannot be measured (per-child auto margins under this container stay unemulated)
    FlexWrapIndefiniteWidth,
    /// `layout.flex_align_content_ignored` — CSS align-content on a wrapping flex container was not imported
    FlexAlignContentIgnored,
    /// `layout.flex_wrap_indeterminate_children` — flex-wrap ignored: child main-axis sizes are indeterminate, so rows cannot be measured (per-child auto margins under this container stay unemulated)
    FlexWrapIndeterminateChildren,
    /// `layout.flex_wrap_node_limit` — flex-wrap rows omitted because the node limit was reached (per-child auto margins under this container stay unemulated)
    FlexWrapNodeLimit,
    // --- transform: `transform` / `transform-origin` degradations.
    /// `transform.unsupported_syntax` — unsupported CSS transform syntax ignored
    TransformUnsupportedSyntax,
    /// `transform.unsupported_function` — unsupported CSS transform function ignored (3D and matrix3d are dropped)
    TransformUnsupportedFunction,
    /// `transform.percentage_translation_dropped` — CSS percentage transform translation dropped on an axis with no definite size
    TransformPercentageTranslationDropped,
    /// `transform.non_finite_matrix` — CSS transform produced a non-finite matrix and was ignored
    TransformNonFiniteMatrix,
    /// `transform.skew_dropped` — CSS transform skew has no node equivalent and was dropped
    TransformSkewDropped,
    /// `transform.degenerate_scale` — CSS transform collapses the element to a zero or non-finite
    ///   scale; it was imported at full size
    TransformDegenerateScale,
    /// `transform.mirroring_absolute` — CSS transform mirroring was imported as its absolute scale
    TransformMirroringAbsolute,
    /// `transform.origin_z_ignored` — CSS transform-origin Z offset ignored during 2D import
    TransformOriginZIgnored,
    /// `transform.scale_not_baked` — CSS transform scale could not be baked into this node's size
    TransformScaleNotBaked,
    /// `transform.scale_baked` — CSS transform scale baked into node size; font sizes and
    ///   descendant sizes are unchanged
    TransformScaleBaked,
    /// `transform.scale_auto_size_ignored` — CSS transform scale ignored on an auto-sized element
    TransformScaleAutoSizeIgnored,
    // --- visual: Fills, borders, gradients, shadows, filters and blending.
    /// `visual.background_repeat_approximated` — directional or spaced CSS background repeat approximated as tile
    BackgroundRepeatApproximated,
    /// `visual.background_tile_size_ignored` — explicit CSS background tile size is not representable and was ignored
    BackgroundTileSizeIgnored,
    /// `visual.background_size_auto_box` — CSS background-size on an element with an automatic width or height needs a used box this importer does not resolve and was approximated as a cover fill
    BackgroundSizeAutoBox,
    /// `visual.background_size_needs_intrinsic_size` — CSS background-size with an automatic or unresolvable axis needs the image's intrinsic size and was approximated as a cover fill
    BackgroundSizeNeedsIntrinsicSize,
    /// `visual.background_position_unsupported` — unsupported CSS background-position was ignored
    BackgroundPositionUnsupported,
    /// `visual.background_image_url_empty` — empty CSS background image URL was ignored
    BackgroundImageUrlEmpty,
    /// `visual.conic_gradient_ignored` — conic gradients are not representable and were ignored
    ConicGradientIgnored,
    /// `visual.background_image_layer_unsupported` — unsupported CSS background-image layer was ignored
    BackgroundImageLayerUnsupported,
    /// `visual.background_color_unresolved` — unresolved CSS background color was ignored
    BackgroundColorUnresolved,
    /// `visual.background_position_dropped` — CSS background-position is not representable and was ignored
    BackgroundPositionDropped,
    /// `visual.border_colors_approximated` — per-side border colors approximated using the first opaque color
    BorderColorsApproximated,
    /// `visual.border_styles_approximated` — mixed per-side border styles approximated using the first style
    BorderStylesApproximated,
    /// `visual.border_style_complex` — complex CSS border style approximated as a solid stroke
    BorderStyleComplex,
    /// `visual.border_style_unsupported` — unsupported CSS border style approximated as a solid stroke
    BorderStyleUnsupported,
    /// `visual.border_radius_elliptical` — elliptical CSS border radii approximated using horizontal radii
    BorderRadiusElliptical,
    /// `visual.border_radius_unsupported` — unsupported CSS border radius was ignored
    BorderRadiusUnsupported,
    /// `visual.box_shadow_layer_unsupported` — unsupported CSS box-shadow layer was ignored
    BoxShadowLayerUnsupported,
    /// `visual.gradient_interpolation_ignored` — CSS gradient color interpolation method was ignored
    GradientInterpolationIgnored,
    /// `visual.linear_gradient_direction_unsupported` — unsupported CSS linear-gradient direction was ignored
    LinearGradientDirectionUnsupported,
    /// `visual.gradient_color_hints_ignored` — CSS gradient color hints are not representable and were ignored
    GradientColorHintsIgnored,
    /// `visual.gradient_color_stop_unsupported` — unsupported CSS gradient color stop was ignored
    GradientColorStopUnsupported,
    /// `visual.gradient_too_few_stops` — CSS gradient with fewer than two usable stops was ignored
    GradientTooFewStops,
    /// `visual.gradient_repeating_approximated` — repeating CSS gradient approximated using one gradient cycle
    GradientRepeatingApproximated,
    /// `visual.gradient_stops_clamped` — out-of-range CSS gradient stops were clamped to the fill bounds
    GradientStopsClamped,
    /// `visual.blur_radius_unsupported` — unsupported CSS blur radius was ignored
    BlurRadiusUnsupported,
    /// `visual.filter_drop_shadow_unsupported` — unsupported CSS filter drop-shadow was ignored
    FilterDropShadowUnsupported,
    /// `visual.filter_function_unsupported` — unsupported CSS filter function was ignored
    FilterFunctionUnsupported,
    /// `visual.backdrop_filter_unsupported` — unsupported CSS backdrop-filter function was ignored
    BackdropFilterUnsupported,
    /// `visual.background_blend_mode_unsupported` — unsupported CSS background-blend-mode was ignored
    BackgroundBlendModeUnsupported,
    /// `visual.mix_blend_mode_on_fills` — CSS mix-blend-mode approximated on individual fills
    MixBlendModeOnFills,
    /// `visual.mix_blend_mode_unsupported` — unsupported CSS mix-blend-mode was ignored
    MixBlendModeUnsupported,
    /// `visual.property_not_representable` — CSS {property} is not representable and was ignored
    PropertyNotRepresentable { property: String },
    /// `visual.gradient_background_size_ignored` — CSS background-size on gradients is not representable and was ignored
    GradientBackgroundSizeIgnored,
    /// `visual.radial_gradient_position_unsupported` — unsupported CSS radial-gradient position was ignored
    RadialGradientPositionUnsupported,
    /// `visual.radial_gradient_elliptical` — elliptical CSS radial-gradient approximated as circular
    RadialGradientElliptical,
    /// `visual.radial_gradient_extent_approximated` — CSS radial-gradient extent keyword was approximated
    RadialGradientExtentApproximated,
    /// `visual.radial_gradient_size_unsupported` — unsupported CSS radial-gradient size was ignored
    RadialGradientSizeUnsupported,
    // --- text: Text-level degradations.
    /// `text.shadow_layer_unsupported` — unsupported CSS text-shadow layer was ignored
    TextShadowLayerUnsupported,
    /// `text.shadow_extra_layers_ignored` — CSS text-shadow layers after the first were ignored
    TextShadowExtraLayersIgnored,
    /// `text.shadow_on_inline_ignored` — CSS text-shadow on an inline element was ignored; the
    ///   shadow is a property of the whole text node
    TextShadowOnInlineIgnored,
    // --- list: List markers.
    /// `list.style_image_ignored` — CSS list-style-image is not imported; the item's text marker is
    ///   used instead
    ListStyleImageIgnored,
    /// `list.marker_position_outside_approximated` — CSS list markers are inlined before the item text, so a `list-style-position: outside` hanging marker is approximated
    ListMarkerPositionOutsideApproximated,
    /// `list.style_type_unsupported` — unsupported CSS list-style-type `{value}` was approximated as a disc
    ListStyleTypeUnsupported { value: String },
    // --- media: Replaced elements: images, `<svg>`, `<picture>`, form controls.
    /// `media.object_fit_scale_down` — CSS object-fit:scale-down approximated as contain
    ObjectFitScaleDown,
    /// `media.object_fit_none_ignored` — CSS object-fit:none has no image-node equivalent and was ignored
    ObjectFitNoneIgnored,
    /// `media.object_position_ignored` — CSS object-position has no image-node equivalent; the
    ///   image remains centered
    ObjectPositionIgnored,
    /// `media.image_mix_blend_mode_unsupported` — unsupported CSS mix-blend-mode was ignored on an image
    ImageMixBlendModeUnsupported,
    /// `media.image_intrinsic_axis_unresolved` — image intrinsic aspect ratio could not resolve the missing axis because the authored size is dynamic or its containing block is indefinite
    ImageIntrinsicAxisUnresolved,
    /// `media.inline_svg_placeholder` — inline <svg> imported as image placeholder
    InlineSvgPlaceholder,
    /// `media.input_type_fallback` — unsupported input type imported as a text input
    InputTypeFallback,
    /// `media.element_placeholder` — <{tag}> imported as a placeholder
    ElementPlaceholder { tag: String },
    /// `media.picture_undecodable_types` — <picture> only offered source types this importer cannot
    ///   decode; the first matching source was imported anyway
    PictureUndecodableTypes,
    // --- table: HTML/CSS tables.
    /// `table.rowspan_ignored` — HTML rowspan is not imported; the cell occupies a single row
    TableRowspanIgnored,
    /// `table.row_groups_unflattened` — CSS un-flattened a table's row groups, so its cells no longer share one column grid and each row's widths are approximated
    TableRowGroupsUnflattened,
    /// `table.indefinite_width_approximated` — a CSS table without a definite width was measured against its containing block, so its column widths are approximate
    TableIndefiniteWidthApproximated,
    // --- resource: External resources: stylesheets, `@import`, images, `<base>`.
    /// `resource.invalid_base_href` — invalid <base href> ignored: {href}
    InvalidBaseHref { href: String },
    /// `resource.base_href_outside_origin` — <base href> outside the HTML project origin ignored: {href}
    BaseHrefOutsideOrigin { href: String },
    /// `resource.external_stylesheet_skipped` — external stylesheet skipped: {url}
    ExternalStylesheetSkipped { url: String },
    /// `resource.image_outside_origin` — image resource outside the HTML project origin, using placeholder: {url}
    ImageOutsideOrigin { url: String },
    /// `resource.image_unavailable` — image resource unavailable, using placeholder: {url}
    ImageUnavailable { url: String },
    /// `resource.css_import_invalid` — invalid CSS @import ignored: {prelude}
    CssImportInvalid { prelude: String },
    /// `resource.css_import_unresolvable` — CSS @import URL could not be resolved or left the project origin: {reference}
    CssImportUnresolvable { reference: String },
    /// `resource.css_import_cycle` — CSS @import cycle ignored: {url}
    CssImportCycle { url: String },
    /// `resource.css_import_depth_limit` — CSS @import depth limit ({max_depth}) reached: {url}
    CssImportDepthLimit { max_depth: usize, url: String },
    /// `resource.css_import_unavailable` — CSS @import unavailable: {url}
    CssImportUnavailable { url: String },
    // --- project: HTML project (folder / zip) selection.
    /// `project.multiple_html_entries` — multiple HTML entries found ({count}); selected {entry}
    MultipleHtmlEntries { count: usize, entry: String },
    // --- snapshot: Live browser-snapshot import.
    /// `snapshot.truncated` — browser snapshot was truncated during extraction
    SnapshotTruncated,
    /// `snapshot.node_limit` — node limit reached (40000), remaining snapshot content dropped
    SnapshotNodeLimit,
    /// `snapshot.tainted_images` — {count} images kept as remote URLs (CORS-tainted)
    SnapshotTaintedImages { count: usize },
    /// `snapshot.invalid_rect` — snapshot node with missing or invalid rect was skipped
    SnapshotInvalidRect,
    /// `snapshot.unknown_kind` — snapshot node with unknown kind was skipped
    SnapshotUnknownKind,
    /// `snapshot.rejected` — {reason}
    SnapshotRejected { reason: String },
    /// `snapshot.unsupported_transform` — unsupported snapshot transform ignored (only matrix rotation imported)
    SnapshotUnsupportedTransform,
}

/// Prefix shared by every `ImportWarning` translation key.
pub const IMPORT_WARNING_KEY_PREFIX: &str = "htmlImport.warn.";

macro_rules! ident_pair {
    ($code:literal) => {
        ($code, concat!("htmlImport.warn.", $code))
    };
}

impl ImportWarning {
    /// Stable machine identifier, e.g. `layout.flex_wrap_node_limit`.
    pub fn code(&self) -> &'static str {
        self.ident().0
    }

    /// Locale-table key holding the panel copy for this warning.
    pub fn i18n_key(&self) -> &'static str {
        self.ident().1
    }

    fn ident(&self) -> (&'static str, &'static str) {
        match self {
            Self::EmptyInput => ident_pair!("content.empty_input"),
            Self::EmptyBody => ident_pair!("content.empty_body"),
            Self::DomDepthTruncated { .. } => ident_pair!("content.dom_depth_truncated"),
            Self::NodeLimitTruncated => ident_pair!("content.node_limit_truncated"),
            Self::NodeLimitMapping => ident_pair!("content.node_limit_mapping"),
            Self::NodeLimitInlineRow => ident_pair!("content.node_limit_inline_row"),
            Self::NodeLimitPseudo => ident_pair!("content.node_limit_pseudo"),
            Self::AtRuleDepthLimit { .. } => ident_pair!("css.at_rule_depth_limit"),
            Self::UnterminatedRule => ident_pair!("css.unterminated_rule"),
            Self::MarkerRulesUnsupported => ident_pair!("css.marker_rules_unsupported"),
            Self::NestingUnsupported => ident_pair!("css.nesting_unsupported"),
            Self::InvalidLayerName { .. } => ident_pair!("css.invalid_layer_name"),
            Self::UnsupportedAtStatement { .. } => ident_pair!("css.unsupported_statement"),
            Self::MediaWithoutViewport => ident_pair!("css.media_without_viewport"),
            Self::InvalidLayerBlockName { .. } => ident_pair!("css.invalid_layer_block_name"),
            Self::UnsupportedContainerBlock => ident_pair!("css.unsupported_container_block"),
            Self::UnsupportedAtBlock { .. } => ident_pair!("css.unsupported_block"),
            Self::WebFontNotDownloaded { .. } => ident_pair!("font.web_font_not_downloaded"),
            Self::PercentageAbsoluteOffsetInferred => {
                ident_pair!("layout.percentage_absolute_offset_inferred")
            }
            Self::PercentageRelativeOffsetInferred => {
                ident_pair!("layout.percentage_relative_offset_inferred")
            }
            Self::AspectRatioNoDefiniteAxis => ident_pair!("layout.aspect_ratio_no_definite_axis"),
            Self::AspectRatioIndefiniteContainer => {
                ident_pair!("layout.aspect_ratio_indefinite_container")
            }
            Self::PositionStickyIgnored => ident_pair!("layout.position_sticky_ignored"),
            Self::GridTracksApproximated => ident_pair!("layout.grid_tracks_approximated"),
            Self::FloatIgnored => ident_pair!("layout.float_ignored"),
            Self::MixBlendModeNoNodeEquivalent => {
                ident_pair!("layout.mix_blend_mode_no_node_equivalent")
            }
            Self::OverflowScrollClipped => ident_pair!("layout.overflow_scroll_clipped"),
            Self::NegativeMarginsIgnored => ident_pair!("layout.negative_margins_ignored"),
            Self::MarginsOnVisualBoxIgnored => ident_pair!("layout.margins_on_visual_box_ignored"),
            Self::InlineMarginWrappingApproximated => {
                ident_pair!("layout.inline_margin_wrapping_approximated")
            }
            Self::ContentBoxPercentageApproximated => {
                ident_pair!("layout.content_box_percentage_approximated")
            }
            Self::GridEmptyCellsPacked => ident_pair!("layout.grid_empty_cells_packed"),
            Self::GridSpanReflowed => ident_pair!("layout.grid_span_reflowed"),
            Self::GridRowsNodeLimit => ident_pair!("layout.grid_rows_node_limit"),
            Self::GridTrackWidthsUnresolved => ident_pair!("layout.grid_track_widths_unresolved"),
            Self::GridTemplateAreasIgnored => ident_pair!("layout.grid_template_areas_ignored"),
            Self::GridRowPlacementIgnored => ident_pair!("layout.grid_row_placement_ignored"),
            Self::GridColumnUnsupported { .. } => ident_pair!("layout.grid_column_unsupported"),
            Self::BlockAutoMarginsIgnored => ident_pair!("layout.block_auto_margins_ignored"),
            Self::AutoMarginNodeLimit => ident_pair!("layout.auto_margin_node_limit"),
            Self::FlowOffsetNoDefiniteSize => ident_pair!("layout.flow_offset_no_definite_size"),
            Self::FlowOffsetNodeLimit => ident_pair!("layout.flow_offset_node_limit"),
            Self::FlowOffsetApproximated => ident_pair!("layout.flow_offset_approximated"),
            Self::FlowOffsetNoWrapper => ident_pair!("layout.flow_offset_no_wrapper"),
            Self::FlexWrapColumnNotEmulated => ident_pair!("layout.flex_wrap_column_not_emulated"),
            Self::FlexWrapReversePlain => ident_pair!("layout.flex_wrap_reverse_plain"),
            Self::FlexWrapIndefiniteWidth => ident_pair!("layout.flex_wrap_indefinite_width"),
            Self::FlexAlignContentIgnored => ident_pair!("layout.flex_align_content_ignored"),
            Self::FlexWrapIndeterminateChildren => {
                ident_pair!("layout.flex_wrap_indeterminate_children")
            }
            Self::FlexWrapNodeLimit => ident_pair!("layout.flex_wrap_node_limit"),
            Self::TransformUnsupportedSyntax => ident_pair!("transform.unsupported_syntax"),
            Self::TransformUnsupportedFunction => ident_pair!("transform.unsupported_function"),
            Self::TransformPercentageTranslationDropped => {
                ident_pair!("transform.percentage_translation_dropped")
            }
            Self::TransformNonFiniteMatrix => ident_pair!("transform.non_finite_matrix"),
            Self::TransformSkewDropped => ident_pair!("transform.skew_dropped"),
            Self::TransformDegenerateScale => ident_pair!("transform.degenerate_scale"),
            Self::TransformMirroringAbsolute => ident_pair!("transform.mirroring_absolute"),
            Self::TransformOriginZIgnored => ident_pair!("transform.origin_z_ignored"),
            Self::TransformScaleNotBaked => ident_pair!("transform.scale_not_baked"),
            Self::TransformScaleBaked => ident_pair!("transform.scale_baked"),
            Self::TransformScaleAutoSizeIgnored => ident_pair!("transform.scale_auto_size_ignored"),
            Self::BackgroundRepeatApproximated => {
                ident_pair!("visual.background_repeat_approximated")
            }
            Self::BackgroundTileSizeIgnored => ident_pair!("visual.background_tile_size_ignored"),
            Self::BackgroundSizeAutoBox => ident_pair!("visual.background_size_auto_box"),
            Self::BackgroundSizeNeedsIntrinsicSize => {
                ident_pair!("visual.background_size_needs_intrinsic_size")
            }
            Self::BackgroundPositionUnsupported => {
                ident_pair!("visual.background_position_unsupported")
            }
            Self::BackgroundImageUrlEmpty => ident_pair!("visual.background_image_url_empty"),
            Self::ConicGradientIgnored => ident_pair!("visual.conic_gradient_ignored"),
            Self::BackgroundImageLayerUnsupported => {
                ident_pair!("visual.background_image_layer_unsupported")
            }
            Self::BackgroundColorUnresolved => ident_pair!("visual.background_color_unresolved"),
            Self::BackgroundPositionDropped => ident_pair!("visual.background_position_dropped"),
            Self::BorderColorsApproximated => ident_pair!("visual.border_colors_approximated"),
            Self::BorderStylesApproximated => ident_pair!("visual.border_styles_approximated"),
            Self::BorderStyleComplex => ident_pair!("visual.border_style_complex"),
            Self::BorderStyleUnsupported => ident_pair!("visual.border_style_unsupported"),
            Self::BorderRadiusElliptical => ident_pair!("visual.border_radius_elliptical"),
            Self::BorderRadiusUnsupported => ident_pair!("visual.border_radius_unsupported"),
            Self::BoxShadowLayerUnsupported => ident_pair!("visual.box_shadow_layer_unsupported"),
            Self::GradientInterpolationIgnored => {
                ident_pair!("visual.gradient_interpolation_ignored")
            }
            Self::LinearGradientDirectionUnsupported => {
                ident_pair!("visual.linear_gradient_direction_unsupported")
            }
            Self::GradientColorHintsIgnored => ident_pair!("visual.gradient_color_hints_ignored"),
            Self::GradientColorStopUnsupported => {
                ident_pair!("visual.gradient_color_stop_unsupported")
            }
            Self::GradientTooFewStops => ident_pair!("visual.gradient_too_few_stops"),
            Self::GradientRepeatingApproximated => {
                ident_pair!("visual.gradient_repeating_approximated")
            }
            Self::GradientStopsClamped => ident_pair!("visual.gradient_stops_clamped"),
            Self::BlurRadiusUnsupported => ident_pair!("visual.blur_radius_unsupported"),
            Self::FilterDropShadowUnsupported => {
                ident_pair!("visual.filter_drop_shadow_unsupported")
            }
            Self::FilterFunctionUnsupported => ident_pair!("visual.filter_function_unsupported"),
            Self::BackdropFilterUnsupported => ident_pair!("visual.backdrop_filter_unsupported"),
            Self::BackgroundBlendModeUnsupported => {
                ident_pair!("visual.background_blend_mode_unsupported")
            }
            Self::MixBlendModeOnFills => ident_pair!("visual.mix_blend_mode_on_fills"),
            Self::MixBlendModeUnsupported => ident_pair!("visual.mix_blend_mode_unsupported"),
            Self::PropertyNotRepresentable { .. } => {
                ident_pair!("visual.property_not_representable")
            }
            Self::GradientBackgroundSizeIgnored => {
                ident_pair!("visual.gradient_background_size_ignored")
            }
            Self::RadialGradientPositionUnsupported => {
                ident_pair!("visual.radial_gradient_position_unsupported")
            }
            Self::RadialGradientElliptical => ident_pair!("visual.radial_gradient_elliptical"),
            Self::RadialGradientExtentApproximated => {
                ident_pair!("visual.radial_gradient_extent_approximated")
            }
            Self::RadialGradientSizeUnsupported => {
                ident_pair!("visual.radial_gradient_size_unsupported")
            }
            Self::TextShadowLayerUnsupported => ident_pair!("text.shadow_layer_unsupported"),
            Self::TextShadowExtraLayersIgnored => ident_pair!("text.shadow_extra_layers_ignored"),
            Self::TextShadowOnInlineIgnored => ident_pair!("text.shadow_on_inline_ignored"),
            Self::ListStyleImageIgnored => ident_pair!("list.style_image_ignored"),
            Self::ListMarkerPositionOutsideApproximated => {
                ident_pair!("list.marker_position_outside_approximated")
            }
            Self::ListStyleTypeUnsupported { .. } => ident_pair!("list.style_type_unsupported"),
            Self::ObjectFitScaleDown => ident_pair!("media.object_fit_scale_down"),
            Self::ObjectFitNoneIgnored => ident_pair!("media.object_fit_none_ignored"),
            Self::ObjectPositionIgnored => ident_pair!("media.object_position_ignored"),
            Self::ImageMixBlendModeUnsupported => {
                ident_pair!("media.image_mix_blend_mode_unsupported")
            }
            Self::ImageIntrinsicAxisUnresolved => {
                ident_pair!("media.image_intrinsic_axis_unresolved")
            }
            Self::InlineSvgPlaceholder => ident_pair!("media.inline_svg_placeholder"),
            Self::InputTypeFallback => ident_pair!("media.input_type_fallback"),
            Self::ElementPlaceholder { .. } => ident_pair!("media.element_placeholder"),
            Self::PictureUndecodableTypes => ident_pair!("media.picture_undecodable_types"),
            Self::TableRowspanIgnored => ident_pair!("table.rowspan_ignored"),
            Self::TableRowGroupsUnflattened => ident_pair!("table.row_groups_unflattened"),
            Self::TableIndefiniteWidthApproximated => {
                ident_pair!("table.indefinite_width_approximated")
            }
            Self::InvalidBaseHref { .. } => ident_pair!("resource.invalid_base_href"),
            Self::BaseHrefOutsideOrigin { .. } => ident_pair!("resource.base_href_outside_origin"),
            Self::ExternalStylesheetSkipped { .. } => {
                ident_pair!("resource.external_stylesheet_skipped")
            }
            Self::ImageOutsideOrigin { .. } => ident_pair!("resource.image_outside_origin"),
            Self::ImageUnavailable { .. } => ident_pair!("resource.image_unavailable"),
            Self::CssImportInvalid { .. } => ident_pair!("resource.css_import_invalid"),
            Self::CssImportUnresolvable { .. } => ident_pair!("resource.css_import_unresolvable"),
            Self::CssImportCycle { .. } => ident_pair!("resource.css_import_cycle"),
            Self::CssImportDepthLimit { .. } => ident_pair!("resource.css_import_depth_limit"),
            Self::CssImportUnavailable { .. } => ident_pair!("resource.css_import_unavailable"),
            Self::MultipleHtmlEntries { .. } => ident_pair!("project.multiple_html_entries"),
            Self::SnapshotTruncated => ident_pair!("snapshot.truncated"),
            Self::SnapshotNodeLimit => ident_pair!("snapshot.node_limit"),
            Self::SnapshotTaintedImages { .. } => ident_pair!("snapshot.tainted_images"),
            Self::SnapshotInvalidRect => ident_pair!("snapshot.invalid_rect"),
            Self::SnapshotUnknownKind => ident_pair!("snapshot.unknown_kind"),
            Self::SnapshotRejected { .. } => ident_pair!("snapshot.rejected"),
            Self::SnapshotUnsupportedTransform => ident_pair!("snapshot.unsupported_transform"),
            Self::MediaQuery(error) => error.ident(),
        }
    }
}

impl MediaQueryError {
    /// Stable machine identifier for this query failure.
    pub fn code(&self) -> &'static str {
        self.ident().0
    }

    /// Locale-table key holding the panel copy for this failure.
    pub fn i18n_key(&self) -> &'static str {
        self.ident().1
    }

    pub(crate) fn ident(&self) -> (&'static str, &'static str) {
        match self {
            Self::EmptyQuery => ident_pair!("css.media_empty_query"),
            Self::UnsupportedType(_) => ident_pair!("css.media_unsupported_type"),
            Self::UnsupportedCondition(_) => ident_pair!("css.media_unsupported_condition"),
            Self::InvalidOrientation(_) => ident_pair!("css.media_invalid_orientation"),
            Self::UnsupportedFeature(_) => ident_pair!("css.media_unsupported_feature"),
            Self::UnsupportedRange(_) => ident_pair!("css.media_unsupported_range"),
            Self::InvalidRange(_) => ident_pair!("css.media_invalid_range"),
            Self::InvalidLength(_) => ident_pair!("css.media_invalid_length"),
        }
    }
}

impl ImportWarning {
    /// Structured fields of this warning, ready for placeholder
    /// substitution against the localized template. Variants with no
    /// field return an empty list.
    pub fn args(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::DomDepthTruncated { max_depth } => vec![("max_depth", max_depth.to_string())],
            Self::AtRuleDepthLimit { max_depth } => vec![("max_depth", max_depth.to_string())],
            Self::InvalidLayerName { name } => vec![("name", name.to_string())],
            Self::UnsupportedAtStatement { name } => vec![("name", name.to_string())],
            Self::InvalidLayerBlockName { name } => vec![("name", name.to_string())],
            Self::UnsupportedAtBlock { name } => vec![("name", name.to_string())],
            Self::WebFontNotDownloaded { family } => vec![("family", family.to_string())],
            Self::GridColumnUnsupported { value } => vec![("value", value.to_string())],
            Self::PropertyNotRepresentable { property } => vec![("property", property.to_string())],
            Self::ListStyleTypeUnsupported { value } => vec![("value", value.to_string())],
            Self::ElementPlaceholder { tag } => vec![("tag", tag.to_string())],
            Self::InvalidBaseHref { href } => vec![("href", href.to_string())],
            Self::BaseHrefOutsideOrigin { href } => vec![("href", href.to_string())],
            Self::ExternalStylesheetSkipped { url } => vec![("url", url.to_string())],
            Self::ImageOutsideOrigin { url } => vec![("url", url.to_string())],
            Self::ImageUnavailable { url } => vec![("url", url.to_string())],
            Self::CssImportInvalid { prelude } => vec![("prelude", prelude.to_string())],
            Self::CssImportUnresolvable { reference } => vec![("reference", reference.to_string())],
            Self::CssImportCycle { url } => vec![("url", url.to_string())],
            Self::CssImportDepthLimit { max_depth, url } => vec![
                ("max_depth", max_depth.to_string()),
                ("url", url.to_string()),
            ],
            Self::CssImportUnavailable { url } => vec![("url", url.to_string())],
            Self::MultipleHtmlEntries { count, entry } => {
                vec![("count", count.to_string()), ("entry", entry.to_string())]
            }
            Self::SnapshotTaintedImages { count } => vec![("count", count.to_string())],
            Self::SnapshotRejected { reason } => vec![("reason", reason.to_string())],
            Self::MediaQuery(error) => error.args(),
            _ => Vec::new(),
        }
    }
}

impl MediaQueryError {
    /// Structured fields of this query failure.
    pub fn args(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::EmptyQuery => Vec::new(),
            Self::UnsupportedType(value) => vec![("name", value.clone())],
            Self::UnsupportedCondition(value) => vec![("input", value.clone())],
            Self::InvalidOrientation(value) => vec![("value", value.clone())],
            Self::UnsupportedFeature(value) => vec![("name", value.clone())],
            Self::UnsupportedRange(value) => vec![("input", value.clone())],
            Self::InvalidRange(value) => vec![("input", value.clone())],
            Self::InvalidLength(value) => vec![("value", value.clone())],
        }
    }
}

/// Render diagnostics into the compatibility string form.
pub fn render_warnings(warnings: &[ImportWarning]) -> Vec<String> {
    warnings.iter().map(ToString::to_string).collect()
}

/// The carrier-shaped parts of one warning: `(code, i18n_key, args, message)`.
///
/// This is the wire between the importer and an editor host's post-import
/// diagnostics row. It is a tuple rather than a named struct because the
/// receiving type — `op_editor_core::html_import_diagnostics::HtmlImportDiagnostic`
/// — lives in a crate this one must NOT depend on (the editor core stays free
/// of the importer's dependency tree, and the browser host builds the same
/// rows from a worker's JSON payload). `op-editor-core` consumes the tuple
/// through `html_import_diagnostics::rows_from_parts`, so the mapping from
/// `ImportWarning` to a row exists exactly once instead of once per host.
pub type ImportDiagnosticParts = (String, String, Vec<(String, String)>, String);

/// Convert typed warnings into [`ImportDiagnosticParts`], owning every field.
///
/// Callers pair this with `op_editor_core::html_import_diagnostics::rows_from_parts`
/// to get the overlay's rows in one line; see [`ImportDiagnosticParts`] for why
/// the conversion is split across the two crates.
pub fn diagnostic_parts(warnings: &[ImportWarning]) -> Vec<ImportDiagnosticParts> {
    warnings
        .iter()
        .map(|warning| {
            (
                warning.code().to_string(),
                warning.i18n_key().to_string(),
                warning
                    .args()
                    .into_iter()
                    .map(|(name, value)| (name.to_string(), value))
                    .collect(),
                warning.to_string(),
            )
        })
        .collect()
}

/// Every code this crate can emit, for catalog and key-coverage tests.
pub const ALL_IMPORT_WARNING_CODES: &[&str] = &[
    "content.empty_input",
    "content.empty_body",
    "content.dom_depth_truncated",
    "content.node_limit_truncated",
    "content.node_limit_mapping",
    "content.node_limit_inline_row",
    "content.node_limit_pseudo",
    "css.at_rule_depth_limit",
    "css.unterminated_rule",
    "css.marker_rules_unsupported",
    "css.nesting_unsupported",
    "css.invalid_layer_name",
    "css.unsupported_statement",
    "css.media_without_viewport",
    "css.invalid_layer_block_name",
    "css.unsupported_container_block",
    "css.unsupported_block",
    "font.web_font_not_downloaded",
    "layout.percentage_absolute_offset_inferred",
    "layout.percentage_relative_offset_inferred",
    "layout.aspect_ratio_no_definite_axis",
    "layout.aspect_ratio_indefinite_container",
    "layout.position_sticky_ignored",
    "layout.grid_tracks_approximated",
    "layout.float_ignored",
    "layout.mix_blend_mode_no_node_equivalent",
    "layout.overflow_scroll_clipped",
    "layout.negative_margins_ignored",
    "layout.margins_on_visual_box_ignored",
    "layout.inline_margin_wrapping_approximated",
    "layout.content_box_percentage_approximated",
    "layout.grid_empty_cells_packed",
    "layout.grid_span_reflowed",
    "layout.grid_rows_node_limit",
    "layout.grid_track_widths_unresolved",
    "layout.grid_template_areas_ignored",
    "layout.grid_row_placement_ignored",
    "layout.grid_column_unsupported",
    "layout.block_auto_margins_ignored",
    "layout.auto_margin_node_limit",
    "layout.flow_offset_no_definite_size",
    "layout.flow_offset_node_limit",
    "layout.flow_offset_approximated",
    "layout.flow_offset_no_wrapper",
    "layout.flex_wrap_column_not_emulated",
    "layout.flex_wrap_reverse_plain",
    "layout.flex_wrap_indefinite_width",
    "layout.flex_align_content_ignored",
    "layout.flex_wrap_indeterminate_children",
    "layout.flex_wrap_node_limit",
    "transform.unsupported_syntax",
    "transform.unsupported_function",
    "transform.percentage_translation_dropped",
    "transform.non_finite_matrix",
    "transform.skew_dropped",
    "transform.degenerate_scale",
    "transform.mirroring_absolute",
    "transform.origin_z_ignored",
    "transform.scale_not_baked",
    "transform.scale_baked",
    "transform.scale_auto_size_ignored",
    "visual.background_repeat_approximated",
    "visual.background_tile_size_ignored",
    "visual.background_size_auto_box",
    "visual.background_size_needs_intrinsic_size",
    "visual.background_position_unsupported",
    "visual.background_image_url_empty",
    "visual.conic_gradient_ignored",
    "visual.background_image_layer_unsupported",
    "visual.background_color_unresolved",
    "visual.background_position_dropped",
    "visual.border_colors_approximated",
    "visual.border_styles_approximated",
    "visual.border_style_complex",
    "visual.border_style_unsupported",
    "visual.border_radius_elliptical",
    "visual.border_radius_unsupported",
    "visual.box_shadow_layer_unsupported",
    "visual.gradient_interpolation_ignored",
    "visual.linear_gradient_direction_unsupported",
    "visual.gradient_color_hints_ignored",
    "visual.gradient_color_stop_unsupported",
    "visual.gradient_too_few_stops",
    "visual.gradient_repeating_approximated",
    "visual.gradient_stops_clamped",
    "visual.blur_radius_unsupported",
    "visual.filter_drop_shadow_unsupported",
    "visual.filter_function_unsupported",
    "visual.backdrop_filter_unsupported",
    "visual.background_blend_mode_unsupported",
    "visual.mix_blend_mode_on_fills",
    "visual.mix_blend_mode_unsupported",
    "visual.property_not_representable",
    "visual.gradient_background_size_ignored",
    "visual.radial_gradient_position_unsupported",
    "visual.radial_gradient_elliptical",
    "visual.radial_gradient_extent_approximated",
    "visual.radial_gradient_size_unsupported",
    "text.shadow_layer_unsupported",
    "text.shadow_extra_layers_ignored",
    "text.shadow_on_inline_ignored",
    "list.style_image_ignored",
    "list.marker_position_outside_approximated",
    "list.style_type_unsupported",
    "media.object_fit_scale_down",
    "media.object_fit_none_ignored",
    "media.object_position_ignored",
    "media.image_mix_blend_mode_unsupported",
    "media.image_intrinsic_axis_unresolved",
    "media.inline_svg_placeholder",
    "media.input_type_fallback",
    "media.element_placeholder",
    "media.picture_undecodable_types",
    "table.rowspan_ignored",
    "table.row_groups_unflattened",
    "table.indefinite_width_approximated",
    "resource.invalid_base_href",
    "resource.base_href_outside_origin",
    "resource.external_stylesheet_skipped",
    "resource.image_outside_origin",
    "resource.image_unavailable",
    "resource.css_import_invalid",
    "resource.css_import_unresolvable",
    "resource.css_import_cycle",
    "resource.css_import_depth_limit",
    "resource.css_import_unavailable",
    "project.multiple_html_entries",
    "snapshot.truncated",
    "snapshot.node_limit",
    "snapshot.tainted_images",
    "snapshot.invalid_rect",
    "snapshot.unknown_kind",
    "snapshot.rejected",
    "snapshot.unsupported_transform",
    "css.media_empty_query",
    "css.media_unsupported_type",
    "css.media_unsupported_condition",
    "css.media_invalid_orientation",
    "css.media_unsupported_feature",
    "css.media_unsupported_range",
    "css.media_invalid_range",
    "css.media_invalid_length",
];
