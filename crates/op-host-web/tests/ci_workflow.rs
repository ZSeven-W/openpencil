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

#[test]
fn release_workflow_documents_current_canvaskit_bundle_path() {
    let workflow = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/rust-release.yml"
    ))
    .expect("rust-release workflow is readable");

    assert!(
        workflow.contains("wasm-bundle-build.yml"),
        "release workflow notes should point at the current CanvasKit bundle workflow"
    );
    assert!(
        !workflow.contains("--features skia"),
        "release workflow must not describe the retired skia wasm build path"
    );
    assert!(
        !workflow.contains("EMSDK"),
        "release workflow must not describe the retired EMSDK wasm build path"
    );
}

#[test]
fn release_workflow_resolves_macos_signing_identity_from_keychain() {
    let workflow = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/rust-release.yml"
    ))
    .expect("rust-release workflow is readable");

    assert!(
        workflow.contains("Resolve macOS signing identity"),
        "release workflow should resolve the imported Developer ID identity before signing"
    );
    assert!(
        workflow.contains("security find-identity -v -p codesigning"),
        "release workflow should query the imported keychain identity instead of guessing it"
    );
    assert!(
        !workflow.contains("Developer ID Application ($APPLE_TEAM_ID)"),
        "release workflow must not hard-code an incomplete Developer ID identity"
    );
}

#[test]
fn release_workflow_notarizes_macos_app_before_packaging_dmg() {
    let workflow = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/rust-release.yml"
    ))
    .expect("rust-release workflow is readable");

    let notarize = workflow
        .find("Notarize macOS app")
        .expect("release workflow should notarize the .app before packaging");
    let package = workflow
        .find("Package DMG (macos)")
        .expect("release workflow should package a DMG after app notarization");
    assert!(
        notarize < package,
        "release workflow should notarize and staple OpenPencil.app before creating the DMG"
    );
    assert!(
        workflow.contains("ditto -c -k --keepParent \"$APP\""),
        "release workflow should submit a zipped .app bundle to Apple notarytool"
    );
    assert!(
        workflow.contains("xcrun stapler staple \"$APP\""),
        "release workflow should staple the notarization ticket to the .app before DMG packaging"
    );
    assert!(
        workflow.contains("xcrun notarytool log"),
        "release workflow should fetch the Apple notary log when notarization fails"
    );
    assert!(
        workflow.contains("::error::macOS app notarization failed"),
        "macOS app notarization failures should block the release"
    );
    assert!(
        workflow.contains("echo \"::error::macOS app notarization failed\"\n            exit 1"),
        "invalid notarization status can be reported with a zero notarytool exit code, so the workflow must exit non-zero explicitly"
    );
    assert!(
        !workflow.contains("exit \"$notary_status\""),
        "workflow must not reuse notarytool's exit code after parsing an Invalid status"
    );
    assert!(
        !workflow.contains("signed but not notarized"),
        "release workflow must not upload macOS artifacts after notarization fails"
    );
}

#[test]
fn macos_bundle_signing_uses_hardened_runtime_for_developer_id() {
    let script = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/bundle-macos.sh"
    ))
    .expect("bundle-macos script is readable");

    assert!(
        script.contains("--timestamp --options runtime"),
        "Developer ID macOS signing should enable hardened runtime and secure timestamp"
    );
    assert!(
        script.contains("sign_macos_code \"$APP/Contents/MacOS/openpencil-desktop\""),
        "bundle script should explicitly sign the main app executable before signing the bundle"
    );
    assert!(
        script.contains("sign_macos_code \"$APP/Contents/MacOS/op\""),
        "bundle script should sign the embedded op CLI before signing the bundle"
    );
}

#[test]
fn release_workflow_installs_nsis_on_windows_runner() {
    let workflow = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/rust-release.yml"
    ))
    .expect("rust-release workflow is readable");

    assert!(
        workflow.contains("Install NSIS (windows)"),
        "release workflow should install NSIS before invoking makensis"
    );
    assert!(
        workflow.contains("choco install nsis"),
        "release workflow should not assume makensis is preinstalled on windows-latest"
    );
    assert!(
        workflow.contains("GITHUB_PATH") && workflow.contains("makensis.exe"),
        "release workflow should add makensis.exe to PATH after installing NSIS"
    );
}

