use crate::{ApplyLimits, CollabApplyError};

/// Transaction-scoped accounting for every validation tree traversal.
pub(crate) struct ValidationBudget {
    visits: usize,
    limit: usize,
}

impl ValidationBudget {
    pub(crate) fn new(limits: &ApplyLimits) -> Self {
        Self {
            visits: 0,
            limit: limits.max_validation_node_visits_per_txn,
        }
    }

    /// Reserve work before performing a traversal with a known node count.
    pub(crate) fn charge(&mut self, count: usize) -> Result<(), CollabApplyError> {
        let actual = self.visits.saturating_add(count);
        if actual > self.limit {
            return Err(CollabApplyError::ValidationBudgetExceeded {
                actual,
                limit: self.limit,
            });
        }
        self.visits = actual;
        Ok(())
    }
}
