/**
 * Sub-agent execution for the orchestrator.
 *
 * Each sub-agent is responsible for generating one spatial section of the
 * design (e.g. "Hero", "Features", "Footer"). This module handles:
 * - Sequential execution
 * - Streaming JSONL parsing and real-time canvas insertion
 * - ID namespace isolation via prefixes
 */

import type { VariableDefinition } from '@/types/variables';
import type { DesignMdSpec } from '@/types/design-md';
import type {
  AIDesignRequest,
  OrchestratorPlan,
  OrchestrationProgress,
  SubTask,
  SubAgentResult,
} from './ai-types';
import { streamChat } from './ai-service';
import { resolveSkills } from '@zseven-w/pen-ai-skills';
import { SUB_AGENT_DEBUG_FLAGS } from './sub-agent-debug-flags';
import { type PreparedDesignPrompt, getSubAgentTimeouts } from './orchestrator-prompt-optimizer';
import {
  buildSubAgentStyleGuideInstruction,
  compactSubAgentSkills,
} from './orchestrator-sub-agent-compact';
import { resolveModelProfile } from './model-profiles';
import {
  expandRootFrameHeight,
  buildVariableContext,
  applyPostStreamingTreeHeuristics,
} from './design-generator';
import { emitProgress } from './orchestrator-progress';
import { StreamingDesignRenderer } from './streaming-design-renderer';
import { buildDesignMdStylePolicy } from './ai-prompts';

export { ensureIdPrefix, ensurePrefixStr } from './streaming-design-renderer';

// ---------------------------------------------------------------------------
// Stream timeout configuration (shared with orchestrator.ts)
// ---------------------------------------------------------------------------

export interface StreamTimeoutConfig {
  hardTimeoutMs: number;
  noTextTimeoutMs: number;
  thinkingResetsTimeout: boolean;
  pingResetsTimeout?: boolean;
  firstTextTimeoutMs?: number;
  thinkingMode?: 'adaptive' | 'disabled' | 'enabled';
  thinkingBudgetTokens?: number;
  effort?: 'low' | 'medium' | 'high' | 'max';
}

// ---------------------------------------------------------------------------
// Sub-agent execution (sequential or concurrent)
// ---------------------------------------------------------------------------

