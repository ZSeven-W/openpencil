//! Exhaustive collaboration policy projection for PropertyPanel actions.
//!
//! Text-field commits are classified separately through
//! `PropertyFocus::collab_document_mutation`. Keeping this button-action
//! match exhaustive makes every new panel action choose its collaboration
//! semantics before it can compile.

use crate::widgets::PropertyPanelAction;

pub fn collab_gate_action(action: &PropertyPanelAction) -> op_editor_core::CollabGateAction {
    use op_editor_core::{
        CollabDocumentMutation as D, CollabGateAction as G, CollabNodeField as F,
        CollabUnsupportedFeature as U,
    };
    use PropertyPanelAction as A;

    match action {
        A::SetFlexLayout(_) => G::Document(D::NodeProperty(F::Layout)),
        A::ToggleSizeFillWidth | A::ToggleSizeHugWidth => G::Document(D::NodeProperty(F::Width)),
        A::ToggleSizeFillHeight | A::ToggleSizeHugHeight => G::Document(D::NodeProperty(F::Height)),
        A::ToggleSizeClipContent => G::Document(D::NodeProperty(F::ClipContent)),
        A::SetLayoutAlign(_) => G::Document(D::NodeProperty(F::AlignItems)),
        A::SetLayoutJustify(_) => G::Document(D::NodeProperty(F::JustifyContent)),
        A::SetLayoutAlignment { .. } => G::Document(D::NodePropertyBatch),
        A::SetPaddingMode(_) => G::Document(D::NodeProperty(F::Padding)),
        A::SetStrokeMode(_) => G::Document(D::NodeProperty(F::Stroke)),

        A::CreateComponent
        | A::DetachComponent
        | A::DetachInstance
        | A::SetInstanceComponent(_) => G::Document(D::Unsupported(U::Components)),
        A::SetNodeBlendMode(_)
        | A::SetNodeMaskType(_)
        | A::SetFillBlendMode { .. }
        | A::SetFillRule(_)
        | A::SetFillType { .. }
        | A::AddFill
        | A::RemoveFill(_)
        | A::MoveFill { .. }
        | A::AddGradientStop
        | A::RemoveGradientStop(_)
        | A::ToggleWidgetChecked(_)
        | A::SetInteractionNavigate { .. }
        | A::SetInteractionPop
        | A::RemoveInteraction => G::Document(D::Unsupported(U::UnsupportedNodeProperty)),
        A::BindColorVariable { .. } | A::UnbindColorVariable(_) => {
            G::Document(D::Unsupported(U::VariablesThemes))
        }
        A::AddEffect(_)
        | A::SetEffectVisible(_, _)
        | A::RemoveEffect(_)
        | A::AdjustEffectParam { .. } => G::Document(D::Unsupported(U::Effects)),
        A::SetImageFillMode(_)
        | A::PickFillImage
        | A::SetImageAdjustment { .. }
        | A::ResetImageAdjustments
        | A::MatchImageAspectRatio
        | A::SelectImageSearchResult(_)
        | A::ApplyGeneratedImage
        | A::RelinkImage => G::Document(D::Unsupported(U::ExternalAssets)),
        A::ClearPageBackground => G::Document(D::Unsupported(U::PageBackground)),
        A::SetTextAlign(_)
        | A::SetTextVerticalAlign(_)
        | A::SetTextGrowth(_)
        | A::SetFontFamilyIndex(_)
        | A::SetFontWeight(_) => G::Document(D::Unsupported(U::Typography)),

        A::SetPropertyTab(_)
        | A::ToggleCornerExpand
        | A::ToggleCompositingPicker(_)
        | A::GoToComponent
        | A::ToggleInstanceComponentPicker
        | A::ToggleFillTypePicker(_)
        | A::OpenColorPicker(_)
        | A::OpenFillColorPicker(_)
        | A::ToggleColorVariablePicker(_)
        | A::ToggleExportScalePicker
        | A::ToggleExportFormatPicker
        | A::SetExportScale(_)
        | A::SetExportFormat(_)
        | A::ExportImageNow
        | A::ToggleEffectAddPicker
        | A::FocusEffectParam { .. }
        | A::OpenEffectColorPicker(_)
        | A::ToggleImageFillPopover
        | A::CloseImageFillPopover
        | A::OpenSelectedIconPicker
        | A::ToggleFontFamilyPicker
        | A::ImportFont
        | A::RemoveImportedFont(_)
        | A::ToggleImageSearchPopover
        | A::ToggleImageGeneratePopover
        | A::RunImageSearch
        | A::RunImageGenerate
        | A::RetryImageGenerate
        | A::OpenImageGenSettings
        | A::ToggleFontWeightPicker
        | A::TogglePaddingModePopover
        | A::ToggleStrokeModePopover
        | A::ToggleInteractionMenu
        | A::Codegen(_) => G::LocalUi,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{
        CollabDocumentMutation as D, CollabGateAction as G, CollabNodeField as F,
        CollabUnsupportedFeature as U,
    };

    #[test]
    fn classifies_allowed_unsupported_and_local_actions() {
        assert_eq!(
            collab_gate_action(&PropertyPanelAction::ToggleSizeClipContent),
            G::Document(D::NodeProperty(F::ClipContent))
        );
        assert_eq!(
            collab_gate_action(&PropertyPanelAction::ClearPageBackground),
            G::Document(D::Unsupported(U::PageBackground))
        );
        assert_eq!(
            collab_gate_action(&PropertyPanelAction::ToggleCornerExpand),
            G::LocalUi
        );
    }
}
