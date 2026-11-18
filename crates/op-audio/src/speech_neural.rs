//! Natural online voices through an installed edge-tts client.

use crate::{import_audio, process, AudioError};
use op_editor_core::audio_tracks::{AudioTrack, Narration};
use std::path::PathBuf;
use std::process::Command;

pub(crate) fn generate(
    text: &str,
    voice: Option<&str>,
    rate: u32,
    id: &str,
) -> Result<AudioTrack, AudioError> {
    let chinese = text.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c));
    let voice = voice.unwrap_or(if chinese {
        "zh-CN-YunxiNeural"
    } else {
        "en-US-GuyNeural"
    });
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("narration.txt");
    let output = directory.path().join("narration.mp3");
    std::fs::write(&input, text)?;
    let python = std::env::var_os("OPENPENCIL_TTS_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| process::executable(if cfg!(windows) { "python" } else { "python3" }));
    let mut command = Command::new(python);
    let percent = (rate as i32 - 200) / 2;
    command
        .args(["-m", "edge_tts", "--file"])
        .arg(&input)
        .arg("--voice")
        .arg(voice)
        .arg(format!("--rate={percent:+}%"))
        .arg("--write-media")
        .arg(&output);
    process::run(
        command,
        &directory.path().join("neural.log"),
        directory.path(),
        120,
    )
    .map_err(|error| match error {
        AudioError::ProcessFailed { .. } => AudioError::NeuralSpeechUnavailable,
        other => other,
    })?;
    let mut track = import_audio(&output, id)?;
    track.name = "Voiceover".into();
    track.narration = Some(Narration {
        text: text.into(),
        provider: Some("neural".into()),
        voice: Some(voice.into()),
        rate,
    });
    Ok(track)
}