export async function executeSubAgents(
  plan: OrchestratorPlan,
  request: AIDesignRequest,
  preparedPrompt: PreparedDesignPrompt,
  progress: OrchestrationProgress,
  concurrency: number = 1,
  callbacks?: {
    onApplyPartial?: (count: number) => void;
    onTextUpdate?: (text: string) => void;
    animated?: boolean;
  },
  abortSignal?: AbortSignal,
): Promise<SubAgentResult[]> {
  const timeoutOptions = getSubAgentTimeouts(preparedPrompt.originalLength, request.model);

  // Sequential path — each subtask runs one at a time
  if (concurrency <= 1) {
    const results: SubAgentResult[] = [];
    for (let i = 0; i < plan.subtasks.length; i++) {
      if (abortSignal?.aborted) break;

      let result = await executeSubAgent(
        plan.subtasks[i],
        plan,
        request,
        preparedPrompt,
        timeoutOptions,
        progress,
        i,
        callbacks,
        undefined,
        abortSignal,
      );

      // Retry once on failure (e.g. socket closed by provider). Skip retry
      // when the provider refused the request shape or content (HTTP 400/451)
      // — retrying the same prompt will hit the same determinstic refusal and
      // just wastes another round-trip (StepFun's 451 scan can take minutes).
      const isNonRetryable =
        !!result.error &&
        /HTTP 4(0[01]|29|51)|content blocked|authentication failed|censorship/i.test(result.error);
      if (result.error && result.nodes.length === 0 && !abortSignal?.aborted && !isNonRetryable) {
        console.warn(`[orchestrator] subtask ${i} failed, retrying: ${result.error}`);
        result = await executeSubAgent(
          plan.subtasks[i],
          plan,
          request,
          preparedPrompt,
          timeoutOptions,
          progress,
          i,
          callbacks,
          undefined,
          abortSignal,
          resolveModelProfile(request.model).tier === 'basic',
        );
      }

      // Minimal-skills fallback: re-run just this failing subtask with a
      // ~3KB kernel prompt (schema + jsonl-format only). Don't re-run any
      // earlier successful subtasks. Skip when the provider gave a
      // deterministic refusal (401/451/content-blocked) — a smaller prompt
      // won't get past the same policy check.
      if (result.error && result.nodes.length === 0 && !abortSignal?.aborted && !isNonRetryable) {
        console.warn(
          `[orchestrator] subtask ${i} still empty after retry, falling back to minimal skills: ${result.error}`,
        );
        result = await executeSubAgent(
          plan.subtasks[i],
          plan,
          request,
          preparedPrompt,
          timeoutOptions,
          progress,
          i,
          callbacks,
          undefined,
          abortSignal,
          true, // reducedComplexity
          true, // minimalSkills
        );
      }

      if (result.error && result.nodes.length === 0) {
        throw new Error(result.error);
      }

      results.push(result);

      if (result.nodes.length > 0) {
        expandRootFrameHeight();
      }
    }
    return results;
  }

  // Concurrent path — screen-grouped parallelism.
  // Subtasks sharing the same screen run sequentially (preserves section order).
  // Different screen groups run in parallel, limited by `concurrency`.
  const total = plan.subtasks.length;
  const results: (SubAgentResult | null)[] = Array.from({ length: total }, () => null);

  // Group subtasks by screen (same logic as orchestrator.ts)
  const screenGroups: number[][] = [];
  const screenMap = new Map<string, number>();
  for (let i = 0; i < total; i++) {
    const screen = plan.subtasks[i].screen ?? plan.subtasks[i].id;
    if (screenMap.has(screen)) {
      screenGroups[screenMap.get(screen)!].push(i);
    } else {
      screenMap.set(screen, screenGroups.length);
      screenGroups.push([i]);
    }
  }

  // Semaphore to limit total concurrent API calls
  let activeSlots = 0;
  const waitQueue: (() => void)[] = [];

  async function acquireSlot() {
    if (activeSlots < concurrency) {
      activeSlots++;
      return;
    }
    await new Promise<void>((resolve) => waitQueue.push(resolve));
    activeSlots++;
  }

  function releaseSlot() {
    activeSlots--;
    if (waitQueue.length > 0) {
      waitQueue.shift()!();
    }
  }

  // Each screen group runs its subtasks sequentially
  const workers = screenGroups.map(async (indices) => {
    for (const idx of indices) {
      if (abortSignal?.aborted) return;

      await acquireSlot();
      try {
        let result = await executeSubAgent(
          plan.subtasks[idx],
          plan,
          request,
          preparedPrompt,
          timeoutOptions,
          progress,
          idx,
          callbacks,
          undefined,
          abortSignal,
        );

        // Minimal-skills fallback — retry just this subtask with a ~3KB
        // kernel prompt if the full-skills attempt produced no nodes.
        // See the sequential path for rationale.
        if (result.error && result.nodes.length === 0 && !abortSignal?.aborted) {
          const nonRetryable =
            /HTTP 4(0[01]|29|51)|content blocked|authentication failed|censorship/i.test(
              result.error,
            );
          if (!nonRetryable) {
            console.warn(
              `[orchestrator] subtask ${idx} empty, falling back to minimal skills: ${result.error}`,
            );
            result = await executeSubAgent(
              plan.subtasks[idx],
              plan,
              request,
              preparedPrompt,
              timeoutOptions,
              progress,
              idx,
              callbacks,
              undefined,
              abortSignal,
              true, // reducedComplexity
              true, // minimalSkills
            );
          }
        }

        results[idx] = result;

        if (result.nodes.length > 0) {
          expandRootFrameHeight(plan.subtasks[idx].parentFrameId ?? undefined);
        }
      } catch (err) {
        results[idx] = {
          subtaskId: plan.subtasks[idx].id,
          nodes: [],
          rawResponse: '',
          error: err instanceof Error ? err.message : 'Unknown error',
        };
      } finally {
        releaseSlot();
      }
    }
  });

  await Promise.all(workers);

  // Collect non-null results
  const collected = results.filter((r): r is SubAgentResult => r !== null);

  // If ALL failed with zero nodes, throw
  const totalNodes = collected.reduce((sum, r) => sum + r.nodes.length, 0);
  if (totalNodes === 0 && collected.length > 0) {
    const errors = collected.filter((r) => r.error).map((r) => r.error!);
    const firstError = errors[0] ?? 'The model failed to generate any design output.';
    throw new Error(firstError);
  }

  return collected;
}

