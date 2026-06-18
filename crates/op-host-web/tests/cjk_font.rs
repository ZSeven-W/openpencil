#[test]
fn web_host_does_not_bundle_fallback_font_files() {
    let assets_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let bundled_fonts = std::fs::read_dir(&assets_dir)
        .expect("assets dir exists")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext, "ttf" | "otf" | "woff" | "woff2"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    assert!(
        bundled_fonts.is_empty(),
        "web host must use browser/system fonts instead of bundled fallback font files: {bundled_fonts:?}"
    );
}

#[test]
fn browser_text_fallback_is_the_default_canvas_text_path() {
    let source = std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable");

    assert!(
        source.contains("export async function opCkInit(canvasId)"),
        "CanvasKit init must not accept bundled font bytes"
    );
    assert!(
        !source.contains("MakeFreeTypeFaceFromData(copyBytes(roboto))"),
        "CanvasKit init must not create bundled Latin fonts"
    );
    assert!(
        !source.contains("MakeFreeTypeFaceFromData(copyBytes(cjk))"),
        "CanvasKit init must not create bundled CJK fonts"
    );
    assert!(
        !source.contains("MakeFreeTypeFaceFromData(copyBytes(emoji))"),
        "CanvasKit init must not create bundled emoji fonts"
    );
    assert!(
        source.contains(
            "const shouldUseBrowserTextFallback = (_t, _emojiRun) => Boolean(browserTextCtx);"
        ),
        "browser/system fonts must be the default text path"
    );
}
