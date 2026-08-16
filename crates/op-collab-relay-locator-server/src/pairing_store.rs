//! Process-local bounded storage for sealed pairing invites.
//!
//! Single-instance only: the map lives in this process, so a multi-replica
//! deployment needs a shared [`PairingCodeStore`] implementation behind the
//! same trait — publish on replica A / claim on replica B otherwise 404s.
//!
//! Two abuse properties are load-bearing:
//! - An id slot is never freed before its expiry, even when the claim budget
//!   runs out (the entry degrades to a tombstone). Deleting it early would
//!   let anyone holding the code burn the budget and re-publish their own
//!   sealed invite under the same id — a session-substitution attack.
//! - Every entry is attributed to the ticket-verified owner key and each
//!   owner is capped, so one account cannot squat the global capacity.

use std::{collections::HashMap, sync::Mutex};

use op_collab_relay_control_plane::{PairingCodeStore, PairingPutOutcome, PairingStoreRejection};
use op_collab_relay_protocol::MAX_SEALED_INVITE_BYTES;

pub(crate) const MAX_PAIRING_CODES: usize = 65_536;
pub(crate) const MAX_PAIRING_CODES_PER_OWNER: usize = 32;
pub(crate) const INITIAL_PAIRING_CODE_CLAIMS: u32 = 32;

struct StoredCode {
    owner: [u8; 32],
    /// Emptied when the claim budget is exhausted; the entry then acts as a
    /// tombstone that keeps the id reserved until expiry.
    sealed: Vec<u8>,
    expires_at_unix: u64,
    claims_left: u32,
}

#[derive(Default)]
struct PairingStoreState {
    codes: HashMap<[u8; 16], StoredCode>,
    owner_counts: HashMap<[u8; 32], usize>,
    /// Earliest expiry across all entries; sweeps run only when the clock
    /// passes it (or capacity pressure demands one), so a full store of live
    /// entries does not pay an O(n) scan per publish.
    earliest_expiry_unix: u64,
}

impl PairingStoreState {
    fn sweep_expired(&mut self, now_unix: u64) {
        let owner_counts = &mut self.owner_counts;
        self.codes.retain(|_, code| {
            let live = code.expires_at_unix > now_unix;
            if !live {
                decrement_owner(owner_counts, &code.owner);
            }
            live
        });
        self.earliest_expiry_unix = self
            .codes
            .values()
            .map(|code| code.expires_at_unix)
            .min()
            .unwrap_or(u64::MAX);
    }

    fn remove(&mut self, code_id: &[u8; 16]) {
        if let Some(code) = self.codes.remove(code_id) {
            decrement_owner(&mut self.owner_counts, &code.owner);
        }
    }
}

fn decrement_owner(owner_counts: &mut HashMap<[u8; 32], usize>, owner: &[u8; 32]) {
    if let Some(count) = owner_counts.get_mut(owner) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            owner_counts.remove(owner);
        }
    }
}

/// Process-local bounded storage for sealed pairing invites.
#[derive(Default)]
pub struct InMemoryPairingStore {
    state: Mutex<PairingStoreState>,
}

impl PairingCodeStore for InMemoryPairingStore {
    fn put(
        &self,
        owner: [u8; 32],
        code_id: [u8; 16],
        sealed: Vec<u8>,
        now_unix: u64,
        expires_at_unix: u64,
    ) -> Result<PairingPutOutcome, PairingStoreRejection> {
        if sealed.is_empty()
            || sealed.len() > MAX_SEALED_INVITE_BYTES
            || expires_at_unix <= now_unix
        {
            return Err(PairingStoreRejection::Invalid);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| PairingStoreRejection::Invalid)?;
        if now_unix >= state.earliest_expiry_unix || state.codes.len() >= MAX_PAIRING_CODES {
            state.sweep_expired(now_unix);
        }
        if let Some(existing) = state.codes.get(&code_id) {
            if existing.expires_at_unix > now_unix {
                // Identical republish is a lost-response retry; anything
                // else (including a tombstone) keeps the id reserved.
                if existing.owner == owner && existing.claims_left > 0 && existing.sealed == sealed
                {
                    return Ok(PairingPutOutcome::AlreadyStored);
                }
                return Err(PairingStoreRejection::DuplicateCode);
            }
            state.remove(&code_id);
        }
        let owner_count = state.owner_counts.get(&owner).copied().unwrap_or(0);
        if owner_count >= MAX_PAIRING_CODES_PER_OWNER {
            return Err(PairingStoreRejection::OwnerQuotaExceeded);
        }
        if state.codes.len() >= MAX_PAIRING_CODES {
            return Err(PairingStoreRejection::CapacityExhausted);
        }
        state.codes.insert(
            code_id,
            StoredCode {
                owner,
                sealed,
                expires_at_unix,
                claims_left: INITIAL_PAIRING_CODE_CLAIMS,
            },
        );
        *state.owner_counts.entry(owner).or_insert(0) += 1;
        state.earliest_expiry_unix = state.earliest_expiry_unix.min(expires_at_unix);
        Ok(PairingPutOutcome::Stored)
    }