// ---------------------------------------------------------------------------
// Single sub-agent execution
// ---------------------------------------------------------------------------

async function executeSubAgent(
  subtask: SubTask,
  plan: OrchestratorPlan,
  request: AIDesignRequest,
  preparedPrompt: PreparedDesignPrompt,
  timeoutOptions: StreamTimeoutConfig,
  progress: OrchestrationProgress,
  index: number,
  callbacks?: {
    onApplyPartial?: (count: number) => void;
    onTextUpdate?: (text: string) => void;
    animated?: boolean;
  },
  promptOverride?: string,
  abortSignal?: AbortSignal,
  reducedComplexity = false,
  minimalSkills = false,
): Promise<SubAgentResult> {
  const animated = callbacks?.animated ?? false;
  const progressEntry = progress.subtasks[index];
  progressEntry.status = 'streaming';
  emitProgress(plan, progress, callbacks);

  // Context hint is set once at orchestrator level (combining all subtask labels)
  // to avoid race conditions during concurrent execution

  const userPrompt = buildSubAgentUserPrompt(
    subtask,
    plan,
    promptOverride ?? preparedPrompt.subAgentPrompt,
    request.prompt,
    request.model,
    request.context?.variables,
    request.context?.themes,
    request.context?.designMd,
  );

  const designMd = request.context?.designMd;
  const variables = request.context?.variables;
  const modelProfile = resolveModelProfile(request.model);
  const isMobileScreen = plan.rootFrame.width <= 480;

  // Build design.md payload for the skill template. If the structured summary
  // is empty (a bare-minimum design.md with only free-form text), fall back to
  // the raw markdown source so the sub-agent still sees the user's spec.
  let designMdContent = '';
  if (designMd) {
    const structured = buildDesignMdStylePolicy(designMd).trim();
    designMdContent = structured || designMd.raw.trim();
  }
  const hasDesignMdContent = designMdContent.length > 0;

  const genCtx = resolveSkills('generation', request.prompt, {
    flags: {
      hasVariables: !!variables && Object.keys(variables).length > 0,
      hasDesignMd: hasDesignMdContent,
      isBasicTier: modelProfile.tier === 'basic',
      // style-defaults.md only loads when no style direction exists at all:
      // - no pre-built style guide selected
      // - no usable design.md content (empty raw + empty structured summary)
      noStyleGuideMatch: !plan.selectedStyleGuideContent && !hasDesignMdContent,
    },
    dynamicContent: hasDesignMdContent ? { designMdContent } : undefined,
    budgetOverride:
      modelProfile.tier === 'basic' ? 5200 : modelProfile.tier === 'standard' ? 6500 : undefined,
  });

  // Debug-flag bisection for the cross-provider empty-response bug.
  // See `sub-agent-debug-flags.ts` for the toggles. All branches here
  // are no-ops when the corresponding flag is false (the default).
  let resolvedSkills = genCtx.skills;
  // `minimalSkills` is the last-ditch fallback: after a full-skill attempt
  // and a reduced-complexity retry both returned empty, re-run this ONE
  // subtask with just the schema + JSONL-format kernel. A 19KB system
  // prompt times out StepFun's safety scanner and occasionally yields
  // pure-reasoning streams on weaker models; the ~3KB kernel gives the
  // model a much better chance of actually emitting nodes.
  if (minimalSkills) {
    resolvedSkills = resolvedSkills.filter(
      (s) => s.meta.name === 'schema' || s.meta.name === 'jsonl-format',
    );
  } else if (SUB_AGENT_DEBUG_FLAGS.SKILLS_MINIMAL_ONLY) {
    resolvedSkills = resolvedSkills.filter(
      (s) => s.meta.name === 'schema' || s.meta.name === 'jsonl-format',
    );
  } else {
    if (SUB_AGENT_DEBUG_FLAGS.SKILLS_DISABLE_ANTI_SLOP) {
      resolvedSkills = resolvedSkills.filter((s) => s.meta.name !== 'anti-slop');
    }
    if (SUB_AGENT_DEBUG_FLAGS.SKILLS_DISABLE_LAYOUT) {
      resolvedSkills = resolvedSkills.filter((s) => s.meta.name !== 'layout');
    }
    if (SUB_AGENT_DEBUG_FLAGS.SKILLS_DISABLE_OVERFLOW) {
      resolvedSkills = resolvedSkills.filter((s) => s.meta.name !== 'overflow');
    }
  }
  resolvedSkills = compactSubAgentSkills(
    resolvedSkills,
    modelProfile.tier,
    isMobileScreen,
    !!plan.selectedStyleGuideContent || !!designMd,
    reducedComplexity,
  );

  const systemPrompt = resolvedSkills.map((s) => s.content).join('\n\n');

  if (SUB_AGENT_DEBUG_FLAGS.LOG_PROMPT_SIZE) {
    const skillNames = resolvedSkills.map((s) => s.meta.name).join(',');
    console.log(
      `[sub-agent] systemPrompt: chars=${systemPrompt.length} userPrompt=${userPrompt.length} skills=${skillNames}`,
    );
  }

  let rawResponse = '';

  const renderer = new StreamingDesignRenderer({
    agentColor: progressEntry.agentColor,
    agentName: progressEntry.agentName,
    idPrefix: subtask.idPrefix,
    parentFrameId: subtask.parentFrameId ?? plan.rootFrame.id,
    animated,
  });

  try {
    for await (const chunk of streamChat(
      systemPrompt,
      [{ role: 'user', content: userPrompt }],
      request.model,
      timeoutOptions,
      request.provider,
      abortSignal,
    )) {
      if (chunk.type === 'text') {
        rawResponse += chunk.content;
        emitProgress(plan, progress, callbacks, rawResponse);

        const count = renderer.feedText(rawResponse);
        if (count > 0) {
          progressEntry.nodeCount += count;
          progress.totalNodes += count;
          callbacks?.onApplyPartial?.(progress.totalNodes);
          emitProgress(plan, progress, callbacks, rawResponse);
        }
      } else if (chunk.type === 'thinking') {
        // Do not expose provider reasoning text in the checklist UI. It is
        // verbose, repetitive, and often duplicates the visible step labels.
        continue;
      } else if (chunk.type === 'error') {
        progressEntry.status = 'error';
        emitProgress(plan, progress, callbacks);
        return {
          subtaskId: subtask.id,
          nodes: renderer.getInsertedNodes(),
          rawResponse,
          error: chunk.content,
        };
      }
    }

    // Fallback batch extraction
    if (renderer.getAppliedIds().size === 0 && rawResponse.trim()) {
      const count = renderer.flushRemaining(rawResponse);
      if (count > 0) {
        progressEntry.nodeCount += count;
        progress.totalNodes += count;
        callbacks?.onApplyPartial?.(progress.totalNodes);
      }
    }

    if (renderer.getAppliedIds().size === 0) {
      renderer.finish();
      progressEntry.status = 'error';
      emitProgress(plan, progress, callbacks);

      // Build a diagnostic error with a preview of what the model returned
      let errorMsg = 'The model response could not be parsed as design nodes.';
      if (rawResponse.trim().length === 0) {
        errorMsg += ' The model returned an empty response.';
      } else {
        // Show a short snippet so the user can diagnose the issue
        const preview = rawResponse.trim().slice(0, 150);
        const hasJson = rawResponse.includes('{') && rawResponse.includes('"type"');
        if (!hasJson) {
          errorMsg +=
            ' The response did not contain valid JSON. Model output: "' +
            preview +
            (rawResponse.length > 150 ? '…' : '') +
            '"';
        } else {
          errorMsg +=
            ' JSON was found but contained no valid PenNode objects (need "id" and "type" fields).';
        }
      }

      return {
        subtaskId: subtask.id,
        nodes: renderer.getInsertedNodes(),
        rawResponse,
        error: errorMsg,
      };
    }

    // Apply tree-aware heuristics now that the full subtree is in the store.
    // During streaming, nodes were inserted individually without children, so
    // tree-aware heuristics (button width, frame height, clipContent) couldn't run.
    const rootId = renderer.getRootId();
    if (rootId) {
      applyPostStreamingTreeHeuristics(rootId);
    }

    progressEntry.status = 'done';
    // Delay indicator removal so the glow effect is visible even when the
    // subtask finishes quickly (e.g. model outputs everything in one chunk).
    renderer.finish(1500);
    emitProgress(plan, progress, callbacks);
    return { subtaskId: subtask.id, nodes: renderer.getInsertedNodes(), rawResponse };
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error';
    progressEntry.status = 'error';
    renderer.finish(1500);
    emitProgress(plan, progress, callbacks);
    return { subtaskId: subtask.id, nodes: renderer.getInsertedNodes(), rawResponse, error: msg };
  }
}

