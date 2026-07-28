#![cfg(feature = "mdns")]

use std::net::IpAddr;
use std::time::{Duration, Instant};

use op_collab_transport::{DiscoveryBrowser, DiscoveryPublisher};

/// Exercises the host multicast stack and is intentionally opt-in: hosted CI
/// networks commonly suppress mDNS. Platform runners and release machines run
/// this test explicitly before shipping collaboration changes.
#[test]
#[ignore = "requires a LAN interface with multicast DNS enabled"]
fn publisher_is_discovered_and_unregisters_cleanly() {
    let mut browser = DiscoveryBrowser::start().expect("start mDNS browser");
    let explicit_address = std::env::var("OP_COLLAB_MDNS_SMOKE_ADDRESS")
        .ok()
        .map(|value| value.parse::<IpAddr>().expect("parse smoke IP address"));
    let publisher = match explicit_address {
        Some(address) => DiscoveryPublisher::start_with_addresses(45_123, &[address]),
        None => DiscoveryPublisher::start(45_123),
    }
    .expect("start mDNS publisher");
    let discovery_id = publisher.discovery_id().to_owned();

    let discovered = wait_until(Duration::from_secs(15), || {
        browser
            .wait(Duration::from_millis(250))
            .into_iter()
            .find(|session| session.discovery_id() == discovery_id)
    })
    .expect("publisher was not discovered before the deadline");
    assert_eq!(
        discovered.primary_address().map(|address| address.port()),
        Some(45_123)
    );

    publisher.stop().expect("unregister mDNS publisher");
    let removed = wait_until(Duration::from_secs(10), || {
        let remains = browser
            .wait(Duration::from_millis(250))
            .iter()
            .any(|session| session.discovery_id() == discovery_id);
        (!remains).then_some(())
    });
    assert!(removed.is_some(), "unregistered publisher remained visible");
    browser.stop().expect("stop mDNS browser");
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires the macOS Bonjour daemon and multicast DNS"]
fn browser_interoperates_with_system_dns_sd_publisher() {
    use std::process::{Child, Command, Stdio};

    struct ChildGuard(Child);

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    const SYSTEM_ID: &str = "0123456789abcdef0123456789abcdef";
    let mut browser = DiscoveryBrowser::start().expect("start Bonjour browser");
    let id_property = format!("id={SYSTEM_ID}");
    let child = Command::new("dns-sd")
        .args([
            "-R",
            "openpencil-system-diagnostic",
            "_openpencil-collab._tcp",
            "local.",
            "45124",
            id_property.as_str(),
            "v=1",
            "p=45124",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start system dns-sd publisher");
    let mut child = ChildGuard(child);

    let discovered = wait_until(Duration::from_secs(15), || {
        browser
            .wait(Duration::from_millis(250))
            .into_iter()
            .find(|session| session.discovery_id() == SYSTEM_ID)
    })
    .expect("system dns-sd publisher was not discovered");
    assert_eq!(
        discovered.primary_address().map(|address| address.port()),
        Some(45_124)
    );

    child.0.kill().expect("stop system dns-sd publisher");
    child.0.wait().expect("reap system dns-sd publisher");
    let removed = wait_until(Duration::from_secs(10), || {
        (!browser
            .wait(Duration::from_millis(250))
            .iter()
            .any(|session| session.discovery_id() == SYSTEM_ID))
        .then_some(())
    });
    assert!(removed.is_some(), "system publisher remained visible");
    browser.stop().expect("stop Bonjour browser");
}

fn wait_until<T>(timeout: Duration, mut probe: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(value) = probe() {
            return Some(value);
        }
        if Instant::now() >= deadline {
            return None;
        }
    }
}
