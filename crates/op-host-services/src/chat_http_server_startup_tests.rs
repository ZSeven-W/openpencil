use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
// Only `TEMP_ID` below needs this, and that static is unix-gated.
#[cfg(unix)]
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::*;

// Only read by the `#[cfg(unix)]` socket test below; dead on Windows.
#[cfg(unix)]
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .use_rustls_tls()
        .build()
        .expect("test HTTP client")
}

/// Budget for "an expected event must eventually happen" waits (a stub
/// reaching its listen line, a probe request arriving, a cancelled worker
/// finishing). These are liveness bounds, not the property under test:
/// a broken path either never fires (and still fails here) or returns
/// the wrong result (caught by the result asserts). Generous, because a
/// loaded machine — CI, or several test binaries running concurrently —
/// can delay a spawn chain or timer by whole seconds, and a tight bound
/// turns that scheduling noise into a spurious failure.
const LIVENESS_BUDGET: Duration = Duration::from_secs(10);

fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}

struct HangingHealthServer {
    base: String,
    accepted: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl HangingHealthServer {
    fn from_listener(listener: TcpListener) -> Self {
        let base = format!(
            "http://{}",
            listener.local_addr().expect("listener address")
        );
        let accepted = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let accepted_worker = Arc::clone(&accepted);
        let release_worker = Arc::clone(&release);
        let thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept health request");
            read_http_headers(&mut stream);
            accepted_worker.store(true, Ordering::Release);
            while !release_worker.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(5));
            }
        });
        Self {
            base,
            accepted,
            release,
            thread: Some(thread),
        }
    }

    fn wait_for_request(&self) -> bool {
        wait_until(LIVENESS_BUDGET, || self.accepted.load(Ordering::Acquire))
    }
}

impl Drop for HangingHealthServer {
    fn drop(&mut self) {
        self.release.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn read_http_headers(stream: &mut TcpStream) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok() && !line.trim().is_empty() {
        line.clear();
    }
}

#[test]
fn startup_argv_uses_a_real_explicit_loopback_port() {
    let port = reserve_loopback_port().expect("reserve loopback port");
    assert_ne!(port, 0);
    assert_eq!(
        opencode_server_args(port),
        vec![
            "serve".to_string(),
            "--hostname=127.0.0.1".to_string(),
            format!("--port={port}"),
        ]
    );
    assert!(!opencode_server_args(port)
        .iter()
        .any(|argument| argument == "--port=0"));
}

#[test]
fn address_in_use_detection_matches_common_runtimes() {
    assert!(reports_address_in_use("error: EADDRINUSE 127.0.0.1"));
    assert!(reports_address_in_use(
        "Address already in use (os error 48)"
    ));
    assert!(!reports_address_in_use("permission denied"));
}

#[test]
fn receiver_drop_interrupts_the_default_health_probe() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind hanging health server");
    let server = HangingHealthServer::from_listener(listener);
    let (tx, rx) = mpsc::channel::<()>(1);
    let base = server.base.clone();
    let worker = crate::chat_runtime::shared_runtime().spawn(async move {
        let mut spawned = None;
        let result = resolve_opencode_server(
            &tx,
            &test_client(),
            "binary-must-not-be-spawned",
            &base,
            &mut spawned,
        )
        .await;
        (result, spawned.is_some())
    });
    assert!(server.wait_for_request(), "default probe must be in flight");

    drop(rx);
    let stopped = crate::chat_runtime::block_on_anywhere(async {
        tokio::time::timeout(LIVENESS_BUDGET, worker).await
    });
    let (result, spawned) = stopped
        .expect("receiver drop must interrupt the default probe")
        .expect("startup worker must not panic");
    assert_eq!(result, Ok(ServerResolution::Cancelled));
    assert!(!spawned, "cancellation must not fall through into spawn");
}

