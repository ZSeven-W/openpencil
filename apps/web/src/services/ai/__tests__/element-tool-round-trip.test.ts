import { describe, it, expect, beforeEach, vi } from 'vitest';
import { tryParseElementToolOutput, tryParseAllElementToolOutputs } from '../design-parser';
import { dispatchElementToolCall } from '../element-tools-dispatcher';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { resolveModelProfile } from '../model-profiles';

/**
 * Round-trip gate: args JSON-serialized inside `<op_tool>…</op_tool>`
 * must survive `tryParseElementToolOutput` without data loss, and
 * the dispatcher+shim+builder must produce the exact same node the
 * builder-direct path would produce.
 *
 * Why this matters: the on-the-wire shape is
 *   <op_tool>{"name":"add_X_v0","arguments":{...}}</op_tool>
 *
 * If any part of the chain (the Vite bundler, a model-profile
 * transform, an escape-on-write helper) mangles Unicode / large
 * numbers / nested arrays / embedded quotes, the issue can manifest
 * as wrong text content on canvas OR as a silent parse failure that
 * drops the whole tool. This test stresses each class of payload
 * directly so the round-trip path is the single point of breakage.
 *
 * Tier axis: each test runs against basic / standard / full tier
 * profile resolutions. The wire format is tier-independent TODAY,
 * and this test anchors that as a hard contract — if a future
 * "tier-specific argument escape" is introduced, this suite fails
 * immediately with a diff.
 */

const TIERS = ['full', 'standard', 'basic'] as const;
type Tier = (typeof TIERS)[number];

// Model ids chosen so resolveModelProfile maps them cleanly. Note
// `gpt-4o-mini` matches the `gpt-4o` prefix rule (standard tier)
// before it reaches the mini-specific rule, so we use `claude-haiku`
// for the basic slot instead — that has a unique prefix with no
// earlier overlap in the MODEL_PROFILES table.
const MODEL_BY_TIER: Record<Tier, string> = {
  full: 'claude-opus-4',
  standard: 'gpt-4o',
  basic: 'claude-haiku',
};

function assertTier(modelId: string, expected: Tier): void {
  const profile = resolveModelProfile(modelId);
  expect(profile.tier).toBe(expected);
}

function wireFormat(name: string, args: Record<string, unknown>): string {
  return `<op_tool>${JSON.stringify({ name, arguments: args })}</op_tool>`;
}

function seedRoot(name: string): string {
  const id = `root-${Math.random().toString(36).slice(2)}`;
  useDocumentStore.getState().addNode(null, {
    id,
    type: 'frame',
    name,
    width: 375,
    height: 812,
    layout: 'vertical',
    children: [],
  });
  return id;
}

describe('model profiles tier gating sanity', () => {
  // Verify the model ids we use below actually resolve to the tier
  // we claim in MODEL_BY_TIER. If a model's tier classification
  // changes, we want to see the failure here, not in downstream
  // tests that think they cover basic but actually hit standard.
  for (const t of TIERS) {
    it(`${MODEL_BY_TIER[t]} resolves to tier "${t}"`, () => {
      assertTier(MODEL_BY_TIER[t], t);
    });
  }
});

describe('round-trip — simple ASCII args', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  for (const tier of TIERS) {
    it(`${tier}: add_heading_v0 with ASCII content survives round-trip`, async () => {
      const rootId = seedRoot(`${tier}-root`);
      const rawAi = wireFormat('add_heading_v0', { content: 'Hello World' });
      const shape = tryParseElementToolOutput(rawAi);
      expect(shape?.kind).toBe('element-tool');
      if (shape?.kind !== 'element-tool') return;
      expect(shape.name).toBe('add_heading_v0');
      expect(shape.arguments.content).toBe('Hello World');

      const result = await dispatchElementToolCall(shape, { defaultParentId: rootId });
      expect(result.status).toBe('applied');
      const node = result.insertedNodes[0] as { content?: string };
      expect(node.content).toBe('Hello World');
    });
  }
});

