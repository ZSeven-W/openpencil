//! Cross-platform single-instance guard + second-launch file forwarding.
//!
//! TS parity: `apps/desktop/main.ts` uses Electron's
//! `requestSingleInstanceLock()` + the `second-instance` event to forward a
//! double-clicked document path into the already-running window. Upstream
//! winit (and the `casement` fork) has no equivalent, and on Windows / Linux
//! the OS file-association launches a fresh process per double-click. Without
//! this guard a second `.op` open would spawn a second editor window instead
//! of surfacing the file in the running one.
//!
//! Mechanism — deliberately free of platform `#[cfg]` and of any new
//! dependency:
//!
//! * The PRIMARY instance binds a fixed loopback TCP port. A successful bind
//!   IS the mutual-exclusion primitive — only one process can hold a given
//!   `127.0.0.1:<port>` at a time, identically on macOS / Windows / Linux.
//! * A SECONDARY instance (bind fails with `AddrInUse`) connects to that port,
//!   sends a one-line `MAGIC\t<path>` frame, and exits — the running editor
//!   opens the file.
//! * If the bind fails for an UNRELATED reason (the port is held by some other
//!   app), the handshake won't echo `OK`, so the secondary falls back to
//!   launching its own window. Single-instance silently degrades; the app
//!   still works.
//!
//! Trust model: the listener is loopback-only and the protocol is gated on a
//! magic header, so a stray local connection can't drive an open. Like
//! Electron/Tauri's default single-instance IPC, same-user loopback peers are
//! trusted (a same-user process could open the file directly anyway). macOS
//! app bundles are already single-instance via LaunchServices + Apple events
//! (`drain_opened_files`); this guard additionally covers the raw binary and
//! the Windows / Linux file-association path.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use winit::event_loop::EventLoopProxy;

use crate::DesktopEvent;

/// App-specific high loopback port. Distinct from the live-MCP default (3100)
/// and from common dev-server ports so an unrelated listener rarely collides.
const INSTANCE_PORT: u16 = 47823;

/// Protocol header. A peer that doesn't open with this isn't our editor, so the
/// secondary won't treat a stray listener as a forward target.
const MAGIC: &str = "OPENPENCIL-OPEN";

/// Shared queue of paths forwarded by secondary launches, drained on the UI
/// thread once the proxy wakes the event loop.
pub type ForwardQueue = Arc<Mutex<VecDeque<PathBuf>>>;

/// Outcome of the single-instance gate.
pub enum Acquire {
    /// This process owns the instance. Hold the returned handle and, once the
    /// event loop exists, call [`PrimaryHandle::spawn_listener`].
    Primary(PrimaryHandle),
    /// A running instance accepted the forwarded file; the caller should exit.
    Forwarded,
}

/// Held by the primary process. Carries the bound listener (when the bind
/// succeeded) so the accept loop can start after the `EventLoopProxy` exists.
pub struct PrimaryHandle {
    listener: Option<TcpListener>,
}

/// Run the gate. `initial_file` is the document parsed from argv, if any.
pub fn acquire(initial_file: Option<&Path>) -> Acquire {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, INSTANCE_PORT));
    match TcpListener::bind(addr) {
        Ok(listener) => Acquire::Primary(PrimaryHandle {
            listener: Some(listener),
        }),
        Err(_) => {
            // Port busy (or bind failed for another reason): try to hand the
            // file to whoever holds it.
            if forward_to_primary(addr, initial_file) {
                return Acquire::Forwarded;
            }
            // No editor answered. The holder may have just exited (the bind
            // failed, then the port freed before our connect), or the bind
            // failed transiently. Try ONCE more to become the real primary
            // before degrading — this closes the bind/connect race window so
            // we don't run as an unguarded "primary" that can't receive
            // forwards while another process owns the port.
            match TcpListener::bind(addr) {
                Ok(listener) => Acquire::Primary(PrimaryHandle {
                    listener: Some(listener),
                }),
                // Still can't bind: launch normally but without a listener —
                // best-effort, single-instance silently degrades this run.
                Err(_) => Acquire::Primary(PrimaryHandle { listener: None }),
            }
        }
    }
}

