//! Bounded, explicit smoke override for slow Antigravity model turns.
//! Normal app launches keep the existing five-minute budget. No model,
//! permission, prompt, or retry policy changes with this diagnostic knob.

use std::time::Duration;

use super::{ANTIGRAVITY_PRINT_TIMEOUT, ANTIGRAVITY_TIMEOUT};

const SMOKE_BUDGET_ENV: &str = "OPENPENCIL_SMOKE_AGY_TIMEOUT_SECS";

pub fn antigravity_timeout() -> Duration {
    budget_from_value(std::env::var(SMOKE_BUDGET_ENV).ok().as_deref())
}

pub(super) fn antigravity_print_timeout() -> String {
    print_budget(antigravity_timeout())
}

fn budget_from_value(value: Option<&str>) -> Duration {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| (300..=1800).contains(seconds))
        .map(Duration::from_secs)
        .unwrap_or(ANTIGRAVITY_TIMEOUT)
}

fn print_budget(wall: Duration) -> String {
    if wall == ANTIGRAVITY_TIMEOUT {
        ANTIGRAVITY_PRINT_TIMEOUT.into()
    } else {
        format!("{}s", wall.as_secs() - 20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_or_unbounded_override_keeps_the_default_deadline() {
        for value in [
            None,
            Some("0"),
            Some("299"),
            Some("1801"),
            Some("forever"),
            Some("-1"),
        ] {
            let budget = budget_from_value(value);
            assert_eq!(budget, Duration::from_secs(300));
            assert_eq!(print_budget(budget), "280s");
        }
    }

    #[test]
    fn explicit_diagnostic_budget_keeps_cli_shutdown_inside_the_wall_clock() {
        for seconds in [300_u64, 900, 1800] {
            let budget = budget_from_value(Some(&seconds.to_string()));
            assert_eq!(budget.as_secs(), seconds);
            assert_eq!(print_budget(budget), format!("{}s", seconds - 20));
        }
    }
}
