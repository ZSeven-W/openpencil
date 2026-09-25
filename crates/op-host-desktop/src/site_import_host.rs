//! Desktop arm of Studio Home's one-click website import.
//!
//! Home records the request on the shared state; this drains it onto a
//! worker thread (fetch through the SSRF-screened importer fetcher, then the
//! whole post-import pipeline — finalize, brand kit + binding, component
//! recognition, audit) and hands the finished document back. The UI thread
//! never blocks on the network or the pipeline.

use std::sync::mpsc::{self, Receiver, Sender};

use op_editor_core::{Locale, SiteImportResult};
use op_host_native::widget_host::WidgetHostNative;

type ImportOutcome = (u64, Result<SiteImportResult, String>);

/// In-flight imports. A result for a request the user already replaced is
/// dropped by the Home state itself.
pub(crate) struct SiteImportJobs {
    tx: Sender<ImportOutcome>,
    rx: Receiver<ImportOutcome>,
    in_flight: usize,
}

impl SiteImportJobs {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            in_flight: 0,
        }
    }

    pub(crate) fn is_pending(&self) -> bool {
        self.in_flight > 0
    }

    /// Start a queued import and land finished ones. Returns whether the
    /// host changed.
    pub(crate) fn drain(&mut self, host: &mut WidgetHostNative) -> bool {
        if let Some((generation, url)) = host.take_home_site_import_request() {
            let locale = host.editor_state().editor_ui.locale;
            self.spawn(generation, url, locale);
        }
        let mut changed = false;
        while let Ok((generation, result)) = self.rx.try_recv() {
            self.in_flight = self.in_flight.saturating_sub(1);
            if let Err(detail) = &result {
                eprintln!("[site-import] failed: {detail}");
            }
            changed |= host.finish_home_site_import(generation, result);
        }
        changed
    }

    fn spawn(&mut self, generation: u64, url: String, locale: Locale) {
        let tx = self.tx.clone();
        self.in_flight += 1;
        let spawned = std::thread::Builder::new()
            .name("op-site-import".into())
            .spawn(move || {
                let result = op_host_services::site_import::import_site_screened(&url, locale)
                    .map_err(|error| error.to_string());
                let _ = tx.send((generation, result));
            });
        if let Err(error) = spawned {
            // Report through the normal path instead of leaving the button
            // spinning forever.
            let _ = self
                .tx
                .send((generation, Err(format!("could not start worker: {error}"))));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_url_reports_back_through_the_worker() {
        // A private address never leaves the machine: the SSRF screen
        // refuses it before any socket opens, so this test is offline.
        let mut host = WidgetHostNative::new();
        let home = &mut host.editor_state_mut().editor_ui.home;
        home.site_import.available = true;
        assert!(home.site_import.press("http://127.0.0.1:9/", 1));
        let mut jobs = SiteImportJobs::new();
        assert!(!jobs.drain(&mut host), "started, nothing landed yet");
        assert!(jobs.is_pending());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while jobs.is_pending() && std::time::Instant::now() < deadline {
            if jobs.drain(&mut host) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(matches!(
            host.editor_state().editor_ui.home.site_import.status,
            op_editor_core::HomeSiteImportStatus::Failed { .. }
        ));
    }
}
