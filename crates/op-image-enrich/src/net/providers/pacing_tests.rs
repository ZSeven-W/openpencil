use std::time::{Duration, Instant};

use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};

use super::{burst_wait_duration, parse_rate_limit_headers, retry_after_seconds, TokenCache};

#[test]
fn parses_burst_limit_shape_and_auth_tier_suffix() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-ratelimit-limit-oauth2_client_credentials_burst",
        HeaderValue::from_static("20/min"),
    );
    headers.insert(
        "x-ratelimit-available-oauth2_client_credentials_burst",
        HeaderValue::from_static("7"),
    );

    let parsed = parse_rate_limit_headers(&headers);

    assert_eq!(parsed.burst_available, Some(7));
    assert_eq!(parsed.burst_window, Some(Duration::from_secs(60)));
}

#[test]
fn available_zero_reports_a_burst_window() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-ratelimit-limit-anon_burst",
        HeaderValue::from_static("20/min"),
    );
    headers.insert(
        "x-ratelimit-available-anon_burst",
        HeaderValue::from_static("0"),
    );

    let parsed = parse_rate_limit_headers(&headers);

    assert_eq!(parsed.burst_available, Some(0));
    assert_eq!(parsed.burst_window, Some(Duration::from_secs(60)));
    assert_eq!(burst_wait_duration(&headers), Some(Duration::from_secs(60)));
}

#[test]
fn missing_rate_limit_headers_do_not_request_a_wait() {
    let headers = HeaderMap::new();
    let parsed = parse_rate_limit_headers(&headers);

    assert_eq!(parsed.burst_available, None);
    assert_eq!(parsed.burst_window, None);
    assert_eq!(parsed.sustained_available, None);
    assert_eq!(burst_wait_duration(&headers), None);
}

#[test]
fn retry_after_uses_integer_seconds_and_defaults_for_http_dates() {
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_static("12"));
    assert_eq!(retry_after_seconds(&headers), 12);

    headers.insert(
        RETRY_AFTER,
        HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
    );
    assert_eq!(retry_after_seconds(&headers), 60);
    assert_eq!(retry_after_seconds(&HeaderMap::new()), 60);
}

#[test]
fn token_cache_applies_the_sixty_second_expiry_margin() {
    let now = Instant::now();
    let mut cache = TokenCache::default();
    cache.insert_at("client", "token".to_string(), 3600, now);

    assert_eq!(
        cache.get_at("client", now + Duration::from_secs(3500)),
        Some("token")
    );
    assert_eq!(
        cache.get_at("client", now + Duration::from_secs(3600)),
        None
    );
}
