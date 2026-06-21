//! `PropertyPanelAction` — the button / checkbox / dropdown actions
//! the property panel emits. Split out of `property_panel.rs` to
//! keep that file under the 800-line ceiling; re-exported from
//! `property_panel` so `widgets::PropertyPanelAction` is unchanged.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignValue {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextVerticalAlignValue {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextGrowthValue {
    Auto,
    FixedWidth,
    FixedWidthHeight,
}

// The font-family catalogue (bundled + system enumeration + search
// filter) lives in `property_panel_typography.rs`; the picker emits
// `SetFontFamilyIndex` into its visible-entries list.

/// Named font-weight options for the typography weight dropdown — a
/// port of the TS `WEIGHT_OPTIONS` (thin … black).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeightChoice {
    Thin,
    ExtraLight,
    Light,
    Regular,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeightChoice {
    pub const ALL: [Self; 9] = [
        Self::Thin,
        Self::ExtraLight,
        Self::Light,
        Self::Regular,
        Self::Medium,
        Self::Semibold,
        Self::Bold,
        Self::ExtraBold,
        Self::Black,
    ];

    /// Numeric weight written to the node.
    pub fn value(self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::ExtraLight => 200,
            Self::Light => 300,
            Self::Regular => 400,
            Self::Medium => 500,
            Self::Semibold => 600,
            Self::Bold => 700,
            Self::ExtraBold => 800,
            Self::Black => 900,
        }
    }

    /// The bare numeric weight string (`"100"` … `"900"`) for the
    /// dropdown's "number + name" rows.
    pub fn numeric_label(self) -> &'static str {
        match self {
            Self::Thin => "100",
            Self::ExtraLight => "200",
            Self::Light => "300",
            Self::Regular => "400",
            Self::Medium => "500",
            Self::Semibold => "600",
            Self::Bold => "700",
            Self::ExtraBold => "800",
            Self::Black => "900",
        }
    }

    /// i18n key for the named label (`text.weight.*`).
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Thin => "text.weight.thin",
            Self::ExtraLight => "text.weight.extralight",
            Self::Light => "text.weight.light",
            Self::Regular => "text.weight.regular",
            Self::Medium => "text.weight.medium",
            Self::Semibold => "text.weight.semibold",
            Self::Bold => "text.weight.bold",
            Self::ExtraBold => "text.weight.extrabold",
            Self::Black => "text.weight.black",
        }
    }

    /// The named choice nearest to a numeric weight — used to show the
    /// node's current weight in the dropdown trigger.
    pub fn nearest(weight: u16) -> Self {
        Self::ALL
            .into_iter()
            .min_by_key(|c| (c.value() as i32 - weight as i32).abs())
            .unwrap_or(Self::Regular)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlignValue {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutJustifyValue {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

/// Actions the Code panel emits. SelectFramework + Copy mutate state
/// directly; Generate/Regenerate/Cancel raise pending flags the host
/// codegen session (P3) drains; Download/ExportBundle are host file IO.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodegenAction {
    SelectFramework(op_editor_core::codegen::Framework),
    Generate,
    Regenerate,
    Cancel,
    Copy,
    Download,
    ExportBundle,
    /// Scroll the framework tab strip one step left (left chevron).
    ScrollFrameworksLeft,
    /// Scroll the framework tab strip one step right (right chevron).
    ScrollFrameworksRight,
}

/// Button / checkbox actions in the property panel that don't
/// map to a text input. The host dispatches these in `apply_press`
/// after the text-input hit-test misses.
///
/// `PartialEq` only (not `Eq`) — `AdjustEffectParam` carries an `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyPanelAction {
    /// User clicked a tab in the pinned Design / Code strip — the host
    /// switches `editor_ui.property_tab`.
    SetPropertyTab(op_editor_core::PropertyTab),
    SetFlexLayout(op_editor_core::FlexLayout),
    ToggleSizeFillWidth,
    ToggleSizeFillHeight,
    ToggleSizeHugWidth,
    ToggleSizeHugHeight,
    ToggleSizeClipContent,
    SetLayoutAlign(LayoutAlignValue),
    SetLayoutJustify(LayoutJustifyValue),
    SetLayoutAlignment {
        justify: LayoutJustifyValue,
        align: LayoutAlignValue,
    },
    /// User clicked the header "Create Component" affordance.
    CreateComponent,
    /// User clicked "Detach component" on a reusable component —
    /// host sheds the `reusable` flag (TS `detachComponent` case 1).
    DetachComponent,
    /// User clicked "Go to component" on an instance — host selects
    /// the master component node.
    GoToComponent,
    /// User clicked "Detach instance" — host materializes the Ref
    /// into an independent subtree (TS `detachComponent` case 2).
    DetachInstance,
    /// User clicked fill `index`'s fill-type dropdown — host toggles
    /// the picker open for that fill (`fill_type_picker` + the index
    /// it targets).
    ToggleFillTypePicker(usize),
    /// User picked a fill type from the dropdown for fill `index`.
    SetFillType {
        index: usize,
        fill_type: op_editor_core::FillType,
    },
    /// User clicked the Fill section header "+" — appends a new fill.
    AddFill,
    /// User clicked fill `index`'s row remove button — removes that
    /// fill.
    RemoveFill(usize),
    /// User clicked the gradient-stops header "+".
    AddGradientStop,
    /// User clicked a gradient stop row remove button.
    RemoveGradientStop(usize),
    /// User clicked a colour swatch (Fill or Stroke section). Host
    /// opens the floating colour picker tied to that target.
    OpenColorPicker(op_editor_core::ColorTarget),
    /// User clicked the colour swatch on fill `index`'s Solid body row.
    /// Host opens the HSV picker bound to that fill via
    /// `open_color_picker_for_fill`. A separate variant from
    /// `OpenColorPicker(ColorTarget::Fill)` so the fill index rides
    /// along without widening the shared `ColorTarget` enum (which
    /// would ripple across the colour-variable / HSV systems).
    OpenFillColorPicker(usize),
    /// User clicked the `{}` affordance beside a fill/stroke colour
    /// row. Host toggles the colour-variable picker for that target.
    ToggleColorVariablePicker(op_editor_core::ColorTarget),
    /// User picked a colour variable row from the inline picker.
    BindColorVariable {
        target: op_editor_core::ColorTarget,
        index: usize,
    },
    /// User clicked the current variable binding row to resolve it
    /// back into a concrete colour.
    UnbindColorVariable(op_editor_core::ColorTarget),
    /// User clicked the Export section's scale dropdown — host
    /// toggles `editor_ui.export_scale_picker_open` (the inline
    /// 1x/2x/3x select popup). Does NOT open the Export modal —
    /// that is reached only via File ▸ Export Image (⇧⌘P).
    ToggleExportScalePicker,
    /// User clicked the Export section's format dropdown — host
    /// toggles `editor_ui.export_format_picker_open` (the inline
    /// PNG/JPEG/WEBP/SVG/PDF select popup).
    ToggleExportFormatPicker,
    /// User picked a scale (1.0 / 2.0 / 3.0) from the inline popup.
    SetExportScale(f32),
    /// User picked a format from the inline popup.
    SetExportFormat(op_editor_core::ExportFormat),
    /// User clicked the Export section's Export button — host queues
    /// `FileAction::ExportImageConfirm` so the document exports at
    /// the chosen scale + format (pops the native Save dialog).
    ExportImageNow,
    /// User clicked the Effects section's "+" — host appends a
    /// default drop shadow to the selected node.
    AddEffect,
    /// User clicked the "✕" on an effect row — host removes the
    /// effect at this index from the selected node.
    RemoveEffect(usize),
    /// User clicked a "−" / "+" stepper on an effect parameter row.
    /// `new_value` is the post-step value (the walker computed it
    /// from the current value ± the step); the host writes it via
    /// `EditorCommand::SetEffectParam`.
    AdjustEffectParam {
        effect: usize,
        field: op_editor_core::EffectField,
        new_value: f32,
    },
    /// User clicked an effect parameter's value — host focuses it
    /// for keyboard entry (`editor_ui.effect_param_focus`). `value`
    /// is the current committed value, used to seed the draft.
    FocusEffectParam {
        effect: usize,
        field: op_editor_core::EffectField,
        value: f32,
    },
    /// User clicked the colour swatch on a Shadow effect's colour
    /// row — host opens the HSV picker bound to
    /// `effect[index].color`.
    OpenEffectColorPicker(usize),
    /// User clicked the `图片` fill body row — host opens an image
    /// fill popover, matching the TS right-panel image editor.
    ToggleImageFillPopover,
    /// User clicked the popover close button.
    CloseImageFillPopover,
    /// User picked a fill/fit/crop/tile mode in the image-fill
    /// popover.
    SetImageFillMode(op_editor_core::ImageFillMode),
    /// User clicked the image-fill popover's upload well — host opens
    /// a file picker and writes the chosen file into the selected
    /// node's primary fill as `PenFill::Image { url: <data-url> }`.
    PickFillImage,
    /// User clicked one of the image adjustment tracks.
    SetImageAdjustment {
        field: op_editor_core::ImageAdjustmentField,
        value: f32,
    },
    /// User clicked the image adjustment reset affordance.
    ResetImageAdjustments,
    /// User clicked the Icon section's icon/library row — host opens
    /// the native Lucide picker in replace-selection mode.
    OpenSelectedIconPicker,
    SetTextAlign(TextAlignValue),
    SetTextVerticalAlign(TextVerticalAlignValue),
    SetTextGrowth(TextGrowthValue),
    ToggleFontFamilyPicker,
    /// User clicked a row in the font-family picker. The index is
    /// into `property_panel_typography::font_picker_entries(...)`
    /// built from the SAME (system list, search) inputs — the host
    /// re-derives the list to resolve the family string.
    SetFontFamilyIndex(usize),
    /// User clicked the image section's Search button — host toggles
    /// `editor_ui.image_panel.search_open` (and seeds the query from
    /// `imageSearchQuery ?? name`, TS `initialQuery`).
    ToggleImageSearchPopover,
    /// User clicked the image section's Generate button.
    ToggleImageGeneratePopover,
    /// User submitted the search popover's query (Enter or the
    /// search icon-button).
    RunImageSearch,
    /// User clicked a result cell — host writes its thumb URL into
    /// the selected image node's `src` (TS `onSelect(thumbUrl)`).
    SelectImageSearchResult(usize),
    /// User clicked Generate in the generate popover's idle view.
    RunImageGenerate,
    /// User clicked Apply on the generated preview — host writes the
    /// preview URL into the node's `src` and closes the popover.
    ApplyGeneratedImage,
    /// User clicked Retry on the preview — back to the idle view.
    RetryImageGenerate,
    /// User clicked "Open Settings" in the not-configured view —
    /// host opens the settings modal on the Images tab.
    OpenImageGenSettings,
    /// User clicked Relink on the local-asset warning row — host
    /// pops a file dialog and rewrites the image node's `src`.
    RelinkImage,
    /// User clicked the typography weight dropdown — host toggles
    /// `editor_ui.font_weight_picker_open`.
    ToggleFontWeightPicker,
    /// User picked a named weight from the dropdown.
    SetFontWeight(FontWeightChoice),
    /// User clicked the padding-section gear — host toggles
    /// `editor_ui.padding_mode_popover_open`.
    TogglePaddingModePopover,
    /// User picked a padding edit mode in the gear popover — host pins
    /// `editor_ui.padding_edit_mode` + reshapes the value.
    SetPaddingMode(op_editor_core::PaddingEditMode),
    /// User clicked the stroke-section gear — host toggles
    /// `editor_ui.stroke_mode_popover_open`.
    ToggleStrokeModePopover,
    /// User picked a stroke edit mode in the gear popover — host pins
    /// `editor_ui.stroke_edit_mode` + reshapes side widths.
    SetStrokeMode(op_editor_core::PaddingEditMode),
    /// User clicked the Widget section's `checked` toggle on a Switch
    /// / Checkbox node — host flips the literal `checked` bool to
    /// `new_value`.
    ToggleWidgetChecked(bool),
    /// Code panel action — see `CodegenAction`.
    Codegen(CodegenAction),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codegen_action_wraps_into_property_action() {
        use op_editor_core::codegen::Framework;
        let a = PropertyPanelAction::Codegen(CodegenAction::SelectFramework(Framework::Vue));
        assert_eq!(
            a,
            PropertyPanelAction::Codegen(CodegenAction::SelectFramework(Framework::Vue))
        );
        // distinct variants are not equal
        assert_ne!(
            PropertyPanelAction::Codegen(CodegenAction::Generate),
            PropertyPanelAction::Codegen(CodegenAction::Copy)
        );
    }
}
