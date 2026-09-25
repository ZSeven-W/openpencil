// Browser boundary: the XHR to the daemon needs a live page; the decisions
// around it (send mode, confirm, landing the reply) are host-unit-tested in
// `widget_host/studio_home_site_import_tests.rs`, and the route itself in
// `op-host-services/src/site_import_route_tests.rs`.
//! Studio Home's website import — the network half on the browser.
//!
//! The widget host records the request; this drains it into one
//! `POST /api/ai/site-import` and lands the reply through
//! `WidgetHost::finish_home_site_import`. The daemon runs the fetch and the
//! whole post-import pipeline, so the page only moves JSON.

use std::cell::RefCell;
use std::rc::Rc;

use crate::repaint_ctx::RepaintContext;

/// The route the daemon serves (`op_host_services::site_import_route`).
const SITE_IMPORT_ROUTE: &str = "/api/ai/site-import";

/// A page fetch + its stylesheets + the pipeline: generous, but bounded so
/// the button never spins forever on a hung daemon.
const SITE_IMPORT_TIMEOUT_MS: u32 = 120_000;

/// Start the queued import, if any. Called from the post-press / post-key
/// drain points (via `studio_web::drain_home_replace_confirm`).
pub(crate) fn drain_home_site_import<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    let (generation, url, locale) = {
        let Ok(mut b) = inner.try_borrow_mut() else {
            return;
        };
        let Some((generation, url)) = b.host_mut().take_home_site_import_request() else {
            return;
        };
        let locale = b.host().editor_state().editor_ui.effective_locale();
        (generation, url, locale)
    };
    let body = serde_json::json!({ "url": url, "locale": locale.code() }).to_string();
    let route = crate::daemon_base::daemon_url(SITE_IMPORT_ROUTE);
    let inner_cb = inner.clone();
    let started = crate::web_image_panel::post_json_with_timeout(
        &route,
        &body,
        SITE_IMPORT_TIMEOUT_MS,
        Rc::new(move |status, text| {
            let result = op_editor_core::parse_site_import_reply(status, &text);
            land(&inner_cb, generation, result);
        }),
    );
    if !started {
        land(
            inner,
            generation,
            Err("the request could not be started".to_string()),
        );
    }
}

fn land<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    generation: u64,
    result: Result<op_editor_core::SiteImportResult, String>,
) {
    let Ok(mut b) = inner.try_borrow_mut() else {
        return;
    };
    b.host_mut().set_clocks(
        crate::listener::now_ms_perf(),
        crate::listener::now_unix_secs(),
    );
    let changed = b.host_mut().finish_home_site_import(generation, result);
    if !changed {
        return;
    }
    b.host_mut().mark_editor_state_dirty();
    let embed = b.host().editor_state().editor_ui.embed;
    let unbind = b.host_mut().take_daemon_file_unbind();
    drop(b);
    // The imported page is a new, untitled deliverable: Save must download
    // it instead of overwriting the file the daemon was bound to.
    if unbind {
        crate::studio_web::unbind_daemon_file(embed);
    }
    crate::repaint_coalescer::request();
}
