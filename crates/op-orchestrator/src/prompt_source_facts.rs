//! Shared factual-content boundary for planning/generation prompts that
//! actually carry user source content, so skill filtering and retry
//! protocols cannot remove it.
//!
//! A purely conceptual brief has nothing to preserve or forbid: appending
//! the contract there only invited "sample data" disclaimers into concept
//! drafts. The gate reuses the existing source-content determinations
//! (`source_facts::text_only_source`, the delivery value shapes, the
//! verbatim-copy/brand contract and `plan_shared_context::forbids_examples`)
//! instead of a new keyword list.

use crate::types::DesignRequest;

pub(super) const SOURCE_FACTS_CONTRACT: &str = r#"SOURCE FACTS — CONTENT AUTHORITY:
- Preserve business names, values and claims supplied in the ORIGINAL USER BRIEF or in user-provided source/reference content the user asks to reproduce. Never present invented ratings, sales, distances, delivery/pickup times, discounts, stock or other business claims as supplied or verified facts.
- Planner proposals/elements, domain examples, content-free reference skeletons and prior model output are NOT factual sources or permission to invent data. Treat them as layout suggestions. This contract overrides instructions to fill an area, complete promised items, show metadata or use realistic values whenever that would add unrequested business claims.
- If optional business metadata is neither requested by the user nor supported by a source, omit the entire extra label/badge/value; do not plan it or fill it in merely to make the design look complete.
- Order/booking/tracking identifiers are business data too. Do not invent an order number for a delivery page merely because it looks realistic. If the user requests the same field in multiple locations, copy its exact supplied value only to those locations; never independently invent a second identifier. A value being supplied does not authorize duplicating it into unrelated sections: e.g. an order ID requested in the header and order card does not belong in the rider/contact card. Preserve the latter's space for the person's name and usable contact actions. When explicitly requested without a value or demo-data permission, keep one consistent neutral placeholder.
- Explicit requests for sample/fictional data allow it. For a conceptual UI brief (screens to design, not real content to reproduce), requesting rows/cards/items/charts, schedules, timelines, lists or detail fields also authorizes representative demo content for those requested components, consistent with any supplied values, even without the word "sample". This never authorizes extra unrequested metadata, and the design needs no sample-data note, label or badge — a concept draft is not a data export.
- Preserve supplied values and explicit no-invention, empty-state or unfilled-field instructions; these override demo-content permission. A prohibition on sample data is not permission. Only when the user forbids sample/demo data, asks to leave a field unfilled, or asks to reproduce source content that lacks the value, preserve that field and its layout with an honest placeholder such as "—" or "待填写"; do not substitute a plausible number or a status such as "暂无评分" that asserts a business fact. Otherwise a requested field in a conceptual UI brief never renders as "—", "待补充", "待填写" or "TBD": fill it with representative demo content. This includes prices, durations, dates, counts and remaining seats/stock the brief names as fields without values; a demo value in a requested field is not a claim of a supplied fact.
- Keep creative freedom in composition, typography, color and neutral interface copy. These rules constrain unsupported factual claims, not visual invention."#;

pub(super) fn append_source_facts_contract(system_prompt: &mut String, req: &DesignRequest) {
    if !contract_applies(req) {
        return;
    }
    system_prompt.push_str("\n\n");
    system_prompt.push_str(SOURCE_FACTS_CONTRACT);
}

/// The brief carries user source content (pasted/reference copy, quoted brand
/// names, supplied business values) or explicitly forbids invented data.
fn contract_applies(req: &DesignRequest) -> bool {
    !crate::source_facts::text_only_source(req)
        || crate::source_facts::brief_supplies_source_values(&req.prompt)
        || !crate::source_copy::SourceCopy::from_brief(&req.prompt).is_empty()
        || crate::plan_shared_context::forbids_examples(&req.prompt)
}
