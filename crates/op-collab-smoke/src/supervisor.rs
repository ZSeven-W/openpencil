use anyhow::{bail, Context, Result};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const OWNER_READY_TIMEOUT: Duration = Duration::from_secs(10);
const PAIR_TIMEOUT: Duration = Duration::from_secs(30);

pub fn run() -> Result<()> {
    let executable = std::env::current_exe().context("resolve smoke executable")?;
    let directory = smoke_directory()?;
    std::fs::create_dir(&directory).context("create smoke directory")?;
    let baseline = run_pair(&executable, &directory, "alternating", None)?;
    println!("scenario alternating passed: {baseline}");
    for scenario in crate::scenario::Scenario::ALL {
        let hash = run_pair(&executable, &directory, scenario.as_str(), Some(scenario))?;
        println!("scenario {} passed: {hash}", scenario.as_str());
    }
    cleanup(&directory);
    println!(
        "two-process collaboration smoke passed: {} scenarios",
        crate::scenario::Scenario::ALL.len() + 1
    );
    Ok(())
}

fn run_pair(
    executable: &Path,
    directory: &Path,
    label: &str,
    scenario: Option<crate::scenario::Scenario>,
) -> Result<String> {
    let port_file = directory.join(format!("{label}.addr"));
    let owner_command = if let Some(scenario) = scenario {
        vec![
            "fault-owner".to_owned(),
            scenario.as_str().to_owned(),
            port_file.to_string_lossy().into_owned(),
        ]
    } else {
        vec!["owner".to_owned(), port_file.to_string_lossy().into_owned()]
    };
    let mut owner = Command::new(executable)
        .args(owner_command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn owner process")?;

    let address = match wait_for_address(&port_file, &mut owner) {
        Ok(address) => address,
        Err(error) => {
            let _ = owner.kill();
            let output = owner
                .wait_with_output()
                .context("collect failed owner process")?;
            cleanup(directory);
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(error).context(format!("owner stderr: {}", stderr.trim()));
        }
    };
    let guest_command = if let Some(scenario) = scenario {
        vec![
            "fault-guest".to_owned(),
            scenario.as_str().to_owned(),
            address,
        ]
    } else {
        vec!["guest".to_owned(), address]
    };
    let guest = Command::new(executable)
        .args(guest_command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let guest = match guest {
        Ok(guest) => guest,
        Err(error) => {
            let _ = owner.kill();
            let _ = owner.wait();
            cleanup(directory);
            return Err(error).context("spawn guest process");
        }
    };
    let (owner, guest) = match wait_for_pair(owner, guest, label) {
        Ok(outputs) => outputs,
        Err(error) => {
            cleanup(directory);
            return Err(error);
        }
    };

    let result = (|| {
        if !owner.status.success() || !guest.status.success() {
            bail!(
                "{label} process failure; owner {}; guest {}",
                output_diagnostic(&owner),
                output_diagnostic(&guest)
            );
        }
        let owner_hash = output_hash(&format!("{label} owner"), &owner.stdout)?;
        let guest_hash = output_hash(&format!("{label} guest"), &guest.stdout)?;
        if owner_hash != guest_hash {
            bail!("{label} owner and guest hashes differ: {owner_hash} != {guest_hash}");
        }
        Ok(owner_hash.to_owned())
    })();
    let _ = std::fs::remove_file(port_file);
    if result.is_err() {
        cleanup(directory);
    }
    result
}

fn wait_for_pair(
    mut owner: std::process::Child,
    mut guest: std::process::Child,
    label: &str,
) -> Result<(std::process::Output, std::process::Output)> {
    let deadline = Instant::now() + PAIR_TIMEOUT;
    let mut timed_out = false;
    loop {
        let owner_status = owner.try_wait().context("poll owner process")?;
        let guest_status = guest.try_wait().context("poll guest process")?;
        if owner_status.is_some() && guest_status.is_some() {
            break;
        }
        if owner_status.is_some_and(|status| !status.success()) {
            let _ = guest.kill();
            break;
        }
        if guest_status.is_some_and(|status| !status.success()) {
            let _ = owner.kill();
            break;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            let _ = owner.kill();
            let _ = guest.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let owner = owner
        .wait_with_output()
        .context("collect owner process output")?;
    let guest = guest
        .wait_with_output()
        .context("collect guest process output")?;
    if timed_out {
        bail!(
            "{label} exceeded the {}s pair deadline; owner {}; guest {}",
            PAIR_TIMEOUT.as_secs(),
            output_diagnostic(&owner),
            output_diagnostic(&guest)
        );
    }
    Ok((owner, guest))
}

fn output_diagnostic(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("status={} stderr={:?}", output.status, stderr.trim())
}

fn wait_for_address(port_file: &Path, owner: &mut std::process::Child) -> Result<String> {
    let deadline = Instant::now() + OWNER_READY_TIMEOUT;
    loop {
        if let Ok(address) = std::fs::read_to_string(port_file) {
            let address = address.trim();
            if !address.is_empty() {
                return Ok(address.to_owned());
            }
        }
        if let Some(status) = owner.try_wait().context("poll owner process")? {
            bail!("owner exited before publishing its address: {status}");
        }
        if Instant::now() >= deadline {
            bail!("owner did not publish its address before the timeout");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn output_hash<'a>(label: &str, bytes: &'a [u8]) -> Result<&'a str> {
    let output =
        std::str::from_utf8(bytes).with_context(|| format!("{label} output is not UTF-8"))?;
    let hash = output.trim();
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label} did not print one canonical hash");
    }
    Ok(hash)
}

fn smoke_directory() -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time is before Unix epoch")?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!(
        "openpencil-collab-smoke-{}-{timestamp}",
        std::process::id()
    )))
}

fn cleanup(directory: &Path) {
    if directory
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.starts_with("openpencil-collab-smoke-"))
    {
        let _ = std::fs::remove_dir_all(directory);
    }
}
