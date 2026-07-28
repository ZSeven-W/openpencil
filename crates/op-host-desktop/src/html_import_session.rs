//! Background HTML / ZIP project import session — mirrors
//! `figma_import_session`: moves the op-html parse (CSS cascade +
//! node mapping + local resource embedding) off the main thread so
//! the editor UI keeps repainting while a page converts.
//!
//! Reuses `figma_import_session::{PreparedImport, PumpOutcome}` and
//! the same `figma_import_in_progress` overlay flag so the paint
//! side needs no new UI state.

use op_editor_core::EditorState;
use op_host_native::WidgetHostNative;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::figma_import_session::{PreparedImport, PumpOutcome};
use crate::html_import_error::HtmlImportError;
use crate::persistence::show_error_dialog_public;
use op_host_services::doc_io::ErrorKind;

/// Synthetic origin for resolving a file-import's relative resource
/// references — same convention as `op-cli`'s `html_cli.rs`.
const LOCAL_RESOURCE_ORIGIN: &str = "https://openpencil.local/";

/// One in-flight HTML page or ZIP project parse — the source path
/// (for the error dialog) plus the worker-thread receiver.
pub struct HtmlImportSession {
    path: PathBuf,
    rx: Receiver<Result<PreparedImport, HtmlImportError>>,
}

/// Spawn a worker thread that reads `path`, converts a single page or
/// packaged project through op-html (including relative resources),
/// and posts the result back through a channel. Returns the session
/// handle.
pub fn spawn(host: &mut WidgetHostNative, path: PathBuf) -> HtmlImportSession {
    let (tx, rx) = mpsc::channel();
    // Same overlay flag + same "no dirty-mark" rationale as the
    // Figma session: the import replaces `editor_state` whole-cloth,
    // so rebuilding the old layout would be wasted work.
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.import_source = op_editor_core::figma_import_state::ImportSource::Html;
    ui.figma_import_in_progress = true;

    let path_for_thread = path.clone();
    let worker_tx = tx.clone();
    let spawned = thread::Builder::new()
        .name("op-html-import".into())
        .spawn(move || {
            let result = parse_path(&path_for_thread);
            let _ = worker_tx.send(result);
        });
    if let Err(err) = spawned {
        // Deliver the failure through the normal pump path (error
        // dialog + overlay-flag clear) instead of crashing the app.
        eprintln!("[import-html] failed to spawn worker: {err}");
        let _ = tx.send(Err(HtmlImportError::WorkerSpawn(err.to_string())));
    }

    HtmlImportSession { path, rx }
}

fn parse_path(path: &Path) -> Result<PreparedImport, HtmlImportError> {
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("HTML Import");
    // Down-scale embedded bitmaps exactly like the Figma import path.
    let transform =
        |bytes: &[u8]| crate::image_downscale::maybe_downscale(bytes).map(|(_mime, out)| out);
    let result = if is_zip_path(path) {
        // `std::io::Error` and `op_html`'s error belong to code this pass
        // does not own, so their messages ride along as text.
        let archive = std::fs::File::open(path)
            .map_err(|error| HtmlImportError::ReadSource(error.to_string()))?;
        let opts = op_html::HtmlImportOptions {
            document_name: Some(file_name.to_string()),
            ..Default::default()
        };
        op_html::import_html_zip_reader_document_with_transform(archive, &opts, Some(&transform))
            .map_err(|error| HtmlImportError::Import(error.to_string()))?
    } else {
        let source_bytes =
            std::fs::read(path).map_err(|error| HtmlImportError::ReadSource(error.to_string()))?;
        let source = op_html::html_encoding::decode_html_bytes(&source_bytes);
        let base_dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        // Relative references only resolve when the importer has a base
        // URL, so file import uses the same synthetic local origin as the
        // CLI (`html_cli.rs`): resolved URLs come back prefixed with the
        // origin and are stripped to same-directory disk lookups. Real
        // remote URLs keep their own scheme, fail the strip, and are
        // rejected by `local_resource_fetch` — file import never touches
        // the network (the importer records a warning instead).
        let fetcher = move |url: &str| {
            let href = url.strip_prefix(LOCAL_RESOURCE_ORIGIN).unwrap_or(url);
            local_resource_fetch(&base_dir, href)
        };
        let opts = op_html::HtmlImportOptions {
            document_name: Some(file_name.to_string()),
            base_url: Some(format!("{LOCAL_RESOURCE_ORIGIN}document.html")),
            ..Default::default()
        };
        op_html::import_html_document(source.as_ref(), &opts, Some(&fetcher), Some(&transform))
    };
    if result.document.children.is_empty() {
        return Err(HtmlImportError::NoImportableContent(
            result.warnings.first().cloned(),
        ));
    }
    let state = EditorState::from_document(result.document);
    Ok(PreparedImport {
        state,
        warnings: result.warnings,
    })
}

fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

/// Resolve a relative resource reference against the HTML file's
/// directory. Refuses anything that is not a plain same-tree
/// relative path: remote URLs, `data:`, absolute paths, and `..`
/// escapes all return `None` (the importer degrades with a warning).
pub(crate) fn local_resource_fetch(dir: &Path, href: &str) -> Option<Vec<u8>> {
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("//")
        || href.starts_with("data:")
        || Path::new(href).is_absolute()
    {
        return None;
    }
    // Strip a `?query` / `#fragment` — cache-busting suffixes like
    // `style.css?v=1` name a real file on disk once the suffix is
    // dropped (mirrors `html_cli.rs`'s local fetch).
    let encoded = href.split_once(['?', '#']).map_or(href, |(path, _)| path);
    if encoded.is_empty() {
        return None;
    }
    // `Url::join` percent-encodes spaces and non-ASCII path bytes before the
    // importer calls this fetcher. Decode them back to a filesystem path, then
    // keep the canonical containment check below as the security boundary —
    // encoded separators or `..` components still cannot escape `dir`.
    let rel = percent_decode_path(encoded)?;
    if rel.contains('\0') || Path::new(&rel).is_absolute() {
        return None;
    }
    let candidate = dir.join(rel);
    let resolved = candidate.canonicalize().ok()?;
    let dir_resolved = dir.canonicalize().ok()?;
    if !resolved.starts_with(&dir_resolved) {
        return None;
    }
    std::fs::read(resolved).ok()
}

