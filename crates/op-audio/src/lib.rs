//! Native media operations. Documents carry embedded audio, never local paths.

use base64::Engine as _;
use op_editor_core::audio_tracks::AudioTrack;
use std::path::Path;
use std::process::Command;

mod error;
mod mix;
mod preview;
mod process;
mod speech;
mod speech_neural;
pub use error::AudioError;
pub use mix::{matching_tracks, mix_mp4, render_pcm};
pub use preview::PreviewPlayer;
pub use speech::{generate_voiceover, generate_voiceover_with_provider};

pub const MAX_AUDIO_BYTES: u64 = 16 * 1024 * 1024;

/// Validate the media stream before embedding its original bytes.
pub fn import_audio(path: &Path, id: &str) -> Result<AudioTrack, AudioError> {
    let size = std::fs::metadata(path)?.len();
    if size == 0 || size > MAX_AUDIO_BYTES {
        return Err(AudioError::FileSize { bytes: size });
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (mime, format) = match extension.as_str() {
        "wav" => ("audio/wav", "wav"),
        "mp3" => ("audio/mpeg", "mp3"),
        "m4a" | "mp4" => ("audio/mp4", "mov"),
        "aac" => ("audio/aac", "aac"),
        "aif" | "aiff" => ("audio/aiff", "aiff"),
        "ogg" => ("audio/ogg", "ogg"),
        "flac" => ("audio/flac", "flac"),
        _ => return Err(AudioError::UnsupportedFormat(extension)),
    };
    let temp = tempfile::tempdir()?;
    let output = temp.path().join("probe.json");
    let mut command = Command::new(process::executable("ffprobe"));
    command
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe",
            "-f",
            format,
            "-select_streams",
            "a:0",
            "-show_entries",
            "format=duration:stream=codec_type",
            "-of",
            "json",
        ])
        .arg(path);
    process::run(command, &output, temp.path(), 30)?;
    let probe: serde_json::Value = serde_json::from_slice(&std::fs::read(output)?)?;
    if probe["streams"].as_array().is_none_or(Vec::is_empty) {
        return Err(AudioError::NoAudioStream);
    }
    let seconds = probe["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s > 0.0 && *s <= 600.0)
        .ok_or(AudioError::InvalidDuration)?;
    let bytes = std::fs::read(path)?;
    if bytes.len() as u64 > MAX_AUDIO_BYTES {
        return Err(AudioError::FileSize {
            bytes: bytes.len() as u64,
        });
    }
    let track = AudioTrack {
        id: id.to_owned(),
        name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Audio")
            .to_owned(),
        source: format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ),
        duration_ms: (seconds * 1000.0).ceil() as u64,
        start_ms: 0,
        trim_start_ms: 0,
        volume: 1.0,
        muted: false,
        trigger: jian_ops_schema::motion::MotionTrigger::Mount,
        clip_id: None,
        narration: None,
    };
    op_editor_core::audio_tracks::validate_audio_tracks(std::slice::from_ref(&track))?;
    Ok(track)
}

pub(crate) fn decoded_source(track: &AudioTrack) -> Result<(Vec<u8>, &'static str), AudioError> {
    let (header, encoded) = track
        .source
        .split_once(',')
        .ok_or(AudioError::InvalidSource)?;
    let format = match header {
        "data:audio/wav;base64" => "wav",
        "data:audio/mpeg;base64" => "mp3",
        "data:audio/mp4;base64" => "mov",
        "data:audio/aac;base64" => "aac",
        "data:audio/aiff;base64" => "aiff",
        "data:audio/ogg;base64" => "ogg",
        "data:audio/flac;base64" => "flac",
        _ => return Err(AudioError::InvalidSource),
    };
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    if bytes.len() as u64 > MAX_AUDIO_BYTES {
        return Err(AudioError::FileSize {
            bytes: bytes.len() as u64,
        });
    }
    Ok((bytes, format))
}

pub fn new_track_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("audio_{now:x}_{:x}", NEXT.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests;
