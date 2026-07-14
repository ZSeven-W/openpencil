#[test]
fn canvaskit_mount_suppresses_browser_context_menu() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/canvaskit.rs"))
        .expect("canvaskit source is readable");

    assert!(
        source.contains("\"contextmenu\""),
        "CanvasKit mount must register a contextmenu listener so right-click stays owned by the editor"
    );
    assert!(
        source.contains("evt.prevent_default();"),
        "contextmenu listener must prevent the browser's native menu"
    );
}

#[test]
fn canvaskit_mount_listens_for_window_resize() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/canvaskit.rs"))
        .expect("canvaskit source is readable");

    assert!(
        source.contains("\"resize\""),
        "CanvasKit mount must listen for window resize so docked DevTools/window changes relayout the editor"
    );
    assert!(
        source.contains("resize_to_window"),
        "resize listener must resize the CanvasKit backend, not only the DOM canvas"
    );
}

#[test]
fn canvaskit_mount_syncs_window_size_before_first_repaint() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/canvaskit.rs"))
        .expect("canvaskit source is readable");
    let start = source
        .find("let inner = Rc::new(RefCell::new(CkInner")
        .expect("mount creates CkInner");
    let first_repaint = source[start..]
        .find("repaint();")
        .map(|idx| start + idx)
        .expect("mount performs an initial repaint");
    let initial_mount = &source[start..first_repaint];

    assert!(
        initial_mount.contains("resize_to_window(&window)"),
        "CanvasKit mount must sync the backend to the current browser viewport before the first repaint"
    );
}

#[test]
fn canvaskit_mount_does_not_panic_if_a11y_mirror_borrow_is_busy() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/canvaskit.rs"))
        .expect("canvaskit source is readable");
    let start = source
        .find("let mirror_target =")
        .expect("a11y mirror listener setup exists");
    let setup = &source[start..source[start..].find("// mousedown").unwrap() + start];

    assert!(
        setup.contains("try_borrow()"),
        "CanvasKit a11y mirror listener setup must use try_borrow so a transient borrow cannot panic the web host"
    );
    assert!(
        !setup.contains(".borrow()"),
        "CanvasKit a11y mirror listener setup must not force-borrow the shared host"
    );
}

#[test]
fn canvaskit_mount_loads_browser_settings_before_fingerprints_and_first_repaint() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/canvaskit.rs"))
        .expect("canvaskit source is readable");
    let host = source.find("WidgetHost::new()").expect("host construction");
    let load = source[host..]
        .find("web_settings::load_into")
        .map(|idx| host + idx)
        .expect("browser settings load");
    let fingerprint = source[load..]
        .find("initial_settings_fingerprint")
        .map(|idx| load + idx)
        .expect("settings fingerprint");
    let first_repaint = source[fingerprint..]
        .find("repaint();")
        .map(|idx| fingerprint + idx)
        .expect("first repaint");

    assert!(host < load && load < fingerprint && fingerprint < first_repaint);
}

#[test]
fn canvaskit_repaint_persists_local_settings_and_syncs_only_credential_changes() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/canvaskit.rs"))
        .expect("canvaskit source is readable");
    let repaint_start = source.find("impl CkInner").expect("CkInner implementation");
    let repaint_end = source[repaint_start..]
        .find("fn sync_a11y")
        .map(|idx| repaint_start + idx)
        .expect("end of repaint method");
    let repaint = &source[repaint_start..repaint_end];

    assert!(repaint.contains("web_settings::save_if_changed"));
    assert!(repaint.contains("web_settings::save_credentials_if_changed"));
    assert!(repaint.contains("web_settings::credential_migration_pending"));
    assert!(repaint.contains("web_credential_sync::credential_changed"));

    let credential_save = repaint
        .find("web_settings::save_credentials_if_changed")
        .expect("credential save");
    let general_save = repaint
        .find("web_settings::save_if_changed")
        .expect("general settings save");
    assert!(
        credential_save < general_save,
        "migration must secure credentials before ordinary settings can overwrite the legacy key"
    );

    let install = source
        .find("repaint_coalescer::install")
        .expect("repaint coalescer installation");
    let sync_start = source
        .find("web_credential_sync::start")
        .expect("credential policy discovery");
    assert!(
        sync_start < install,
        "credential sync state is reset before repaint callbacks can queue changes"
    );
}

#[test]
fn canvaskit_mount_queues_an_initial_snapshot_only_when_local_credentials_exist() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/canvaskit.rs"))
        .expect("canvaskit source is readable");
    let load = source
        .find("let credential_load = crate::web_settings::load_into")
        .expect("mount records whether an independent credential snapshot loaded");
    let conditional = source[load..]
        .find("let initial_credential_json = credential_load")
        .map(|index| load + index)
        .expect("credential snapshot presence gates the initial server sync");
    let policy = source[conditional..]
        .find("web_credential_sync::start")
        .map(|index| conditional + index)
        .expect("policy discovery resets state before the initial snapshot is queued");
    let changed = source[policy..]
        .find("web_credential_sync::credential_changed")
        .map(|index| policy + index)
        .expect("an existing credential snapshot queues after policy discovery starts");

    assert!(load < conditional && conditional < policy && policy < changed);
}

#[test]
fn credential_status_requests_use_a_finite_timeout() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/live_sync.rs"))
        .expect("live sync source is readable");
    let get_with_status = source
        .split("pub fn get_with_status")
        .nth(1)
        .and_then(|body| body.split("pub fn post_json").next())
        .expect("status-aware GET implementation");
    let post_with_status = source
        .split("pub fn post_json_with_status")
        .nth(1)
        .expect("status-aware POST implementation");

    assert!(get_with_status.contains("xhr.set_timeout(STATUS_REQUEST_TIMEOUT_MS)"));
    assert!(post_with_status.contains("xhr.set_timeout(STATUS_REQUEST_TIMEOUT_MS)"));
}
