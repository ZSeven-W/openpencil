//! Tests for [`crate::endpoint`] — the outbound-destination guard.

use crate::endpoint::{
    design_md_poll_url, design_md_start_url, design_md_url, ingest_url, mcp_url,
    normalize_endpoint, DEFAULT_ENDPOINT,
};

#[test]
fn accepts_ipv4_and_rewrites_localhost() {
    assert_eq!(
        normalize_endpoint("127.0.0.1:3100").as_deref(),
        Some("127.0.0.1:3100")
    );
    // `localhost` is rewritten: the live endpoint's admission gate refuses a
    // `Host` header carrying a DNS name.
    assert_eq!(
        normalize_endpoint("localhost:3100").as_deref(),
        Some("127.0.0.1:3100")
    );
    assert_eq!(
        normalize_endpoint("LocalHost:3100").as_deref(),
        Some("127.0.0.1:3100")
    );
}

#[test]
fn tolerates_a_scheme_trailing_slashes_and_surrounding_space() {
    assert_eq!(
        normalize_endpoint("http://127.0.0.1:3100").as_deref(),
        Some("127.0.0.1:3100")
    );
    assert_eq!(
        normalize_endpoint("HTTPS://localhost:3100/").as_deref(),
        Some("127.0.0.1:3100")
    );
    assert_eq!(
        normalize_endpoint("  127.0.0.1:3100///  ").as_deref(),
        Some("127.0.0.1:3100")
    );
}

#[test]
fn rejects_every_non_loopback_host() {
    for raw in [
        "example.com:3100",
        "http://evil.test:80",
        "192.168.1.5:3100",
        "10.0.0.1:3100",
        "0.0.0.0:3100",
        "127.0.0.2:3100",
        // A host that merely ends in the loopback literal must not slip past.
        "evil127.0.0.1:3100",
        "127.0.0.1.evil.test:3100",
        // Userinfo trick: the real host is after the `@`.
        "127.0.0.1@evil.test:3100",
        "[::2]:3100",
        "[::1]:3100",
        "http://[::1]:3100",
    ] {
        assert_eq!(
            normalize_endpoint(raw),
            None,
            "expected {raw:?} to be refused"
        );
    }
}

#[test]
fn rejects_malformed_ports() {
    for raw in [
        "127.0.0.1",
        "127.0.0.1:",
        "127.0.0.1:0",
        "127.0.0.1:65536",
        "127.0.0.1:999999",
        "127.0.0.1:-1",
        "127.0.0.1:+80",
        "127.0.0.1:3100abc",
        "127.0.0.1:31 00",
        "127.0.0.1:0x50",
        "",
        "   ",
    ] {
        assert_eq!(
            normalize_endpoint(raw),
            None,
            "expected {raw:?} to be refused"
        );
    }
}

#[test]
fn rejects_a_path_or_query_after_the_port() {
    for raw in [
        "127.0.0.1:3100/mcp",
        "127.0.0.1:3100?x=1",
        "127.0.0.1:3100#f",
    ] {
        assert_eq!(
            normalize_endpoint(raw),
            None,
            "expected {raw:?} to be refused"
        );
    }
}

#[test]
fn keeps_the_numeric_value_of_a_zero_padded_port() {
    assert_eq!(
        normalize_endpoint("127.0.0.1:00080").as_deref(),
        Some("127.0.0.1:80")
    );
    assert_eq!(
        normalize_endpoint("127.0.0.1:65535").as_deref(),
        Some("127.0.0.1:65535")
    );
    assert_eq!(
        normalize_endpoint("127.0.0.1:1").as_deref(),
        Some("127.0.0.1:1")
    );
}

#[test]
fn default_endpoint_normalizes_to_itself() {
    assert_eq!(
        normalize_endpoint(DEFAULT_ENDPOINT).as_deref(),
        Some(DEFAULT_ENDPOINT)
    );
}

#[test]
fn urls_are_plain_http_on_the_given_authority() {
    assert_eq!(
        ingest_url("127.0.0.1:3100"),
        "http://127.0.0.1:3100/api/import/web-snapshot"
    );
    assert_eq!(mcp_url("127.0.0.1:3100"), "http://127.0.0.1:3100/mcp");
    assert_eq!(
        design_md_url("127.0.0.1:3100"),
        "http://127.0.0.1:3100/api/generate/design-md"
    );
    assert_eq!(
        design_md_start_url("127.0.0.1:3100"),
        design_md_url("127.0.0.1:3100")
    );
    assert_eq!(
        design_md_poll_url("127.0.0.1:3100", "0123456789abcdef0123456789abcdef").as_deref(),
        Some("http://127.0.0.1:3100/api/generate/design-md/0123456789abcdef0123456789abcdef")
    );
}

#[test]
fn a_design_job_id_cannot_escape_its_url_segment() {
    for job_id in [
        "",
        "0123456789abcdef0123456789abcde",
        "0123456789abcdef0123456789abcdef0",
        "0123456789ABCDEF0123456789ABCDEF",
        "../../mcp.......................",
        "0123456789abcdef/123456789abcdef",
        "0123456789abcdef%2f123456789abc",
        "0123456789abcdef_123456789abcdef",
    ] {
        assert_eq!(
            design_md_poll_url("127.0.0.1:3100", job_id),
            None,
            "{job_id}"
        );
    }
}
