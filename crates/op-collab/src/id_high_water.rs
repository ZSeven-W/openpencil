use jian_ops_schema::PenDocument;

use crate::{
    apply_budget::ValidationBudget, apply_tree::document_ids, ApplyLimits, CollabApplyError,
    InsertPolicy, NamespacedId, PeerNamespace,
};

pub(crate) enum IdCounterTracker {
    Standalone,
    Namespaced {
        namespace: PeerNamespace,
        minimum_counter: Option<u64>,
        next_counter: Option<u64>,
    },
}

impl IdCounterTracker {
    pub(crate) fn from_policy(policy: &InsertPolicy) -> Self {
        match policy {
            InsertPolicy::StandaloneTrusted => Self::Standalone,
            InsertPolicy::RequireNamespace {
                namespace,
                next_counter,
            } => Self::Namespaced {
                namespace: namespace.clone(),
                minimum_counter: *next_counter,
                next_counter: *next_counter,
            },
        }
    }

    pub(crate) fn admit<'a>(
        &mut self,
        ids: impl Iterator<Item = &'a str>,
    ) -> Result<(), CollabApplyError> {
        let Self::Namespaced {
            namespace,
            minimum_counter,
            next_counter,
        } = self
        else {
            return Ok(());
        };
        let minimum = *minimum_counter;
        for id in ids {
            let parsed =
                NamespacedId::try_from(id).map_err(|_| CollabApplyError::NamespaceViolation {
                    id: id.to_owned(),
                    namespace: namespace.to_string(),
                })?;
            if parsed.namespace() != namespace {
                return Err(CollabApplyError::NamespaceViolation {
                    id: id.to_owned(),
                    namespace: namespace.to_string(),
                });
            }
            let Some(minimum) = minimum else {
                return Err(CollabApplyError::IdCounterExhausted { id: id.to_owned() });
            };
            if parsed.counter() < minimum {
                return Err(CollabApplyError::IdCounterBehind {
                    id: id.to_owned(),
                    actual: parsed.counter(),
                    minimum,
                });
            }
            if parsed.counter() == u64::MAX {
                return Err(CollabApplyError::IdCounterExhausted { id: id.to_owned() });
            }
            let proposed = parsed.counter() + 1;
            *next_counter = Some(next_counter.map_or(proposed, |seen| seen.max(proposed)));
        }
        Ok(())
    }

    pub(crate) fn next_counter(&self) -> Option<u64> {
        match self {
            Self::Standalone => None,
            Self::Namespaced { next_counter, .. } => *next_counter,
        }
    }
}

pub(crate) fn document_contains_namespace(
    document: &PenDocument,
    namespace: &PeerNamespace,
    limits: &ApplyLimits,
) -> Result<bool, CollabApplyError> {
    let mut budget = ValidationBudget::new(limits);
    let (ids, _) = document_ids(document, limits, &mut budget)?;
    Ok(ids.iter().any(|id| {
        NamespacedId::try_from(id.as_str()).is_ok_and(|parsed| parsed.namespace() == namespace)
    }))
}
