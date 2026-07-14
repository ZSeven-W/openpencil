//! Windows fallback locations used when a GUI launch inherits a minimal PATH.

use std::path::PathBuf;

pub(crate) fn user_bin_dirs() -> Vec<PathBuf> {
    user_bin_dirs_from(
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(dirs::home_dir),
        std::env::var_os("APPDATA").map(PathBuf::from),
        std::env::var_os("GROK_BIN_DIR").map(PathBuf::from),
    )
}

fn user_bin_dirs_from(
    local_appdata: Option<PathBuf>,
    home: Option<PathBuf>,
    appdata: Option<PathBuf>,
    grok_bin_dir: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = local_appdata {
        dirs.push(dir.join("agy").join("bin"));
    } else if let Some(dir) = home.as_ref() {
        dirs.push(dir.join("AppData").join("Local").join("agy").join("bin"));
    }
    if let Some(dir) = grok_bin_dir {
        dirs.push(dir);
    }
    if let Some(dir) = home {
        dirs.push(dir.join(".grok").join("bin"));
    }
    if let Some(dir) = appdata {
        dirs.push(dir.join("npm"));
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_official_antigravity_and_grok_locations() {
        let dirs = user_bin_dirs_from(
            Some(PathBuf::from(r"C:\Users\Ada\AppData\Local")),
            Some(PathBuf::from(r"C:\Users\Ada")),
            Some(PathBuf::from(r"C:\Users\Ada\AppData\Roaming")),
            None,
        );
        assert!(dirs.contains(
            &PathBuf::from(r"C:\Users\Ada\AppData\Local")
                .join("agy")
                .join("bin")
        ));
        assert!(dirs.contains(&PathBuf::from(r"C:\Users\Ada").join(".grok").join("bin")));
    }

    #[test]
    fn respects_custom_grok_bin_dir() {
        let custom = PathBuf::from(r"D:\Tools\grok");
        assert_eq!(
            user_bin_dirs_from(None, None, None, Some(custom.clone())),
            [custom]
        );
    }
}
