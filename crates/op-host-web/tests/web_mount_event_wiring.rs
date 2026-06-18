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
