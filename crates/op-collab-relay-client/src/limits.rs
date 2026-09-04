use std::time::Duration;

use op_collab_relay_protocol::{RelayWaitingAdvertisementV1, RELAY_OWNER_LANE_RECYCLE_SECS};

pub const DEFAULT_OWNER_LANE_COUNT: usize = 4;
pub const MAX_OWNER_LANE_COUNT: usize = 8;
pub const MAX_RELAY_BINARY_BYTES: usize = 64 * 1024;
pub const MAX_RELAY_CONNECTION_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct RelayLimits {
    pub connect: Duration,
    pub hello: Duration,
    pub pair: Duration,
    /// How long ONE owner lane may sit waiting for a guest before the client
    /// retires it and dials a fresh one.
    ///
    /// The relay hard-closes an unpaired peer after its own waiting window
    /// (`op_collab_relay_server::RelayConfig::waiting_timeout`, which operators
    /// tune with `OPENPENCIL_COLLAB_RELAY_WAITING_TIMEOUT_SECS`), so an owner
    /// lane parked on the long [`RelayLimits::pair`] budget never recycles on
    /// its own schedule: it either learns about the close late or, behind a NAT
    /// that silently reaps the idle flow, not at all. Either way the owner is
    /// absent from the relay's waiting queue while it re-dials, and a guest
    /// that registers in that window has no counterpart to pair with. Staying
    /// under the server window keeps the recycle client-driven, bounded, and
    /// observable.
    ///
    /// Both halves of the contract are pinned in
    /// `op_collab_relay_protocol::pairing_window`: this budget is
    /// [`RELAY_OWNER_LANE_RECYCLE_SECS`], and the relay refuses to start with a
    /// waiting window below [`MIN_RELAY_WAITING_TIMEOUT_SECS`], so neither side
    /// can drift without the other failing.
    pub owner_pair: Duration,
    /// WebSocket-level liveness-probe cadence for an active paired tunnel.
    ///
    /// The inner Noise/TCP stream may be intentionally quiet while an editor is
    /// merely open in the foreground. Keeping this comfortably below both idle
    /// windows proves the authenticated relay path is still alive anyway.
    pub keepalive: Duration,
    pub idle: Duration,
    pub lifetime: Duration,
    pub retry: Duration,
    pub stop: Duration,
    pub max_binary_bytes: usize,
    pub max_connection_bytes: u64,
}

impl Default for RelayLimits {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(10),
            hello: Duration::from_secs(10),
            pair: Duration::from_secs(5 * 60),
            owner_pair: Duration::from_secs(RELAY_OWNER_LANE_RECYCLE_SECS),
            keepalive: Duration::from_secs(30),
            idle: Duration::from_secs(2 * 60),
            lifetime: Duration::from_secs(24 * 60 * 60),
            retry: Duration::from_secs(1),
            stop: Duration::from_secs(5),
            max_binary_bytes: MAX_RELAY_BINARY_BYTES,
            max_connection_bytes: MAX_RELAY_CONNECTION_BYTES,
        }
    }
}

impl std::fmt::Debug for RelayLimits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelayLimits")
            .field("connect", &self.connect)
            .field("hello", &self.hello)
            .field("pair", &self.pair)
            .field("owner_pair", &self.owner_pair)
            .field("keepalive", &self.keepalive)
            .field("idle", &self.idle)
            .field("lifetime", &self.lifetime)
            .field("retry", &self.retry)
            .field("stop", &self.stop)
            .field("max_binary_bytes", &self.max_binary_bytes)
            .field("max_connection_bytes", &self.max_connection_bytes)
            .finish()
    }
}

/// Smallest first-connection waiting budget an owner lane may be given.
///
/// Only reachable with a deliberately tiny `owner_pair` (tests); it keeps a
/// degenerate schedule from turning lane recycling into a reconnect spin.
const MIN_OWNER_LANE_PAIR_BUDGET: Duration = Duration::from_millis(50);

