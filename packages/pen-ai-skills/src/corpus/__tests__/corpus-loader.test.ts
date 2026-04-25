import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, writeFileSync, mkdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { loadCorpus } from '../corpus-loader';

const REPO_CORPUS_DIR = join(__dirname, '..', '..', '..', 'corpus', 'ab-v0');
const REPO_CORPUS_V1_DIR = join(__dirname, '..', '..', '..', 'corpus', 'ab-v1');

let tmp: string;

beforeEach(() => {
  tmp = mkdtempSync(join(tmpdir(), 'corpus-loader-'));
});
afterEach(() => {
  rmSync(tmp, { recursive: true, force: true });
});

describe('loadCorpus — real corpus', () => {
  it('loads all 24 yaml files (stable order)', () => {
    const prompts = loadCorpus(REPO_CORPUS_DIR);
    expect(prompts).toHaveLength(24);
    const ids = prompts.map((p) => p.id);
    expect(new Set(ids).size).toBe(24);
  });

  it('24 prompts split 8 per category', () => {
    const prompts = loadCorpus(REPO_CORPUS_DIR);
    const byCat = new Map<string, number>();
    for (const p of prompts) byCat.set(p.category, (byCat.get(p.category) ?? 0) + 1);
    expect(byCat.get('mobile')).toBe(8);
    expect(byCat.get('dashboard')).toBe(8);
    expect(byCat.get('landing')).toBe(8);
  });

  it('each category has 4 obvious + 4 optional', () => {
    const prompts = loadCorpus(REPO_CORPUS_DIR);
    for (const cat of ['mobile', 'dashboard', 'landing'] as const) {
      const forCat = prompts.filter((p) => p.category === cat);
      const obvious = forCat.filter((p) => p.difficulty === 'obvious').length;
      const optional = forCat.filter((p) => p.difficulty === 'optional').length;
      expect(obvious, `${cat} obvious count`).toBe(4);
      expect(optional, `${cat} optional count`).toBe(4);
    }
  });

  it('every obvious prompt has expected_tool_if_any (loader contract)', () => {
    const prompts = loadCorpus(REPO_CORPUS_DIR);
    const obviousMissingTool = prompts
      .filter((p) => p.difficulty === 'obvious' && !p.expected_tool_if_any)
      .map((p) => p.id);
    expect(obviousMissingTool).toEqual([]);
  });

  it('expected_tool_if_any references a real add_*_v0 name shape', () => {
    const prompts = loadCorpus(REPO_CORPUS_DIR);
    for (const p of prompts) {
      if (p.expected_tool_if_any) {
        expect(p.expected_tool_if_any).toMatch(/^add_[a-z_]+_v0$/);
      }
    }
  });
});

describe('loadCorpus — v1 supplemental corpus (new tools)', () => {
  // v1 covers tools added after the v0 freeze, in chronological batches:
  // 2026-04-22 (12 tools) / 2026-04-24 (5 tools, 4 v0 + first v1) /
  // 2026-04-25 (9 tools, 7 v0 + 2 v1) / 2026-04-27 (1 tool, sidebar nav).
  // All obvious — one prompt per tool so A/B runs can measure routing +
  // legality on the new surface without re-running the v0 corpus. See
  // `corpus/ab-v1/README.md`.
  it('loads 27 prompts, all obvious, one per new tool', () => {
    const prompts = loadCorpus(REPO_CORPUS_V1_DIR);
    expect(prompts).toHaveLength(27);
    expect(new Set(prompts.map((p) => p.id)).size).toBe(27);
    for (const p of prompts) {
      expect(p.difficulty).toBe('obvious');
      expect(p.expected_tool_if_any).toMatch(/^add_[a-z_]+_v\d+$/);
    }
  });

  it('covers 2026-04-22 (12) + 2026-04-24 (5) + 2026-04-25 (9) + 2026-04-27 (1) batches', () => {
    const prompts = loadCorpus(REPO_CORPUS_V1_DIR);
    const tools = new Set(prompts.map((p) => p.expected_tool_if_any));
    expect(tools).toEqual(
      new Set([
        // 2026-04-22 batch (12 tools)
        'add_textarea_v0',
        'add_skeleton_v0',
        'add_select_v0',
        'add_chart_line_v0',
        'add_chart_pie_v0',
        'add_image_placeholder_v0',
        'add_comment_v0',
        'add_modal_shell_v0',
        'add_status_badge_v0',
        'add_tooltip_v0',
        'add_metric_comparison_v0',
        'add_notification_row_v0',
        // 2026-04-24 batch (5 tools) — 4 v0 + first v1
        'add_upload_dropzone_v0',
        'add_otp_input_v0',
        'add_attachment_row_v0',
        'add_chat_bubble_v0',
        'add_modal_shell_v1',
        // 2026-04-25 batch (9 tools) — 7 new v0 + 2 new v1
        'add_social_login_row_v0',
        'add_pricing_card_v0',
        'add_stat_card_v0',
        'add_range_slider_v0',
        'add_phone_input_v0',
        'add_input_with_action_v0',
        'add_cookie_banner_v0',
        'add_toast_v1',
        'add_empty_chart_v1',
        // 2026-04-27 batch (1 tool) — desktop sidebar nav rail
        'add_sidebar_nav_v0',
      ]),
    );
  });

  it('every prompt has must_contain_roles anchoring to its expected tool', () => {
    // Structural gate: every v1 prompt names at least one role in
    // expected.must_contain_roles so the scorer can check the LLM's
    // output actually emitted the shape. Empty expected would let a
    // blank design pass.
    const prompts = loadCorpus(REPO_CORPUS_V1_DIR);
    for (const p of prompts) {
      expect(p.expected.must_contain_roles, `${p.id} missing must_contain_roles`).toBeDefined();
      expect((p.expected.must_contain_roles ?? []).length).toBeGreaterThan(0);
    }
  });
});

