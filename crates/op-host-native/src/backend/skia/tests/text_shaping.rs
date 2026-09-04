//! Typeface resolution, weighted measurement, and multiline height —
//! the `SkiaMeasure` parity checks against what paint actually draws.
//!
//! Split out of `backend/skia/tests.rs` to keep every file under the
//! repo's 800-line cap.

use super::*;

#[test]
fn explicit_family_typeface_lookup_is_cached() {
    let _guard = crate::font_registry_test_support::lock();
    let mut be = NativeBackend::with_dpi(1.0);
    // A *concurrent* test registering bundled/imported fonts bumps the
    // process-global font generation, which clears the resolver's per-char
    // cache (`refresh_if_stale`) — spuriously breaking the `+ 1` counts.
    // Retry until we get one clean measurement window: a warm-up lookup first
    // syncs the resolver to the current generation (so a *prior* bump can't
    // clear mid-measure), then we require the cache to behave AND the global
    // generation to hold steady across the window. Under no contention the
    // first attempt succeeds.
    for attempt in 0..32 {
        // Warm-up on a DISTINCT char settles the resolver against the current
        // generation and seeds the cache, so `baseline` is stable.
        let _ = be.typeface_for_family_char('Z', "Georgia", 400);
        let gen = jian_skia::font_generation();
        let baseline = be.family_typeface_cache_len();

        let first = be
            .typeface_for_family_char('A', "Georgia", 400)
            .map(|tf| tf.unique_id());
        let grew_by_one = be.family_typeface_cache_len() == baseline + 1;

        let second = be
            .typeface_for_family_char('A', "Georgia", 400)
            .map(|tf| tf.unique_id());
        let still_one = be.family_typeface_cache_len() == baseline + 1;

        // A clean window: no concurrent generation bump cleared the cache,
        // and caching behaved as asserted. Otherwise retry.
        if jian_skia::font_generation() == gen && grew_by_one && second == first && still_one {
            return;
        }
        assert!(
            attempt < 31,
            "typeface cache never stabilized under concurrent font registration"
        );
    }
}

#[test]
fn skia_measure_matches_native_weighted_font_resolution() {
    let _guard = crate::font_registry_test_support::lock();
    // Same set as `font_fallback_tests::register_test_bundled_fonts` —
    // registration is process-global and first-call-wins, so the two lists
    // must agree or the registered set depends on test order.
    jian_skia::register_bundled_fonts(vec![
        include_bytes!("../../../../../op-host-desktop/assets/fonts/CormorantGaramond-VF.ttf")
            .to_vec(),
        include_bytes!("../../../../../op-host-desktop/assets/fonts/Inter-VF.ttf").to_vec(),
        include_bytes!("../../../../../op-host-desktop/assets/fonts/Outfit-VF.ttf").to_vec(),
    ]);

    struct Case {
        text: &'static str,
        family: &'static str,
        size: f32,
        weight: u16,
    }

    let cases = [
        Case {
            text: "$48,920",
            family: "Cormorant Garamond",
            size: 40.0,
            weight: 500,
        },
        Case {
            text: "$48,920",
            family: "Cormorant Garamond",
            size: 40.0,
            weight: 600,
        },
        Case {
            text: "Julian Thorne",
            family: "Inter",
            size: 14.0,
            weight: 600,
        },
        Case {
            text: "中文字体",
            family: "system-ui",
            size: 18.0,
            weight: 400,
        },
    ];

    let skia = jian_skia::SkiaMeasure::new();
    let mut native = NativeBackend::with_dpi(1.0);

    for case in cases {
        let run = StyledRun {
            text: case.text,
            font_family: Some(case.family),
            font_size: case.size,
            font_weight: case.weight,
            font_style: FontStyleKind::Normal,
            letter_spacing: 0.0,
        };
        let skia_width = skia
            .measure(&MeasureRequest {
                runs: &[run],
                line_height: 0.0,
                max_width: None,
            })
            .width;
        let native_width = native.measure_text_family_styled(
            case.text,
            case.size,
            case.family,
            case.weight,
            false,
        );
        let rel = (skia_width - native_width).abs() / skia_width.max(native_width).max(1.0);
        println!(
            "font parity text={:?} family={:?} size={} weight={} skia={:.3} native={:.3} diff={:.2}%",
            case.text,
            case.family,
            case.size,
            case.weight,
            skia_width,
            native_width,
            rel * 100.0
        );
        assert!(
            rel <= 0.02,
            "width drift exceeded 2% for {:?} / {:?}: skia={:.3} native={:.3} diff={:.2}%",
            case.text,
            case.family,
            skia_width,
            native_width,
            rel * 100.0
        );
    }
}

#[cfg_attr(
    target_os = "windows",
    ignore = "WINDOWS_SKIA_DIRECTWRITE_TEXT_MEASURE_ABORT: SkiaMeasure multiline height parity aborts in Windows CI; macOS and Linux keep coverage"
)]
#[test]
fn skia_measure_multiline_height_matches_native_painted_line_height() {
    struct Case {
        text: &'static str,
        size: f32,
        line_height: f32,
        lines: u16,
    }

    let cases = [
        Case {
            text: "contact\nhello@darkbrew.cz",
            size: 16.0,
            line_height: 0.0,
            lines: 2,
        },
        Case {
            text: "first\nsecond\nthird",
            size: 18.0,
            line_height: 0.0,
            lines: 3,
        },
        Case {
            text: "contact\nhello@darkbrew.cz",
            size: 16.0,
            line_height: 1.5,
            lines: 2,
        },
        Case {
            text: "first\nsecond\nthird",
            size: 18.0,
            line_height: 1.5,
            lines: 3,
        },
    ];

    let skia = jian_skia::SkiaMeasure::new();
    for case in cases {
        let run = StyledRun {
            text: case.text,
            font_family: Some("system-ui"),
            font_size: case.size,
            font_weight: 400,
            font_style: FontStyleKind::Normal,
            letter_spacing: 0.0,
        };
        let measured = skia.measure(&MeasureRequest {
            runs: &[run],
            line_height: case.line_height,
            max_width: None,
        });
        let native_line_height = case.size
            * if case.line_height > 0.0 {
                case.line_height
            } else {
                1.2
            };
        let native_height = f32::from(case.lines) * native_line_height;
        let rel =
            (measured.height - native_height).abs() / measured.height.max(native_height).max(1.0);
        println!(
            "multiline height parity text={:?} size={} line_height={} skia={:.3} native={:.3} lines={} diff={:.2}%",
            case.text,
            case.size,
            case.line_height,
            measured.height,
            native_height,
            measured.line_count,
            rel * 100.0
        );
        assert_eq!(
            measured.line_count, case.lines,
            "literal newline line count drifted for {:?}",
            case.text
        );
        assert!(
            rel <= 0.02,
            "height drift exceeded 2% for {:?}: skia={:.3} native={:.3} diff={:.2}%",
            case.text,
            measured.height,
            native_height,
            rel * 100.0
        );
    }
}