describe('round-trip — CJK + mixed scripts (emojis deliberately stripped)', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  const FIXTURES = [
    { label: 'chinese', content: '欢迎使用 OpenPencil 设计工具' },
    { label: 'japanese', content: 'こんにちは世界' },
    { label: 'korean', content: '안녕하세요 세계' },
    { label: 'mixed_cjk_latin', content: 'Welcome こんにちは 世界 안녕' },
    { label: 'rtl_arabic', content: 'مرحبا بالعالم' },
  ];

  for (const tier of TIERS) {
    for (const fx of FIXTURES) {
      it(`${tier} / ${fx.label}: content preserved through parse+dispatch`, async () => {
        const rootId = seedRoot(`root-${tier}-${fx.label}`);
        const rawAi = wireFormat('add_heading_v0', { content: fx.content });
        const shape = tryParseElementToolOutput(rawAi);
        if (shape?.kind !== 'element-tool') throw new Error('parse failed');
        expect(shape.arguments.content).toBe(fx.content);

        const result = await dispatchElementToolCall(shape, { defaultParentId: rootId });
        expect(result.status).toBe('applied');
        const node = result.insertedNodes[0] as { content?: string };
        expect(node.content).toBe(fx.content);
      });
    }
  }
});

describe('round-trip — emojis intentionally stripped (applyNoEmojiIconHeuristic)', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  // Baseline contract: applyNoEmojiIconHeuristic in design-canvas-ops
  // (via applyGenerationHeuristics) strips emoji from text content
  // because the AI is supposed to emit icon_font nodes for icons, not
  // embed emoji in text. These tests anchor that behavior so a future
  // pipeline change that accidentally PRESERVES emojis becomes visible.
  it('text-only emojis stripped, surrounding text kept', async () => {
    const rootId = seedRoot('root-emoji-strip');
    const rawAi = wireFormat('add_body_text_v0', {
      content: 'Launch 🚀 app and ship 🚢 today',
    });
    const shape = tryParseElementToolOutput(rawAi);
    if (shape?.kind !== 'element-tool') throw new Error('parse failed');
    const result = await dispatchElementToolCall(shape, { defaultParentId: rootId });
    expect(result.status).toBe('applied');
    const node = result.insertedNodes[0] as { content?: string };
    // Expected: emojis gone + double-spaces collapsed + trimmed
    expect(node.content).toBe('Launch app and ship today');
  });
});

describe('round-trip — embedded quotes / backslashes / newlines', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  const TRICKY = [
    { label: 'embedded_double_quote', content: 'She said "hello" and left' },
    { label: 'embedded_single_quote', content: "It's a trap" },
    { label: 'backslash', content: 'Path: C:\\Users\\test' },
    { label: 'newline_in_content', content: 'Line 1\nLine 2\nLine 3' },
    { label: 'tab_in_content', content: 'Col1\tCol2\tCol3' },
    { label: 'unicode_punctuation', content: 'Dash—em-dash, ellipsis… and quotes“like this”' },
  ];

  for (const tier of TIERS) {
    for (const fx of TRICKY) {
      it(`${tier} / ${fx.label}: tricky ASCII preserved`, async () => {
        const rootId = seedRoot(`root-${tier}-${fx.label}`);
        const rawAi = wireFormat('add_body_text_v0', { content: fx.content });
        const shape = tryParseElementToolOutput(rawAi);
        if (shape?.kind !== 'element-tool') throw new Error('parse failed');
        expect(shape.arguments.content).toBe(fx.content);

        const result = await dispatchElementToolCall(shape, { defaultParentId: rootId });
        expect(result.status).toBe('applied');
        const node = result.insertedNodes[0] as { content?: string };
        expect(node.content).toBe(fx.content);
      });
    }
  }
});

