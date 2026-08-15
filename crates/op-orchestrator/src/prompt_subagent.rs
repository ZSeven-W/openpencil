//! `build_subagent_prompt` and its core builder.

use super::*;

/// 单个 sub-agent 的 LLM 调用输入。
///
/// * `reduced_complexity` — When `true` and the model is Basic tier,
///   narrows the skill set to the `retryAllowed` 8-skill set (drops
///   `elements` and other non-essential skills).  For Standard/Full
///   tier this is a no-op.  Port of the `reducedComplexity` param in
///   `executeSubAgent` (orchestrator-sub-agent.ts:349).
/// * `minimal_skills` — When `true`, strips the skill set down to only
///   `schema` (last-ditch fallback for models whose safety scanner times out
///   on the full prompt). The output protocol still comes from `SCRIPT_FORMAT`.
/// * `components` — the document's reusable-component registry. When
///   non-empty it injects an AVAILABLE COMPONENTS manifest + raises the
///   `hasReusableComponents` flag (loads the `component-composition`
///   skill). Empty (the default path) ⇒ prompt unchanged.
pub fn build_subagent_prompt(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    components: &ComponentLibrary,
) -> (CallRequest, SkillLoadReport) {
    build_subagent_prompt_with_screen_routes(
        subtask,
        plan,
        req,
        abort,
        reduced_complexity,
        minimal_skills,
        components,
        &[],
    )
}

/// Production prompt builder with the document-wide screen route inventory.
///
/// The public compatibility wrapper above passes an empty inventory so direct
/// callers that do not own a document snapshot keep byte-identical prompts.
/// Generation paths call this variant after resolving normalized plan groups
/// (or loop continuation's live screens) through navigation's shared route
/// allocator.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_subagent_prompt_with_screen_routes(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    components: &ComponentLibrary,
    screen_routes: &[(String, String)],
) -> (CallRequest, SkillLoadReport) {
    // Script-gen is THE subagent protocol on every rung. Retry flags narrow
    // the skill set only; they never switch the output protocol.
    let script_on = true;
    build_subagent_prompt_core(
        subtask,
        plan,
        req,
        abort,
        reduced_complexity,
        minimal_skills,
        script_on,
        components,
        screen_routes,
    )
}

