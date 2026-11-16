use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::OnceLock;
use std::thread::JoinHandle;

use super::encoding_error;
use crate::ExportError;

pub(super) fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("ffmpeg")
            .args(["-hide_banner", "-encoders"])
            .stdin(Stdio::null())
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .any(|line| line.split_whitespace().nth(1) == Some("libx264"))
            })
    })
}

pub(super) struct Encoder {
    child: Child,
    input: Option<ChildStdin>,
    errors: Option<JoinHandle<String>>,
}

impl Encoder {
    pub(super) fn open(
        path: &Path,
        (width, height): (i32, i32),
        fps: u32,
    ) -> Result<Self, ExportError> {
        let mut child = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-f",
                "rawvideo",
                "-pixel_format",
                "bgra",
                "-video_size",
                &format!("{width}x{height}"),
                "-framerate",
                &fps.to_string(),
                "-i",
                "pipe:0",
                "-an",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-profile:v",
                "high",
                "-preset",
                "veryfast",
                "-threads",
                "2",
                "-movflags",
                "+faststart",
            ])
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(encoding_error)?;
        let input = child.stdin.take();
        let mut stderr = child.stderr.take().expect("piped encoder diagnostics");
        let errors = std::thread::spawn(move || {
            let mut buffer = [0; 4096];
            let mut bounded = Vec::new();
            while let Ok(count) = stderr.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                bounded.extend_from_slice(&buffer[..count.min(16_384 - bounded.len())]);
            }
            String::from_utf8_lossy(&bounded).trim().to_owned()
        });
        Ok(Self {
            child,
            input,
            errors: Some(errors),
        })
    }

    pub(super) fn push(&mut self, pixels: &[u8], _: u64) -> Result<(), ExportError> {
        self.input
            .as_mut()
            .expect("piped encoder input")
            .write_all(pixels)
            .map_err(encoding_error)
    }

    pub(super) fn finish(mut self, _: u64) -> Result<(), ExportError> {
        self.input.take();
        let status = self.child.wait().map_err(encoding_error)?;
        let errors = self
            .errors
            .take()
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default();
        if status.success() {
            Ok(())
        } else {
            Err(encoding_error(format!("{status}: {errors}")))
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        self.input.take();
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(reader) = self.errors.take() {
            let _ = reader.join();
        }
    }
}
