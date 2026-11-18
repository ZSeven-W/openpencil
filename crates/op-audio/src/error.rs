#[derive(Debug)]
pub enum AudioError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Base64(base64::DecodeError),
    Track(op_editor_core::audio_tracks::AudioTrackError),
    FileSize { bytes: u64 },
    UnsupportedFormat(String),
    NoAudioStream,
    InvalidDuration,
    InvalidSource,
    InvalidNarration,
    NeuralSpeechUnavailable,
    UnsupportedPlatform,
    ProcessFailed { program: String, detail: String },
    TimedOut(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "audio I/O failed: {e}"),
            Self::Json(e) => write!(f, "audio metadata failed: {e}"),
            Self::Base64(e) => write!(f, "invalid embedded audio: {e}"),
            Self::Track(e) => e.fmt(f),
            Self::FileSize { bytes } => write!(f, "audio file is {bytes} bytes; use a nonempty file up to 16 MiB"),
            Self::UnsupportedFormat(ext) => write!(f, "unsupported audio format: {ext}"),
            Self::NoAudioStream => f.write_str("the file contains no audio stream"),
            Self::InvalidDuration => f.write_str("audio must have a duration between zero and ten minutes"),
            Self::InvalidSource => f.write_str("unsupported embedded audio source"),
            Self::InvalidNarration => f.write_str("use 1–10000 characters, a voice name up to 128 characters, and a rate between 80 and 400"),
            Self::NeuralSpeechUnavailable => f.write_str("Online voiceover failed. Check the connection and install or update edge-tts in the Python environment used by OpenPencil."),
            Self::UnsupportedPlatform => f.write_str("automatic narration is available on macOS, Windows and Linux"),
            Self::ProcessFailed { program, detail } => write!(f, "{program} failed: {detail}"),
            Self::TimedOut(program) => write!(f, "{program} timed out"),
        }
    }
}
impl std::error::Error for AudioError {}
impl From<std::io::Error> for AudioError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for AudioError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<base64::DecodeError> for AudioError {
    fn from(e: base64::DecodeError) -> Self {
        Self::Base64(e)
    }
}
impl From<op_editor_core::audio_tracks::AudioTrackError> for AudioError {
    fn from(e: op_editor_core::audio_tracks::AudioTrackError) -> Self {
        Self::Track(e)
    }
}
