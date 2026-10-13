//! Compatible blocking entry points for the judged search ladder.

use super::*;

/// Visual-judge variant of the desktop enrichment ladder (the no-judge
/// ladder is `resolve::fetch_first_image_resolved`).
pub fn fetch_first_image_url_blocking_with_judge(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
    judge: &dyn ImageRelevanceJudge,
    intent: &str,
) -> Option<String> {
    fetch_first_image_url_blocking_with_judge_and_deadline(
        query,
        aspect_ratio,
        credentials,
        used_urls,
        judge,
        intent,
        None,
    )
}

/// Preserve the caller's absolute deadline while seeking a better image
/// after finding a usable Weak candidate. `None` uses the local opportunity
/// window only; the original no-Weak search path remains unchanged.
pub fn fetch_first_image_url_blocking_with_judge_and_deadline(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
    judge: &dyn ImageRelevanceJudge,
    intent: &str,
    deadline: Option<Instant>,
) -> Option<String> {
    fetch_first_image_resolved_blocking_with_judge_and_deadline(
        query,
        aspect_ratio,
        credentials,
        used_urls,
        judge,
        intent,
        deadline,
    )
    .map(|resolved| resolved.src)
}

/// [`fetch_first_image_url_blocking_with_judge`] with the chosen hit's
/// provenance (provider stage, provider query, catalogue title).
pub fn fetch_first_image_resolved_blocking_with_judge(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
    judge: &dyn ImageRelevanceJudge,
    intent: &str,
) -> Option<ResolvedImage> {
    fetch_first_image_resolved_blocking_with_judge_and_deadline(
        query,
        aspect_ratio,
        credentials,
        used_urls,
        judge,
        intent,
        None,
    )
}

pub fn fetch_first_image_resolved_blocking_with_judge_and_deadline(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
    judge: &dyn ImageRelevanceJudge,
    intent: &str,
    deadline: Option<Instant>,
) -> Option<ResolvedImage> {
    fetch_first_image_resolved_blocking_with_judge_for_view(
        query,
        aspect_ratio,
        credentials,
        used_urls,
        judge,
        intent,
        deadline,
        None,
    )
}

/// Automatic-slot entry point. Unknown/manual geometry uses `None`, retaining
/// full-source judging. View preparation never changes the returned asset.
#[allow(clippy::too_many_arguments)]
pub fn fetch_first_image_url_blocking_with_judge_for_view(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
    judge: &dyn ImageRelevanceJudge,
    intent: &str,
    deadline: Option<Instant>,
    view: Option<crate::ImageJudgeView>,
) -> Option<String> {
    fetch_first_image_resolved_blocking_with_judge_for_view(
        query,
        aspect_ratio,
        credentials,
        used_urls,
        judge,
        intent,
        deadline,
        view,
    )
    .map(|resolved| resolved.src)
}

#[allow(clippy::too_many_arguments)]
pub fn fetch_first_image_resolved_blocking_with_judge_for_view(
    query: &str,
    aspect_ratio: Option<ImageAspectRatio>,
    credentials: Option<&WebOpenverseCredentials>,
    used_urls: &Mutex<HashSet<String>>,
    judge: &dyn ImageRelevanceJudge,
    intent: &str,
    deadline: Option<Instant>,
    view: Option<crate::ImageJudgeView>,
) -> Option<ResolvedImage> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    crate::net::block_on_image_runtime(async {
        let (selected, delivery_deadline) = fetch_first_image_url_with_judge(
            query,
            aspect_ratio,
            credentials,
            used_urls,
            judge,
            intent,
            deadline,
            view,
        )
        .await?;
        Some(
            super::super::original_delivery::upgrade_selected_image(
                selected,
                used_urls,
                delivery_deadline,
            )
            .await,
        )
    })
}
