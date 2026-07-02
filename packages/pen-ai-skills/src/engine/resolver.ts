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
 * Match a single keyword against a (lowercased) user message.
 *
 * For ASCII keywords this uses **word-boundary regex matching** so that
 * a keyword like `form` doesn't false-trigger on words that merely
 * CONTAIN it as a substring (`platform`, `information`, `perform`,
 * `format`, `transform`). The previous implementation used naive
 * `String.includes()` and any short keyword in the registry would
 * match every prompt that mentioned an unrelated word with the same
 * letters.
 *
 * For non-ASCII keywords (CJK, etc.) word boundaries don't apply —
 * Chinese characters have no whitespace separators, so `\b` would
 * never match. We fall back to the original substring approach for
 * any keyword that contains a non-ASCII character.
 *
 * Multi-word ASCII keywords like `sign up` and `react-native` are
 * supported: spaces and hyphens are non-word characters in regex, so
 * `\bsign up\b` and `\breact-native\b` both function as expected.
 */
function matchKeyword(msg: string, kw: string): boolean {
  // Non-ASCII path: keep the original substring behavior so CJK
  // keywords like `表单` / `登录` still match.
  // eslint-disable-next-line no-control-regex
  if (!/^[\x00-\x7f]+$/.test(kw)) {
    return msg.includes(kw);
  }
  // Empty / whitespace-only keyword: never match.
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
