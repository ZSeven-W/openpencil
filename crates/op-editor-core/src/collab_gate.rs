//! Shared M1 collaboration mutation policy.
//!
//! This is intentionally transport-neutral. Native and web hosts ask the same
//! policy before invoking a mutator; the collaboration transaction/diff layer
//! remains the final fail-closed check after a gesture. Standalone behavior is
//! unchanged: the M1 support matrix applies only while a document is bound to
//! a collaboration session.

use crate::collab_ui_state::{
    CollabConnectionPhase, CollabNoticeKind, CollabPendingEditUi, CollabRejectUiCode, CollabUiRole,
    CollabUiState,
};
use crate::{EditorCommand, EditorState, IdAllocError, IdAllocator, NodeFlag, PropertyFocus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabEditSource {
    User,
    Ai,
    Mcp,
    Import,
    ExternalSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabNodeField {
    Name,
    X,
    Y,
    Rotation,
    Opacity,
    Width,
    Height,
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
    Layout,
    Gap,
    Padding,
    JustifyContent,
    AlignItems,
    ClipContent,
    CornerRadius,
    Fill,
    Stroke,
    Content,
    X2,
    Y2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabUnsupportedFeature {
    PageStructure,
    PageBackground,
    VariablesThemes,
    Components,
    UIKit,
    ExternalAssets,
    ClipboardPaste,
    Duplicate,
    BulkWrite,
    ReplaceDocument,
    RootMetadata,
    Typography,
    Effects,
    VisibilityAndLocking,
    NodeReplacement,
    UnsupportedNodeProperty,
    UnsupportedNodeKind,
}

impl CollabUnsupportedFeature {
    pub const fn i18n_key(self) -> &'static str {
        match self {
            Self::PageStructure => "collab.gate.pages",
            Self::PageBackground => "collab.gate.pageBackground",
            Self::VariablesThemes => "collab.gate.variablesThemes",
            Self::Components => "collab.gate.components",
            Self::UIKit => "collab.gate.uikit",
            Self::ExternalAssets => "collab.gate.externalAssets",
            Self::ClipboardPaste => "collab.gate.clipboardPaste",
            Self::Duplicate => "collab.gate.duplicate",
            Self::BulkWrite => "collab.gate.bulkWrite",
            Self::ReplaceDocument => "collab.gate.replaceDocument",
            Self::RootMetadata => "collab.gate.rootMetadata",
            Self::Typography => "collab.gate.typography",
            Self::Effects => "collab.gate.effects",
            Self::VisibilityAndLocking => "collab.gate.visibilityLocking",
            Self::NodeReplacement => "collab.gate.nodeReplacement",
            Self::UnsupportedNodeProperty => "collab.gate.nodeProperty",
            Self::UnsupportedNodeKind => "collab.gate.nodeKind",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabDocumentMutation {
    BasicNodeInsert,
    NodeDelete,
    NodeMove,
    Group,
    Ungroup,
    NodeProperty(CollabNodeField),
    NodePropertyBatch,
    Unsupported(CollabUnsupportedFeature),
}

impl CollabDocumentMutation {
    pub const fn unsupported_feature(self) -> Option<CollabUnsupportedFeature> {
        match self {
            Self::Unsupported(feature) => Some(feature),
            Self::BasicNodeInsert
            | Self::NodeDelete
            | Self::NodeMove
            | Self::Group
            | Self::Ungroup
            | Self::NodeProperty(_)
            | Self::NodePropertyBatch => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabGateAction {
    /// Selection, viewport, tool, copy, export, and other non-document state.
    LocalUi,
    Document(CollabDocumentMutation),
    /// Existing global snapshot undo. W6 replaces this with selective undo.
    GlobalUndo,
    GlobalRedo,
    SaveShared,
    /// Save a detached copy; the host must clear collaboration file identity.
    SaveFork,
    /// New/Open/Open Recent/whole-document replacement.
    ReplaceDocument,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabGateReason {
    SessionTransition,
    ReadOnly,
    PendingEdit,
    Unsupported(CollabUnsupportedFeature),
    AiMcpDisabled,
    UndoUnavailable,
    RedoUnavailable,
    OwnerOnlySave,
    LeaveSessionFirst,
}

/// Fail-closed error from a policy-checked collaboration command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabApplyError {
    Gate(CollabGateReason),
    IdAllocation(IdAllocError),
}

impl std::fmt::Display for CollabApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gate(error) => write!(f, "collaboration command rejected: {error:?}"),
            Self::IdAllocation(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for CollabApplyError {}

impl CollabGateReason {
    pub const fn i18n_key(self) -> &'static str {
        match self {
            Self::SessionTransition => "collab.gate.sessionTransition",
            Self::ReadOnly => "collab.gate.readOnly",
            Self::PendingEdit => "collab.gate.pendingEdit",
            Self::Unsupported(feature) => feature.i18n_key(),
            Self::AiMcpDisabled => "collab.gate.aiMcp",
            Self::UndoUnavailable => "collab.gate.undoUnavailable",
            Self::RedoUnavailable => "collab.gate.redoUnavailable",
            Self::OwnerOnlySave => "collab.gate.ownerOnlySave",
            Self::LeaveSessionFirst => "collab.gate.leaveSessionFirst",
        }
    }

    /// Bounded, credential-free UI projection for a rejected local action.
    ///
    /// Keeping this mapping beside the policy prevents native UI, live MCP,
    /// and future hosts from disagreeing about the reason shown to the user.
    pub const fn notice_kind(self) -> CollabNoticeKind {
        match self {
            Self::Unsupported(feature) => CollabNoticeKind::UnsupportedEdit(feature),
            Self::ReadOnly | Self::SessionTransition | Self::OwnerOnlySave => {
                CollabNoticeKind::Reject(CollabRejectUiCode::ReadOnly)
            }
            Self::PendingEdit => CollabNoticeKind::Reject(CollabRejectUiCode::Conflict),
            Self::AiMcpDisabled | Self::UndoUnavailable | Self::RedoUnavailable => {
                CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported)
            }
            Self::LeaveSessionFirst => {
                CollabNoticeKind::UnsupportedEdit(CollabUnsupportedFeature::ReplaceDocument)
            }
        }
    }
}

impl PropertyFocus {
    /// Exhaustive M1 classification for every editable PropertyPanel draft.
    pub const fn collab_document_mutation(self) -> CollabDocumentMutation {
        use CollabDocumentMutation as D;
        use CollabNodeField as F;
        use CollabUnsupportedFeature as U;
        use PropertyFocus as P;

        match self {
            P::PositionX => D::NodeProperty(F::X),
            P::PositionY => D::NodeProperty(F::Y),
            P::Rotation => D::NodeProperty(F::Rotation),
            P::PositionR | P::CornerTL | P::CornerTR | P::CornerBL | P::CornerBR => {
                D::NodeProperty(F::CornerRadius)
            }
            P::SizeW => D::NodeProperty(F::Width),
            P::SizeH => D::NodeProperty(F::Height),
            P::LayoutGap => D::NodeProperty(F::Gap),
            P::PaddingTop | P::PaddingRight | P::PaddingBottom | P::PaddingLeft => {
                D::NodeProperty(F::Padding)
            }
            P::Opacity => D::NodeProperty(F::Opacity),
            P::FillHex(_) | P::FillOpacity(_) => D::NodeProperty(F::Fill),
            P::StrokeHex
            | P::StrokeWidth
            | P::StrokeTopWidth
            | P::StrokeRightWidth
            | P::StrokeBottomWidth
            | P::StrokeLeftWidth => D::NodeProperty(F::Stroke),

            P::PageBackgroundHex => D::Unsupported(U::PageBackground),
            P::EffectRadius(_) => D::Unsupported(U::Effects),
            P::ImageTileScale => D::Unsupported(U::ExternalAssets),
            P::GradientAngle | P::GradientStopHex(_) | P::GradientStopOffset(_) => {
                D::Unsupported(U::UnsupportedNodeProperty)
            }
            P::PolygonSides | P::EllipseStart | P::EllipseSweep | P::EllipseInnerRadius => {
                D::Unsupported(U::UnsupportedNodeProperty)
            }
            P::FontSize | P::FontWeight | P::LineHeight | P::LetterSpacing => {
                D::Unsupported(U::Typography)
            }
            P::WidgetPlaceholder
            | P::WidgetValue
            | P::WidgetLabel
            | P::WidgetLeadingIcon
            | P::WidgetTrailingIcon
            | P::WidgetBindKey
            | P::WidgetMin
            | P::WidgetMax
            | P::WidgetStep => D::Unsupported(U::UnsupportedNodeProperty),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollabGatePolicy {
    pub phase: CollabConnectionPhase,
    pub role: Option<CollabUiRole>,
    pub pending_edit: CollabPendingEditUi,
}

impl Default for CollabGatePolicy {
    fn default() -> Self {
        Self {
            phase: CollabConnectionPhase::Idle,
            role: None,
            pending_edit: CollabPendingEditUi::None,
        }
    }
}

impl From<&CollabUiState> for CollabGatePolicy {
    fn from(state: &CollabUiState) -> Self {
        Self {
            phase: state.phase,
            role: state.authenticated_session().map(|session| session.role),
            pending_edit: state.pending_edit,
        }
    }
}

impl CollabGatePolicy {
    pub fn check(
        self,
        action: CollabGateAction,
        source: CollabEditSource,
    ) -> Result<(), CollabGateReason> {
        if action == CollabGateAction::LocalUi {
            return Ok(());
        }

        // Discovery does not bind or replace the current document.
        if matches!(
            self.phase,
            CollabConnectionPhase::Idle | CollabConnectionPhase::Discovering
        ) {
            return Ok(());
        }

        if action == CollabGateAction::SaveFork {
            return Ok(());
        }

        if matches!(
            self.phase,
            CollabConnectionPhase::Starting
                | CollabConnectionPhase::Joining
                | CollabConnectionPhase::Authenticating
        ) {
            return Err(CollabGateReason::SessionTransition);
        }

        match action {
            CollabGateAction::LocalUi | CollabGateAction::SaveFork => Ok(()),
            CollabGateAction::GlobalUndo => Err(CollabGateReason::UndoUnavailable),
            CollabGateAction::GlobalRedo => Err(CollabGateReason::RedoUnavailable),
            CollabGateAction::ReplaceDocument => Err(CollabGateReason::LeaveSessionFirst),
            CollabGateAction::SaveShared => {
                if self.phase == CollabConnectionPhase::Active
                    && self.role == Some(CollabUiRole::Owner)
                {
                    Ok(())
                } else if self.phase == CollabConnectionPhase::Active {
                    Err(CollabGateReason::OwnerOnlySave)
                } else {
                    Err(CollabGateReason::ReadOnly)
                }
            }
            CollabGateAction::Document(mutation) => {
                if self.phase != CollabConnectionPhase::Active
                    || !matches!(self.role, Some(CollabUiRole::Owner | CollabUiRole::Editor))
                {
                    return Err(CollabGateReason::ReadOnly);
                }
                if self.pending_edit.blocks_another_edit() {
                    return Err(CollabGateReason::PendingEdit);
                }
                if matches!(source, CollabEditSource::Ai | CollabEditSource::Mcp) {
                    return Err(CollabGateReason::AiMcpDisabled);
                }
                if let Some(feature) = mutation.unsupported_feature() {
                    return Err(CollabGateReason::Unsupported(feature));
                }
                if matches!(
                    source,
                    CollabEditSource::Import | CollabEditSource::ExternalSync
                ) {
                    return Err(CollabGateReason::Unsupported(
                        CollabUnsupportedFeature::BulkWrite,
                    ));
                }
                Ok(())
            }
        }
    }

    pub fn check_command(
        self,
        command: &EditorCommand,
        source: CollabEditSource,
    ) -> Result<(), CollabGateReason> {
        self.check(command.collab_gate_action(), source)
    }
}

impl CollabUiState {
    pub fn gate(
        &self,
        action: CollabGateAction,
        source: CollabEditSource,
    ) -> Result<(), CollabGateReason> {
        CollabGatePolicy::from(self).check(action, source)
    }
}

impl EditorCommand {
    /// Exhaustive M1 classification. Adding a command variant forces this
    /// match to be updated instead of silently enabling a new mutation.
    pub fn collab_gate_action(&self) -> CollabGateAction {
        use CollabDocumentMutation as D;
        use CollabGateAction as A;
        use CollabNodeField as F;
        use CollabUnsupportedFeature as U;
        use EditorCommand as C;

        match self {
            C::SetVariableColor { .. }
            | C::SetVariableScalar { .. }
            | C::CreateVariable { .. }
            | C::DeleteVariable { .. }
            | C::RenameVariable { .. }
            | C::SetVariables { .. }
            | C::UpsertVariables { .. }
            | C::SetThemes { .. }
            | C::MergeThemePreset { .. } => A::Document(D::Unsupported(U::VariablesThemes)),

            // Active theme pins are editor-local preview state, not document
            // variables/theme definitions.
            C::SetActiveAxisValue { .. } | C::CycleActiveAxisValue { .. } => A::LocalUi,

            C::InsertNode { kind, .. } => {
                if is_basic_m1_node_kind(kind) {
                    A::Document(D::BasicNodeInsert)
                } else {
                    A::Document(D::Unsupported(U::UnsupportedNodeKind))
                }
            }
            C::UpdateNode { .. } => A::Document(D::NodePropertyBatch),
            C::PatchNodeData { .. } => A::Document(D::Unsupported(U::UnsupportedNodeProperty)),
            C::DeleteNode { .. } | C::DeleteSelected => A::Document(D::NodeDelete),
            C::MoveNode { .. } | C::ReorderSelected { .. } => A::Document(D::NodeMove),
            C::CopyNode { .. } | C::DuplicateSelected { .. } => {
                A::Document(D::Unsupported(U::Duplicate))
            }
            C::ReplaceNode { .. } | C::ReplaceSubtree { .. } => {
                A::Document(D::Unsupported(U::NodeReplacement))
            }
            C::BatchInsert { .. }
            | C::InsertSubtree { .. }
            | C::InsertAuthoredSubtree { .. }
            | C::RefineDesign { .. }
            | C::Batch { .. }
            | C::ReplaceAllMatchingProperties { .. }
            | C::PromoteLegacyWidgets => A::Document(D::Unsupported(U::BulkWrite)),

            C::MergeAppState { .. } | C::SetDesignMd { .. } | C::UpsertScreen { .. } => {
                A::Document(D::Unsupported(U::RootMetadata))
            }
            C::UpsertComponent { .. }
            | C::InstantiateComponent { .. }
            | C::CreateComponent { .. }
            | C::DeleteComponent { .. }
            | C::RenameComponent { .. } => A::Document(D::Unsupported(U::Components)),
            C::InstantiateKitComponent { .. } => A::Document(D::Unsupported(U::UIKit)),

            C::SetActivePage { .. }
            | C::ClearSelection
            | C::SetSelection { .. }
            | C::SetSelectionSet { .. }
            | C::ToggleNodeSelection { .. }
            | C::SetViewport { .. }
            | C::SetActiveTool { .. }
            | C::CopySelected => A::LocalUi,
            C::AddPage { .. }
            | C::RenamePage { .. }
            | C::DeletePage { .. }
            | C::DuplicatePage { .. }
            | C::ReorderPage { .. } => A::Document(D::Unsupported(U::PageStructure)),

            C::SetNodeFlag {
                flag: NodeFlag::Collapsed,
                ..
            } => A::LocalUi,
            C::SetNodeFlag { .. } => A::Document(D::Unsupported(U::VisibilityAndLocking)),
            C::SetNodeFlip { .. } | C::SetEllipseArc { .. } => {
                A::Document(D::Unsupported(U::UnsupportedNodeProperty))
            }
            C::AddNodeEffect { .. }
            | C::RemoveNodeEffect { .. }
            | C::SetEffectParam { .. }
            | C::SetEffectColor { .. } => A::Document(D::Unsupported(U::Effects)),

            C::Undo => A::GlobalUndo,
            C::Redo => A::GlobalRedo,
            C::CutSelected | C::PasteClipboard { .. } => {
                A::Document(D::Unsupported(U::ClipboardPaste))
            }
            C::NudgeSelected { .. } | C::AlignSelected { .. } => A::Document(D::NodePropertyBatch),
            C::GroupSelected => A::Document(D::Group),
            C::UngroupSelected => A::Document(D::Ungroup),
            C::SetNodeRotation { .. } => A::Document(D::NodeProperty(F::Rotation)),
            C::SetNodeText { .. } => A::Document(D::NodeProperty(F::Content)),
            C::SetNodeCornerRadius { .. } => A::Document(D::NodeProperty(F::CornerRadius)),
            C::SetNodeFontSize { .. }
            | C::SetNodeFontWeight { .. }
            | C::ReplaceFontFamily { .. } => A::Document(D::Unsupported(U::Typography)),
            C::SetNodeStrokeHex { .. }
            | C::SetNodeStrokeWidth { .. }
            | C::SetNodeStrokeSideWidth { .. } => A::Document(D::NodeProperty(F::Stroke)),
            C::SetNodeFillHex { .. } => A::Document(D::NodeProperty(F::Fill)),
            C::SetNodeName { .. } => A::Document(D::NodeProperty(F::Name)),
            C::ImportSvg { .. } => A::Document(D::Unsupported(U::ExternalAssets)),
            C::SetNodeLayoutProp { property, .. } => match property.as_str() {
                "layout" => A::Document(D::NodeProperty(F::Layout)),
                "gap" => A::Document(D::NodeProperty(F::Gap)),
                "padding" => A::Document(D::NodeProperty(F::Padding)),
                "justifyContent" => A::Document(D::NodeProperty(F::JustifyContent)),
                "alignItems" => A::Document(D::NodeProperty(F::AlignItems)),
                "width" => A::Document(D::NodeProperty(F::Width)),
                "height" => A::Document(D::NodeProperty(F::Height)),
                "clipContent" => A::Document(D::NodeProperty(F::ClipContent)),
                "opacity" => A::Document(D::NodeProperty(F::Opacity)),
                "letterSpacing" | "lineHeight" | "fontFamily" | "textAlign"
                | "textAlignVertical" | "textGrowth" => A::Document(D::Unsupported(U::Typography)),
                _ => A::Document(D::Unsupported(U::UnsupportedNodeProperty)),
            },
        }
    }
}

impl EditorState {
    /// Apply a command only after the shared collaboration policy accepts it.
    ///
    /// Hosts should still wrap direct gesture mutators in the explicit
    /// `begin_local_edit` / `end_local_edit` transaction and run the exact M1
    /// diff validator. This helper closes the large command-based surface
    /// (keyboard, menu, AI/MCP adapters) without changing standalone `apply`.
    pub fn apply_collab_gated(
        &mut self,
        command: EditorCommand,
        policy: CollabGatePolicy,
        source: CollabEditSource,
    ) -> Result<bool, CollabGateReason> {
        policy.check_command(&command, source)?;
        Ok(self.apply(command))
    }

    /// Policy-check and apply a command with the session id allocator.
    ///
    /// The policy runs before any mutation or allocation.
    pub fn apply_collab_gated_with_allocator(
        &mut self,
        command: EditorCommand,
        policy: CollabGatePolicy,
        source: CollabEditSource,
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, CollabApplyError> {
        policy
            .check_command(&command, source)
            .map_err(CollabApplyError::Gate)?;
        self.apply_with_allocator(command, allocator)
            .map_err(CollabApplyError::IdAllocation)
    }
}

fn is_basic_m1_node_kind(kind: &str) -> bool {
    matches!(
        kind.trim().to_ascii_lowercase().as_str(),
        // `rect` is the canonical command spelling used by
        // `command_node::builders`; `rectangle` is accepted defensively for
        // callers that use the schema variant name.
        "rect" | "rectangle" | "ellipse" | "polygon" | "line" | "path" | "text" | "frame"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeId;

    fn active(role: CollabUiRole) -> CollabGatePolicy {
        CollabGatePolicy {
            phase: CollabConnectionPhase::Active,
            role: Some(role),
            pending_edit: CollabPendingEditUi::None,
        }
    }

    #[test]
    fn standalone_keeps_every_existing_action_available() {
        let policy = CollabGatePolicy::default();
        assert!(policy
            .check(
                CollabGateAction::Document(CollabDocumentMutation::Unsupported(
                    CollabUnsupportedFeature::RootMetadata,
                )),
                CollabEditSource::Mcp,
            )
            .is_ok());
        assert!(policy
            .check(CollabGateAction::GlobalUndo, CollabEditSource::User)
            .is_ok());
    }

    #[test]
    fn viewer_pending_and_external_sources_fail_closed() {
        assert_eq!(
            active(CollabUiRole::Viewer).check(
                CollabGateAction::Document(CollabDocumentMutation::NodeDelete),
                CollabEditSource::User,
            ),
            Err(CollabGateReason::ReadOnly)
        );
        let mut editor = active(CollabUiRole::Editor);
        editor.pending_edit = CollabPendingEditUi::Submitting;
        assert_eq!(
            editor.check(
                CollabGateAction::Document(CollabDocumentMutation::NodeDelete),
                CollabEditSource::User,
            ),
            Err(CollabGateReason::PendingEdit)
        );
        assert_eq!(
            active(CollabUiRole::Editor).check(
                CollabGateAction::Document(CollabDocumentMutation::NodeDelete),
                CollabEditSource::Ai,
            ),
            Err(CollabGateReason::AiMcpDisabled)
        );
    }

    #[test]
    fn save_undo_and_replace_follow_collaboration_semantics() {
        assert!(active(CollabUiRole::Owner)
            .check(CollabGateAction::SaveShared, CollabEditSource::User)
            .is_ok());
        assert_eq!(
            active(CollabUiRole::Editor)
                .check(CollabGateAction::SaveShared, CollabEditSource::User),
            Err(CollabGateReason::OwnerOnlySave)
        );
        assert_eq!(
            active(CollabUiRole::Owner).check(CollabGateAction::GlobalUndo, CollabEditSource::User),
            Err(CollabGateReason::UndoUnavailable)
        );
        assert_eq!(
            active(CollabUiRole::Owner)
                .check(CollabGateAction::ReplaceDocument, CollabEditSource::User),
            Err(CollabGateReason::LeaveSessionFirst)
        );
        assert!(active(CollabUiRole::Viewer)
            .check(CollabGateAction::SaveFork, CollabEditSource::User)
            .is_ok());
    }

    #[test]
    fn command_classification_is_allowlist_based() {
        let layout = EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new("n1"),
            property: "padding".to_string(),
            value: crate::LayoutPropValue::Number(8.0),
        };
        assert_eq!(
            layout.collab_gate_action(),
            CollabGateAction::Document(CollabDocumentMutation::NodeProperty(
                CollabNodeField::Padding
            ))
        );

        let typography = EditorCommand::SetNodeFontSize {
            node_id: NodeId::new("n1"),
            font_size: 16.0,
        };
        assert_eq!(
            typography.collab_gate_action(),
            CollabGateAction::Document(CollabDocumentMutation::Unsupported(
                CollabUnsupportedFeature::Typography
            ))
        );

        let icon = EditorCommand::InsertNode {
            kind: "icon".to_string(),
            name: "Icon".to_string(),
            x: 0,
            y: 0,
            width: 16,
            height: 16,
            fill_hex: None,
            target_parent: NodeId::NONE,
            page_id: None,
        };
        assert_eq!(
            icon.collab_gate_action(),
            CollabGateAction::Document(CollabDocumentMutation::Unsupported(
                CollabUnsupportedFeature::UnsupportedNodeKind
            ))
        );

        let rectangle = EditorCommand::InsertNode {
            kind: "rect".to_string(),
            name: "Rectangle".to_string(),
            x: 0,
            y: 0,
            width: 16,
            height: 16,
            fill_hex: None,
            target_parent: NodeId::NONE,
            page_id: None,
        };
        assert_eq!(
            rectangle.collab_gate_action(),
            CollabGateAction::Document(CollabDocumentMutation::BasicNodeInsert)
        );
    }

    #[test]
    fn property_drafts_and_notice_projection_are_typed() {
        assert_eq!(
            PropertyFocus::PositionX.collab_document_mutation(),
            CollabDocumentMutation::NodeProperty(CollabNodeField::X)
        );
        assert_eq!(
            PropertyFocus::FontSize.collab_document_mutation(),
            CollabDocumentMutation::Unsupported(CollabUnsupportedFeature::Typography)
        );
        assert_eq!(
            CollabGateReason::LeaveSessionFirst.notice_kind(),
            CollabNoticeKind::UnsupportedEdit(CollabUnsupportedFeature::ReplaceDocument)
        );
        assert_eq!(
            CollabGateReason::AiMcpDisabled.notice_kind(),
            CollabNoticeKind::Reject(CollabRejectUiCode::Unsupported)
        );
    }

    #[test]
    fn gated_apply_blocks_viewer_before_mutation() {
        let mut state = EditorState::new();
        let before = state.doc.clone();
        let result = state.apply_collab_gated(
            EditorCommand::AddPage {
                name: Some("Blocked".to_string()),
                children: None,
            },
            active(CollabUiRole::Viewer),
            CollabEditSource::User,
        );
        assert_eq!(result, Err(CollabGateReason::ReadOnly));
        assert_eq!(state.doc, before);
    }
}