// ---------------------------------------------------------------------------
// Sub-agent prompt builder
// ---------------------------------------------------------------------------

function buildSubAgentUserPrompt(
  subtask: SubTask,
  plan: OrchestratorPlan,
  compactPrompt: string,
  fullPrompt: string,
  modelId?: string,
  variables?: Record<string, VariableDefinition>,
  themes?: Record<string, string[]>,
  designMd?: DesignMdSpec,
): string {
  const { region } = subtask;
  const modelTier = resolveModelProfile(modelId).tier;

  // Show all sections with their element boundaries so the model knows exact scope
  const sectionList = plan.subtasks
    .map((st) => {
      const marker = st.id === subtask.id ? ' ← YOU' : '';
      const elems = st.elements ? ` [${st.elements}]` : '';
      return `- ${st.label}${elems} (${st.region.width}x${st.region.height})${marker}`;
    })
    .join('\n');

  // Build explicit boundary instruction when elements are specified
  const myElements = subtask.elements
    ? `\nYOUR ELEMENTS: ${subtask.elements}\nDo NOT generate elements listed in other sections — they handle their own content.`
    : '';

  const rootBgColor = extractRootFrameFillColor(plan);
  const rootBgHint = rootBgColor
    ? `The page root frame already has background color ${rootBgColor} — your section inherits it.`
    : `The page root frame already carries the background color — your section inherits it.`;

  // When the user has a design.md, any numeric padding/spacing hint below must
  // defer to design.md. Otherwise use the legacy desktop default.
  const paddingHint = designMd
    ? `Use padding/spacing that matches the design.md "LAYOUT PRINCIPLES" and "COMPONENT STYLES" blocks below — those numbers OVERRIDE any generic defaults in these layout constraints.`
    : `Use padding=[0,80] for horizontal page margins.`;

  let prompt = `Page sections:\n${sectionList}\n\nGenerate ONLY "${subtask.label}" (~${region.height}px of content).${myElements}\n${compactPrompt}

CRITICAL LAYOUT CONSTRAINTS:
- Root frame: id="${subtask.idPrefix}-root", width="fill_container", height="fit_content", layout="vertical". NEVER use fixed pixel height on root — let content determine height.
- Target content amount: ~${region.height}px tall. Generate enough elements to fill this area.
- ALL nodes must be descendants of the root frame. No floating/orphan nodes.
- NEVER set x or y on children inside layout frames.
- Use "fill_container" for children that stretch, "fit_content" for shrink-wrap sizing.
- Use justifyContent="space_between" to distribute items (e.g. navbar: logo | links | CTA). ${paddingHint}
- For side-by-side layouts, nest a horizontal frame with child frames using "fill_container" width.
- SECTION BACKGROUND: do NOT set \`fill\` on your section root frame. ${rootBgHint} Hardcoding a "safe dark" fill (e.g. #000 / #0A0A0A / #111) will cover the intended background and break theme switching. Only set \`fill\` on cards, buttons, chips, badges, and other visually distinct components — never on the section container itself.
- IDs prefix="${subtask.idPrefix}-". No <step> tags. Output \`\`\`json immediately.`;

  // Phone mockup guidance is only relevant when this subtask is actually
  // rendering a phone mockup. Injecting it everywhere causes weaker models
  // (e.g. MiniMax M2) to wrap unrelated sections — bottom-nav, footers — in
  // a fake "Phone Mockup" frame and stuff the rest of the design inside.
  if (needsPhoneMockupInstruction(subtask.label, compactPrompt, fullPrompt, plan.rootFrame.width)) {
    prompt += `\n\nPHONE MOCKUP RULE:
- Phone mockup = ONE frame node, cornerRadius 32. If a placeholder label is needed, allow exactly ONE centered text child inside the phone; otherwise no children.
- Never place placeholder text below the phone as a sibling. NEVER use ellipse for the phone bezel.`;
  }

  // Prevent sub-agents from generating a duplicate status bar on mobile,
  // and explicitly tell them NOT to wrap their section in a phone mockup —
  // the design is already a mobile screen.
  if (plan.rootFrame.width <= 480) {
    prompt += `\n\nMOBILE STATUS BAR: A status bar (time, signal, wifi, battery) has ALREADY been pre-inserted as the first child of the root page frame. Do NOT generate any status bar, system chrome, or OS-level indicators. Start your content directly.`;
    prompt += `\n\nNO PHONE MOCKUP WRAPPER: The whole design IS a mobile screen. Do NOT wrap your section in a phone-shaped frame (cornerRadius 32 dark bezel, fixed 260-300px width, name "Phone Mockup"). Your section's root frame must use width="fill_container" and contain only the content that belongs to this section — never the entire app's children.`;
  }

  if (subtask.existingSectionLabels && subtask.existingSectionLabels.length > 0) {
    const existing = subtask.existingSectionLabels.map((n) => `"${n}"`).join(', ');
    prompt += `\n\nAPPEND MODE: The page already contains these sibling sections (read-only, already on canvas): ${existing}.
- Your root frame will be inserted as a NEW sibling at the end of that list.
- Do NOT re-emit any of the sections listed above. Do NOT emit any status bar or system chrome — that is also already on the page.
- Do NOT wrap your output in a phone mockup or a full-page container.
- Internal headings/titles within YOUR new section are fine — only the top-level sibling sections above are off-limits.
- Match the visual style (colors, cornerRadius, padding, gap) already established by those existing siblings.
- Output ONLY this one new section — a single root frame with its content.`;
  }

  if (needsNativeDenseCardInstruction(subtask.label, compactPrompt, fullPrompt)) {
    prompt += `\n\nNATIVE DENSE-CARD MODE (must be solved during generation):
- If you create a horizontal row with 5+ cards (or cards become narrow), compact each card natively BEFORE output.
- Each card: max 2 text blocks only (title + one short metric). Remove long descriptions.
- Rewrite long copy into concise keyword phrases. Never use truncation marks ("..." or "…").
- Prefer removing non-essential decorative elements before shrinking readability.
- Do NOT rely on post-processing to prune card content.`;
  }
  if (needsTableStructureInstruction(subtask.label, compactPrompt, fullPrompt)) {
    prompt += `\n\nTABLE MODE (must be structured natively):
- Build table as explicit grid frames, NOT a single long text line.
- Header must be its own horizontal row with separate cell frames for each column.
- Body rows must align to the same column structure as header.
- Keep level badge/chip inside the level cell; do not merge multiple columns into one text node.
- In table rows, avoid badge/button auto-style patterns unless the node is explicitly a chip.`;
  }
  if (needsHeroPhoneTwoColumnInstruction(subtask.label, compactPrompt, fullPrompt)) {
    prompt += `\n\nHERO PHONE LAYOUT MODE (desktop):
- Use a horizontal two-column hero layout: left = headline/subtitle/CTA, right = phone mockup.
- Keep phone as a sibling in the same horizontal row, NOT stacked below the headline.
- Only use stacked layout for mobile/narrow viewport sections.`;
  }

  // Style guide injection precedence:
  // 1. designMd (user's own design system) — highest, OVERRIDES everything else
  // 2. Selected pre-built style guide content — middle
  // 3. AI-generated styleGuide from planning (existing fallback) — lowest
  if (designMd) {
    const policy = buildDesignMdStylePolicy(designMd);
    if (policy) {
      prompt += `\n\nDESIGN SYSTEM (from design.md — follow these EXACTLY; they OVERRIDE any other style guide, default padding, or component convention):\n${policy}`;
    }
  } else if (plan.selectedStyleGuideContent) {
    prompt += `\n\n${buildSubAgentStyleGuideInstruction(
      plan.selectedStyleGuideContent,
      plan.styleGuideName,
      modelTier,
    )}`;
    if (/[\u4e00-\u9fff\u3040-\u309f\u30a0-\u30ff\uac00-\ud7af]/.test(fullPrompt)) {
      prompt +=
        '\n\nCJK OVERRIDE: The user prompt contains Chinese/Japanese/Korean text. Replace ALL heading/display fonts with "Noto Sans SC" (or "Noto Sans JP"/"Noto Sans KR" as appropriate). Keep body font as "Inter". Never use Latin-only display fonts like JetBrains Mono, Space Grotesk, Cormorant Garamond, etc. for CJK headings. Line heights for CJK: headings 1.3-1.4, body 1.6-1.8. Letter spacing: always 0 for CJK.';
    }
  } else if (plan.styleGuide) {
    const sg = plan.styleGuide;
    const p = sg.palette;
    prompt += `\n\nSTYLE GUIDE (use these consistently):
- Background: ${p.background}  Surface: ${p.surface}
- Text: ${p.text}  Secondary: ${p.secondary}
- Accent: ${p.accent}  Accent2: ${p.accent2}  Border: ${p.border}
- Heading font: ${sg.fonts.heading}  Body font: ${sg.fonts.body}
- Aesthetic: ${sg.aesthetic}`;
  }

  const varContext = buildVariableContext(variables, themes);
  if (varContext) {
    prompt += '\n\n' + varContext;
  }

  return prompt;
}

