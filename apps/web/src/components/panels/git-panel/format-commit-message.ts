// apps/web/src/components/panels/git-panel/format-commit-message.ts
//
// Parser 用于自动保存提交消息。 Phase 4c 提供最小的
// "auto: HH:MM" format from the autosave subscriber. Phase 6 将
// extend this to include diff summary suffixes once computeDiff is
// 有线通过。

export interface ParsedAutosaveMessage {
  /** The HH:MM timestamp string from the message. */
  time: string;
  /** Optional diff 摘要后缀 (Phase 6)。 */
  summary: string | null;
}

/**
 * Parse 自动保存提交消息。 Returns 结构化对象，或 null
 * if the message doesn't match the autosave format.
 *
 * Accepted 格式：
 * "auto: HH:MM" (Phase 4c baseline)
 * "auto: HH:MM — n frames, m nodes modified" (Phase 6 with diff suffix)
 */
export function parseAutosaveMessage(message: string): ParsedAutosaveMessage | null {
  const match = message.match(/^auto:\s*(\d{2}:\d{2})(?:\s*—\s*(.+))?$/);
  if (!match) return null;
  return {
    time: match[1],
    summary: match[2] ?? null,
  };
}
