use super::*;
use std::collections::BTreeMap;

fn fixture(name: &str, parameters: &[(&str, Vec<f32>)]) -> ShaderFillBody {
    ShaderFillBody {
        preset: Some(name.into()),
        sksl: None,
        uniforms: Some(
            parameters
                .iter()
                .map(|(k, v)| (k.to_string(), ShaderUniformValue::Vec(v.clone())))
                .collect::<BTreeMap<_, _>>(),
        ),
        explain: None,
        opacity: None,
        blend_mode: None,
    }
}

#[test]
fn camera_paths_preserve_runtime_uniforms_and_constant_array_indices() {
    let body = fixture(
        "candlesticks_3d",
        &[
            ("bars", vec![100., 110., 90., 105., 105., 115., 100., 112.]),
            (
                "viewPath",
                vec![0., 0., 12., 1., 500., 900., 2., 1., 9., 2., 450., 800.],
            ),
            (
                "cameraPath",
                vec![0., 0.2, 0.1, 1000., 1., 2., 0.4, 0.3, 1200., 1.5],
            ),
        ],
    );
    let expanded = expand(&body).unwrap();
    assert!(expanded.sksl.contains("return bars[1];"));
    assert!(expanded.sksl.contains("sampleView"));
    assert!(expanded.sksl.contains("sampleCamera"));
    assert!(!expanded.sksl.contains("bars[i]"));
    assert_eq!(motion_clock(&expanded), &[1.0]);
    assert_eq!(
        expanded
            .uniforms
            .iter()
            .find(|u| u.name == "viewPath")
            .unwrap()
            .values
            .len(),
        16
    );
    assert_eq!(
        expanded
            .uniforms
            .iter()
            .find(|u| u.name == "cameraPath")
            .unwrap()
            .values
            .len(),
        8
    );
}

fn motion_clock(preset: &ExpandedShaderPreset) -> &[f32] {
    &preset
        .uniforms
        .iter()
        .find(|u| u.name == "motionClock")
        .unwrap()
        .values
}

#[test]
fn delayed_bursts_do_not_drive_the_editor_clock_unless_requested() {
    let scheduled = [("burst", vec![12., 1., 220., 90.])];
    let preset = expand(&fixture("spark_burst", &scheduled)).unwrap();
    assert_eq!(motion_clock(&preset), &[1.0]);
    let mut ambient = scheduled.to_vec();
    ambient.push(("motionClock", vec![0.]));
    let preset = expand(&fixture("spark_burst", &ambient)).unwrap();
    assert_eq!(motion_clock(&preset), &[0.0]);
    assert_eq!(
        motion_clock(&expand(&fixture("spark_burst", &[])).unwrap()),
        &[0.0]
    );
}

#[test]
fn invalid_or_unbounded_data_is_rejected_before_source_generation() {
    for (name, data) in [
        ("bars", vec![]),
        ("bars", vec![1., 2., 0.]),
        ("bars", vec![1., 0., 0., 2.]),
        ("bars", vec![1.; (MAX_CANDLES + 1) * 4]),
        (
            "viewPath",
            vec![1., 0., 12., 1., 500., 900., 1., 1., 9., 2., 450., 800.],
        ),
        ("cameraPath", vec![0., 0.2, 0.1, 1., 1.]),
    ] {
        assert!(
            expand(&fixture("candlesticks_3d", &[(name, data)])).is_none(),
            "{name}"
        );
    }
}

#[test]
fn one_row_camera_paths_are_valid_static_shots() {
    let expanded = expand(&fixture(
        "candlesticks_3d",
        &[("cameraPath", vec![0., 0.2, 0.1, 1000., 1.])],
    ))
    .unwrap();
    assert!(expanded.sksl.contains("return cameraPath[0];"));
}