    fn claim(&self, code_id: &[u8; 16], now_unix: u64) -> Option<Vec<u8>> {
        let mut state = self.state.lock().ok()?;
        let code = state.codes.get_mut(code_id)?;
        if code.expires_at_unix <= now_unix {
            state.remove(code_id);
            return None;
        }
        if code.claims_left == 0 {
            // Tombstone: budget exhausted, id stays reserved until expiry.
            return None;
        }
        let sealed = code.sealed.clone();
        code.claims_left = code.claims_left.saturating_sub(1);
        if code.claims_left == 0 {
            code.sealed = Vec::new();
        }
        Some(sealed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_id(index: usize) -> [u8; 16] {
        let mut code_id = [0_u8; 16];
        code_id[..8].copy_from_slice(&(index as u64).to_be_bytes());
        code_id
    }

    const OWNER_A: [u8; 32] = [0xA1; 32];
    const OWNER_B: [u8; 32] = [0xB2; 32];
    const FAR: u64 = u64::MAX - 1;

    #[test]
    fn put_and_claim_round_trip() {
        let store = InMemoryPairingStore::default();
        let code_id = code_id(1);
        let sealed = vec![1, 2, 3, 4];
        assert_eq!(
            store.put(OWNER_A, code_id, sealed.clone(), 1, FAR),
            Ok(PairingPutOutcome::Stored)
        );
        assert_eq!(store.claim(&code_id, 1), Some(sealed));
    }

    #[test]
    fn identical_republish_is_idempotent_but_foreign_collisions_are_refused() {
        let store = InMemoryPairingStore::default();
        let code_id = code_id(2);
        store.put(OWNER_A, code_id, vec![1], 1, FAR).expect("store");
        assert_eq!(
            store.put(OWNER_A, code_id, vec![1], 2, FAR),
            Ok(PairingPutOutcome::AlreadyStored),
            "a lost-response retry must succeed"
        );
        assert_eq!(
            store.put(OWNER_A, code_id, vec![2], 2, FAR),
            Err(PairingStoreRejection::DuplicateCode),
            "same owner, different blob is not a retry"
        );
        assert_eq!(
            store.put(OWNER_B, code_id, vec![1], 2, FAR),
            Err(PairingStoreRejection::DuplicateCode)
        );
        assert_eq!(store.claim(&code_id, 3), Some(vec![1]));
    }

    #[test]
    fn claim_evicts_expired_code() {
        let store = InMemoryPairingStore::default();
        let code_id = code_id(3);
        store
            .put(OWNER_A, code_id, vec![5, 6], 1, 100)
            .expect("store");
        assert_eq!(store.claim(&code_id, 100), None);
        assert_eq!(store.claim(&code_id, 101), None);
        assert_eq!(store.state.lock().expect("lock").codes.len(), 0);
        assert!(store.state.lock().expect("lock").owner_counts.is_empty());
    }

    #[test]
    fn exhausted_claims_leave_a_tombstone_that_blocks_substitution() {
        let store = InMemoryPairingStore::default();
        let code_id = code_id(4);
        let sealed = vec![7, 8, 9];
        store
            .put(OWNER_A, code_id, sealed.clone(), 1, FAR)
            .expect("store");
        for _ in 0..INITIAL_PAIRING_CODE_CLAIMS {
            assert_eq!(store.claim(&code_id, 1), Some(sealed.clone()));
        }
        assert_eq!(store.claim(&code_id, 1), None, "budget exhausted");
        // The id stays reserved: an attacker who burned the budget cannot
        // slide their own sealed invite under the same code.
        assert_eq!(
            store.put(OWNER_B, code_id, vec![0xEE], 2, FAR),
            Err(PairingStoreRejection::DuplicateCode)
        );
        // Not even the original owner may resurrect it with fresh bytes.
        assert_eq!(
            store.put(OWNER_A, code_id, sealed, 2, FAR),
            Err(PairingStoreRejection::DuplicateCode),
            "tombstoned entries never match the idempotent-retry arm"
        );
    }

    #[test]
    fn per_owner_quota_is_enforced_and_released_on_expiry() {
        let store = InMemoryPairingStore::default();
        for index in 0..MAX_PAIRING_CODES_PER_OWNER {
            store
                .put(OWNER_A, code_id(index), vec![1], 1, 100)
                .expect("fill owner quota");
        }
        assert_eq!(
            store.put(
                OWNER_A,
                code_id(MAX_PAIRING_CODES_PER_OWNER),
                vec![1],
                1,
                100
            ),
            Err(PairingStoreRejection::OwnerQuotaExceeded)
        );
        // A different owner is unaffected.
        assert_eq!(
            store.put(OWNER_B, code_id(usize::MAX), vec![1], 1, 100),
            Ok(PairingPutOutcome::Stored)
        );
        // Once the first owner's codes expire, the quota frees up.
        assert_eq!(
            store.put(
                OWNER_A,
                code_id(MAX_PAIRING_CODES_PER_OWNER),
                vec![1],
                100,
                200
            ),
            Ok(PairingPutOutcome::Stored)
        );
    }

    #[test]
    fn capacity_cap_allows_expired_entry_sweep() {
        // Distinct owners so the per-owner quota is not the limiting factor.
        fn owner(index: usize) -> [u8; 32] {
            let mut owner = [0_u8; 32];
            owner[..8].copy_from_slice(&(index as u64).to_be_bytes());
            owner
        }
        let store = InMemoryPairingStore::default();
        for index in 0..MAX_PAIRING_CODES {
            store
                .put(owner(index), code_id(index), vec![1], 1, FAR)
                .expect("fill live store");
        }
        assert_eq!(
            store.put(OWNER_B, code_id(MAX_PAIRING_CODES), vec![2], 1, FAR),
            Err(PairingStoreRejection::CapacityExhausted)
        );

        let expiring = InMemoryPairingStore::default();
        for index in 0..MAX_PAIRING_CODES {
            expiring
                .put(owner(index), code_id(index), vec![1], 1, 100)
                .expect("fill expiring store");
        }
        let replacement_id = code_id(MAX_PAIRING_CODES);
        expiring
            .put(OWNER_B, replacement_id, vec![2], 100, 200)
            .expect("expired sweep frees capacity");
        assert_eq!(expiring.claim(&replacement_id, 101), Some(vec![2]));
        assert_eq!(expiring.state.lock().expect("lock").codes.len(), 1);
    }

    #[test]
    fn invalid_blobs_and_backdated_expiry_are_rejected() {
        let store = InMemoryPairingStore::default();
        assert_eq!(
            store.put(
                OWNER_A,
                code_id(8),
                vec![0; MAX_SEALED_INVITE_BYTES],
                1,
                FAR
            ),
            Ok(PairingPutOutcome::Stored),
            "opaque storage retains the v0.8.4 sealed-blob ceiling"
        );
        assert_eq!(
            store.put(
                OWNER_A,
                code_id(9),
                vec![0; MAX_SEALED_INVITE_BYTES + 1],
                1,
                FAR
            ),
            Err(PairingStoreRejection::Invalid)
        );
        assert_eq!(
            store.put(OWNER_A, code_id(9), Vec::new(), 1, FAR),
            Err(PairingStoreRejection::Invalid)
        );
        assert_eq!(
            store.put(OWNER_A, code_id(9), vec![1], 100, 100),
            Err(PairingStoreRejection::Invalid),
            "an entry expiring at or before now is dead on arrival"
        );
    }
}
