import type { DesignContext } from '../engine/types';

export type { DesignContext };

export function createDesignContext(documentPath: string | null): DesignContext {
  const now = new Date().toISOString();
  return {
    documentPath,
    createdAt: now,
    updatedAt: now,
    designSystem: {},
    structure: {},
    preferences: {},
  };
}

interface OrchestratorPlanLike {
  styleGuide?: {
    palette?: string[];
    fonts?: string[];
    aesthetic?: string;
  };
  subtasks?: Array<{ label: string }>;
}

export function extractDesignContext(
  existing: DesignContext,
  plan: OrchestratorPlanLike,
): DesignContext {
  return {
    ...existing,
    updatedAt: new Date().toISOString(),
    designSystem: {
      ...existing.designSystem,
      palette: plan.styleGuide?.palette ?? existing.designSystem.palette,
      aesthetic: plan.styleGuide?.aesthetic ?? existing.designSystem.aesthetic,
      typography: plan.styleGuide?.fonts?.join(', ') ?? existing.designSystem.typography,
    },
    structure: {
      ...existing.structure,
      sections: plan.subtasks?.map((s) => s.label) ?? existing.structure.sections,
    },
  };
}

export function mergePreference(
  ctx: DesignContext,
  override: { what: string; from: string; to: string },
): DesignContext {
  const overrides = [...(ctx.preferences.overrides ?? [])];
  const existingIdx = overrides.findIndex((o) => o.what === override.what);
  if (existingIdx >= 0) {
    overrides[existingIdx] = override;
  } else {
    overrides.push(override);
  }
  return {
    ...ctx,
    updatedAt: new Date().toISOString(),
    preferences: { ...ctx.preferences, overrides },
  };
}

export function contextToPromptString(ctx: DesignContext): string {
  const parts: string[] = ['## Document Design Context'];
  if (ctx.designSystem.aesthetic) parts.push(`Aesthetic: ${ctx.designSystem.aesthetic}`);
  if (ctx.designSystem.palette?.length)
    parts.push(`Palette: ${ctx.designSystem.palette.join(', ')}`);
  if (ctx.designSystem.typography) parts.push(`Typography: ${ctx.designSystem.typography}`);
  if (ctx.structure.pageType) parts.push(`Page Type: ${ctx.structure.pageType}`);
  if (ctx.preferences.overrides?.length) {
    parts.push('User Preferences:');
    for (const o of ctx.preferences.overrides) {
      parts.push(`  - ${o.what}: changed from ${o.from} to ${o.to}`);
    }
  }
  return parts.join('\n');
}
