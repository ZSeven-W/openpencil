//! The shipped scene-template documents and their daemon routes.
//!
//! Split out of `scene_template_catalog.rs` (pure code motion) so the
//! catalogue spine stays under the 800-line cap as the library grows: these
//! two tables gain a line per template, the parser and scene enum do not.
//! Re-exported from the spine, so every import path is unchanged.

/// The shipped documents.
///
/// Embedded on desktop so opening a template needs no filesystem. NOT embedded
/// on `wasm32`: the documents are ~6.2 MiB of JSON that every visitor would
/// download before the editor painted, to open a panel most sessions never
/// touch. There the document is fetched from the daemon when a template is
/// actually instantiated — see [`scene_template_document_route`] and
/// `op_editor_core::web_assets`.
macro_rules! template_document {
    ($file:literal) => {{
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(include_str!(concat!(
                "../assets/scene_templates/",
                $file,
                ".op"
            )))
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::web_assets::installed_str(concat!(
                "/pkg/assets/",
                "scene_templates",
                "/",
                $file,
                ".op"
            ))
        }
    }};
}

/// Daemon route carrying a template's document.
///
/// `None` for an unknown id. Always present for a shipped template on both
/// platforms — native simply has no use for it, because its bytes are already
/// in the binary.
pub fn scene_template_document_route(template_id: &str) -> Option<&'static str> {
    macro_rules! route {
        ($file:literal) => {
            Some(concat!(
                "/pkg/assets/",
                "scene_templates",
                "/",
                $file,
                ".op"
            ))
        };
    }
    match template_id {
        "screenshot-tutorial" => route!("screenshot-tutorial"),
        "knowledge-carousel" => route!("knowledge-carousel"),
        "before-after" => route!("before-after"),
        "slide-deck" => route!("slide-deck"),
        "knowledge-card-vertical" => route!("knowledge-card-vertical"),
        "knowledge-card-square" => route!("knowledge-card-square"),
        "pitch-deck-dark" => route!("pitch-deck-dark"),
        "lecture-deck-light" => route!("lecture-deck-light"),
        "minimal-keynote" => route!("minimal-keynote"),
        "gradient-tech" => route!("gradient-tech"),
        "saas-landing-orange" => route!("saas-landing-orange"),
        "product-landing-light" => route!("product-landing-light"),
        "punch-quote-card" => route!("punch-quote-card"),
        "journal-checklist-card" => route!("journal-checklist-card"),
        "data-report-infographic" => route!("data-report-infographic"),
        "steps-flow-infographic" => route!("steps-flow-infographic"),
        "event-poster-deck" => route!("event-poster-deck"),
        "pitfall-list-infographic" => route!("pitfall-list-infographic"),
        "spine-culture-card" => route!("spine-culture-card"),
        "metric-single-card" => route!("metric-single-card"),
        "quote-frame-card" => route!("quote-frame-card"),
        "daily-sign-card" => route!("daily-sign-card"),
        "price-tier-card" => route!("price-tier-card"),
        "notice-board-card" => route!("notice-board-card"),
        "milestone-timeline-infographic" => route!("milestone-timeline-infographic"),
        "concept-contrast-infographic" => route!("concept-contrast-infographic"),
        "ranking-board-infographic" => route!("ranking-board-infographic"),
        "faq-thread-infographic" => route!("faq-thread-infographic"),
        "data-story-infographic" => route!("data-story-infographic"),
        "challenge-tracker-infographic" => route!("challenge-tracker-infographic"),
        "ecosystem-map-infographic" => route!("ecosystem-map-infographic"),
        "do-dont-comparison" => route!("do-dont-comparison"),
        "myth-truth-comparison" => route!("myth-truth-comparison"),
        "pricing-tiers-comparison" => route!("pricing-tiers-comparison"),
        "scenario-guide-comparison" => route!("scenario-guide-comparison"),
        "spec-table-comparison" => route!("spec-table-comparison"),
        "three-way-comparison" => route!("three-way-comparison"),
        "time-shift-comparison" => route!("time-shift-comparison"),
        "tradeoff-scale-comparison" => route!("tradeoff-scale-comparison"),
        "version-diff-comparison" => route!("version-diff-comparison"),
        "app-onboarding-triptych" => route!("app-onboarding-triptych"),
        "diy-blueprint-guide" => route!("diy-blueprint-guide"),
        "photo-composition-tutorial" => route!("photo-composition-tutorial"),
        "recipe-four-step" => route!("recipe-four-step"),
        "skincare-routine-cards" => route!("skincare-routine-cards"),
        "software-step-tutorial" => route!("software-step-tutorial"),
        "storage-makeover-steps" => route!("storage-makeover-steps"),
        "weekly-report-lesson" => route!("weekly-report-lesson"),
        "workout-breakdown-guide" => route!("workout-breakdown-guide"),
        "bookreview-silk-carousel" => route!("bookreview-silk-carousel"),
        "cityguide-film-carousel" => route!("cityguide-film-carousel"),
        "datareport-grid-carousel" => route!("datareport-grid-carousel"),
        "opinion-longform-carousel" => route!("opinion-longform-carousel"),
        "qa-chalkboard-carousel" => route!("qa-chalkboard-carousel"),
        "story-night-carousel" => route!("story-night-carousel"),
        "toolkit-notebook-carousel" => route!("toolkit-notebook-carousel"),
        "tutorial-journal-carousel" => route!("tutorial-journal-carousel"),
        "yearreview-mineral-carousel" => route!("yearreview-mineral-carousel"),
        "sounding-navy-deck" => route!("sounding-navy-deck"),
        "tidemark-slate-deck" => route!("tidemark-slate-deck"),
        "banxin-rule-deck" => route!("banxin-rule-deck"),
        "gridpaper-graphite-deck" => route!("gridpaper-graphite-deck"),
        "dossier-linen-deck" => route!("dossier-linen-deck"),
        "ledger-tick-deck" => route!("ledger-tick-deck"),
        "ai-support-pitch-deck" => route!("ai-support-pitch-deck"),
        "quarterly-review-deck" => route!("quarterly-review-deck"),
        "quicksort-lecture-deck" => route!("quicksort-lecture-deck"),
        "brand-concept-sheet" => route!("brand-concept-sheet"),
        "logo-qa-board" => route!("logo-qa-board"),
        "event-invitation-card" => route!("event-invitation-card"),
        "livestream-teaser-card" => route!("livestream-teaser-card"),
        "hiring-poster-card" => route!("hiring-poster-card"),
        "course-enroll-card" => route!("course-enroll-card"),
        "conference-agenda-card" => route!("conference-agenda-card"),
        "product-launch-card" => route!("product-launch-card"),
        "annual-report-card" => route!("annual-report-card"),
        "music-fest-poster-card" => route!("music-fest-poster-card"),
        "book-club-invite-card" => route!("book-club-invite-card"),
        "compound-effect-card" => route!("compound-effect-card"),
        "analytics-metric-card" => route!("analytics-metric-card"),
        "coffee-order-app" => route!("coffee-order-app"),
        "budget-ledger-app" => route!("budget-ledger-app"),
        "habit-fitness-app" => route!("habit-fitness-app"),
        "coffee-counter-desktop" => route!("coffee-counter-desktop"),
        "sales-dashboard-web" => route!("sales-dashboard-web"),
        "onboarding-training-deck" => route!("onboarding-training-deck"),
        "research-findings-deck" => route!("research-findings-deck"),
        "daybreak-coffee-site" => route!("daybreak-coffee-site"),
        "openpencil-intro-deck" => route!("openpencil-intro-deck"),
        "openpencil-intro-deck-43" => route!("openpencil-intro-deck-43"),
        "coffee-world-carousel" => route!("coffee-world-carousel"),
        "focus-mode-tutorial" => route!("focus-mode-tutorial"),
        "weekly-review-infographic" => route!("weekly-review-infographic"),
        "idea-to-publish-flow" => route!("idea-to-publish-flow"),
        "blank-vs-example-contrast" => route!("blank-vs-example-contrast"),
        "city-music-fest-poster" => route!("city-music-fest-poster"),
        _ => None,
    }
}

