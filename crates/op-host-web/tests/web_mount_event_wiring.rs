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