#[test]
fn release_workflow_distinguishes_published_npm_packages_from_check_failures() {
    let workflow = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.github/workflows/rust-release.yml"
    ))
    .expect("rust-release workflow is readable");

    assert!(
        workflow.contains("already published; skipping"),
        "release workflow should skip npm packages that are already published"
    );
    assert!(
        workflow.contains("E404") && workflow.contains("failed to check npm package"),
        "release workflow should publish only on npm 404 and fail on other npm view errors"
    );
}

#[test]
fn web_smoke_page_uses_current_canvaskit_bundle_command() {
    let smoke = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/smoke/step-1b.html"))
        .expect("web smoke page is readable");

    assert!(
        smoke.contains("--features canvaskit"),
        "web smoke page should show the current CanvasKit bundle build command"
    );
    assert!(
        smoke.contains("mod.mount_ck('op')"),
        "web smoke page should mount the production CanvasKit shell"
    );
    assert!(
        !smoke.contains("mod.mount('op')"),
        "web smoke page must not mount the default wasm stub"
    );
    assert!(
        !smoke.contains("--features skia"),
        "web smoke page must not point users at the retired skia wasm build"
    );
    assert!(
        !smoke.contains("EMSDK"),
        "web smoke page must not require the retired EMSDK setup"
    );
}

#[test]
fn web_integration_notes_avoid_legacy_ambiguous_markers() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "Cargo.toml",
        "src/dom_io.rs",
        "src/file_actions.rs",
        "src/iconify_web.rs",
        "src/lib.rs",
        "src/live_sync.rs",
        "src/raf_pump.rs",
        "src/web_ai_transport.rs",
        "src/web_chat.rs",
        "src/web_clipboard.rs",
        "src/web_fonts.rs",
    ];

    for file in files {
        let text = std::fs::read_to_string(manifest_dir.join(file))
            .unwrap_or_else(|err| panic!("{file} is readable: {err}"));
        assert!(
            !text.contains("UNVERIFIED in-browser"),
            "{file} should name the concrete smoke/daemon boundary instead of a blanket UNVERIFIED marker"
        );
        assert!(
            !text.contains("EMSDK"),
            "{file} should describe the current CanvasKit/web-sys boundary instead of the retired EMSDK path"
        );
    }
}

#[test]
fn browser_smoke_script_and_ci_cover_canvas_and_daemon_paths() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = std::fs::read_to_string(repo.join("tools/check-web-browser-smoke.sh"))
        .expect("browser smoke script is readable");
    let workflow = std::fs::read_to_string(repo.join(".github/workflows/wasm-bundle-build.yml"))
        .expect("wasm bundle workflow is readable");

    assert!(
        script.contains("--dump-dom"),
        "browser smoke should drive a real headless browser, not only curl"
    );
    assert!(
        script.contains("/smoke/step-1b.html"),
        "browser smoke should cover the pure CanvasKit mount harness"
    );
    assert!(
        script.contains("/api/mcp/server"),
        "browser smoke should cover the daemon health path"
    );
    assert!(
        script.contains("data-op-smoke=\"ok\""),
        "browser smoke should assert the DOM success marker written after mount"
    );
    assert!(
        workflow.contains("tools/check-web-browser-smoke.sh"),
        "bundle workflow should run the browser smoke after building the bundle"
    );
}

#[test]
fn web_rust_publish_has_docker_and_start_script_entrypoints() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dockerfile =
        std::fs::read_to_string(repo.join("Dockerfile.web-rust")).expect("Dockerfile readable");
    let start_script = std::fs::read_to_string(repo.join("scripts/start-web-rust.sh"))
        .expect("web rust start script is readable");

    assert!(
        dockerfile.contains("op-host-web-server --serve-web"),
        "web Rust Docker image should publish through the headless serve-web daemon"
    );
    assert!(
        start_script.contains("tools/check-wasm-bundle.sh"),
        "start script should build or verify the deployable CanvasKit bundle"
    );
    assert!(
        start_script.contains("op-host-web-server") && start_script.contains("--serve-web"),
        "start script should publish through op-host-web-server --serve-web"
    );
    assert!(
        start_script.contains("OPENPENCIL_WEB_BUNDLE_DIR"),
        "start script should point the daemon at the built web bundle explicitly"
    );
}
