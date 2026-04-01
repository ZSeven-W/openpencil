import type { HistoryEntry, Phase } from '../engine/types';

export type { HistoryEntry };

let idCounter = 0;

export function createHistoryEntry(params: {
  documentPath: string;
  prompt: string;
  phase: Phase;
  skillsUsed: string[];
  nodeCount: number;
  sectionTypes: string[];
  validationScore?: number;
  validationRounds?: number;
  headingFont?: string;
  palette?: string;
  creativeVariant?: string;
}): HistoryEntry {
  return {
    id: `gen-${Date.now()}-${++idCounter}`,
    timestamp: new Date().toISOString(),
    documentPath: params.documentPath,
    input: {
      prompt: params.prompt,
      phase: params.phase,
      skillsUsed: params.skillsUsed,
    },
    output: {
      nodeCount: params.nodeCount,
      sectionTypes: params.sectionTypes,
      validationScore: params.validationScore,
      validationRounds: params.validationRounds,
      headingFont: params.headingFont,
      palette: params.palette,
      creativeVariant: params.creativeVariant,
    },
  };
}

export function updateFeedback(
  entry: HistoryEntry,
  feedback: NonNullable<HistoryEntry['feedback']>,
): HistoryEntry {
  return { ...entry, feedback };
}

export function getRecentEntries(
  entries: HistoryEntry[],
  limit: number,
  documentPath?: string,
): HistoryEntry[] {
  const filtered = documentPath ? entries.filter((e) => e.documentPath === documentPath) : entries;
  return filtered.slice(-limit);
}

export function historyToPromptString(entries: HistoryEntry[]): string {
  if (!entries.length) return '';
  const lines = ['## Recent Generation History'];
  for (const entry of entries) {
    const score = entry.output.validationScore ? ` (score: ${entry.output.validationScore})` : '';
    const feedback = entry.feedback ? ` [${entry.feedback}]` : '';
    lines.push(`- "${entry.input.prompt}" → ${entry.output.nodeCount} nodes${score}${feedback}`);
  }
  return lines.join('\n');
}
