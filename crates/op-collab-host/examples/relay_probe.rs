//! Scratch reproduction harness for the public-relay reconnect loop.
//!
//! Not part of the product: it drives `CollabRuntime` headlessly with the
//! real device-login bridge so an owner and a guest can meet over the real
//! relay without a GUI.
//!
//! Usage: run `mktemp -d` once, then paste its output as `PROBE_ROOT` in both
//! terminals so they share only the invite file, not account configuration.
//!   # Terminal 1
//!   PROBE_ROOT=/tmp/<random>; mkdir -m 700 "$PROBE_ROOT/owner"
//!   HOME="$PROBE_ROOT/owner" PROBE_INVITE_FILE="$PROBE_ROOT/invite" \
//!     cargo run -p op-collab-host --example relay_probe -- owner
//!   # Terminal 2, using the same random PROBE_ROOT
//!   PROBE_ROOT=/tmp/<random>; mkdir -m 700 "$PROBE_ROOT/guest"
//!   HOME="$PROBE_ROOT/guest" cargo run -p op-collab-host --example relay_probe -- guest \
//!     < "$PROBE_ROOT/invite"
//!
//! Set `PROBE_AUTO_APPROVE=1` on both processes only for isolated test
//! accounts after comparing identities out of band. Probe logs can contain
//! local paths, account metadata, and network diagnostics; do not publish them.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use jian_ops_schema::PenDocument;
use op_collab::canonical_document_hash;
use op_collab_host::{
    install_blocking_executor, BlockingExecutor, CollabRuntime, HeadlessCollabHost,
};
use op_editor_core::{
    AccountState, CollabConnectionPhase, CollabTransportCapabilities, CollabUiAction,
};

struct Executor(tokio::runtime::Runtime);

impl BlockingExecutor for Executor {
    fn block_on_erased(
        &self,
        future: std::pin::Pin<&mut (dyn std::future::Future<Output = ()> + '_)>,
    ) {
        self.0.block_on(future);
    }
}

fn stamp(started: Instant) -> String {
    format!("{:7.3}s", started.elapsed().as_secs_f64())
}

/// The shared document: one rectangle whose `x` a simulated drag moves.
fn document_at(x: i64) -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "rectangle",
            "id": "drag-target",
            "name": "Drag target",
            "x": x,
            "y": 0,
            "width": 20,
            "height": 20
        }]
    }))
    .expect("probe document")
}

fn document_hash(host: &HeadlessCollabHost) -> String {
    canonical_document_hash(&host.editor_state().doc)
        .map(|hash| hash.to_string())
        .unwrap_or_else(|error| format!("<{error:?}>"))
}

fn write_invite_file(path: &Path, invite: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    writeln!(file, "{invite}")?;
    file.sync_all()
}

/// Where the simulated pointer gesture is.
enum Drag {
    Waiting,
    Holding { step: u32, base: i64 },
}

