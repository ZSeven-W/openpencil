use super::*;
use jian_core::motion::ClipTrigger;

fn wav(seconds: f64) -> Vec<u8> {
    let rate = 8000u32;
    let samples = (seconds * f64::from(rate)) as u32;
    let data = samples * 2;
    let mut bytes = Vec::new();
    bytes.extend(b"RIFF");
    bytes.extend((data + 36).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * 2).to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data.to_le_bytes());
    for n in 0..samples {
        bytes.extend(
            (((f64::from(n) * 440.0 * std::f64::consts::TAU / f64::from(rate)).sin() * 8000.0)
                as i16)
                .to_le_bytes(),
        );
    }
    bytes
}

#[test]
fn invalid_narration_is_rejected_before_a_process_runs() {
    assert!(matches!(
        generate_voiceover("", None, 200, "id"),
        Err(AudioError::InvalidNarration)
    ));
    assert!(matches!(
        generate_voiceover("hello", Some("-o /tmp/x"), 200, "id"),
        Err(AudioError::InvalidNarration)
    ));
    assert!(matches!(
        generate_voiceover("hello", None, 0, "id"),
        Err(AudioError::InvalidNarration)
    ));
}

#[test]
fn a_silent_movie_needs_no_media_process() {
    let root = serde_json::from_str(r#"{"type":"frame","id":"root"}"#).unwrap();
    assert_eq!(
        mix_mp4(vec![1, 2, 3], &root, &ClipTrigger::Mount, 1000).unwrap(),
        vec![1, 2, 3]
    );
}

#[test]
#[ignore = "requires ffmpeg and ffprobe; runs in the native audio integration check"]
fn imported_audio_is_embedded_and_muxed_at_the_authored_offset() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("voice.wav");
    std::fs::write(&input, wav(0.4)).unwrap();
    let mut track = import_audio(&input, "voice").unwrap();
    track.start_ms = 300;
    assert!((399..=401).contains(&track.duration_ms));
    assert!(track.source.starts_with("data:audio/wav;base64,"));
    let root = serde_json::from_value(
        serde_json::json!({"type":"frame","id":"root","audioTracks":[track]}),
    )
    .unwrap();
    let video = directory.path().join("video.mp4");
    assert!(Command::new(process::executable("ffmpeg"))
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=white:s=16x16:r=10:d=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p"
        ])
        .arg(&video)
        .status()
        .unwrap()
        .success());
    let output = mix_mp4(
        std::fs::read(video).unwrap(),
        &root,
        &ClipTrigger::Mount,
        1000,
    )
    .unwrap();
    let mixed = directory.path().join("mixed.mp4");
    std::fs::write(&mixed, output).unwrap();
    let decoded = Command::new(process::executable("ffmpeg"))
        .args(["-v", "error", "-i"])
        .arg(mixed)
        .args([
            "-map", "0:a:0", "-f", "s16le", "-ar", "8000", "-ac", "1", "pipe:1",
        ])
        .output()
        .unwrap();
    assert!(decoded.status.success());
    let samples: Vec<i16> = decoded
        .stdout
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    let power = |a: usize, b: usize| {
        samples[a..b]
            .iter()
            .map(|s| f64::from(*s).powi(2))
            .sum::<f64>()
            / (b - a) as f64
    };
    assert!(
        power(0, 1600) < 100.0,
        "the authored 300 ms delay must remain silent"
    );
    assert!(
        power(3200, 4400) > 10000.0,
        "the imported voice must be audible after its offset"
    );
}
