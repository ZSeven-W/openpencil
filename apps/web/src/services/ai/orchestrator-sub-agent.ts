/**
 * 子代理执行器。
 *
 * 每个子代理只负责页面中的一个空间区块，例如 Hero、Features、Footer。
 * 这个模块负责：
 * - 顺序或并发调度子代理
 * - 解析流式 JSONL，并实时把节点插入画布
 * - 用前缀隔离不同子任务的节点 ID 命名空间
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
// 流式超时配置（与 orchestrator.ts 共用）
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
// 子代理执行（顺序或并发）
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

  // 顺序路径：子任务逐个执行
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

      // 如果失败就重试一次，例如 provider 中途关掉 socket。
      // 但遇到 400 / 451 这类确定性拒绝时不要重试，
      // 因为同样的 prompt 再发一次通常只会得到同样的结果。
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

      // 最小技能兜底：
      // 只重跑当前失败的这一个子任务，并把 system prompt 压到约 3KB，
      // 只保留 schema + jsonl-format 这样的核心约束。
      // 已经成功的子任务不重跑。
      // 对于 401 / 451 / content-blocked 这类确定性拒绝，缩 prompt 也没用，直接跳过。
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

  // 并发路径：按 screen 分组后并行执行。
  // 同一 screen 内的子任务仍按顺序执行，保证区块顺序稳定；
  // 不同 screen 之间再受 `concurrency` 限制做并发。
  const total = plan.subtasks.length;
  const results: (SubAgentResult | null)[] = Array.from({ length: total }, () => null);

  // 按 screen 给子任务分组（逻辑与 orchestrator.ts 保持一致）
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

  // 用一个简单信号量限制并发 API 调用数
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

  // 每个 screen group 内部依然串行执行
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

        // 最小技能兜底：
        // full-skills 没产出任何节点时，就只用一个 ~3KB 的内核 prompt 重跑当前子任务。
        // 具体原因和顺序路径一致。
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

  // 收集非空结果
  const collected = results.filter((r): r is SubAgentResult => r !== null);

  // 如果全部失败且一个节点都没生成出来，就整体抛错
  const totalNodes = collected.reduce((sum, r) => sum + r.nodes.length, 0);
  if (totalNodes === 0 && collected.length > 0) {
    const errors = collected.filter((r) => r.error).map((r) => r.error!);
    const firstError = errors[0] ?? 'The model failed to generate any design output.';
    throw new Error(firstError);
  }

  return collected;
}

// ---------------------------------------------------------------------------
// 单个子代理的执行过程
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

  // 上下文提示由 orchestrator 层统一设置，
  // 这样并发执行时不会出现多个子代理互相覆盖的问题。

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

  // 为技能模板准备 design.md 内容。
  // 如果结构化摘要为空（例如只有一段自由文本），
  // 就退回原始 markdown，让子代理至少能看到用户规范。
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
      // `style-defaults.md` 只有在完全没有风格来源时才启用：
      // - 没选预置 style guide
      // - design.md 也没有可用内容
      noStyleGuideMatch: !plan.selectedStyleGuideContent && !hasDesignMdContent,
    },
    dynamicContent: hasDesignMdContent ? { designMdContent } : undefined,
    budgetOverride:
      modelProfile.tier === 'basic' ? 5200 : modelProfile.tier === 'standard' ? 6500 : undefined,
  });

  // 调试开关：用来排查跨 provider 的“空响应”问题。
  // 开关定义见 `sub-agent-debug-flags.ts`，默认都是 no-op。
  let resolvedSkills = genCtx.skills;
  // `minimalSkills` 是最后一层保底：
  // 如果 full-skills 和 reduced-complexity 都没有节点产出，
  // 就只重跑当前这一个子任务，并把 system prompt 压缩成 schema + JSONL 内核。
  // 这么做是因为超长 prompt 在一些 provider 上会卡安全扫描，
  // 或者让弱模型只输出 reasoning、不落实际节点。
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
        // 不把 provider 的 reasoning 文本直接显示到 checklist UI。
        // 它通常很长、重复，而且和步骤标签表达的是同一件事。
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

    // 兜底：如果流式过程没能成功落节点，再做一次整段批量提取
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

      // 构造带预览的诊断错误，方便快速看清模型到底返回了什么
      let errorMsg = 'The model response could not be parsed as design nodes.';
      if (rawResponse.trim().length === 0) {
        errorMsg += ' The model returned an empty response.';
      } else {
        // 带上一小段预览，便于快速判断是哪类问题
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

    // 现在整棵子树都已经在 store 里，可以补跑树感知启发式。
    // 流式阶段节点是一个个插入的，依赖完整子树的规则当时还跑不了。
    const rootId = renderer.getRootId();
    if (rootId) {
      applyPostStreamingTreeHeuristics(rootId);
    }

    progressEntry.status = 'done';
    // 稍微延迟移除指示器，
    // 避免“模型一口气吐完整段”时 glow 效果一闪而过。
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

  // 把所有 section 和元素边界都列出来，让模型知道自己的精确职责范围
  const sectionList = plan.subtasks
    .map((st) => {
      const marker = st.id === subtask.id ? ' ← YOU' : '';
      const elems = st.elements ? ` [${st.elements}]` : '';
      return `- ${st.label}${elems} (${st.region.width}x${st.region.height})${marker}`;
    })
    .join('\n');

  // 如果子任务显式给了 elements，就额外追加一段边界约束说明
  const myElements = subtask.elements
    ? `\nYOUR ELEMENTS: ${subtask.elements}\nDo NOT generate elements listed in other sections — they handle their own content.`
    : '';

  const rootBgColor = extractRootFrameFillColor(plan);
  const rootBgHint = rootBgColor
    ? `The page root frame already has background color ${rootBgColor} — your section inherits it.`
    : `The page root frame already carries the background color — your section inherits it.`;

  // 一旦用户提供 design.md，下面所有 padding / spacing 默认值都必须让位给它；
  // 没有 design.md 时才退回旧的桌面端默认值。
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

  // 手机样机提示只在当前子任务真的需要画手机样机时才有意义。
  // 如果到处都注入，一些较弱模型会把无关区块也包进假的 Phone Mockup。
  if (needsPhoneMockupInstruction(subtask.label, compactPrompt, fullPrompt, plan.rootFrame.width)) {
    prompt += `\n\nPHONE MOCKUP RULE:
- Phone mockup = ONE frame node, cornerRadius 32. If a placeholder label is needed, allow exactly ONE centered text child inside the phone; otherwise no children.
- Never place placeholder text below the phone as a sibling. NEVER use ellipse for the phone bezel.`;
  }

  // 在移动端明确禁止重复生成状态栏，
  // 也禁止再套一层手机样机，因为整个页面本身已经是手机屏。
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

  // 风格注入优先级：
  // 1. design.md（用户自己的设计系统）最高
  // 2. 选中的预置 style guide 次之
  // 3. 规划阶段 AI 推断出的 style guide 最后兜底
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
 * 取出计划根框架上的第一个实色填充颜色（如果存在）。
 * 这样子代理 prompt 就能看到自己继承的真实背景色，
 * 不至于在 section 根上再硬塞一个“安全深色背景”。
 */
function extractRootFrameFillColor(plan: OrchestratorPlan): string | null {
  const fill = plan.rootFrame?.fill;
  if (!fill || !Array.isArray(fill) || fill.length === 0) return null;
  const first = fill[0] as { type?: string; color?: string };
  if (first?.type === 'solid' && typeof first.color === 'string') return first.color;
  return null;
}

/**
 * 判断当前子代理是否应该看到“手机样机”相关提示。
 *
 * 移动端（根宽度 <= 480）永远不需要，
 * 因为整个设计本身已经是一块手机屏，再给这个提示只会误导模型去套假边框。
 *
 * 桌面端只有在当前子任务确实负责渲染手机样机时才需要，
 * 比如 Hero 里的 App 预览或设备展示位。
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

/** 仅供测试使用的入口，避免单测里真的去发模型请求。 */
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