// ---------------------------------------------------------------------------
// Instruction detection helpers
// ---------------------------------------------------------------------------

function needsNativeDenseCardInstruction(
  subtaskLabel: string,
  compactPrompt: string,
  fullPrompt: string,
): boolean {
  const text = `${subtaskLabel}\n${compactPrompt}\n${fullPrompt}`.toLowerCase();
  if (
    /(dense|密集|多卡片|卡片过多|超过\s*4\s*个|5\+\s*cards?|cards?\s*row|一行.*卡片|横排.*卡片)/.test(
      text,
    )
  ) {
    return true;
  }
  if (/(cefr|a1[\s-]*c2|a1|a2|b1|b2|c1|c2|词库分级|分级词库|学习阶段|等级)/.test(text)) {
    return true;
  }
  if (
    /(feature\s*cards?|cards?\s*section|词库|词汇|card)/.test(text) &&
    /(a1|b1|c1|c2|cefr|等级|阶段)/.test(text)
  ) {
    return true;
  }
  return false;
}

function needsTableStructureInstruction(
  subtaskLabel: string,
  compactPrompt: string,
  fullPrompt: string,
): boolean {
  const text = `${subtaskLabel}\n${compactPrompt}\n${fullPrompt}`.toLowerCase();
  if (
    /(table|grid|tabular|表格|表头|表体|列|行|字段|等级|级别|词汇量|适用人群|对应考试)/.test(text)
  ) {
    return true;
  }
  if (/(cefr|a1[\s-]*c2|a1|a2|b1|b2|c1|c2)/.test(text) && /(level|table|表格|等级)/.test(text)) {
    return true;
  }
  return false;
}