#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    use super::*;

    fn temp_path(label: &str) -> PathBuf {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "openpencil-opencode-{label}-{}-{id}",
            std::process::id()
        ))
    }

    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\"'\"'"))
    }

    fn write_script(label: &str, body: &str) -> PathBuf {
        let path = temp_path(label);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write test script");
        let mut permissions = fs::metadata(&path).expect("script metadata").permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&path, permissions).expect("make test script executable");
        path
    }

    fn direct_test_command(binary: &str, args: &[String]) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(binary);
        command.args(args);
        command.process_group(0);
        command
    }

    fn read_port(path: &Path) -> Option<u16> {
        fs::read_to_string(path).ok()?.trim().parse().ok()
    }

    /// How long the terminated stub tree may take to disappear from the
    /// process table. Generous: the SIGTERM'd descendant is briefly an
    /// orphaned zombie until launchd/init reaps it, and a loaded machine
    /// stretches that window.
    const TREE_REAP_BUDGET: Duration = Duration::from_secs(5);

    fn process_alive(pid: i32) -> bool {
        // SAFETY: signal 0 is a read-only existence probe for an exact
        // positive pid written by this test's own stub.
        (unsafe { libc::kill(pid, 0) }) == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    /// Assert that the caller's final tree cleanup actually cleaned the
    /// retained stub tree (leader + backgrounded `sleep 30`, pids written
    /// by the stub itself).
    ///
    /// Deliberately NOT `terminate_result.is_ok()`: on macOS, `killpg`
    /// reports `EPERM` when the group's only remaining member is an
    /// orphaned zombie that init has not reaped yet, so under load a fully
    /// successful cleanup can still surface as a transient `Err` from
    /// `terminate_tokio_process_tree`. The contract cancellation must keep
    /// is observable — no process from the retained tree survives — so
    /// that is what we poll for. A tree that really escaped cleanup (e.g.
    /// a detached handle) keeps its 30s sleep alive and still fails here.
    fn assert_retained_tree_cleaned(
        pid_file: &Path,
        terminate: &std::io::Result<std::process::ExitStatus>,
        context: &str,
    ) {
        let pids: Vec<i32> = fs::read_to_string(pid_file)
            .expect("stub pid file")
            .split_whitespace()
            .map(|pid| pid.parse().expect("numeric stub pid"))
            .collect();
        assert_eq!(pids.len(), 2, "stub must report leader + descendant pids");
        let deadline = Instant::now() + TREE_REAP_BUDGET;
        while pids.iter().any(|&pid| process_alive(pid)) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let survivors: Vec<i32> = pids
            .iter()
            .copied()
            .filter(|&pid| process_alive(pid))
            .collect();
        for &pid in &survivors {
            // SAFETY: exact still-live test child pid, force-killed only
            // as cleanup before reporting the regression.
            let _ = unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        assert!(
            survivors.is_empty(),
            "{context}: surviving pids {survivors:?}, terminate result {terminate:?}"
        );
    }

    #[test]
    fn receiver_drop_during_listen_retains_child_for_final_tree_cleanup() {
        let marker = temp_path("listen-marker");
        let pid_file = temp_path("listen-pids");
        // The pid file is written before the marker so it is guaranteed
        // present once the test observes the marker.
        let script = write_script(
            "listen-cancel.sh",
            &format!(
                "sleep 30 &\nprintf '%s %s\\n' \"$$\" \"$!\" > {}\nprintf started > {}\nwait",
                shell_quote(&pid_file),
                shell_quote(&marker),
            ),
        );
        let (tx, rx) = mpsc::channel::<()>(1);
        let worker = crate::chat_runtime::shared_runtime().spawn(async move {
            let mut spawned = None;
            let result = spawn_opencode_server(
                &tx,
                script.to_string_lossy().as_ref(),
                &mut spawned,
                direct_test_command,
            )
            .await;
            let retained = spawned.is_some();
            let terminate = match spawned {
                Some(mut child) => {
                    op_process_io::terminate_tokio_process_tree(
                        &mut child,
                        Duration::from_millis(100),
                    )
                    .await
                }
                None => Err(std::io::Error::other("no retained child to clean")),
            };
            (result, retained, terminate, script)
        });
        assert!(
            wait_until(LIVENESS_BUDGET, || marker.exists()),
            "fake OpenCode must reach its listen wait"
        );

        drop(rx);
        let stopped = crate::chat_runtime::block_on_anywhere(async {
            tokio::time::timeout(LIVENESS_BUDGET, worker).await
        });
        let (result, retained, terminate, script) = stopped
            .expect("receiver drop must interrupt the listen handshake")
            .expect("startup worker must not panic");
        assert_eq!(result, Ok(None));
        assert!(retained, "spawned child must remain caller-owned");
        assert_retained_tree_cleaned(
            &pid_file,
            &terminate,
            "retained child tree must be cleanable",
        );
        let _ = fs::remove_file(script);
        let _ = fs::remove_file(marker);
        let _ = fs::remove_file(pid_file);
    }

    #[test]
    fn address_in_use_is_retried_with_fresh_explicit_ports() {
        let log = temp_path("retry-argv");
        let script = write_script(
            "retry.sh",
            &format!(
                "printf '%s\\n' \"$*\" >> {}\nprintf '%s\\n' EADDRINUSE >&2\nexit 1",
                shell_quote(&log)
            ),
        );
        let (tx, _rx) = mpsc::channel::<()>(1);
        let binary = script.to_string_lossy().to_string();
        let (result, retained) = crate::chat_runtime::block_on_anywhere(async move {
            let mut spawned = None;
            let result =
                spawn_opencode_server(&tx, &binary, &mut spawned, direct_test_command).await;
            let retained = spawned.is_some();
            if let Some(mut child) = spawned {
                let _ = child.wait().await;
            }
            (result, retained)
        });
        assert!(matches!(result, Err(OpenCodeError::ServerExited { .. })));
        assert!(retained, "final failed attempt remains caller-owned");

        let invocations = fs::read_to_string(&log).expect("read invocation log");
        let lines: Vec<_> = invocations.lines().collect();
        assert_eq!(lines.len(), SERVER_BIND_ATTEMPTS);
        for line in lines {
            let port = line
                .split_whitespace()
                .find_map(|argument| argument.strip_prefix("--port="))
                .and_then(|port| port.parse::<u16>().ok())
                .expect("explicit numeric --port argument");
            assert_ne!(port, 0);
        }
        let _ = fs::remove_file(script);
        let _ = fs::remove_file(log);
    }

    #[test]
    fn receiver_drop_interrupts_post_spawn_identity_probe_and_retains_child() {
        let port_file = temp_path("announced-port");
        let gate = temp_path("announce-gate");
        let pid_file = temp_path("probe-pids");
        // The pid file is written before the listen announcement so it is
        // guaranteed present before cancellation can trigger cleanup.
        let script = write_script(
            "post-probe.sh",
            &format!(
                "port=\nfor argument in \"$@\"; do\n  case \"$argument\" in\n    --port=*) port=${{argument#--port=}} ;;\n  esac\ndone\nprintf '%s' \"$port\" > {}\nwhile [ ! -f {} ]; do sleep 0.01; done\nsleep 30 &\nprintf '%s %s\\n' \"$$\" \"$!\" > {}\nprintf 'opencode server listening on http://127.0.0.1:%s\\n' \"$port\"\nwait",
                shell_quote(&port_file),
                shell_quote(&gate),
                shell_quote(&pid_file),
            ),
        );
        let closed_port = reserve_loopback_port().expect("reserve closed default port");
        let default_url = format!("http://127.0.0.1:{closed_port}");
        let binary = script.to_string_lossy().to_string();
        let (tx, rx) = mpsc::channel::<()>(1);
        let worker = crate::chat_runtime::shared_runtime().spawn(async move {
            let mut spawned = None;
            let result = resolve_opencode_server_with(
                &tx,
                &test_client(),
                &binary,
                &default_url,
                &mut spawned,
                direct_test_command,
            )
            .await;
            let retained = spawned.is_some();
            let terminate = match spawned {
                Some(mut child) => {
                    op_process_io::terminate_tokio_process_tree(
                        &mut child,
                        Duration::from_millis(100),
                    )
                    .await
                }
                None => Err(std::io::Error::other("no retained child to clean")),
            };
            (result, retained, terminate)
        });

        let mut announced_port = None;
        assert!(
            wait_until(LIVENESS_BUDGET, || {
                announced_port = read_port(&port_file);
                announced_port.is_some()
            }),
            "fake OpenCode must receive its explicit port"
        );
        let listener = TcpListener::bind(("127.0.0.1", announced_port.unwrap()))
            .expect("bind announced health listener");
        let server = HangingHealthServer::from_listener(listener);
        fs::write(&gate, b"ready").expect("release listen announcement");
        assert!(
            server.wait_for_request(),
            "post-spawn identity probe must be in flight"
        );

        drop(rx);
        let stopped = crate::chat_runtime::block_on_anywhere(async {
            tokio::time::timeout(LIVENESS_BUDGET, worker).await
        });
        let (result, retained, terminate) = stopped
            .expect("receiver drop must interrupt post-spawn probe")
            .expect("startup worker must not panic");
        assert_eq!(result, Ok(ServerResolution::Cancelled));
        assert!(retained, "spawned child must remain caller-owned");
        assert_retained_tree_cleaned(
            &pid_file,
            &terminate,
            "retained child tree must be cleanable",
        );

        drop(server);
        let _ = fs::remove_file(script);
        let _ = fs::remove_file(port_file);
        let _ = fs::remove_file(gate);
        let _ = fs::remove_file(pid_file);
    }
}