/// Return the document JSON for a template id.
///
/// The match is exhaustive over what ships; [`super::scene_template_catalogue`]
/// verifies every catalogue entry resolves here on native, so a template added
/// to the TOML without its document fails at first use rather than presenting a
/// card that does nothing when clicked.
///
/// On `wasm32` a `None` for a SHIPPED id means "not fetched yet" rather than
/// "no such template" — callers there must distinguish the two through
/// [`scene_template_document_route`], which answers for the id alone.
pub fn scene_template_document(template_id: &str) -> Option<&'static str> {
    match template_id {
        "screenshot-tutorial" => template_document!("screenshot-tutorial"),
        "knowledge-carousel" => template_document!("knowledge-carousel"),
        "before-after" => template_document!("before-after"),
        "slide-deck" => template_document!("slide-deck"),
        "knowledge-card-vertical" => template_document!("knowledge-card-vertical"),
        "knowledge-card-square" => template_document!("knowledge-card-square"),
        "pitch-deck-dark" => template_document!("pitch-deck-dark"),
        "lecture-deck-light" => template_document!("lecture-deck-light"),
        "minimal-keynote" => template_document!("minimal-keynote"),
        "gradient-tech" => template_document!("gradient-tech"),
        "saas-landing-orange" => template_document!("saas-landing-orange"),
        "product-landing-light" => template_document!("product-landing-light"),
        "punch-quote-card" => template_document!("punch-quote-card"),
        "journal-checklist-card" => template_document!("journal-checklist-card"),
        "data-report-infographic" => template_document!("data-report-infographic"),
        "steps-flow-infographic" => template_document!("steps-flow-infographic"),
        "event-poster-deck" => template_document!("event-poster-deck"),
        "pitfall-list-infographic" => template_document!("pitfall-list-infographic"),
        "spine-culture-card" => template_document!("spine-culture-card"),
        "metric-single-card" => template_document!("metric-single-card"),
        "quote-frame-card" => template_document!("quote-frame-card"),
        "daily-sign-card" => template_document!("daily-sign-card"),
        "price-tier-card" => template_document!("price-tier-card"),
        "notice-board-card" => template_document!("notice-board-card"),
        "milestone-timeline-infographic" => template_document!("milestone-timeline-infographic"),
        "concept-contrast-infographic" => template_document!("concept-contrast-infographic"),
        "ranking-board-infographic" => template_document!("ranking-board-infographic"),
        "faq-thread-infographic" => template_document!("faq-thread-infographic"),
        "data-story-infographic" => template_document!("data-story-infographic"),
        "challenge-tracker-infographic" => template_document!("challenge-tracker-infographic"),
        "ecosystem-map-infographic" => template_document!("ecosystem-map-infographic"),
        "do-dont-comparison" => template_document!("do-dont-comparison"),
        "myth-truth-comparison" => template_document!("myth-truth-comparison"),
        "pricing-tiers-comparison" => template_document!("pricing-tiers-comparison"),
        "scenario-guide-comparison" => template_document!("scenario-guide-comparison"),
        "spec-table-comparison" => template_document!("spec-table-comparison"),
        "three-way-comparison" => template_document!("three-way-comparison"),
        "time-shift-comparison" => template_document!("time-shift-comparison"),
        "tradeoff-scale-comparison" => template_document!("tradeoff-scale-comparison"),
        "version-diff-comparison" => template_document!("version-diff-comparison"),
        "app-onboarding-triptych" => template_document!("app-onboarding-triptych"),
        "diy-blueprint-guide" => template_document!("diy-blueprint-guide"),
        "photo-composition-tutorial" => template_document!("photo-composition-tutorial"),
        "recipe-four-step" => template_document!("recipe-four-step"),
        "skincare-routine-cards" => template_document!("skincare-routine-cards"),
        "software-step-tutorial" => template_document!("software-step-tutorial"),
        "storage-makeover-steps" => template_document!("storage-makeover-steps"),
        "weekly-report-lesson" => template_document!("weekly-report-lesson"),
        "workout-breakdown-guide" => template_document!("workout-breakdown-guide"),
        "bookreview-silk-carousel" => template_document!("bookreview-silk-carousel"),
        "cityguide-film-carousel" => template_document!("cityguide-film-carousel"),
        "datareport-grid-carousel" => template_document!("datareport-grid-carousel"),
        "opinion-longform-carousel" => template_document!("opinion-longform-carousel"),
        "qa-chalkboard-carousel" => template_document!("qa-chalkboard-carousel"),
        "story-night-carousel" => template_document!("story-night-carousel"),
        "toolkit-notebook-carousel" => template_document!("toolkit-notebook-carousel"),
        "tutorial-journal-carousel" => template_document!("tutorial-journal-carousel"),
        "yearreview-mineral-carousel" => template_document!("yearreview-mineral-carousel"),
        "sounding-navy-deck" => template_document!("sounding-navy-deck"),
        "tidemark-slate-deck" => template_document!("tidemark-slate-deck"),
        "banxin-rule-deck" => template_document!("banxin-rule-deck"),
        "gridpaper-graphite-deck" => template_document!("gridpaper-graphite-deck"),
        "dossier-linen-deck" => template_document!("dossier-linen-deck"),
        "ledger-tick-deck" => template_document!("ledger-tick-deck"),
        "ai-support-pitch-deck" => template_document!("ai-support-pitch-deck"),
        "quarterly-review-deck" => template_document!("quarterly-review-deck"),
        "quicksort-lecture-deck" => template_document!("quicksort-lecture-deck"),
        "brand-concept-sheet" => template_document!("brand-concept-sheet"),
        "logo-qa-board" => template_document!("logo-qa-board"),
        "event-invitation-card" => template_document!("event-invitation-card"),
        "livestream-teaser-card" => template_document!("livestream-teaser-card"),
        "hiring-poster-card" => template_document!("hiring-poster-card"),
        "course-enroll-card" => template_document!("course-enroll-card"),
        "conference-agenda-card" => template_document!("conference-agenda-card"),
        "product-launch-card" => template_document!("product-launch-card"),
        "annual-report-card" => template_document!("annual-report-card"),
        "music-fest-poster-card" => template_document!("music-fest-poster-card"),
        "book-club-invite-card" => template_document!("book-club-invite-card"),
        "compound-effect-card" => template_document!("compound-effect-card"),
        "analytics-metric-card" => template_document!("analytics-metric-card"),
        "coffee-order-app" => template_document!("coffee-order-app"),
        "budget-ledger-app" => template_document!("budget-ledger-app"),
        "habit-fitness-app" => template_document!("habit-fitness-app"),
        "coffee-counter-desktop" => template_document!("coffee-counter-desktop"),
        "sales-dashboard-web" => template_document!("sales-dashboard-web"),
        "onboarding-training-deck" => template_document!("onboarding-training-deck"),
        "research-findings-deck" => template_document!("research-findings-deck"),
        "daybreak-coffee-site" => template_document!("daybreak-coffee-site"),
        "openpencil-intro-deck" => template_document!("openpencil-intro-deck"),
        "openpencil-intro-deck-43" => template_document!("openpencil-intro-deck-43"),
        "coffee-world-carousel" => template_document!("coffee-world-carousel"),
        "focus-mode-tutorial" => template_document!("focus-mode-tutorial"),
        "weekly-review-infographic" => template_document!("weekly-review-infographic"),
        "idea-to-publish-flow" => template_document!("idea-to-publish-flow"),
        "blank-vs-example-contrast" => template_document!("blank-vs-example-contrast"),
        "city-music-fest-poster" => template_document!("city-music-fest-poster"),
        _ => None,
    }
}
