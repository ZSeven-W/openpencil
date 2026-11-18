use crate::{decoded_source, process, AudioError};
use jian_core::motion::ClipTrigger;
use jian_ops_schema::node::PenNode;
use op_editor_core::audio_tracks::{collect_audio_tracks, validate_audio_tracks, AudioTrack};
use std::process::Command;

pub fn matching_tracks<'a>(root: &'a PenNode, trigger: &ClipTrigger) -> Vec<&'a AudioTrack> {
    collect_audio_tracks(root, trigger)
        .into_iter()
        .map(|(_, track)| track)
        .filter(|t| !t.muted)
        .collect()
}

/// Mux soundtracks after native frame encoding. Never silently emit a silent file.
pub fn mix_mp4(
    video: Vec<u8>,
    root: &PenNode,
    trigger: &ClipTrigger,
    duration_ms: u64,
) -> Result<Vec<u8>, AudioError> {
    let tracks: Vec<_> = matching_tracks(root, trigger)
        .into_iter()
        .filter(|track| track.start_ms < duration_ms)
        .collect();
    if tracks.is_empty() {
        return Ok(video);
    }
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("picture.mp4");
    let output = directory.path().join("soundtrack.mp4");
    std::fs::write(&input, video)?;
    let mut command = Command::new(process::executable("ffmpeg"));
    command
        .args([
            "-v",
            "error",
            "-nostdin",
            "-y",
            "-protocol_whitelist",
            "file,pipe",
            "-i",
        ])
        .arg(input);
    let filter = add_audio_inputs(&mut command, directory.path(), &tracks, 1)?;
    command
        .arg("-filter_complex")
        .arg(filter)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "[audio]",
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-shortest",
        ])
        .args(["-movflags", "+faststart"])
        .arg(&output);
    process::run(
        command,
        &directory.path().join("mix.log"),
        directory.path(),
        120,
    )?;
    Ok(std::fs::read(output)?)
}

pub fn render_pcm(
    root: &PenNode,
    trigger: &ClipTrigger,
    duration_ms: u64,
) -> Result<Vec<u8>, AudioError> {
    let duration_ms = duration_ms.min(op_editor_core::audio_tracks::audio_duration_ms(
        root, trigger,
    ));
    let tracks = matching_tracks(root, trigger);
    if tracks.is_empty() {
        return Ok(Vec::new());
    }
    let directory = tempfile::tempdir()?;
    let output = directory.path().join("preview.pcm");
    let mut command = Command::new(process::executable("ffmpeg"));
    command.args(["-v", "error", "-nostdin", "-y"]);
    let filter = add_audio_inputs(&mut command, directory.path(), &tracks, 0)?;
    command
        .arg("-filter_complex")
        .arg(filter)
        .args(["-map", "[audio]", "-t"])
        .arg(format!("{:.3}", duration_ms as f64 / 1000.0))
        .args(["-ar", "44100", "-ac", "2", "-f", "s16le"])
        .arg(&output);
    process::run(
        command,
        &directory.path().join("preview.log"),
        directory.path(),
        120,
    )?;
    Ok(std::fs::read(output)?)
}

fn add_audio_inputs(
    command: &mut Command,
    directory: &std::path::Path,
    tracks: &[&AudioTrack],
    first: usize,
) -> Result<String, AudioError> {
    let owned: Vec<_> = tracks.iter().map(|t| (*t).clone()).collect();
    validate_audio_tracks(&owned)?;
    let mut filter = Vec::new();
    let mut labels = String::new();
    for (index, track) in tracks.iter().enumerate() {
        let (bytes, format) = decoded_source(track)?;
        let path = directory.join(format!("audio-{index}.bin"));
        std::fs::write(&path, bytes)?;
        command
            .args(["-protocol_whitelist", "file,pipe", "-f", format, "-i"])
            .arg(path);
        let n = index + first;
        filter.push(format!(
            "[{n}:a:0]atrim=start={:.3},asetpts=PTS-STARTPTS,volume={},adelay={}:all=1[a{n}]",
            track.trim_start_ms as f64 / 1000.0,
            track.volume,
            track.start_ms
        ));
        labels.push_str(&format!("[a{n}]"));
    }
    filter.push(format!("{labels}amix=inputs={}:duration=longest:normalize=0,alimiter=limit=0.95:level=false,apad[audio]",tracks.len()));
    Ok(filter.join(";"))
}