/// Connect to the running primary and forward the document path. Returns true
/// only when the peer acknowledged with `OK` (confirming it is our editor).
fn forward_to_primary(addr: SocketAddr, initial_file: Option<&Path>) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
    let path = initial_file
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    // One line: `MAGIC\t<path>\n`. An empty path is a bare "focus me" ping.
    if stream
        .write_all(format!("{MAGIC}\t{path}\n").as_bytes())
        .is_err()
    {
        return false;
    }
    let _ = stream.flush();
    let mut reply = String::new();
    if BufReader::new(&mut stream).read_line(&mut reply).is_err() {
        return false;
    }
    reply.trim() == "OK"
}

impl PrimaryHandle {
    /// Start accepting forwarded opens. Each valid frame pushes its path onto
    /// `queue` and wakes the event loop via `DesktopEvent::ForwardedFileReady`
    /// so the UI thread can open it + raise the window. No-op when the bind
    /// failed (degraded single-instance — see module docs).
    pub fn spawn_listener(self, proxy: EventLoopProxy<DesktopEvent>, queue: ForwardQueue) {
        let Some(listener) = self.listener else {
            return;
        };
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                if accept_frame(stream, &queue) {
                    // Wake the UI thread to drain the queue + raise the window
                    // (true even for a bare ping with no path — the user
                    // double-clicked the app, so surface it).
                    let _ = proxy.send_event(DesktopEvent::ForwardedFileReady);
                }
            }
        });
    }
}

/// Read one frame, validate the magic header, enqueue any non-empty path, and
/// reply `OK` / `ERR` so the secondary can confirm it reached our editor.
/// Returns true when the frame was a valid OpenPencil request (the caller
/// should then wake the loop). Kept proxy-free so it is unit-testable without
/// a winit event loop.
fn accept_frame(mut stream: TcpStream, queue: &ForwardQueue) -> bool {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let mut line = String::new();
    let read = {
        let mut reader = BufReader::new(&mut stream);
        reader.read_line(&mut line)
    };
    if read.is_err() {
        return false;
    }
    let line = line.trim_end_matches(['\r', '\n']);
    // Not our protocol (some other app holds the port) → reject, don't wake.
    let Some((magic, path)) = line.split_once('\t') else {
        let _ = stream.write_all(b"ERR\n");
        return false;
    };
    if magic != MAGIC {
        let _ = stream.write_all(b"ERR\n");
        return false;
    }
    if !path.is_empty() {
        if let Ok(mut queue) = queue.lock() {
            queue.push_back(PathBuf::from(path));
        }
    }
    let _ = stream.write_all(b"OK\n");
    let _ = stream.flush();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end of the wire protocol: a manually-bound listener (standing in
    /// for the primary) + a `forward_to_primary` call (the secondary) must
    /// round-trip the document path onto the shared queue. Kept off the
    /// `PrimaryHandle`/`EventLoop` path so it runs on any test thread (winit
    /// refuses to build an event loop off the macOS main thread).
    #[test]
    fn forward_protocol_round_trips_path() {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, INSTANCE_PORT));
        let listener = match TcpListener::bind(addr) {
            Ok(listener) => listener,
            // Port already held (sandbox / parallel run) — skip, don't flake.
            Err(_) => return,
        };
        let queue: ForwardQueue = Arc::new(Mutex::new(VecDeque::new()));
        let server_queue = queue.clone();
        let server = std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                accept_frame(stream, &server_queue)
            } else {
                false
            }
        });

        let path = std::env::temp_dir().join("single-instance-fixture.op");
        assert!(
            forward_to_primary(addr, Some(&path)),
            "secondary should forward"
        );
        assert!(
            server.join().unwrap(),
            "accept_frame should accept a valid frame"
        );
        assert_eq!(
            queue.lock().unwrap().pop_front().as_deref(),
            Some(path.as_path()),
        );
    }

    /// A frame without the magic header (some other app on the port) must be
    /// rejected and must not enqueue anything.
    #[test]
    fn non_magic_frame_is_rejected() {
        use std::io::Write;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("ephemeral bind");
        let addr = listener.local_addr().unwrap();
        let queue: ForwardQueue = Arc::new(Mutex::new(VecDeque::new()));
        let server_queue = queue.clone();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            accept_frame(stream, &server_queue)
        });

        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(b"GET / HTTP/1.1\r\n").unwrap();
        stream.flush().unwrap();
        assert!(!server.join().unwrap(), "non-magic frame must be rejected");
        assert!(queue.lock().unwrap().is_empty());
    }
}
