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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamilyChoice {
    Inter,
    Poppins,
    Roboto,
    Montserrat,
    OpenSans,
    Lato,
    Raleway,
    DmSans,
    PlayfairDisplay,
    Nunito,
    SourceSans3,
    Arial,
    Helvetica,
    Georgia,
    CourierNew,
}

impl FontFamilyChoice {
    pub const ALL: [Self; 15] = [
        Self::Inter,
        Self::Poppins,
        Self::Roboto,
        Self::Montserrat,
        Self::OpenSans,
        Self::Lato,
        Self::Raleway,
        Self::DmSans,
        Self::PlayfairDisplay,
        Self::Nunito,
        Self::SourceSans3,
        Self::Arial,
        Self::Helvetica,
        Self::Georgia,
        Self::CourierNew,
    ];

    pub fn family(self) -> &'static str {
        match self {
            Self::Inter => "Inter",
            Self::Poppins => "Poppins",
            Self::Roboto => "Roboto",
            Self::Montserrat => "Montserrat",
            Self::OpenSans => "Open Sans",
            Self::Lato => "Lato",
            Self::Raleway => "Raleway",
            Self::DmSans => "DM Sans",
            Self::PlayfairDisplay => "Playfair Display",
            Self::Nunito => "Nunito",
            Self::SourceSans3 => "Source Sans 3",
            Self::Arial => "Arial",
            Self::Helvetica => "Helvetica",
            Self::Georgia => "Georgia",
            Self::CourierNew => "Courier New",
        }
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

/// Button / checkbox actions in the property panel that don't
/// map to a text input. The host dispatches these in `apply_press`
/// after the text-input hit-test misses.
///
/// `PartialEq` only (not `Eq`) — `AdjustEffectParam` carries an `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyPanelAction {
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
    /// User clicked the Fill section's fill-type dropdown — host
    /// toggles `Document.ui.fill_type_picker_open`.
    ToggleFillTypePicker,
    /// User picked a fill type from the dropdown.
    SetFillType(op_editor_core::FillType),
    /// User clicked the Fill section header "+".
    AddFill,
    /// User clicked the Fill section row remove button.
    RemoveFill,
    /// User clicked the gradient-stops header "+".
    AddGradientStop,
    /// User clicked a gradient stop row remove button.
    RemoveGradientStop(usize),
    /// User clicked a colour swatch (Fill or Stroke section). Host
    /// opens the floating colour picker tied to that target.
    OpenColorPicker(op_editor_core::ColorTarget),
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
    SetFontFamily(FontFamilyChoice),
}
