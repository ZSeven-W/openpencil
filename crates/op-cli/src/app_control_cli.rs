use serde_json::json;
use std::env;
use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const PID_FILE_NAME: &str = "openpencil-mcp-server.pid";
const PORT_FILE_NAME: &str = "openpencil-mcp-server.port";
const MINIMAL_DOCUMENT: &str = "{\n  \"version\": \"0.8.0\",\n  \"name\": \"OpenPencil CLI Session\",\n  \"children\": []\n}\n";

#[derive(Debug, Clone, Copy)]
struct RunningMcp {
    pid: u32,
    port: u16,
}

pub(crate) fn run_start(port: u16, document_path: Option<&str>) -> Result<String, String> {
    if let Some(existing) = running_mcp_from_pid_file() {
        return Ok(start_json(existing.pid, existing.port, None));
    }

    let document = match document_path {
        Some(path) => PathBuf::from(path),
        None => default_document_path()?,
    };
    ensure_document_file(&document)?;

    let binary = find_desktop_binary()?;
    let mut child = Command::new(&binary)
        .arg("--mcp-http")
        .arg(port.to_string())
        .arg(&document)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn {} --mcp-http: {e}", binary.display()))?;

    let pid = child.id();
    write_manager_files(pid, port)?;

    for _ in 0..30 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(start_json(pid, port, Some(&document)));
        }
        if let Ok(Some(status)) = child.try_wait() {
            remove_manager_files();
            return Err(format!(
                "OpenPencil MCP server exited before accepting connections: {status}"
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }

    Ok(start_json(pid, port, Some(&document)))
}

pub(crate) fn run_stop() -> Result<String, String> {
    if let Some(info) = running_mcp_from_pid_file() {
        terminate_pid(info.pid)?;
        remove_manager_files();
        return Ok(json!({
            "ok": true,
            "running": false,
            "message": "OpenPencil MCP server stopped",
        })
        .to_string());
    }

    remove_manager_files();
    Ok(json!({
        "ok": true,
        "running": false,
        "message": "No running MCP server found",
    })
    .to_string())
}

pub(crate) fn ensure_document_file(path: &Path) -> Result<(), String> {
    if path.exists() {
        if path.is_file() {
            return Ok(());
        }
        return Err(format!("{} exists but is not a file", path.display()));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(path, MINIMAL_DOCUMENT).map_err(|e| format!("write {}: {e}", path.display()))
}

fn start_json(pid: u32, port: u16, document_path: Option<&Path>) -> String {
    let mut value = json!({
        "ok": true,
        "running": true,
        "pid": pid,
        "port": port,
        "url": format!("http://127.0.0.1:{port}"),
        "mcpUrl": format!("http://127.0.0.1:{port}/mcp"),
    });
    if let Some(path) = document_path {
        value["documentPath"] = json!(path.display().to_string());
    }
    value.to_string()
}

fn running_mcp_from_pid_file() -> Option<RunningMcp> {
    let (pid_file, port_file) = manager_files();
    let pid = fs::read_to_string(&pid_file)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()?;
    if !is_pid_alive(pid) {
        remove_manager_files();
        return None;
    }
    let port = fs::read_to_string(&port_file)
        .ok()
        .and_then(|text| text.trim().parse::<u16>().ok())
        .unwrap_or(3100);
    Some(RunningMcp { pid, port })
}

fn write_manager_files(pid: u32, port: u16) -> Result<(), String> {
    let (pid_file, port_file) = manager_files();
    fs::write(&pid_file, pid.to_string())
        .map_err(|e| format!("write {}: {e}", pid_file.display()))?;
    fs::write(&port_file, port.to_string())
        .map_err(|e| format!("write {}: {e}", port_file.display()))
}

fn remove_manager_files() {
    let (pid_file, port_file) = manager_files();
    let _ = fs::remove_file(pid_file);
    let _ = fs::remove_file(port_file);
}

fn manager_files() -> (PathBuf, PathBuf) {
    let dir = env::temp_dir();
    (dir.join(PID_FILE_NAME), dir.join(PORT_FILE_NAME))
}

fn default_document_path() -> Result<PathBuf, String> {
    home_dir().map(|home| home.join(".openpencil").join("cli-session.op"))
}

fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "home directory not available; pass --file <path.op>".to_string())
}

fn find_desktop_binary() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("OPENPENCIL_DESKTOP_BIN").map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "OPENPENCIL_DESKTOP_BIN points to a missing file: {}",
            path.display()
        ));
    }

    for path in desktop_binary_candidates() {
        if path.is_file() {
            return Ok(path);
        }
    }
    Err("OpenPencil desktop binary not found; set OPENPENCIL_DESKTOP_BIN or build openpencil-desktop".into())
}

fn desktop_binary_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    if let Some(dir) = exe_dir {
        candidates.push(dir.join(desktop_binary_name()));
        candidates.push(dir.join("OpenPencil"));
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("target").join("debug").join(desktop_binary_name()));
        candidates.push(
            cwd.join("target")
                .join("release")
                .join(desktop_binary_name()),
        );
    }

    if cfg!(target_os = "macos") {
        if let Ok(home) = home_dir() {
            candidates.push(
                home.join("Applications")
                    .join("OpenPencil.app")
                    .join("Contents")
                    .join("MacOS")
                    .join("OpenPencil"),
            );
        }
        candidates.push(
            PathBuf::from("/Applications")
                .join("OpenPencil.app")
                .join("Contents")
                .join("MacOS")
                .join("OpenPencil"),
        );
    } else if cfg!(target_os = "windows") {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            candidates.push(
                local_app_data
                    .join("Programs")
                    .join("openpencil")
                    .join("OpenPencil.exe"),
            );
        }
        candidates.push(PathBuf::from(r"C:\Program Files\OpenPencil\OpenPencil.exe"));
        candidates.push(PathBuf::from(
            r"C:\Program Files (x86)\OpenPencil\OpenPencil.exe",
        ));
    } else {
        candidates.push(PathBuf::from("/usr/bin/openpencil"));
        candidates.push(PathBuf::from("/usr/local/bin/openpencil"));
        if let Ok(home) = home_dir() {
            candidates.push(home.join(".local").join("bin").join("openpencil"));
            collect_app_images(&home.join("Applications"), &mut candidates);
            collect_app_images(&home.join("Downloads"), &mut candidates);
        }
    }
    candidates
}

fn desktop_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "openpencil-desktop.exe"
    } else {
        "openpencil-desktop"
    }
}

fn collect_app_images(dir: &Path, candidates: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("OpenPencil") && name.ends_with(".AppImage") {
            candidates.push(path);
        }
    }
}

#[cfg(unix)]
fn is_pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_pid_alive(pid: u32) -> bool {
    let filter = format!("PID eq {pid}");
    Command::new("tasklist")
        .arg("/FI")
        .arg(filter)
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| stdout.contains(&pid.to_string()))
        .unwrap_or(false)
}

#[cfg(unix)]
fn terminate_pid(pid: u32) -> Result<(), String> {
    Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|_| ())
        .map_err(|e| format!("kill {pid}: {e}"))
}

#[cfg(windows)]
fn terminate_pid(pid: u32) -> Result<(), String> {
    Command::new("taskkill")
        .arg("/PID")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|_| ())
        .map_err(|e| format!("taskkill {pid}: {e}"))
}