/// First-connection waiting budget for owner lane `index` of `lane_count`.
///
/// Every lane is dialled at session start, so one shared budget expires them
/// all in the same instant and empties the relay's waiting queue for the whole
/// re-dial — precisely the window in which a joining guest finds no owner to
/// pair with. Giving lane `index` a proportional slice of the first window
/// staggers the recycles permanently: after its first cycle each lane runs the
/// full [`RelayLimits::owner_pair`] budget, offset from its neighbours by one
/// slice, so at most one lane is ever re-dialling.
pub(crate) fn owner_lane_first_pair_budget(
    owner_pair: Duration,
    index: usize,
    lane_count: usize,
) -> Duration {
    let lanes = u32::try_from(lane_count.max(1)).unwrap_or(u32::MAX);
    let slot = u32::try_from(index.min(lane_count.saturating_sub(1)).saturating_add(1))
        .unwrap_or(u32::MAX)
        .min(lanes);
    let budget = (owner_pair / lanes).saturating_mul(slot).min(owner_pair);
    budget.max(MIN_OWNER_LANE_PAIR_BUDGET.min(owner_pair))
}

#[cfg(test)]
mod tests {
    use op_collab_relay_protocol::MIN_RELAY_WAITING_TIMEOUT_SECS;

    use super::*;

    #[test]
    fn default_owner_pair_budget_stays_under_every_permitted_relay_window() {
        // The relay refuses to start below MIN_RELAY_WAITING_TIMEOUT_SECS, so
        // clearing that bound clears every window an operator can configure.
        assert!(
            RelayLimits::default().owner_pair < Duration::from_secs(MIN_RELAY_WAITING_TIMEOUT_SECS)
        );
    }

    #[test]
    fn first_pair_budgets_are_staggered_across_the_default_lane_pool() {
        let owner_pair = RelayLimits::default().owner_pair;
        let budgets: Vec<Duration> = (0..DEFAULT_OWNER_LANE_COUNT)
            .map(|index| owner_lane_first_pair_budget(owner_pair, index, DEFAULT_OWNER_LANE_COUNT))
            .collect();
        assert_eq!(
            budgets,
            vec![
                Duration::from_millis(11_250),
                Duration::from_millis(22_500),
                Duration::from_millis(33_750),
                Duration::from_millis(45_000),
            ]
        );
        // Strictly increasing: no two lanes ever recycle in the same instant.
        assert!(budgets.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(budgets.iter().all(|budget| *budget <= owner_pair));
    }

    #[test]
    fn a_single_lane_pool_keeps_the_whole_budget() {
        let owner_pair = Duration::from_secs(45);
        assert_eq!(owner_lane_first_pair_budget(owner_pair, 0, 1), owner_pair);
    }

    #[test]
    fn a_degenerate_schedule_never_produces_a_zero_budget() {
        let tiny = Duration::from_millis(4);
        for index in 0..8 {
            assert_eq!(owner_lane_first_pair_budget(tiny, index, 8), tiny);
        }
        assert_eq!(
            owner_lane_first_pair_budget(Duration::ZERO, 0, 4),
            Duration::ZERO
        );
    }

    fn owner_budget(stagger: Option<LaneStagger>) -> PairBudget {
        PairBudget::Owner(OwnerPairBudget {
            unrenewable_cap: RelayLimits::default().owner_pair,
            renewable_cap: RelayLimits::default().pair,
            stagger,
        })
    }

    #[test]
    fn a_relay_without_an_advertisement_leaves_the_fallback_untouched() {
        // The compiled-in constants are the fallback, never the contract: a
        // relay that predates the capability, or an intermediary that strips
        // the header, must not change how a lane behaves.
        assert_eq!(
            owner_budget(None).resolve(None),
            RelayLimits::default().owner_pair
        );
        assert_eq!(
            PairBudget::Fixed(Duration::from_secs(99)).resolve(None),
            Duration::from_secs(99)
        );
    }

    #[test]
    fn a_guest_budget_ignores_what_the_relay_advertises() {
        // A guest is a one-shot join, not a standing member of the queue.
        let lease = RelayWaitingAdvertisementV1::new(60, true).expect("advertisement");
        assert_eq!(
            PairBudget::Fixed(Duration::from_secs(99)).resolve(Some(lease)),
            Duration::from_secs(99)
        );
    }

    #[test]
    fn a_leased_relay_lets_an_owner_lane_stop_churning() {
        let limits = RelayLimits::default();
        let lease =
            RelayWaitingAdvertisementV1::new(12 * 60 * 60, true).expect("lease advertisement");
        assert_eq!(owner_budget(None).resolve(Some(lease)), limits.pair);

        // A relay that still runs a fixed countdown keeps the lane on the
        // short recycle budget.
        let countdown = RelayWaitingAdvertisementV1::new(60, false).expect("advertisement");
        assert_eq!(
            owner_budget(None).resolve(Some(countdown)),
            limits.owner_pair
        );
    }

    #[test]
    fn the_first_cycle_stays_staggered_at_whatever_scale_the_relay_allows() {
        // Regression guard: the stagger travels as a slot, not a duration. If
        // it were resolved before the advertisement, a lease-backed pool would
        // hand every lane the same budget and retire all of them in the same
        // instant — emptying the waiting queue, which is the failure the
        // stagger exists to prevent.
        let lease =
            RelayWaitingAdvertisementV1::new(12 * 60 * 60, true).expect("lease advertisement");
        let budgets: Vec<Duration> = (0..DEFAULT_OWNER_LANE_COUNT)
            .map(|index| {
                owner_budget(Some(LaneStagger {
                    index,
                    lane_count: DEFAULT_OWNER_LANE_COUNT,
                }))
                .resolve(Some(lease))
            })
            .collect();

        assert!(budgets.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            budgets.last().copied(),
            Some(RelayLimits::default().pair),
            "the last lane runs the full leased window"
        );
        assert!(budgets
            .iter()
            .all(|budget| *budget > RelayLimits::default().owner_pair || *budget == budgets[0]));
    }

    #[test]
    fn an_out_of_range_index_is_clamped_to_the_last_lane() {
        let owner_pair = Duration::from_secs(45);
        assert_eq!(
            owner_lane_first_pair_budget(owner_pair, 99, 4),
            owner_lane_first_pair_budget(owner_pair, 3, 4)
        );
    }
}

/// A lane's slot in the owner pool, used to stagger first-cycle recycles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LaneStagger {
    pub(crate) index: usize,
    pub(crate) lane_count: usize,
}

