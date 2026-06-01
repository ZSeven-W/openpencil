use std::path::PathBuf;

pub(crate) fn resolve_file_path_arg(value: &str) -> String {
    if value.starts_with("live://") {
        return value.to_string();
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        return path.display().to_string();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path).display().to_string())
        .unwrap_or_else(|_| value.to_string())
}

pub(crate) fn resolve_file_path_value(value: &str) -> serde_json::Value {
    serde_json::Value::String(resolve_file_path_arg(value))
}
