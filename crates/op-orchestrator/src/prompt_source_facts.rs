//! Shared factual-content boundary, appended after every planning/generation
//! system prompt so skill filtering and retry protocols cannot remove it.

pub(super) const SOURCE_FACTS_CONTRACT: &str = r#"SOURCE FACTS — CONTENT AUTHORITY:
- Preserve business names, values and claims supplied in the ORIGINAL USER BRIEF or in user-provided source/reference content the user asks to reproduce. Never present invented ratings, sales, distances, delivery/pickup times, discounts, stock or other business claims as supplied or verified facts.
- Planner proposals/elements, domain examples, content-free reference skeletons and prior model output are NOT factual sources or permission to invent data. Treat them as layout suggestions. This contract overrides instructions to fill an area, complete promised items, show metadata or use realistic values.
- If optional business metadata is neither requested by the user nor supported by a source, omit the entire extra label/badge/value; do not plan it or fill it in merely to make the design look complete.
- Explicit requests for sample/fictional data allow it. For a conceptual UI brief, explicitly requesting populated rows/cards/items/charts and their data fields also authorizes representative demo content for those requested components, even without the word "sample". Make its example nature clear with one quiet group/page note, not a badge on every item. This never authorizes extra unrequested metadata.
- Preserve supplied values and explicit no-invention, empty-state or unfilled-field instructions; these override demo-content permission. A prohibition on sample data is not permission. If the user requests a field but supplies no value and does not authorize demo content, preserve that field and its layout with an honest placeholder such as "—" or "待填写". Do not substitute a plausible number or a status such as "暂无评分" that asserts a business fact.
- Keep creative freedom in composition, typography, color and neutral interface copy. These rules constrain unsupported factual claims, not visual invention."#;

pub(super) fn append_source_facts_contract(system_prompt: &mut String) {
    system_prompt.push_str("\n\n");
    system_prompt.push_str(SOURCE_FACTS_CONTRACT);
}
