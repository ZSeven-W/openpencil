use std::{
    env,
    fs::{self, File},
    io::{self, Error, ErrorKind, Read},
    path::{Path, PathBuf},
    str,
};

/// Downloads a file from a URL,
///
/// optionally resuming partial downloads and caching the result in `OUT_DIR/.cache/skia-bindings`.
///
/// The url's filename will be used as a the key for caching and resuming downloads.
pub fn download(url: impl AsRef<str>, resume_and_cache: bool) -> io::Result<Vec<u8>> {
    let url = url.as_ref();

    // `file` URL, empty hostname, absolute path
    if let Some(file_path) = url.strip_prefix("file://") {
        return fs::read(Path::new(file_path));
    }

    // `file` URLs with non-empty hostname or relative paths are unsupported.
    if url.starts_with("file:") {
        eprintln!("Unsupported file: URL {url}");
        return Err(ErrorKind::Unsupported.into());
    }

    // Specify the directory where the downloaded files are stored.
    let Ok(out_dir) = env::var("OUT_DIR") else {
        eprintln!("OUT_DIR not available");
        return Err(ErrorKind::Unsupported.into());
    };

    let mut out_dir = PathBuf::from(&out_dir);

    let Some(file_name) = url.split('/').next_back() else {
        eprintln!("Failed to extract filename from `{url}`");
        return Err(ErrorKind::InvalidInput.into());
    };

    let mut command = std::process::Command::new("curl");
    command
        // follow redirects
        .arg("-L")
        // fail fast with no "error pages" output. more of a hint though, so we might still get error on stdout.
        // so make sure to check the actual status returned.
        .arg("-f")
        // no progress meter but keep error messages.
        .arg("-sS");

    let cache_file_path = out_dir.join(".cache").join(file_name);
    if resume_and_cache {
        // resumed transfer offset
        command
            .arg("-C")
            .arg("-")
            .arg("--create-dirs")
            // directory + filename to save files in
            .arg("--output")
            .arg(cache_file_path.to_str().unwrap());
    }

    // file url
    command.arg(url);

    match command.output() {
        Ok(out) => {
            // read bytes from the file
            if out.status.success() {
                if resume_and_cache {
                    let mut file = File::open(cache_file_path)?;
                    let mut result = Vec::with_capacity(file.metadata()?.len() as usize);
                    file.read_to_end(&mut result)?;
                    Ok(result)
                } else {
                    Ok(out.stdout)
                }
            } else {
                Err(io::Error::other(format!(
                    "curl error code: {:?}\ncurl stderr: {:?}",
                    out.status
                        .code()
                        .map(|i| i.to_string())
                        .unwrap_or(String::from("no status code")),
                    str::from_utf8(&out.stderr).unwrap_or("no stderr")
                )))
            }
        }
        Err(e) => Err(Error::other(format!("curl command error : {e:#?}"))),
    }
}
