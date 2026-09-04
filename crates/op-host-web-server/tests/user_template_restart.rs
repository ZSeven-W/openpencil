//! A real headless daemon must rebuild saved user templates on every start.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

struct ScratchHome(PathBuf);

impl ScratchHome {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "op-host-web-server-user-templates-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("create scratch home");
        Self(path)
    }
}

impl Drop for ScratchHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Daemon {
    child: Child,
    port: u16,
    token: String,
}

impl Daemon {
    fn stop(&mut self) {
        drop(self.child.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self.child.try_wait().expect("poll daemon").is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        self.child.kill().expect("kill stuck daemon");
        let _ = self.child.wait();
        panic!("managed daemon did not exit after stdin EOF");
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            drop(self.child.stdin.take());
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn write_template(home: &Path, name: &str, node_id: &str) {
    let dir = home
        .join(".openpencil")
        .join("templates")
        .join("headless-deck");
    std::fs::create_dir_all(&dir).expect("create template directory");
    std::fs::write(
        dir.join("meta.json"),
        format!(r#"{{"name":{name:?},"frames":1,"frameWidth":100,"frameHeight":100}}"#),
    )
    .expect("write template metadata");
    std::fs::write(
        dir.join("document.op"),
        format!(
            r##"{{"version":"1.0.0","children":[{{"type":"frame","id":{node_id:?},"name":{name:?},"x":0,"y":0,"width":100,"height":100,"fill":[{{"type":"solid","color":"#ff0000"}}]}}]}}"##
        ),
    )
    .expect("write template document");
}

fn spawn_managed(home: &Path) -> Daemon {
    let mut child = Command::new(env!("CARGO_BIN_EXE_op-host-web-server"))
        .args([
            "--serve-web",
            "--managed",
            "--port",
            "0",
            "--allow-origin",
            "vscode-webview://user-template-restart-test",
        ])
        .env("HOME", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn managed daemon");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout piped"))
        .read_line(&mut line)
        .expect("read handshake");
    let handshake: serde_json::Value = serde_json::from_str(&line).expect("handshake JSON");
    Daemon {
        child,
        port: handshake["port"].as_u64().expect("handshake port") as u16,
        token: handshake["token"]
            .as_str()
            .expect("handshake token")
            .to_string(),
    }
}

fn post_mcp(daemon: &Daemon, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", daemon.port)).expect("connect daemon");
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-OpenPencil-Token: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        daemon.port,
        daemon.token,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes()).expect("write request");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("read response");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default()
}

fn get_document(daemon: &Daemon) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", daemon.port)).expect("connect daemon");
    let request = format!(
        "GET /api/mcp/document HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-OpenPencil-Token: {}\r\nConnection: close\r\n\r\n",
        daemon.port, daemon.token
    );
    stream.write_all(request.as_bytes()).expect("write request");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("read response");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    response
}

fn assert_template_loaded(daemon: &Daemon, name: &str) {
    let list = post_mcp(
        daemon,
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_scene_templates","arguments":{}}}"#,
    );
    assert!(list.contains("user:headless-deck"), "{list}");
    assert!(list.contains(name), "{list}");

    let used = post_mcp(
        daemon,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"use_scene_template","arguments":{"templateId":"user:headless-deck"}}}"#,
    );
    assert!(!used.contains(r#""isError":true"#), "{used}");
    let document = get_document(daemon);
    let expected_name = format!(r#""name":"{name}""#);
    assert!(
        document.contains(&expected_name),
        "template was listed but not applied; use reply: {used}; document: {document}"
    );
}

#[test]
fn managed_server_reloads_saved_user_templates_after_restart() {
    let home = ScratchHome::new();
    write_template(&home.0, "First Boot", "loaded-before-restart");

    let mut first = spawn_managed(&home.0);
    assert_template_loaded(&first, "First Boot");
    first.stop();

    // A changed on-disk artifact must win in the next process. This proves
    // startup scans the standard directory, rather than relying on registry
    // state left by a GUI construction in the previous process.
    write_template(&home.0, "Second Boot", "loaded-after-restart");
    let mut second = spawn_managed(&home.0);
    assert_template_loaded(&second, "Second Boot");
    assert!(!get_document(&second).contains("First Boot"));
    second.stop();
}
