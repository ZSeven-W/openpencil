use crate::AudioError;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn executable(name: &str) -> PathBuf {
    // Finder launches do not inherit a shell's Homebrew PATH.
    if cfg!(target_os = "macos") {
        for directory in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
            let path = Path::new(directory).join(name);
            if path.is_file() {
                return path;
            }
        }
    }
    PathBuf::from(name)
}

pub fn run(
    mut command: Command,
    stdout: &Path,
    directory: &Path,
    timeout: u64,
) -> Result<(), AudioError> {
    let program = command.get_program().to_string_lossy().into_owned();
    let error_path = directory.join("process-error.txt");
    command
        .stdin(Stdio::null())
        .stdout(std::fs::File::create(stdout)?)
        .stderr(std::fs::File::create(&error_path)?);
    let mut child = command.spawn().map_err(|error| AudioError::ProcessFailed {
        program: program.clone(),
        detail: if program.ends_with("ffmpeg") || program.ends_with("ffprobe") {
            format!("{error}. Install FFmpeg (ffmpeg and ffprobe) for audio import and MP4 mixing.")
        } else {
            error.to_string()
        },
    })?;
    let start = Instant::now();
    loop {
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        };
        if let Some(status) = status {
            if status.success() {
                return Ok(());
            }
            let mut detail = String::new();
            std::fs::File::open(&error_path)?
                .take(4096)
                .read_to_string(&mut detail)?;
            return Err(AudioError::ProcessFailed { program, detail });
        }
        if start.elapsed() >= Duration::from_secs(timeout) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AudioError::TimedOut(program));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
