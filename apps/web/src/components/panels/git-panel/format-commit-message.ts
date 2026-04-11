// apps/web/src/components/panels/git-panel/format-commit-message.ts
//
// Parser for autosave commit messages. Phase 4c ships the minimal
// "auto: HH:MM" format from the autosave subscriber. Phase 6 will
// extend this to include diff summary suffixes once computeDiff is
// wired through.

export interface ParsedAutosaveMessage {
  /** The HH:MM timestamp string from the message. */
  time: string;
  /** Optional diff summary suffix (Phase 6). */
  summary: string | null;
}

/**
 * Parse an autosave commit message. Returns a structured object, or null
 * if the message doesn't match the autosave format.
 *
 * Accepted formats:
 *   "auto: HH:MM"                              (Phase 4c baseline)
 *   "auto: HH:MM — N frames, M nodes modified" (Phase 6 with diff suffix)
 */
export function parseAutosaveMessage(message: string): ParsedAutosaveMessage | null {
  const match = message.match(/^auto:\s*(\d{2}:\d{2})(?:\s*—\s*(.+))?$/);
  if (!match) return null;
  return {
    time: match[1],
    summary: match[2] ?? null,
  };
}