fn main() {
    let started = Instant::now();
    let mut args = std::env::args().skip(1);
    let role = args.next().unwrap_or_default();
    assert!(
        args.next().is_none(),
        "invite codes must be read from stdin"
    );
    let invite_file = match role.as_str() {
        "owner" => Some(PathBuf::from(
            std::env::var_os("PROBE_INVITE_FILE")
                .expect("owner requires a new PROBE_INVITE_FILE path"),
        )),
        "guest" => None,
        other => panic!("unknown role {other}"),
    };
    let guest_code = if role == "guest" {
        let mut code = String::new();
        std::io::stdin()
            .read_line(&mut code)
            .expect("read invite code from stdin");
        let code = code.trim().to_string();
        assert!(!code.is_empty(), "guest requires an invite code on stdin");
        Some(code)
    } else {
        None
    };
    let auto_approve = std::env::var("PROBE_AUTO_APPROVE").is_ok_and(|value| value == "1");
    eprintln!(
        "WARNING: relay probe logs are sensitive; do not publish paths, account metadata, or network diagnostics."
    );
    if auto_approve {
        eprintln!(
            "WARNING: PROBE_AUTO_APPROVE=1 bypasses interactive identity confirmation; use isolated test accounts only."
        );
    }
    let seconds: u64 = std::env::var("PROBE_SECONDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(180);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let _ = install_blocking_executor(std::sync::Arc::new(Executor(runtime)));

    let dir = op_config_store::openpencil_dir().expect("openpencil dir");
    println!("[{}] config dir = {}", stamp(started), dir.display());
    assert!(
        op_auth_bridge::available(),
        "no auth prebuilt for this target"
    );
    let app_version = std::env::var("PROBE_APP_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    let config = op_auth_bridge::desktop_init_config(&dir, &app_version);
    assert!(op_auth_bridge::init(&config), "auth init failed");
    assert!(op_auth_bridge::restore(), "auth restore failed");
    let account = match op_auth_bridge::poll(op_auth_bridge::SESSION_HANDLE) {
        op_auth_bridge::AuthStatus::SignedIn {
            display_name,
            username,
            ..
        } => AccountState::signed_in_profile(display_name, username),
        other => panic!("not signed in: {other:?}"),
    };
    println!("[{}] signed in", stamp(started));
    // Mirror the desktop's post-restore refresh window: the private runtime
    // revalidates the persisted credential on a background thread and the host
    // keeps polling the session handle while it does.
    for _ in 0..40 {
        let _ = op_auth_bridge::poll(op_auth_bridge::SESSION_HANDLE);
        std::thread::sleep(Duration::from_millis(50));
    }
    println!("[{}] session settled", stamp(started));

    let drag_enabled = std::env::var("PROBE_DRAG").is_ok_and(|value| value == "1");
    // How many 20 ms poll ticks one simulated gesture is held for.
    let drag_ticks: u32 = std::env::var("PROBE_DRAG_TICKS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(60);
    let drag_gap = Duration::from_secs(
        std::env::var("PROBE_DRAG_GAP_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(6),
    );

    let mut host = HeadlessCollabHost::new();
    host.editor_state_mut().editor_ui.account = account;
    host.editor_state_mut().doc = document_at(0);
    // The macOS keychain is unavailable to an unsigned terminal binary, so the
    // probe pins the file key store (one key per HOME) instead.
    let mut collab = CollabRuntime::with_key_store(std::sync::Arc::new(
        op_collab_transport::FileKeyStore::new(dir.join("collaboration")),
    ));
    collab.set_transport_capabilities(CollabTransportCapabilities::RELAY_AND_MANUAL_JOIN);

    let action = match role.as_str() {
        "owner" => CollabUiAction::Start,
        "guest" => CollabUiAction::JoinAddress {
            endpoint: guest_code.expect("guest requires an invite code"),
        },
        _ => unreachable!("role validated before authentication"),
    };
    host.editor_state_mut().editor_ui.collab.pending_action = Some(action);

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut drag = Drag::Waiting;
    let mut drag_count: i64 = 0;
    let mut next_drag_at = Instant::now();
    let mut last_hash = document_hash(&host);
    println!("[{}] hash {last_hash}", stamp(started));
    let mut last_phase = CollabConnectionPhase::Idle;
    let mut announced_invite = false;
    let mut last_notice = String::new();
    let mut last_participants = 0_usize;
    while Instant::now() < deadline {
        let _ = op_auth_bridge::poll(op_auth_bridge::SESSION_HANDLE);
        collab.refresh_availability(&mut host);
        if !collab.local_edit_in_flight() {
            collab.drain_ui_action(&mut host);
        }
        collab.poll(&mut host);
        for status in collab.drain_status_events() {
            println!("[{}] status {status:?}", stamp(started));
        }
        let collab_ui = &host.editor_state().editor_ui.collab;
        let phase = collab_ui.phase;
        if phase != last_phase {
            println!("[{}] phase {last_phase:?} -> {phase:?}", stamp(started));
            last_phase = phase;
        }
        let notice = format!("{:?}", collab_ui.notice);
        if notice != last_notice {
            println!("[{}] notice {notice}", stamp(started));
            last_notice = notice;
        }
        let participants = collab_ui.participants().len();
        if participants != last_participants {
            println!("[{}] participants = {participants}", stamp(started));
            last_participants = participants;
        }
        if !announced_invite {
            if let Some(invite) = collab_ui
                .public_session()
                .and_then(op_editor_core::CollabPublicSessionUi::invite)
            {
                let path = invite_file.as_ref().expect("owner invite file path");
                write_invite_file(path, invite.as_str()).unwrap_or_else(|error| {
                    panic!("write new invite file {}: {error}", path.display())
                });
                println!(
                    "[{}] invite written to {} (content intentionally not logged)",
                    stamp(started),
                    path.display()
                );
                announced_invite = true;
            }
        }
        // Headless confirmation is deliberately opt-in because this probe uses
        // real accounts and a real relay. Compare identities out of band first.
        if auto_approve {
            let approve = collab_ui
                .pending_admissions()
                .first()
                .map(|pending| CollabUiAction::ApproveAdmissionEditor {
                    request_key: pending.request_key().clone(),
                })
                .or_else(|| {
                    collab_ui.pending_owner_confirmation().map(|pending| {
                        CollabUiAction::ConfirmOwnerIdentity {
                            request_key: pending.request_key().clone(),
                        }
                    })
                });
            if let Some(approve) = approve {
                println!("[{}] explicit auto-action {approve:?}", stamp(started));
                host.editor_state_mut().editor_ui.collab.pending_action = Some(approve);
            }
        }
        // One simulated pointer gesture is one collaboration transaction, exactly
        // as the desktop host drives it: press opens the capture, the document
        // moves on every frame of the drag, release closes it.
        let participants_now = host.editor_state().editor_ui.collab.participants().len();
        match drag {
            Drag::Waiting => {
                if drag_enabled && participants_now >= 2 && Instant::now() >= next_drag_at {
                    // Every gesture starts where the last one stopped, so no
                    // drag can be a no-op that never reaches the peer.
                    let base = drag_count * i64::from(drag_ticks);
                    if collab.begin_local_edit(&mut host) {
                        drag_count += 1;
                        println!("[{}] drag begin", stamp(started));
                        drag = Drag::Holding { step: 0, base };
                    } else {
                        next_drag_at = Instant::now() + Duration::from_secs(1);
                    }
                }
            }
            Drag::Holding { step, base } => {
                if step < drag_ticks {
                    host.editor_state_mut().doc = document_at(base + i64::from(step));
                    drag = Drag::Holding {
                        step: step + 1,
                        base,
                    };
                } else {
                    let outcome = collab.finish_local_edit(&mut host);
                    println!(
                        "[{}] drag end outcome={outcome:?} hash={}",
                        stamp(started),
                        document_hash(&host)
                    );
                    drag = Drag::Waiting;
                    next_drag_at = Instant::now() + drag_gap;
                }
            }
        }
        // The desktop publishes the pointer every frame; during a drag that is a
        // continuous stream of presence broadcasts alongside the gesture.
        let cursor = match drag {
            Drag::Holding { step, base } => {
                Some(((base + i64::from(step)) as f64, f64::from(step)))
            }
            // A live user's pointer keeps moving between gestures, so presence
            // keeps flowing even when the document is not changing.
            Drag::Waiting => Some((started.elapsed().as_millis() as f64 % 512.0, 7.0)),
        };
        let _ = collab.publish_local_presence(&mut host, cursor);
        let hash = document_hash(&host);
        if hash != last_hash {
            if !matches!(drag, Drag::Holding { .. }) {
                println!("[{}] hash {hash}", stamp(started));
            }
            last_hash = hash;
        }

        std::thread::sleep(Duration::from_millis(20));
    }
    println!("[{}] probe finished", stamp(started));
    collab.leave(&mut host);
    if let Some(path) = invite_file {
        if let Err(error) = std::fs::remove_file(&path) {
            eprintln!(
                "warning: failed to remove invite file {}: {error}",
                path.display()
            );
        }
    }
}
