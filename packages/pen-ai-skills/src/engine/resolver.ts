import type { SkillTrigger, SkillRegistryEntry } from './types';

export function matchTrigger(
  trigger: SkillTrigger,
  userMessage: string,
  flags: Record<string, boolean>,
): boolean {
  if (trigger === null) return true;

  if ('keywords' in trigger) {
    const msg = userMessage.toLowerCase();
    return trigger.keywords.some((kw) => matchKeyword(msg, kw.toLowerCase()));
  }

  if ('flags' in trigger) {
    return trigger.flags.every((flag) => flags[flag] === true);
  }

  return false;
}

/**
 * Match 针对（小写）
 *
 * 用户消息的单个关键字。 For ASCII 关键字，此关键字使用 **字边界正则表达式匹配**，因此像 `form`
 * 这样的关键字不会错误触发
 * 仅将 CONTAIN 作为子字符串的单词（`platform`、`information`、`perform`、`format`、`tra
 * nsform`）。 The 以前的实现使用了朴素的 `String.includes()`
 * ，并且注册表中的任何短关键字都会匹配提到具有相同字母的不相关单词的每个提示。 For 非 ASCII 关键字（CJK
 * 等）字边界不适用 — Chinese 字符没有空格分隔符，因此 `\b` 永远不会匹配。对于包含非 ASCII
 * 字符的任何关键字，We 会回退到原始子字符串方法。支持 Multi-word ASCII 关键字，如 `sign up` 和
 *
 * `react-native`：空格和连字符是正则表达式中的非单词字符，因此 `\bsign up\b` 和
 * `\breact-nat
 * ive\b` 都按预期运
 * 行。
 *
 *
 *
 *
 */
function matchKeyword(msg: string, kw: string): boolean {
  // Non-ASCII 路径：保留原始子字符串行为，因此 CJK
  // ` 表单 ` / ` 登录 ` 等关键字仍然匹配。
// eslint-disable-next-line no-control-regex
  if (!/^[\x00-\x7f]+$/.test(kw)) {
    return msg.includes(kw);
  }
  // Empty / 仅空白关键字：从不匹配。
  if (kw.trim().length === 0) return false;
  const escaped = kw.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`\\b${escaped}\\b`, 'i');
  return re.test(msg);
}

export function filterByIntent(
  skills: SkillRegistryEntry[],
  userMessage: string,
  flags: Record<string, boolean>,
): SkillRegistryEntry[] {
  return skills
    .filter((skill) => matchTrigger(skill.meta.trigger, userMessage, flags))
    .sort((a, b) => a.meta.priority - b.meta.priority);
}

export function injectDynamicContent(
  content: string,
  dynamicContent?: Record<string, string>,
): string {
  if (!dynamicContent) return content;
  return content.replace(/\{\{(\w+)\}\}/g, (_match, key) => {
    if (key in dynamicContent) return dynamicContent[key];
    console.warn(`[pen-ai-skills] Missing dynamic content for placeholder: {{${key}}}`);
    return '';
  });
}
