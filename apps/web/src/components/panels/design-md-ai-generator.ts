import { useState, useRef, useEffect, useCallback } from 'react';
import { useAIStore } from '@/stores/ai-store';
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDesignMdStore } from '@/stores/design-md-store';
import { streamChat } from '@/services/ai/ai-service';
import { parseDesignMd } from '@/utils/design-md-parser';
import type { PenNode } from '@/types/pen';

// ---------------------------------------------------------------------------
// AI auto-generate prompt
// ---------------------------------------------------------------------------

const DESIGN_MD_SYSTEM_PROMPT = `You are a Design Systems Lead. Analyze the provided PenNode design tree and generate a comprehensive design.md in the Google Stitch format.

OUTPUT FORMAT — a complete markdown document with these sections:

# Design System: [Project Name]

## 1. Visual Theme & Atmosphere
Describe the mood, density, and aesthetic philosophy using evocative adjectives.

## 2. Color Palette & Roles
For each color found in the design:
- **Descriptive Name** (#HEX) — Functional role (e.g. "Primary CTA", "Background", "Body text")

## 3. Typography Rules
- Font families used, weight hierarchy, size scale, line-height conventions.

## 4. Component Stylings
- **Buttons**: shape, colors, padding, states
- **Cards**: corners, shadows, internal padding
- **Inputs**: borders, backgrounds
- **Navigation**: layout, spacing

## 5. Layout Principles
- Grid system, whitespace strategy, spacing units, responsive breakpoints.

## 6. Design System Notes
- Key language/terms to use when generating new designs in this style.

RULES:
- Use descriptive natural language, NOT technical jargon (e.g. "subtly rounded corners" not "rounded-lg").
- Pair ALL colors with exact hex codes.
- Explain functional roles for every design element.
- Output ONLY the markdown document, starting with "# Design System:".
- NO preamble, NO commentary, NO tool calls, NO code fences around the output.
- Do NOT use <tool_call> tags or any tool invocations. Just output the markdown text directly.`;

// ---------------------------------------------------------------------------
// Clean AI response artifacts
// ---------------------------------------------------------------------------