/// Core of [`build_subagent_prompt`] — `script_on` is a parameter so tests can
/// exercise both protocol arms directly without depending on the
/// `reduced_complexity` / `minimal_skills` derivation.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_subagent_prompt_core(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    script_on: bool,
    components: &ComponentLibrary,
    screen_routes: &[(String, String)],
) -> (CallRequest, SkillLoadReport) {
    // Apply tier-gated filtering, then resolve the generation skill set under
    // the budget — that order, not the reverse; see the resolve call below.
    let model_id = req.model.as_deref().unwrap_or("");
    let tier = resolve_model_profile(model_id).tier;

    // Available reusable components → AVAILABLE COMPONENTS manifest +
    // `hasReusableComponents` flag (loads the `component-composition` skill).
    // `None` when the registry is empty, so the default no-component path is
    // byte-for-byte unchanged. `script_on` is already resolved by the caller
    // (a `build_subagent_prompt_core` parameter, not re-derived here) so the
    // manifest's ref-syntax example always matches the protocol this prompt
    // actually uses (SCRIPT_FORMAT vs NODE_FORMAT below).
    let component_manifest = available_components_manifest(components, script_on);
    let has_reusable_components = component_manifest.is_some();

    // design.md payload for the `{{designMdContent}}` template. If the
    // structured policy summary is empty (a bare-minimum design.md with only
    // free-form text), fall back to the raw markdown so the sub-agent still
    // sees the spec. Port of orchestrator-sub-agent.ts:379-384.
    let design_md_content = req
        .design_md
        .as_ref()
        .map(|spec| {
            let structured = build_design_md_style_policy(spec);
            let structured = structured.trim();
            if structured.is_empty() {
                spec.raw.trim().to_string()
            } else {
                structured.to_string()
            }
        })
        .unwrap_or_default();
    let has_design_md = !design_md_content.is_empty();
    // Rust `OrchestratorPlan` carries only the style-guide NAME (the TS
    // `selectedStyleGuideContent` content field has no Rust equivalent yet),
    // so `style_guide_name.is_some()` is the faithful proxy for "a guide was
    // selected". Port of the flag block in orchestrator-sub-agent.ts:396-416.
    let no_style_guide_match = plan.style_guide_name.is_none() && !has_design_md;

    let mut flags = HashMap::new();
    flags.insert("isBasicTier".to_string(), tier == ModelTier::Basic);
    flags.insert("hasDesignMd".to_string(), has_design_md);
    // No existing-document variable context is wired into `DesignRequest`
    // (TS sources this from `request.context.variables`), so this is always
    // false on the Rust path today.
    flags.insert("hasVariables".to_string(), false);
    flags.insert("noStyleGuideMatch".to_string(), no_style_guide_match);
    // Element-tools (N-tool) path is not ported to Rust (feature-flag off in
    // TS production); `elements`/`elements-cookbook` therefore stay gated off.
    flags.insert("hasMcpTools".to_string(), false);
    flags.insert("hasReusableComponents".to_string(), has_reusable_components);

    let mut dynamic_content = HashMap::new();
    if has_design_md {
        dynamic_content.insert("designMdContent".to_string(), design_md_content);
    }

    let is_mobile_screen = is_mobile_full_screen(plan);
    // Look up the planner-selected style guide by name and build a block that
    // injects its palette/fonts into the sub-agent prompt (port of
    // `buildSubAgentStyleGuideInstruction`). When present this REPLACES the
    // generic `design-system` skill.
    let style_guide_instruction =
        build_style_guide_instruction(plan.style_guide_name.as_deref(), tier);
    let resolved_style_instruction = build_resolved_style_instruction_for_plan(plan);
    // `design-system` is dropped when ANOTHER styling source already covers it:
    // the `design-md` skill (`has_design_md`), the `style-defaults` skill (loads
    // on `noStyleGuideMatch`), OR a style instruction block just built.
    // Keeping it alongside any of those would inject design-system's conflicting
    // "output ONLY a JSON token object" header redundantly (Codex review).
    let design_system_covered = has_design_md
        || no_style_guide_match
        || style_guide_instruction.is_some()
        || resolved_style_instruction.is_some();
    let explicit_tokens = extract_explicit_design_tokens(&req.prompt);
    let explicit_token_instruction = explicit_design_token_instruction(explicit_tokens);

    // Mobile UI guardrails live in the flag-gated `mobile-ui` skill rather than
    // inline prompt strings (user direction 2026-06-23: control generation via
    // skills, not hardcoded text in prompt.rs). Set the gate flag — matching the
    // old inline `plan.root_frame.width <= 480.0` condition exactly — and inject
    // the few dynamic values those rules reference.
    let is_mobile_layout = plan.root_frame.width <= 480.0;
    flags.insert("isMobileScreen".to_string(), is_mobile_layout);
    if is_mobile_layout {
        let mobile_spacing = explicit_tokens.spacing.map(format_design_number);
        let mobile_radius = explicit_tokens.radius.map(format_design_number);
        let rhythm = if let Some(spacing) = mobile_spacing.as_deref() {
            format!(
                "MOBILE VERTICAL RHYTHM: Keep every section root height=\"fit_content\" and do not insert blank spacer frames or empty bands to fill the planned region. Use {spacing}px as the default gap/spacing for header-to-search, search-to-next-section, module, grid, card, and internal component rhythm. Do not mix 16/20/24/32px gaps when the user explicitly requested {spacing}px spacing."
            )
        } else {
            "MOBILE VERTICAL RHYTHM: Keep every section root height=\"fit_content\" and do not insert blank spacer frames or empty bands to fill the planned region. Header-to-search spacing should be 12px, search-to-next-section 12-16px, major module gaps 16-24px, and internal gaps usually 8/12px.".to_string()
        };
        dynamic_content.insert("mobileRhythm".to_string(), rhythm);
        dynamic_content.insert(
            "mobileSearchRadius".to_string(),
            mobile_radius.as_deref().unwrap_or("8-12").to_string(),
        );
        dynamic_content.insert(
            "mobileGridGap".to_string(),
            mobile_spacing.as_deref().unwrap_or("12").to_string(),
        );
    }

    // Tier-scaled budget override (orchestrator-sub-agent.ts:414-415).
    // Basic mobile carries the `mobile-app` + `mobile-ui` domain skills after the
    // compact filter; the `mobile-ui` rules used to be appended to the user prompt
    // (uncounted) and now live in a budgeted skill, so the budget grows by ~its
    // size — the TOTAL prompt is unchanged, the rules just moved user→system.
    // A deck board gets its own arm for the same reason mobile does: the deck
    // teaching (`slides` + `deck-patterns` + `deck-contract`, ~5600 tokens)
    // sits on top of ~6000 of always-kept Base skills, so under the plain
    // 5200 / 6500 arms it is dropped for BudgetExhausted and the model designs
    // slides with no slide guidance.
    //
    // The arm is the Generation phase default rather than a literal, because
    // the deck path IS the worst case that default was last sized for
    // (`Phase::Generation` moved 12000 → 13200 when `deck-contract` landed).
    // Restating it as a number is what let the old 11500 rot when the corpus
    // grew under it: the deck skills were then silently dropped/tail-cut,
    // which `prompt_deck_skill_tests` now asserts against.
    //
    // Cards (DS P1.5) join the same arm for the same reason: the plain
    // Basic 5200 arm's always-kept Base skills alone resolve ~5440 tokens
    // (0815 measurement), so the card contract would otherwise be dropped
    // for BudgetExhausted on EVERY card prompt a weak model sees — the
    // 2026-08-04 `slides` failure, in a card jacket.
    //
    // Measured 2026-08-09 on that file's fixtures, every resolved skill
    // untruncated: Basic 11529/13200, Standard and Full both 12548/13200.
    // Standard lands on Full's exact skill set here — at an unbounded budget
    // it also carries `design-principles` (12986), and at 13200 the deck
    // corpus crowds that Knowledge skill out. That is NOT this arm's doing:
    // Full tier reads the same default and loses it identically, so the deck
    // load simply fills the phase. Buying it back means raising the phase
    // default, which belongs to the corpus owner, not to this override.
    let is_deck = is_deck_board(plan);
    let is_card = is_card_board(plan);
    let deck_budget = Phase::Generation.default_budget();
    let budget_override = match tier {
        ModelTier::Basic if is_mobile_layout || is_mobile_screen => Some(9200),
        ModelTier::Basic if is_deck || is_card => Some(deck_budget),
        ModelTier::Basic => Some(5200),
        ModelTier::Standard if is_mobile_layout => Some(9500),
        ModelTier::Standard if is_deck || is_card => Some(deck_budget),
        ModelTier::Standard => Some(6500),
        ModelTier::Full => None,
    };

    // Force-include the component-instance teaching whenever a reusable-component
    // library is loaded. When a library is present the model already receives the
    // AVAILABLE COMPONENTS *list*; the `component-composition` skill carries the
    // HOW-to-instantiate teaching (`ref` + `descendants` syntax) — without it the
    // model gets the catalog but no usable instruction and emits 0 refs. On the
    // tight non-mobile Basic budget (5200, of which base skills already use
    // ~3900) the skill is otherwise dropped by BudgetExhausted, so we pin it
    // (budget-exempt) here. Empty on every no-library path ⇒ no change there.
    let pinned_skills = if has_reusable_components {
        vec!["component-composition".to_string()]
    } else {
        Vec::new()
    };
    let opts = ResolveOptions {
        flags,
        dynamic_content,
        budget_override,
        pinned_skills,
        ..Default::default()
    };
    let intent = subtask_intent(req, subtask);
    // Filter BEFORE the budget knapsack, on every path.
    //
    // This used to be the Basic-mobile path only; everything else resolved
    // first and compacted second, which meant the knapsack paid for skills the
    // compaction was about to delete. `design-system` (554 tokens) is the
    // standing example: `design_system_covered` is true on essentially every
    // real request, so the budget bought it, the filter dropped it, and the
    // 554 tokens were never returned to the skills that had just lost to it —
    // a deck prompt reported 12548/13200 while `design-principles` (438) sat
    // in the dropped list as BudgetExhausted, because at knapsack time only 98
    // tokens were actually free. Ordering, not sizing: raising the ceiling
    // would have hidden it rather than fixed it.
    let (mut filtered, resolve_report, filter_drops) =
        resolve_generation_skills_after_prompt_filter(
            &intent,
            model_id,
            &opts,
            tier,
            is_mobile_screen,
            design_system_covered,
            minimal_skills,
            reduced_complexity,
        );
    // Script-gen REPLACES the raw-JSONL output format — carrying a JSONL skill
    // alongside it feeds the model two contradictory output contracts. Keep
    // this guard even though the JSONL skills are no longer mounted by the
    // generation registry.
    if script_on {
        filtered.retain(|s| {
            s.skill_name() != "jsonl-format" && s.skill_name() != "jsonl-format-simplified"
        });
    }

    let mut system_prompt = filtered
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    system_prompt.push_str("\n\n");
    // The public subagent path always appends SCRIPT_FORMAT. The NODE_FORMAT
    // branch is retained for direct core callers/tests that intentionally
    // exercise the legacy dialect.
    system_prompt.push_str(if script_on {
        SCRIPT_FORMAT
    } else {
        NODE_FORMAT
    });
    // Append the selected style guide's palette/fonts so the sub-agent follows
    // it instead of inventing a conflicting one.
    if let Some(sg) = &style_guide_instruction {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(sg);
    }
    if let Some(resolved) = &resolved_style_instruction {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(resolved);
    }
    if let Some(instruction) = &explicit_token_instruction {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(instruction);
        system_prompt
            .push_str("\nThis instruction overrides style-guide examples when they conflict.");
    }
    // Available-components manifest LAST — recency wins, and the model should
    // consult the concrete id list right before producing nodes. The
    // `component-composition` skill (loaded via `hasReusableComponents`) carries
    // the `ref` + `descendants` syntax; this block carries the actual ids.
    if let Some(manifest) = &component_manifest {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(manifest);
    }

    let section_list = plan
        .subtasks
        .iter()
        .map(|st| {
            let marker = if st.id == subtask.id { " <- YOU" } else { "" };
            let elements = st
                .elements
                .as_ref()
                .map(|items| format!(" [{items}]"))
                .unwrap_or_default();
            format!(
                "- {}{} ({:.0}x{:.0}){}",
                st.label, elements, st.region.width, st.region.height, marker
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let my_elements = subtask
        .elements
        .as_ref()
        .map(|items| {
            format!("\nYOUR ELEMENTS: {items}\nDo NOT generate elements listed in other sections.")
        })
        .unwrap_or_default();
    let explicit_user_token_block = explicit_token_instruction
        .as_ref()
        .map(|instruction| format!("{instruction}\n\n"))
        .unwrap_or_default();
    let screen_route_block = screen_route_prompt_block(screen_routes);
    let spacing_rule = if is_mobile_layout {
        "SPACING CONSISTENCY — MOBILE CONTENT RAIL: The root page may keep 0 horizontal padding for full-width status/navigation/full-bleed media. This ordinary transparent root-direct section owns padding:[0,24] exactly once; do not duplicate it on an inner wrapper. If this section is a clipped horizontal scroller, keep its section full width, inset its header 24px on both sides, and give the clipped viewport a 24px leading inset with a flush 0px trailing edge."
    } else {
        "SPACING CONSISTENCY: Use a single outer content gutter and consistent internal gaps. Do not create nested wrappers with conflicting padding or content touching edges."
    };

    // Two constraints differ by output protocol. The public subagent path uses
    // the script-gen branch; the raw-JSONL branch is legacy-only for direct
    // core callers.
    let (root_rule, nesting_rule, output_rule) = if script_on {
        (
            format!("Create EXACTLY ONE section root frame first: const sec = I(null, {{type:\"frame\", name:\"{}\", width:\"fill_container\", height:\"fit_content\", layout:\"vertical\"}}); build everything else by calling I(parent, {{...}}) with `sec` or a returned id as parent. NEVER set a fixed pixel height on the root.", subtask.label),
            "Nest via the returned id ONLY: const row = I(sec, {...}); const cell = I(row, {...}); I(cell, {type:\"text\",...}). LOOP over a data array to emit repeated rows/cards. A cell inserted into the section/table directly renders as a full-width band, not a table cell.".to_string(),
            "Output ONLY the JavaScript program (calls to I(...)) -- no prose, no markdown fences.".to_string(),
        )
    } else {
        (
            format!("Root frame: id=\"{}-root\", width=\"fill_container\", height=\"fit_content\", layout=\"vertical\". NEVER use fixed pixel height on root -- let content determine height.", subtask.id_prefix),
            "ALL nodes must be descendants of the root frame -- every non-root node must be nested under its parent (row -> its cards -> each card's content), in whichever format the system prompt specifies. No floating/orphan nodes; a flat sibling list with no parent links collapses into a vertical stack.".to_string(),
            format!("IDs prefix=\"{}-\". Output ONLY the structured nodes in the EXACT format the system prompt specifies above -- no prose, no extra wrapping.", subtask.id_prefix),
        )
    };
    let mut user_prompt = format!(
        "Page sections:\n{}\n\n\
Generate ONLY \"{}\" (~{:.0}px of content).{}\n\
Overall design: {}\n\n\
{}\
{}\
CRITICAL LAYOUT CONSTRAINTS:\n\
- {}\n\
- Target content amount: ~{:.0}px tall. Generate enough elements to fill this area.\n\
- DENSITY: Do NOT pack the area edge-to-edge. Prefer fewer, stronger modules with visible negative space; most sections should have 3-5 primary rows/cards at most.\n\
- VISUAL HIERARCHY: Each section must have one clear focal element, secondary supporting text, and quieter metadata. Avoid equal-weight blocks competing for attention.\n\
- {spacing_rule}\n\
- CRAFT POLISH: Add refinement through restrained 1px low-contrast borders, tonal surfaces, small state badges, and subtle shadows. Avoid template-like thick outlines, giant pills, or flat blocks with no micro-detail.\n\
- MEDIA CONSISTENCY: Use photographic images sparingly and keep them visually consistent in subject, crop, tone, and radius. For food/category UI, prefer cohesive icon or illustration tiles over random unrelated photos.\n\
- ICON SCALE: Icons support content; keep most icons 16-22px inside 36-48px controls. Avoid oversized circular icon bubbles or repeated identical icon treatments unless the design brief calls for them.\n\
- ACCENT DISCIPLINE: Reserve saturated accent color for one primary CTA or promo plus small highlights. Do not apply it to every icon, label, border, and large surface at once.\n\
- SIGNATURE MOMENT: Give the first viewport one memorable focal module that feels custom to the brief: distinctive composition, branded surface, expressive hero image/illustration, or an editorial product moment. Supporting modules must stay quieter.\n\
- WOW FACTOR: Make the design feel bespoke through domain-specific composition, confident cropping, custom icon rhythm, and one precise visual idea. Do not rely on generic tinted wrappers, heavy shadows, emoji, or repeated rounded boxes to look polished.\n\
- COMPOSITIONAL CONTRAST: Create interest through scale contrast, asymmetric balance, layered depth, and one clear focal path. Do not make every card, chip, icon, and CTA the same visual weight.\n\
- PREMIUM DETAIL: Use high-quality details such as precise alignment, consistent radii, quiet dividers, tonal badges, subtle image masks, and purposeful shadows. Prefer one polished detail over many loud decorations.\n\
- NO DECORATION SPAM: Do not add random blobs, repeated icon circles, excessive gradients, oversized badges, or unrelated photos just to make the design look busy. Every visual flourish must support the product story.\n\
- {}\n\
- NEVER set x or y on children inside layout frames.\n\
- Use \"fill_container\" for children that stretch, \"fit_content\" for shrink-wrap sizing.\n\
- SECTION BACKGROUND: do NOT set fill on your section root frame. Only set fill on cards, buttons, chips, badges, and other visually distinct components.\n\
- TYPOGRAPHY HIERARCHY: Do NOT make every text bold. Use 700 only for primary headings, 600 for buttons/key labels, 500 for short chips/nav labels, and 400 for body text, placeholders, subtitles, metadata, and captions.\n\
- ICONS: use icon_font with lucide iconFontName; never use path nodes for icons.\n\
- {}",
        section_list,
        subtask.label,
        subtask.region.height,
        my_elements,
        req.prompt,
        screen_route_block,
        explicit_user_token_block,
        root_rule,
        subtask.region.height,
        nesting_rule,
        output_rule,
    );

    // (Mobile UI guardrails now load from the `mobile-ui` skill — see the
    // `isMobileScreen` flag + dynamic-content setup above.)

    // Quality-rejection feedback echoed back into a SAME-tier retry instead
    // of silently narrowing the skill set — the content was otherwise
    // real, so the model just needs to fix the flagged issue. Two sources,
    // two wordings (`plan::RetryFeedback`):
    // - `SelfCheck` — `orchestration_self_check` rejected it BEFORE
    //   insertion (retry ladder attempt 2; `retry::is_self_check_rejection`).
    // - `Geometry` — the REAL resolved layout of an already-INSERTED
    //   subtree proved a structural violation (the `geometry_echo` step,
    //   `concurrent::run_subtask_retry_ladder`'s tail).
    if let Some(feedback) = subtask.retry_feedback.as_ref() {
        let block = match feedback {
            crate::plan::RetryFeedback::SelfCheck(reason) => format!(
                "\n\nSELF-CHECK FIX REQUIRED: your previous attempt at this exact \
                 section was rejected before insertion for this reason: {reason}\n\
- Regenerate the section addressing that reason specifically — do not change \
  anything else about the approach.\n\
- Keep using the full skill set and design detail from your previous attempt; \
  the rejection was a geometry/structure issue, not a signal to simplify."
            ),
            crate::plan::RetryFeedback::Geometry(reason) => format!(
                "\n\nGEOMETRY FIX REQUIRED: the REAL resolved layout of your previous \
                 attempt at this exact section has these structural problems:\n{reason}\n\
- Regenerate the section fixing exactly these — do not change anything else \
  about the approach.\n\
- Keep using the full skill set and design detail from your previous attempt; \
  these are layout/structure problems, not a signal to simplify."
            ),
        };
        user_prompt.push_str(&block);
    }

    // Port of orchestrator-sub-agent.ts:739-748 — APPEND MODE prompt injection.
    if let Some(labels) = subtask.existing_section_labels.as_ref() {
        if !labels.is_empty() {
            let existing = labels
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(", ");
            user_prompt.push_str(&format!(
                "\n\nAPPEND MODE: The page already contains these sibling sections (read-only, already on canvas): {existing}.\n\
- Your root frame will be inserted as a NEW sibling at the end of that list.\n\
- Do NOT re-emit any of the sections listed above. Do NOT emit any status bar or system chrome — that is also already on the page.\n\
- Do NOT wrap your output in a phone mockup or a full-page container.\n\
- Internal headings/titles within YOUR new section are fine — only the top-level sibling sections above are off-limits.\n\
- Match the visual style (colors, cornerRadius, padding, gap) already established by those existing siblings.\n\
- Output ONLY this one new section — a single root frame with its content."
            ));
        }
    }

    // Port of getSubAgentTimeouts(preparedPrompt.originalLength, model):
    // `originalLength` = normalized user prompt length; here `req.prompt.len()`
    // is the closest equivalent (we don't have a separate "normalized" form).
    let profile = resolve_model_profile(model_id);
    let t = apply_profile_to_timeouts(
        sub_agent_timeouts(req.prompt.len(), tier),
        profile.timeout_multiplier,
    );

    // Assemble the per-subtask skill-load report from the FINAL skill set
    // (post tier/dedup filtering). `budget_max` reflects the tier budget
    // override. Full-tier falls through to `Phase::Generation::default_budget()`
    // (13200 today — see that constant's doc comment for both raises: 8000 →
    // 12000 because image-rich data-list sections overflowed and truncated
    // their scripts to zero generated nodes, then 12000 → 13200 when
    // `deck-contract` joined the deck corpus). This used to be a bare literal
    // that only affected this diagnostic number — `resolve_skills` (called
    // above via `resolve_generation_skills`) independently fell back to the
    // OLD default for a `None` override, so Full tier's real skill trimming
    // silently ran at one number while this report claimed another. Deriving
    // both from the same constant keeps them honest, and is why the deck arm
    // above reads the constant instead of restating it.
    let budget_max = budget_override.unwrap_or_else(|| Phase::Generation.default_budget());
    let included: Vec<SkillLoadEntry> = filtered
        .iter()
        .map(|s| SkillLoadEntry {
            name: s.skill_name().to_string(),
            category: s.meta.category,
            token_count: s.token_count,
            truncated: s.truncated,
        })
        .collect();
    let budget_used: u32 = included.iter().map(|e| e.token_count).sum();
    // Merge resolve-time drops (IntentMiss + BudgetExhausted from B0) with
    // tier/dedup/mode drops captured by apply_skill_filter (B3b).
    let mut dropped = resolve_report.dropped;
    dropped.extend(
        filter_drops
            .into_iter()
            .map(|(name, reason)| op_ai_skills::DroppedSkill { name, reason }),
    );
    let report = SkillLoadReport {
        included,
        dropped,
        budget_used,
        budget_max,
    };

    (
        CallRequest {
            system_prompt,
            user_prompt,
            model: req.model.clone(),
            provider: req.provider.clone(),
            timeout: t.hard,
            abort,
            no_text_timeout: Some(t.no_text),
            first_text_timeout: Some(t.first_text),
        },
        report,
    )
}

fn screen_route_prompt_block(screen_routes: &[(String, String)]) -> String {
    if screen_routes.is_empty() {
        return String::new();
    }

    let rows = screen_routes
        .iter()
        .map(|(name, route)| {
            let name = serde_json::to_string(name).expect("serializing a string cannot fail");
            let route = serde_json::to_string(route).expect("serializing a string cannot fail");
            format!("- {name} -> {route}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "DOCUMENT SCREEN ROUTES (use these exact route values in schema-encoded navigation \
         actions; never invent another route):\n{rows}\n\n"
    )
}
