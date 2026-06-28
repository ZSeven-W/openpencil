//! Design fonts shipped inside the renderer.
//!
//! Designs (Pencil `.pen` / `.op`) reference Google Fonts like Inter,
//! Space Grotesk or Manrope that are not installed on a typical machine.
//! Without them the skia paragraph shaper falls back to the platform
//! default for both painting *and* measuring — wrong glyphs, wrong
//! weight, and (because `fit_content` heights come from the measured
//! glyphs) every text block lands at the wrong height. Bundling the
//! common families and registering them with `jian-skia` at startup
//! makes the headless render-shots path and the editor canvas resolve
//! them with no system install (parity with Pencil, which ships fonts).
//!
//! All bundled files are SIL Open Font License (OFL) — redistributable.
//! This is a curated common set, not all of Google Fonts; an
//! unbundled-and-uninstalled family still falls back to the default.

/// OFL `.ttf` blobs embedded at compile time. Variable fonts (`*-VF`)
/// carry the full weight axis; a few families ship as static weights.
const BUNDLED: &[&[u8]] = &[
    include_bytes!("../assets/fonts/Inter-VF.ttf"),
    include_bytes!("../assets/fonts/SpaceGrotesk-VF.ttf"),
    include_bytes!("../assets/fonts/Manrope-VF.ttf"),
    include_bytes!("../assets/fonts/Outfit-VF.ttf"),
    include_bytes!("../assets/fonts/DMSans-VF.ttf"),
    include_bytes!("../assets/fonts/DMSerifDisplay-Regular.ttf"),
    include_bytes!("../assets/fonts/DMMono-Regular.ttf"),
    include_bytes!("../assets/fonts/DMMono-Medium.ttf"),
    include_bytes!("../assets/fonts/InstrumentSerif-Regular.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-VF.ttf"),
    include_bytes!("../assets/fonts/CormorantGaramond-VF.ttf"),
];

/// Register the bundled fonts with `jian-skia` so its draw + measure
/// paths resolve them. Idempotent (first call wins); call once before
/// any layout / render pass.
pub fn register() {
    jian_skia::register_bundled_fonts(BUNDLED.iter().map(|b| b.to_vec()).collect());
}
