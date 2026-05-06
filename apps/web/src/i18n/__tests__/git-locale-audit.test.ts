import { describe, it, expect } from 'vitest';
import en from '../locales/en';

// 此组中的 Keys 绕过下面的审核规则。 Add 此处的条目 ONLY
// 代表真正的未来路线图副本的字符串（即该功能尚未发布，“Phase n”文本是有意的）。 Do NOT 使用白名单来掩盖过时的副本 -
// 而是修复副本。
const GIT_COPY_AUDIT_ALLOWLIST = new Set<string>();

// ── 助手──────────────────────────────────────────────────────────────────

type Violation = { key: string; value: string; reason: string };

const gitEntries = Object.entries(en).filter(([key]) => key.startsWith('git.'));

// Rule 1：对于已完成的阶段，没有任何值提及“进入 Phase <数字>”。
function ruleComingInPhase(entries: [string, string][]): Violation[] {
  return entries
    .filter(([key, value]) => {
      if (GIT_COPY_AUDIT_ALLOWLIST.has(key)) return false;
      return /coming in Phase \d/i.test(value);
    })
    .map(([key, value]) => ({ key, value, reason: 'value contains "coming in Phase N"' }));
}

// Rule 2：git.placeholder.* 下没有键（这些是死的 UI 脚手架标签）。
function rulePlaceholderKey(entries: [string, string][]): Violation[] {
  return entries
    .filter(([key]) => {
      if (GIT_COPY_AUDIT_ALLOWLIST.has(key)) return false;
      return key.startsWith('git.placeholder.');
    })
    .map(([key, value]) => ({
      key,
      value,
      reason: 'key matches git.placeholder.* (dead scaffolding — delete the key)',
    }));
}

// Rule 3：没有包含单词“占位符”的值（不区分大小写）。 UI 字符串应该描述真实的行为，而不是标记未来的插槽。
function rulePlaceholderValue(entries: [string, string][]): Violation[] {
  return entries
    .filter(([key, value]) => {
      if (GIT_COPY_AUDIT_ALLOWLIST.has(key)) return false;
      return /placeholder/i.test(value);
    })
    .map(([key, value]) => ({ key, value, reason: 'value contains "placeholder"' }));
}

// ── 测试──────────────────────────────────────────────────────────────────────

describe('Git locale audit (en.ts)', () => {
  it('no git.* value says "coming in Phase N"', () => {
    const violations = ruleComingInPhase(gitEntries);
    expect(violations).toEqual([]);
  });

  it('no git.placeholder.* key exists (dead keys must be deleted)', () => {
    const violations = rulePlaceholderKey(gitEntries);
    expect(violations).toEqual([]);
  });

  it('no git.* value contains the word "placeholder"', () => {
    const violations = rulePlaceholderValue(gitEntries);
    expect(violations).toEqual([]);
  });
});
