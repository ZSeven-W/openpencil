//! Two-process M1 collaboration smoke.
//!
//! This binary is intentionally available only with `test-issuer`. It uses
//! public deterministic credentials under a `.invalid` issuer and can never
//! authenticate against the production trust root.

mod auth;
mod fault_guest;
mod fault_owner;
mod fault_transport;
mod fixtures;
mod guest;
mod owner;
mod scenario;
mod supervisor;

use anyhow::{bail, Result};
use std::net::SocketAddr;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    match arguments.next().and_then(|value| value.into_string().ok()) {
        Some(command) if command == "run" => {
            require_no_more(arguments)?;
            supervisor::run()
        }
        Some(command) if command == "owner" => {
            let path = required_path(arguments.next(), "owner requires a port-file path")?;
            require_no_more(arguments)?;
            println!("{}", owner::run(&path)?);
            Ok(())
        }
        Some(command) if command == "guest" => {
            let address = required_address(arguments.next(), "guest requires a socket address")?;
            require_no_more(arguments)?;
            println!("{}", guest::run(address)?);
            Ok(())
        }
        Some(command) if command == "lan-owner" => {
            let address =
                required_address(arguments.next(), "lan-owner requires a bind address")?;
            require_no_more(arguments)?;
            let result = owner::run_lan(address)?;
            println!(
                "{}",
                lan_evidence(
                    "owner",
                    result.bound_address,
                    &result.canonical_hash
                )
            );
            Ok(())
        }
        Some(command) if command == "lan-guest" => {
            let address =
                required_address(arguments.next(), "lan-guest requires the owner address")?;
            require_no_more(arguments)?;
            let canonical_hash = guest::run(address)?;
            println!("{}", lan_evidence("guest", address, &canonical_hash));
            Ok(())
        }
        Some(command) if command == "fault-owner" => {
            let scenario = required_scenario(arguments.next())?;
            let path = required_path(arguments.next(), "fault-owner requires a port-file path")?;
            require_no_more(arguments)?;
            println!("{}", fault_owner::run(scenario, &path)?);
            Ok(())
        }
        Some(command) if command == "fault-guest" => {
            let scenario = required_scenario(arguments.next())?;
            let address = arguments
                .next()
                .and_then(|value| value.into_string().ok())
                .ok_or_else(|| anyhow::anyhow!("fault-guest requires a socket address"))?
                .parse()?;
            require_no_more(arguments)?;
            println!("{}", fault_guest::run(scenario, address)?);
            Ok(())
        }
        _ => bail!(
            "usage: op-collab-smoke <run|owner PORT_FILE|guest ADDRESS|lan-owner BIND_ADDRESS|lan-guest OWNER_ADDRESS|fault-owner SCENARIO PORT_FILE|fault-guest SCENARIO ADDRESS>"
        ),
    }
}

fn required_path(value: Option<std::ffi::OsString>, message: &str) -> Result<PathBuf> {
    value
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("{message}"))
}

fn required_address(value: Option<std::ffi::OsString>, message: &str) -> Result<SocketAddr> {
    value
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| anyhow::anyhow!("{message}"))?
        .parse()
        .map_err(Into::into)
}

fn lan_evidence(role: &str, owner_endpoint: SocketAddr, canonical_hash: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": "openpencil-p2p-lan-smoke/v1",
        "implementation_version": env!("CARGO_PKG_VERSION"),
        "transport_protocol_version": op_collab_transport::TRANSPORT_PROTOCOL_VERSION,
        "role": role,
        "owner_endpoint": owner_endpoint,
        "canonical_hash": canonical_hash,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}

fn require_no_more(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<()> {
    if arguments.next().is_some() {
        bail!("unexpected extra argument");
    }
    Ok(())
}

fn required_scenario(value: Option<std::ffi::OsString>) -> Result<scenario::Scenario> {
    let value = value
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| anyhow::anyhow!("fault command requires a scenario"))?;
    scenario::Scenario::parse(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lan_evidence_is_machine_readable_and_versioned() {
        let evidence = lan_evidence(
            "owner",
            "192.168.1.20:45123".parse().unwrap(),
            "a".repeat(64).as_str(),
        );
        assert_eq!(
            evidence["schema"],
            serde_json::json!("openpencil-p2p-lan-smoke/v1")
        );
        assert_eq!(evidence["role"], serde_json::json!("owner"));
        assert_eq!(
            evidence["owner_endpoint"],
            serde_json::json!("192.168.1.20:45123")
        );
        assert_eq!(evidence["canonical_hash"].as_str().unwrap().len(), 64);
    }
}