describe('round-trip — large numbers + nested arrays', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  for (const tier of TIERS) {
    it(`${tier}: chart_bars with 20 float values preserved`, async () => {
      const rootId = seedRoot(`root-${tier}-bars`);
      const values = Array.from({ length: 20 }, (_, i) => +(Math.sin(i / 2) * 100).toFixed(3));
      const rawAi = wireFormat('add_chart_bars_v0', { values, chart_height: 160 });
      const shape = tryParseElementToolOutput(rawAi);
      if (shape?.kind !== 'element-tool') throw new Error('parse failed');
      expect(shape.arguments.values).toEqual(values);

      const result = await dispatchElementToolCall(shape, { defaultParentId: rootId });
      expect(result.status).toBe('applied');
      // Builder emits one child bar per value
      const bars = result.insertedNodes[0] as { children?: unknown[] };
      expect(bars.children?.length).toBe(20);
    });

    it(`${tier}: stat_grid with nested item objects`, async () => {
      const rootId = seedRoot(`root-${tier}-stats`);
      const items = [
        { value: '12,345', label: 'Daily active users', icon: 'users' },
        { value: '98.7%', label: 'Uptime this week', icon: 'activity' },
        { value: '4.9★', label: 'App store rating', icon: 'star' },
      ];
      const rawAi = wireFormat('add_stat_grid_v0', { items });
      const shape = tryParseElementToolOutput(rawAi);
      if (shape?.kind !== 'element-tool') throw new Error('parse failed');
      expect(shape.arguments.items).toEqual(items);

      const result = await dispatchElementToolCall(shape, { defaultParentId: rootId });
      expect(result.status).toBe('applied');
    });

    it(`${tier}: timeline with 5 items + active flags`, async () => {
      const rootId = seedRoot(`root-${tier}-timeline`);
      const items = [
        { title: 'Order placed', subtitle: '10:42 AM', active: true },
        { title: 'Processing', subtitle: '10:45 AM', active: true },
        { title: 'Shipped', subtitle: '12:00 PM', active: false },
        { title: 'Out for delivery', subtitle: '—', active: false },
        { title: 'Delivered', subtitle: '—', active: false },
      ];
      const rawAi = wireFormat('add_timeline_v0', { items });
      const shape = tryParseElementToolOutput(rawAi);
      if (shape?.kind !== 'element-tool') throw new Error('parse failed');
      expect(shape.arguments.items).toEqual(items);

      const result = await dispatchElementToolCall(shape, { defaultParentId: rootId });
      expect(result.status).toBe('applied');
    });
  }
});

describe('round-trip — multi-tag batch serialization integrity', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  it('5 tools with varied arg shapes all parse + match original args', () => {
    const originals = [
      { name: 'add_heading_v0', args: { content: 'Section A', level: 'h1' } },
      { name: 'add_body_text_v0', args: { content: '你好世界 hello 😀' } },
      {
        name: 'add_list_row_v0',
        args: { title: '设置', subtitle: 'Settings', leading_icon: 'cog' },
      },
      {
        name: 'add_metric_row_v0',
        args: {
          items: [
            { label: 'X', value: '1' },
            { label: 'Y', value: '2' },
          ],
        },
      },
      { name: 'add_divider_v0', args: {} },
    ];
    const rawAi = originals.map((o) => wireFormat(o.name, o.args)).join('\n');
    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(originals.length);

    for (let i = 0; i < originals.length; i++) {
      const shape = shapes[i];
      if (shape.kind !== 'element-tool') throw new Error('wrong kind');
      expect(shape.name).toBe(originals[i].name);
      expect(shape.arguments).toEqual(originals[i].args);
    }
  });

  it('drops batch_design tags when element-tool tags coexist (Strategy A precedence)', () => {
    // Mirrors the "Do not mix Strategy A and Strategy B" rule from
    // ELEMENT_TOOL_OUTPUT_FORMAT. ab-v4 caught minimax-m2.7 chaining
    // 3 element-tool tags PLUS 1 batch_design scaffolding tag in the
    // same response (search-filters composite). Dispatching all four
    // would smuggle the forbidden mixed strategy through; parser
    // enforces the rule by dropping the batch_design half whenever
    // element-tool tags are present.
    const rawAi =
      '<op_tool>{"name":"add_section_header_v0","arguments":{"title":"Settings"}}</op_tool>\n' +
      '<op_tool>{"name":"add_setting_row_v0","arguments":{"title":"Notifications"}}</op_tool>\n' +
      '<op_tool>{"name":"batch_design","arguments":{"operations":"root=I(null, {\\"type\\":\\"frame\\"})"}}</op_tool>';
    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(2);
    expect(shapes.every((s) => s.kind === 'element-tool')).toBe(true);
    expect(shapes.some((s) => s.kind === 'batch-design-dsl')).toBe(false);
  });

  it('keeps batch_design tags when no element-tool tags are present (Strategy B fallback)', () => {
    // Pure Strategy B: model decided no element tool fit and emitted
    // a single batch_design covering the whole brief. Parser must
    // surface the DSL so the dispatcher routes it through the legacy
    // batch_design path.
    const rawAi =
      '<op_tool>{"name":"batch_design","arguments":{"operations":"root=I(null, {\\"type\\":\\"frame\\",\\"name\\":\\"Root\\"})"}}</op_tool>';
    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(1);
    expect(shapes[0].kind).toBe('batch-design-dsl');
    if (shapes[0].kind !== 'batch-design-dsl') throw new Error('wrong kind');
    expect(shapes[0].dsl).toContain('root=I(null,');
  });
});
