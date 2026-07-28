use crate::{
    ApplyLimits, CanonicalHash, CanonicalHashError, CollabApplyError, CollabTxn, PageRef,
    PeerNamespace, Role,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffContext {
    pub namespace: PeerNamespace,
    pub role: Role,
    pub next_id_counter: Option<u64>,
    pub limits: ApplyLimits,
}

impl DiffContext {
    pub fn new(namespace: PeerNamespace, role: Role, next_id_counter: Option<u64>) -> Self {
        Self {
            namespace,
            role,
            next_id_counter,
            limits: ApplyLimits::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SupportedNodeField {
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

impl SupportedNodeField {
    pub(crate) const fn wire_name(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::X => "x",
            Self::Y => "y",
            Self::Rotation => "rotation",
            Self::Opacity => "opacity",
            Self::Width => "width",
            Self::Height => "height",
            Self::MinWidth => "minWidth",
            Self::MaxWidth => "maxWidth",
            Self::MinHeight => "minHeight",
            Self::MaxHeight => "maxHeight",
            Self::Layout => "layout",
            Self::Gap => "gap",
            Self::Padding => "padding",
            Self::JustifyContent => "justifyContent",
            Self::AlignItems => "alignItems",
            Self::ClipContent => "clipContent",
            Self::CornerRadius => "cornerRadius",
            Self::Fill => "fill",
            Self::Stroke => "stroke",
            Self::Content => "content",
            Self::X2 => "x2",
            Self::Y2 => "y2",
        }
    }

    pub(crate) fn from_wire_name(value: &str) -> Option<Self> {
        Some(match value {
            "name" => Self::Name,
            "x" => Self::X,
            "y" => Self::Y,
            "rotation" => Self::Rotation,
            "opacity" => Self::Opacity,
            "width" => Self::Width,
            "height" => Self::Height,
            "minWidth" => Self::MinWidth,
            "maxWidth" => Self::MaxWidth,
            "minHeight" => Self::MinHeight,
            "maxHeight" => Self::MaxHeight,
            "layout" => Self::Layout,
            "gap" => Self::Gap,
            "padding" => Self::Padding,
            "justifyContent" => Self::JustifyContent,
            "alignItems" => Self::AlignItems,
            "clipContent" => Self::ClipContent,
            "cornerRadius" => Self::CornerRadius,
            "fill" => Self::Fill,
            "stroke" => Self::Stroke,
            "content" => Self::Content,
            "x2" => Self::X2,
            "y2" => Self::Y2,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Missing,
    Value(serde_json::Value),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeFieldChange {
    pub page: PageRef,
    pub node_id: String,
    pub field: SupportedNodeField,
    pub before: FieldValue,
    pub desired: FieldValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralMove {
    pub node_id: String,
    pub from_parent: Option<String>,
    pub from_index: u32,
    pub to_parent: Option<String>,
    pub to_index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructuralChanges {
    pub inserted_ids: Vec<String>,
    pub deleted_ids: Vec<String>,
    pub moves: Vec<StructuralMove>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditChanges {
    Property(Vec<NodeFieldChange>),
    Structure(StructuralChanges),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SupportedDiff {
    pub txn: CollabTxn,
    pub changes: EditChanges,
}

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("before document is invalid: {0}")]
    InvalidBefore(#[source] CollabApplyError),
    #[error("after document is invalid: {0}")]
    InvalidAfter(#[source] CollabApplyError),
    #[error("documents contain no collaboration-visible change")]
    NoDocumentChange,
    #[error("document field `{field}` is not supported by M1 collaboration")]
    UnsupportedDocumentField { field: String },
    #[error("page structure or metadata is not supported by M1 collaboration")]
    UnsupportedPageChange,
    #[error("node `{node_id}` changed canonical kind from `{before}` to `{after}`")]
    NodeKindChanged {
        node_id: String,
        before: String,
        after: String,
    },
    #[error("node `{node_id}` kind `{kind}` is not supported by the M1 edit matrix")]
    UnsupportedNodeKind { node_id: String, kind: String },
    #[error("node `{node_id}` field `{field}` is not supported by the M1 edit matrix")]
    UnsupportedNodeField { node_id: String, field: String },
    #[error("node `{node_id}` field `{field}` contains an unsupported value")]
    UnsupportedFieldValue { node_id: String, field: String },
    #[error("node `{node_id}` cannot move between collaboration page roots")]
    CrossPageMove { node_id: String },
    #[error("M1 does not combine existing-node property edits with structural edits")]
    MixedPropertyAndStructure,
    #[error("collaboration index does not fit the fixed-width wire representation")]
    IndexOverflow,
    #[error("generated transaction has {actual} operations; limit is {limit}")]
    TooManyOperations { actual: usize, limit: usize },
    #[error("generated transaction failed exact replay: {0}")]
    Replay(#[source] CollabApplyError),
    #[error("generated transaction replay hash differs from authored after document")]
    ReplayHashMismatch {
        expected: CanonicalHash,
        actual: CanonicalHash,
    },
    #[error("collaboration diff hashing failed: {0}")]
    CanonicalHash(#[from] CanonicalHashError),
    #[error("collaboration diff serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("internal structural diff could not reproduce the authored document")]
    StructuralPlanningMismatch,
}
