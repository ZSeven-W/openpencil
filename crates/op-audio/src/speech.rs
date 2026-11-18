use crate::{import_audio, process, AudioError};
use op_editor_core::audio_request::VoiceoverProvider;
use op_editor_core::audio_tracks::{AudioTrack, Narration};
use std::process::Command;

pub fn generate_voiceover(
    text: &str,
    voice: Option<&str>,
    rate: u32,
    id: &str,
) -> Result<AudioTrack, AudioError> {
    generate_voiceover_with_provider(text, voice, rate, VoiceoverProvider::System, id)
}

pub fn generate_voiceover_with_provider(
    text: &str,
    voice: Option<&str>,
    rate: u32,
    provider: VoiceoverProvider,
    id: &str,
) -> Result<AudioTrack, AudioError> {
    if text.trim().is_empty()
        || text.chars().count() > 10_000
        || !(80..=400).contains(&rate)
        || voice
            .is_some_and(|v| v.len() > 128 || v.starts_with('-') || v.contains(['\0', '\n', '\r']))
    {
        return Err(AudioError::InvalidNarration);
    }
    if provider == VoiceoverProvider::Neural {
        return crate::speech_neural::generate(text, voice, rate, id);
    }
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("narration.txt");
    std::fs::write(&input, text)?;
    let audio = directory.path().join(if cfg!(target_os = "macos") {
        "speech.aiff"
    } else {
        "speech.wav"
    });
    let mut command;
    if cfg!(target_os = "macos") {
        command = Command::new("/usr/bin/say");
        command
            .arg("-r")
            .arg(rate.to_string())
            .arg("-f")
            .arg(&input)
            .arg("-o")
            .arg(&audio);
        let selected = voice.or_else(|| {
            text.chars()
                .any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
                .then_some("Tingting")
        });
        if let Some(voice) = selected {
            command.arg("-v").arg(voice);
        }
    } else if cfg!(target_os = "linux") {
        command = Command::new(process::executable("espeak-ng"));
        command
            .arg("-s")
            .arg(rate.to_string())
            .arg("-f")
            .arg(&input)
            .arg("-w")
            .arg(&audio);
        if let Some(voice) = voice {
            command.arg("-v").arg(voice);
        }
    } else if cfg!(target_os = "windows") {
        command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command",
            "Add-Type -AssemblyName System.Speech; $s=New-Object System.Speech.Synthesis.SpeechSynthesizer; if($env:OP_SPEECH_VOICE){$s.SelectVoice($env:OP_SPEECH_VOICE)}; $s.Rate=[int]$env:OP_SPEECH_RATE; $s.SetOutputToWaveFile($env:OP_SPEECH_OUTPUT); $s.Speak([IO.File]::ReadAllText($env:OP_SPEECH_INPUT)); $s.Dispose()"])
            .env("OP_SPEECH_INPUT", &input).env("OP_SPEECH_OUTPUT", &audio)
            .env("OP_SPEECH_VOICE", voice.unwrap_or(""))
            .env("OP_SPEECH_RATE", ((rate as i32-180)/20).clamp(-10,10).to_string());
    } else {
        return Err(AudioError::UnsupportedPlatform);
    }
    process::run(
        command,
        &directory.path().join("speech.log"),
        directory.path(),
        120,
    )?;
    let mut track = import_audio(&audio, id)?;
    track.name = "Voiceover".into();
    track.narration = Some(Narration {
        provider: Some("system".into()),
        text: text.into(),
        voice: voice.map(str::to_owned),
        rate,
    });
    Ok(track)
}
