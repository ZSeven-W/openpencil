//! Playback of prepared PCM. Seeking slices the PCM before starting the player.

use crate::{process, AudioError};
use std::io::Write;
use std::process::{Child, Command, Stdio};

pub struct PreviewPlayer {
    child: Child,
    _directory: tempfile::TempDir,
}

impl PreviewPlayer {
    pub fn start(pcm: &[u8], start_ms: u64) -> Result<Option<Self>, AudioError> {
        let offset = (start_ms.saturating_mul(44100) / 1000).saturating_mul(4) as usize;
        if offset >= pcm.len() {
            return Ok(None);
        }
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("playback.wav");
        let mut output = std::fs::File::create(&path)?;
        write_wav(&mut output, &pcm[offset..])?;
        drop(output);
        let mut command;
        if cfg!(target_os = "macos") {
            command = Command::new("/usr/bin/afplay");
            command.arg(&path);
        } else if cfg!(target_os = "windows") {
            command = Command::new("powershell.exe");
            command.args(["-NoProfile","-NonInteractive","-Command","$p=New-Object System.Media.SoundPlayer; $p.SoundLocation=$env:OP_AUDIO_PREVIEW; $p.PlaySync()"])
                .env("OP_AUDIO_PREVIEW",&path);
        } else if cfg!(target_os = "linux") {
            command = Command::new(process::executable("ffplay"));
            command
                .args(["-nodisp", "-autoexit", "-v", "error"])
                .arg(&path);
        } else {
            return Err(AudioError::UnsupportedPlatform);
        }
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Some(Self {
            child,
            _directory: directory,
        }))
    }
}

impl Drop for PreviewPlayer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn write_wav(output: &mut impl Write, pcm: &[u8]) -> std::io::Result<()> {
    let length = u32::try_from(pcm.len()).map_err(std::io::Error::other)?;
    output.write_all(b"RIFF")?;
    output.write_all(&(length + 36).to_le_bytes())?;
    output.write_all(b"WAVEfmt ")?;
    output.write_all(&16u32.to_le_bytes())?;
    output.write_all(&1u16.to_le_bytes())?;
    output.write_all(&2u16.to_le_bytes())?;
    output.write_all(&44100u32.to_le_bytes())?;
    output.write_all(&176400u32.to_le_bytes())?;
    output.write_all(&4u16.to_le_bytes())?;
    output.write_all(&16u16.to_le_bytes())?;
    output.write_all(b"data")?;
    output.write_all(&length.to_le_bytes())?;
    output.write_all(pcm)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pcm_preview_writes_a_portable_stereo_wav() {
        let mut wav = Vec::new();
        write_wav(&mut wav, &[1, 2, 3, 4]).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[44..], &[1, 2, 3, 4]);
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 44100);
    }
}
