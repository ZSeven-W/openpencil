#[test]
fn rust_check_runs_canvaskit_widget_host_tests() {
    let workflow = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/rust-check.yml"
    ))
    .expect("rust-check workflow is readable");

    assert!(
        workflow.contains("Run CanvasKit web host tests"),
        "rust-check should name the CanvasKit web host test step"
    );
    assert!(
        workflow.contains("cargo test -p op-host-web"),
        "rust-check should run op-host-web tests explicitly"
    );
    assert!(
        workflow.contains("--features canvaskit"),
        "rust-check must enable the production CanvasKit web host feature"
    );
}