fn percent_decode_path(encoded: &str) -> Option<String> {
    let source = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(source.len());
    let mut index = 0;
    while index < source.len() {
        if source[index] != b'%' {
            decoded.push(source[index]);
            index += 1;
            continue;
        }
        let high = hex_value(*source.get(index + 1)?)?;
        let low = hex_value(*source.get(index + 2)?)?;
        decoded.push((high << 4) | low);
        index += 3;
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Non-blocking drain — same contract as
/// `figma_import_session::pump`.
pub fn pump(
    host: &mut WidgetHostNative,
    session: &mut Option<HtmlImportSession>,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> PumpOutcome {
    let Some(sess) = session.as_mut() else {
        return PumpOutcome::Idle;
    };
    match sess.rx.try_recv() {
        Ok(Ok(mut prepared)) => {
            for warning in &prepared.warnings {
                eprintln!("[import-html] warning: {warning}");
            }
            // A freshly constructed EditorState starts at revision 0 with a
            // saved baseline of 0. HTML imports have no adjacent `.op` yet,
            // so explicitly mint an unsaved revision before installation.
            prepared.state.mark_document_changed();
            if !host.install_imported_state_with_drop_hook(prepared.state, || {
                crate::heap_pressure::schedule_relief("HTML replaced-document drop");
            }) {
                host.editor_state_mut().editor_ui.figma_import_in_progress = false;
                host.mark_editor_state_dirty();
                *session = None;
                return PumpOutcome::Cancelled;
            }
            // HTML imports do not auto-create an adjacent `.op`; their next
            // Save intentionally routes through Save As.
            *current_path = None;
            refresh_title(window);
            *session = None;
            PumpOutcome::CompletedOk
        }
        Ok(Err(e)) => {
            eprintln!("[import-html] {e}");
            show_error_dialog_public(host, ErrorKind::Open, Some(&sess.path), &e.to_string());
            host.editor_state_mut().editor_ui.figma_import_in_progress = false;
            host.mark_editor_state_dirty();
            *session = None;
            PumpOutcome::CompletedErr
        }
        Err(TryRecvError::Empty) => PumpOutcome::StillPending,
        Err(TryRecvError::Disconnected) => {
            eprintln!("[import-html] worker thread terminated without sending a result");
            let detail = "HTML import worker exited unexpectedly";
            show_error_dialog_public(host, ErrorKind::Open, Some(&sess.path), detail);
            host.editor_state_mut().editor_ui.figma_import_in_progress = false;
            host.mark_editor_state_dirty();
            *session = None;
            PumpOutcome::CompletedErr
        }
    }
}

/// Drop the active session (if any) and clear the in-progress flag —
/// called when another document-replacing action starts while an
/// HTML import is still parsing. Mirrors
/// `figma_import_session::cancel`.
pub fn cancel(host: &mut WidgetHostNative, session: &mut Option<HtmlImportSession>) {
    if session.is_some() {
        eprintln!("[import-html] cancelling in-flight session — superseded");
        *session = None;
        if host.editor_state().editor_ui.figma_import_in_progress {
            host.editor_state_mut().editor_ui.figma_import_in_progress = false;
            host.mark_editor_state_dirty();
        }
    }
}

fn refresh_title(window: Option<&winit::window::Window>) {
    let Some(window) = window else { return };
    window.set_title("OpenPencil");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    fn temp_path(tag: &str, extension: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "op-html-session-{tag}-{}-{nanos}.{extension}",
            std::process::id()
        ))
    }

    fn project_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (path, bytes) in entries {
            writer.start_file(*path, options).expect("start zip entry");
            writer.write_all(bytes).expect("write zip entry");
        }
        writer.finish().expect("finish zip").into_inner()
    }

    #[test]
    fn installed_html_import_is_dirty_until_saved() {
        let (tx, rx) = mpsc::channel();
        tx.send(Ok(PreparedImport {
            state: EditorState::new(),
            warnings: Vec::new(),
        }))
        .expect("queue prepared import");
        let mut session = Some(HtmlImportSession {
            path: PathBuf::from("fixture.html"),
            rx,
        });
        let mut host = WidgetHostNative::new();
        let mut current_path = Some(PathBuf::from("old.op"));

        assert_eq!(
            pump(&mut host, &mut session, &mut current_path, None),
            PumpOutcome::CompletedOk
        );
        assert!(host.editor_state().is_dirty());
        assert!(host.editor_state().editor_ui.document_dirty);
        assert!(current_path.is_none());
    }

    #[test]
    fn parse_path_fetches_relative_stylesheet() {
        let dir = temp_path("resources", "dir");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("site styles.css"), b".x { color: #ff0000 }").unwrap();
        std::fs::write(dir.join("hero icon.png"), b"\x89PNG\r\n\x1a\nfixture").unwrap();
        std::fs::write(
            dir.join("page.html"),
            b"<html><head><link rel=\"stylesheet\" href=\"site styles.css\"></head>\
              <body><img src=\"hero icon.png\"><p class=\"x\">t</p></body></html>",
        )
        .unwrap();
        let prepared = parse_path(&dir.join("page.html")).expect("parse");
        assert!(
            prepared
                .warnings
                .iter()
                .all(|w| !w.contains("external stylesheet skipped")
                    && !w.contains("image resource unavailable")),
            "percent-encoded relative resources must be fetched: {:?}",
            prepared.warnings
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_path_decodes_gbk_html_from_meta_charset() {
        let path = temp_path("gbk", "html");
        let mut source = b"<html><head><meta charset=\"gbk\"></head><body><p>".to_vec();
        source.extend_from_slice(b"\xC4\xE3\xBA\xC3");
        source.extend_from_slice(b"</p></body></html>");
        std::fs::write(&path, source).expect("write GBK fixture");

        let prepared = parse_path(&path).expect("parse GBK HTML");
        let document = serde_json::to_string(&prepared.state.doc).expect("serialize document");

        let _ = std::fs::remove_file(&path);
        assert!(
            document.contains("你好"),
            "meta charset must be applied before parsing: {document}"
        );
    }

    #[test]
    fn local_fetch_confines_to_directory() {
        let dir = temp_path("fetch", "dir");
        let _ = std::fs::create_dir_all(dir.join("sub"));
        std::fs::write(dir.join("a.css"), b"x").unwrap();
        std::fs::write(dir.join("hero icon.png"), b"z").unwrap();
        std::fs::write(dir.join("图 标.png"), b"u").unwrap();
        std::fs::write(dir.join("sub").join("b.css"), b"y").unwrap();
        assert_eq!(
            local_resource_fetch(&dir, "a.css").as_deref(),
            Some(b"x".as_ref())
        );
        assert_eq!(
            local_resource_fetch(&dir, "sub/b.css").as_deref(),
            Some(b"y".as_ref())
        );
        // Cache-busting query / fragment suffixes still resolve to the file.
        assert_eq!(
            local_resource_fetch(&dir, "a.css?v=2").as_deref(),
            Some(b"x".as_ref())
        );
        assert_eq!(
            local_resource_fetch(&dir, "a.css#top").as_deref(),
            Some(b"x".as_ref())
        );
        assert_eq!(
            local_resource_fetch(&dir, "hero%20icon.png").as_deref(),
            Some(b"z".as_ref())
        );
        assert_eq!(
            local_resource_fetch(&dir, "%E5%9B%BE%20%E6%A0%87.png").as_deref(),
            Some(b"u".as_ref())
        );
        assert!(local_resource_fetch(&dir, "../outside.css").is_none());
        assert!(local_resource_fetch(&dir, "%2e%2e/outside.css").is_none());
        assert!(local_resource_fetch(&dir, "%2Fetc/hosts").is_none());
        assert!(local_resource_fetch(&dir, "/etc/hosts").is_none());
        assert!(local_resource_fetch(&dir, "https://a.dev/x.css").is_none());
        assert!(local_resource_fetch(&dir, "data:text/css,x").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_path_imports_uppercase_zip_project_with_resources() {
        let path = temp_path("project", "ZIP");
        let bytes = project_zip(&[
            (
                "saved-page/index.html",
                br#"<html><head><link rel="stylesheet" href="css/site.css"></head>
                    <body><img src="assets/hero.png"><p class="hot">ZIP</p></body></html>"#,
            ),
            ("saved-page/css/site.css", b".hot { color: #ff0000 }"),
            ("saved-page/assets/hero.png", b"\x89PNG\r\n\x1a\nfixture"),
        ]);
        std::fs::write(&path, bytes).expect("write zip fixture");

        let prepared = parse_path(&path).expect("parse zip project");

        let _ = std::fs::remove_file(&path);
        assert!(!prepared.state.doc.children.is_empty());
        assert!(
            prepared.warnings.iter().all(|warning| {
                !warning.contains("external stylesheet skipped")
                    && !warning.contains("image resource unavailable")
            }),
            "project resources must resolve from the zip: {:?}",
            prepared.warnings
        );
    }

    #[test]
    fn parse_path_reports_invalid_zip() {
        let path = temp_path("invalid", "zip");
        std::fs::write(&path, b"not a zip archive").expect("write invalid zip fixture");

        let error = match parse_path(&path) {
            Ok(_) => panic!("invalid zip must fail"),
            Err(error) => error.to_string(),
        };

        let _ = std::fs::remove_file(&path);
        assert!(!error.is_empty());
    }
}
