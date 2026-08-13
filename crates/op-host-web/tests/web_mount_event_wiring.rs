#[test]
fn canvaskit_mount_suppresses_browser_context_menu() {
    let source = canvaskit_source();

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
    let source = canvaskit_source();

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
fn canvaskit_mount_releases_drags_from_the_window() {
    let source = canvaskit_source();
    let release = source
        .split("Window-level mouseup")
        .nth(1)
        .and_then(|body| body.split("// wheel").next())
        .expect("window-level release listener");

    assert!(release.contains("&win_target, \"mouseup\""));
    assert!(release.contains("apply_release_with_viewport"));
    assert!(
        !release.contains("&canvas_target, \"mouseup\""),
        "canvas-only mouseup strands a drag released outside the canvas"
    );
}

#[test]
fn canvaskit_mount_syncs_window_size_before_first_repaint() {
    let source = canvaskit_source();
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
    let source = canvaskit_source();
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
    let source = canvaskit_source();
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
    let source = canvaskit_source();
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
    let sync_reset = source
        .find("web_credential_sync::reset")
        .expect("credential sync state reset");
    assert!(
        sync_reset < install,
        "credential sync state is reset before repaint callbacks can queue changes"
    );
}

#[test]
fn canvaskit_mount_queues_an_initial_snapshot_only_when_local_credentials_exist() {
    let source = canvaskit_source();
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

    assert!(get_with_status.contains("xhr.set_timeout(DAEMON_FETCH_TIMEOUT_MS)"));
    assert!(post_with_status.contains("xhr.set_timeout(DAEMON_FETCH_TIMEOUT_MS)"));
}

#[test]
fn bridge_handle_init_recovers_ready_without_emitting_it_directly() {
    // `ready` must stay serialized after the bootstrap reset — the init handler
    // only stores the token and fires the one-shot late-init recovery hook, so a
    // post-timeout init still reaches `ready` (via the tokened recovery reset)
    // without the handler emitting `ready` itself. Structural guard: the true
    // interleaving (init landing during vs after the reset round-trip) is
    // runtime-only and can't be exercised natively.
    let source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/vscode_bridge.rs"))
            .expect("vscode_bridge source is readable");
    let handle_init = source
        // Split on the bare name — `handle_init` carries generic parameters, so
        // matching `fn handle_init(` silently stops finding the body.
        .split("fn handle_init")
        .nth(1)
        .and_then(|body| body.split("pub(crate) fn register_late_init_hook").next())
        .expect("handle_init implementation");

    assert!(
        !handle_init.contains("emit_ready"),
        "handle_init must not emit `ready` directly — it is serialized after the bootstrap reset"
    );
    assert!(
        handle_init.contains("LATE_INIT_HOOK") && handle_init.contains(".take()"),
        "handle_init must take() the one-shot late-init hook so a post-timeout init still reaches ready"
    );
    assert!(
        handle_init.contains("token_changed")
            && handle_init.contains("web_auth_sync::refresh_status(inner)"),
        "a newly installed managed token must immediately refresh auth status so the account button does not wait for the 30 s health tick"
    );
}

#[test]
fn canvaskit_bootstrap_completion_rechecks_bridge_token_live() {
    // Closes the late-init race: readiness is decided from the LIVE token at
    // completion time, not only the `managed` flag captured before the reset was
    // issued (a slow host's init can land anywhere in the reset's round-trip).
    let source = canvaskit_source();
    let managed = source
        .find("let managed = crate::live_sync::bridge_token().is_some();")
        .expect("bootstrap captures the pre-reset managed flag");
    let drive = source[managed..]
        .find("start_bootstrap_reset(base, complete, BOOTSTRAP_RESET_RETRIES);")
        .map(|idx| managed + idx)
        .expect("bootstrap drives the fallback reset");
    let completion = &source[managed..drive];

    assert!(
        completion.matches("bridge_token().is_some()").count() >= 2,
        "bootstrap completion must re-check bridge_token() LIVE, in addition to the pre-reset capture"
    );
    assert!(
        completion.contains("run_late_init_recovery")
            && completion.contains("register_late_init_hook"),
        "both the inline and hook recovery paths must route through the shared run_late_init_recovery"
    );
}

/// The CanvasKit host source: the `canvaskit.rs` spine plus every sibling
/// module under `canvaskit/` (the file was split at the 800-line ceiling).
fn canvaskit_source() -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut parts = vec![std::fs::read_to_string(format!("{root}/canvaskit.rs"))
        .expect("canvaskit spine is readable")];
    let mut siblings: Vec<std::path::PathBuf> = std::fs::read_dir(format!("{root}/canvaskit"))
        .expect("canvaskit module directory is readable")
        .map(|entry| entry.expect("canvaskit module entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    siblings.sort();
    for path in siblings {
        parts.push(std::fs::read_to_string(&path).expect("canvaskit module is readable"));
    }
    parts.join("\n")
}