function needsHeroPhoneTwoColumnInstruction(
  subtaskLabel: string,
  compactPrompt: string,
  fullPrompt: string,
): boolean {
  const text = `${subtaskLabel}\n${compactPrompt}\n${fullPrompt}`.toLowerCase();
  const heroLike = /(hero|首页首屏|首屏|横幅|banner)/.test(text);
  const phoneLike = /(phone|mockup|screenshot|截图|手机|app\s*screen|应用截图)/.test(text);
  return heroLike && phoneLike;
}

/**
 * Pull the first solid fill color off the plan's root frame, if any.
 * Used in the sub-agent prompt so the model sees the actual background
 * color it is inheriting and has no excuse to hedge with a hardcoded
 * "safe dark" fill on its own section root.
 */
function extractRootFrameFillColor(plan: OrchestratorPlan): string | null {
  const fill = plan.rootFrame?.fill;
  if (!fill || !Array.isArray(fill) || fill.length === 0) return null;
  const first = fill[0] as { type?: string; color?: string };
  if (first?.type === 'solid' && typeof first.color === 'string') return first.color;
  return null;
}

/**
 * Decide whether this sub-agent should see the phone mockup style guide.
 *
 * Mobile generation (root width <= 480) NEVER needs it: the design itself
 * is already a phone screen, so the prompt would only confuse the model
 * into wrapping its section in a fake bezel.
 *
 * Desktop generation needs it only when this specific sub-task is the one
 * rendering a phone mockup (hero showcase, app preview, etc.).
 */
function needsPhoneMockupInstruction(
  subtaskLabel: string,
  compactPrompt: string,
  fullPrompt: string,
  rootFrameWidth: number,
): boolean {
  if (rootFrameWidth <= 480) return false;
  const text = `${subtaskLabel}\n${compactPrompt}\n${fullPrompt}`.toLowerCase();
  return /(phone\s*mockup|app\s*mockup|app\s*screen|app\s*screenshot|device\s*frame|手机\s*样机|手机\s*模型|应用\s*截图|应用\s*预览)/.test(
    text,
  );
}

/** Test-only entry point so unit tests don't have to stand up real model calls. */
export function buildSubAgentUserPromptForTest(args: {
  subtask: SubTask;
  plan: OrchestratorPlan;
  compactPrompt: string;
  fullPrompt: string;
  modelId?: string;
  variables?: Record<string, VariableDefinition>;
  themes?: Record<string, string[]>;
  designMd?: DesignMdSpec;
}): string {
  return buildSubAgentUserPrompt(
    args.subtask,
    args.plan,
    args.compactPrompt,
    args.fullPrompt,
    args.modelId,
    args.variables,
    args.themes,
    args.designMd,
  );
}