/// An owner lane's waiting budget, resolved against the relay's advertisement.
///
/// Two ceilings, because the right budget depends on something only the relay
/// knows. When the relay retires un-paired peers on a fixed countdown the lane
/// must recycle itself first, so it stays on the short `unrenewable_cap`. When
/// the relay advertises a renewable waiting lease it will hold the slot for as
/// long as the lane answers pings, so the lane may park on the much longer
/// `renewable_cap` and stop churning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OwnerPairBudget {
    pub(crate) unrenewable_cap: Duration,
    pub(crate) renewable_cap: Duration,
    /// Present only on a lane's first connection.
    pub(crate) stagger: Option<LaneStagger>,
}

/// How one connection decides how long it may sit un-paired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PairBudget {
    /// The caller's budget, used as-is.
    ///
    /// A guest is a one-shot join rather than a standing member of the relay's
    /// waiting queue, so it keeps the long [`RelayLimits::pair`] window and is
    /// unaffected by what the relay advertises.
    Fixed(Duration),
    /// An owner lane, which narrows towards whatever the relay advertises.
    Owner(OwnerPairBudget),
}

impl PairBudget {
    /// Resolve the budget once the relay's advertisement is known.
    ///
    /// The advertisement can only ever narrow the client's own ceiling or
    /// unlock the wider lease ceiling; it is never adopted verbatim, and its
    /// absence leaves the compiled-in fallback untouched.
    pub(crate) fn resolve(self, advertised: Option<RelayWaitingAdvertisementV1>) -> Duration {
        match self {
            Self::Fixed(budget) => budget,
            Self::Owner(owner) => {
                let budget = match advertised {
                    Some(advertisement) => {
                        advertisement.derive_lane_budget(owner.unrenewable_cap, owner.renewable_cap)
                    }
                    None => owner.unrenewable_cap,
                };
                match owner.stagger {
                    Some(stagger) => {
                        owner_lane_first_pair_budget(budget, stagger.index, stagger.lane_count)
                    }
                    None => budget,
                }
            }
        }
    }
}
