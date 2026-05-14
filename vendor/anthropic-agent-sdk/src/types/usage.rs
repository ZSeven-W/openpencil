//! Usage data types for OAuth/Max Plan users

use serde::{Deserialize, Serialize};

/// Usage limit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLimit {
    /// Percentage of limit used (0-100)
    pub utilization: f64,
    /// ISO 8601 timestamp when the limit resets
    pub resets_at: String,
}

/// Usage data from Claude API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    /// 5-hour rolling window usage
    #[serde(rename = "fiveHour")]
    pub five_hour: UsageLimit,
    /// 7-day (weekly) usage across all models
    #[serde(rename = "sevenDay")]
    pub seven_day: UsageLimit,
    /// 7-day OAuth apps usage
    #[serde(rename = "sevenDayOauthApps")]
    pub seven_day_oauth_apps: UsageLimit,
    /// 7-day Opus-specific usage
    #[serde(rename = "sevenDayOpus")]
    pub seven_day_opus: UsageLimit,
}

impl UsageData {
    /// Check if approaching any usage limit (>80%)
    #[must_use]
    pub fn is_approaching_limit(&self) -> bool {
        self.five_hour.utilization > 80.0
            || self.seven_day.utilization > 80.0
            || self.seven_day_oauth_apps.utilization > 80.0
            || self.seven_day_opus.utilization > 80.0
    }

    /// Get the highest utilization across all limits
    pub fn max_utilization(&self) -> f64 {
        [
            self.five_hour.utilization,
            self.seven_day.utilization,
            self.seven_day_oauth_apps.utilization,
            self.seven_day_opus.utilization,
        ]
        .into_iter()
        .fold(0.0, f64::max)
    }
}