function cleanAIResult(raw: string): string {
  let text = raw.trim();

  // Remove <tool_call>...</tool_call> blocks (XML-style tool calls)
  text = text.replace(/<tool_call>[\s\S]*?<\/tool_call>/g, '');

  // Remove preamble before the first markdown heading
  const headingIdx = text.search(/^#\s+/m);
  if (headingIdx > 0) {
    text = text.substring(headingIdx);
  }

  // Strip wrapping code fences
  if (text.startsWith('```')) {
    text = text.replace(/^```(?:markdown|md)?\n?/, '').replace(/\n?```$/, '');
  }

  // Remove JSON tool-call artifacts (e.g. {"name":"Write","arguments":...})
  text = text.replace(/\{"name"\s*:\s*"(?:Write|Read|Edit|Bash)"[^}]*\}\s*/g, '');

  // Remove lines that are tool call fragments or AI narration
  text = text
    .split('\n')
    .filter((line) => {
      const trimmed = line.trim();
      if (trimmed.startsWith('{"name"') || trimmed.startsWith('{"tool_use_id"')) return false;
      if (/^\{"file_path"\s*:/.test(trimmed)) return false;
      // Drop leftover tool_call tags
      if (trimmed === '<tool_call>' || trimmed === '</tool_call>') return false;
      return true;
    })
    .join('\n');

  // Strip code fence blocks containing JSON tool calls
  text = text.replace(/```json\s*\{[^`]*?"(?:file_path|name|arguments)"[^`]*?```/gs, '');

  // Collapse excessive blank lines
  text = text.replace(/\n{4,}/g, '\n\n\n');

  return text.trim();
}

// ---------------------------------------------------------------------------
// Build a compact summary of a design node tree
// ---------------------------------------------------------------------------

function summarizeNode(n: PenNode, depth = 0): string {
  const indent = '  '.repeat(depth);
  const props: string[] = [];
  if (n.name) props.push(`"${n.name}"`);
  if (n.role) props.push(`role=${n.role}`);
  if ('fill' in n && Array.isArray(n.fill)) {
    for (const f of n.fill) {
      if (f.type === 'solid' && f.color) props.push(`fill:${f.color}`);
      if (f.type === 'linear_gradient') props.push('fill:gradient');
    }
  }
  if ('stroke' in n && n.stroke) {
    const sf = n.stroke.fill?.[0];
    if (sf?.type === 'solid' && sf.color) props.push(`stroke:${sf.color}/${n.stroke.thickness}`);
  }
  if ('content' in n && n.content) props.push(`"${String(n.content).slice(0, 40)}"`);
  if ('fontSize' in n) props.push(`${n.fontSize}px`);
  if ('fontFamily' in n) props.push(`font:${n.fontFamily}`);
  if ('fontWeight' in n) props.push(`w:${n.fontWeight}`);
  if ('width' in n) props.push(`w=${n.width}`);
  if ('height' in n) props.push(`h=${n.height}`);
  if ('cornerRadius' in n && n.cornerRadius) props.push(`r=${n.cornerRadius}`);
  if ('gap' in n && n.gap) props.push(`gap=${n.gap}`);
  if ('padding' in n && n.padding) props.push(`pad=${JSON.stringify(n.padding)}`);
  if ('layout' in n && n.layout && n.layout !== 'none') props.push(`layout=${n.layout}`);
  if ('justifyContent' in n && n.justifyContent) props.push(`justify=${n.justifyContent}`);
  if ('alignItems' in n && n.alignItems) props.push(`align=${n.alignItems}`);
  if ('effects' in n && Array.isArray(n.effects) && n.effects.length > 0) {
    props.push(`effects=${n.effects.map((e) => e.type).join(',')}`);
  }
  if ('opacity' in n && n.opacity !== undefined && n.opacity !== 1)
    props.push(`opacity=${n.opacity}`);

  const line = `${indent}${n.type} ${props.join(' ')}`;
  const childLines: string[] = [];
  if ('children' in n && Array.isArray(n.children) && depth < 5) {
    for (const child of n.children.slice(0, 40)) {
      childLines.push(summarizeNode(child as PenNode, depth + 1));
    }
  }
  return [line, ...childLines].join('\n');
}

// ---------------------------------------------------------------------------
// Hook: useDesignMdAIGenerator
// ---------------------------------------------------------------------------

export function useDesignMdAIGenerator() {
  const setDesignMd = useDesignMdStore((s) => s.setDesignMd);
  const [isGenerating, setIsGenerating] = useState(false);
  const generateAbortRef = useRef<AbortController | null>(null);

  // Cleanup abort on unmount
  useEffect(
    () => () => {
      generateAbortRef.current?.abort();
    },
    [],
  );

  const handleAutoGenerate = useCallback(async () => {
    if (isGenerating) {
      generateAbortRef.current?.abort();
      setIsGenerating(false);
      return;
    }

    const model = useAIStore.getState().model;
    const modelGroups = useAIStore.getState().modelGroups;
    const provider = modelGroups.find((g) => g.models.some((m) => m.value === model))?.provider;
    if (!model || !provider) return;

    const doc = useDocumentStore.getState().document;
    const activePageId = useCanvasStore.getState().activePageId;

    // Get nodes from the active page
    const nodes = getActivePageChildren(doc, activePageId);
    if (nodes.length === 0) return;

    // Build a compact summary of the design tree
    const treeSummary = nodes
      .slice(0, 10)
      .map((n) => summarizeNode(n as PenNode))
      .join('\n\n');

    // Variable summary
    let varSummary = '';
    if (doc.variables && Object.keys(doc.variables).length > 0) {
      varSummary =
        '\n\nDESIGN VARIABLES:\n' +
        Object.entries(doc.variables)
          .map(([name, def]) => {
            const val = Array.isArray(def.value)
              ? String(def.value[0]?.value ?? '')
              : String(def.value);
            return `- ${name} (${def.type}): ${val}`;
          })
          .join('\n');
    }

    const userMessage = `Analyze this PenNode design tree and generate a comprehensive design.md:\n\nProject: ${doc.name ?? 'Untitled'}\n\nDesign tree (PenNode format — type followed by properties):\n${treeSummary}${varSummary}`;

    setIsGenerating(true);
    const abortController = new AbortController();
    generateAbortRef.current = abortController;

    try {
      let result = '';
      for await (const chunk of streamChat(
        DESIGN_MD_SYSTEM_PROMPT,
        [{ role: 'user', content: userMessage }],
        model,
        { thinkingMode: 'disabled', effort: 'high' },
        provider,
        abortController.signal,
      )) {
        if (chunk.type === 'text') {
          result += chunk.content;
        }
        if (chunk.type === 'error') break;
      }
      if (result.trim()) {
        const cleaned = cleanAIResult(result);
        if (cleaned) {
          const spec = parseDesignMd(cleaned);
          setDesignMd(spec);
        }
      }
    } finally {
      setIsGenerating(false);
      generateAbortRef.current = null;
    }
  }, [isGenerating, setDesignMd]);

  const hasAI = useAIStore((s) => s.availableModels.length > 0);

  return { isGenerating, hasAI, handleAutoGenerate };
}