describe('loadCorpus — validation errors', () => {
  function writeYaml(name: string, body: string): void {
    writeFileSync(join(tmp, name), body, 'utf-8');
  }

  it('rejects missing id', () => {
    writeYaml(
      'bad.yaml',
      `category: mobile
difficulty: optional
prompt: hi
expected: {}
`,
    );
    expect(() => loadCorpus(tmp)).toThrow(/field "id" must be a non-empty string/);
  });

  it('rejects invalid category', () => {
    writeYaml(
      'bad.yaml',
      `id: x
category: web
difficulty: optional
prompt: hi
expected: {}
`,
    );
    expect(() => loadCorpus(tmp)).toThrow(/invalid category "web"/);
  });

  it('rejects invalid difficulty', () => {
    writeYaml(
      'bad.yaml',
      `id: x
category: mobile
difficulty: hard
prompt: hi
expected: {}
`,
    );
    expect(() => loadCorpus(tmp)).toThrow(/invalid difficulty "hard"/);
  });

  it('rejects obvious without expected_tool_if_any', () => {
    writeYaml(
      'bad.yaml',
      `id: x
category: mobile
difficulty: obvious
prompt: hi
expected: {}
`,
    );
    expect(() => loadCorpus(tmp)).toThrow(/obvious requires expected_tool_if_any/);
  });

  it('rejects duplicate ids across files', () => {
    writeYaml(
      'a.yaml',
      `id: same
category: mobile
difficulty: optional
prompt: a
expected: {}
`,
    );
    writeYaml(
      'b.yaml',
      `id: same
category: mobile
difficulty: optional
prompt: b
expected: {}
`,
    );
    expect(() => loadCorpus(tmp)).toThrow(/duplicate prompt id "same"/);
  });

  it('rejects malformed min_roles values', () => {
    writeYaml(
      'bad.yaml',
      `id: x
category: mobile
difficulty: optional
prompt: hi
expected:
  min_roles:
    card: negative
`,
    );
    expect(() => loadCorpus(tmp)).toThrow(/min_roles.card must be a non-negative number/);
  });

  it('accepts empty expected object', () => {
    writeYaml(
      'ok.yaml',
      `id: x
category: mobile
difficulty: optional
prompt: hi
expected: {}
`,
    );
    const out = loadCorpus(tmp);
    expect(out).toHaveLength(1);
    expect(out[0].expected).toEqual({});
  });

  it('ignores non-yaml files in directory', () => {
    writeYaml('README.md', '# hi');
    writeYaml(
      'ok.yaml',
      `id: x
category: mobile
difficulty: optional
prompt: hi
expected: {}
`,
    );
    const out = loadCorpus(tmp);
    expect(out).toHaveLength(1);
  });

  it('throws on nonexistent directory', () => {
    const missing = join(tmp, 'does-not-exist');
    expect(() => loadCorpus(missing)).toThrow(/cannot read directory/);
  });

  it('yaml files sorted by filename (deterministic)', () => {
    mkdirSync(tmp, { recursive: true });
    writeYaml(
      'z.yaml',
      `id: z-id
category: mobile
difficulty: optional
prompt: z
expected: {}
`,
    );
    writeYaml(
      'a.yaml',
      `id: a-id
category: mobile
difficulty: optional
prompt: a
expected: {}
`,
    );
    const out = loadCorpus(tmp);
    expect(out.map((p) => p.id)).toEqual(['a-id', 'z-id']);
  });
});
