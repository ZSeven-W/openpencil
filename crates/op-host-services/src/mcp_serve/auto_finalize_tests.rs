use super::*;
use op_editor_core::PenNodeExt;
use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

#[test]
fn due_requires_a_write_idle_time_and_new_revision() {
    let now = Instant::now();
    let mut auto = AutoFinalize {
        last_write_at: None,
        finalized_at_revision: None,
        idle_after: Duration::from_secs(10),
    };
    assert!(!auto.due(now + Duration::from_secs(10), 7));

    auto.note_write(now);
    assert!(!auto.due(now + Duration::from_secs(9), 7));
    assert!(auto.due(now + Duration::from_secs(10), 7));

    auto.finalized_at_revision = Some(7);
    assert!(!auto.due(now + Duration::from_secs(10), 7));
    assert!(auto.due(now + Duration::from_secs(10), 8));
}

#[test]
fn from_env_supports_disable_and_idle_override() {
    let _lock = env_lock();
    let _disable = EnvGuard::set(AUTO_FINALIZE_ENV, "0");
    let _idle = EnvGuard::set(AUTO_FINALIZE_IDLE_SECS_ENV, "3");
    let disabled = AutoFinalize::from_env();
    assert_eq!(disabled.idle_after, Duration::MAX);
    assert!(!disabled.due(Instant::now() + Duration::from_secs(100), 1));
    drop(_disable);

    let enabled = AutoFinalize::from_env();
    assert_eq!(enabled.idle_after, Duration::from_secs(3));
}

#[test]
fn from_env_uses_default_for_missing_or_invalid_threshold() {
    let _lock = env_lock();
    let _mode = EnvGuard::unset(AUTO_FINALIZE_ENV);
    let _idle = EnvGuard::set(AUTO_FINALIZE_IDLE_SECS_ENV, "not-a-number");
    assert_eq!(AutoFinalize::from_env().idle_after, DEFAULT_IDLE_AFTER);
}

#[test]
fn disabled_env_keeps_stdio_eof_from_finalizing() {
    let _lock = env_lock();
    let _disable = EnvGuard::set(AUTO_FINALIZE_ENV, "0");
    let _idle = EnvGuard::set(AUTO_FINALIZE_IDLE_SECS_ENV, "0");
    let path = std::env::temp_dir().join(format!(
        "openpencil-mcp-no-finalize-{}-{}.op",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));
    std::fs::write(
        &path,
        r##"{"version":"1.0.0","children":[{
          "type":"frame","id":"screen","name":"Screen",
          "width":390,"height":844,"layout":"vertical",
          "children":[
            {"type":"frame","id":"status","name":"Status Bar","role":"status-bar","width":"fill_container","height":62},
            {"type":"frame","id":"nav","name":"Bottom Tab Bar","role":"bottom-tab-bar","width":"fill_container","height":72,"layout":"horizontal"},
            {"type":"frame","id":"content","name":"Content","width":"fill_container","height":200}
          ]
        }]}"##,
    )
    .expect("seed document");
    let mut state = super::super::load_editor_state(&path).expect("load document");
    let write = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"batch_design","arguments":{"operations":"I(null,{\"type\":\"rectangle\",\"name\":\"Write\",\"x\":500,\"y\":0,\"width\":20,\"height\":20})"}}}"#;
    let mut input = std::io::Cursor::new(format!("{write}\n").into_bytes());
    let mut output = Vec::new();
    let mut auto = AutoFinalize::from_env();
    let shutdown = std::sync::atomic::AtomicBool::new(false);
    run_stdio_session(
        &mut input,
        &mut output,
        &mut state,
        &path,
        &mut auto,
        &shutdown,
    )
    .expect("stdio session");

    let saved = super::super::load_editor_state(&path).expect("saved document");
    let children = saved.active_children()[0]
        .children()
        .expect("screen children");
    assert_eq!(children.get(1).map(|node| node.id_str()), Some("nav"));
    assert_eq!(children.last().map(|node| node.id_str()), Some("content"));
    assert!(!auto.due(Instant::now(), state.document_revision()));
    let _ = std::fs::remove_file(path);
}
