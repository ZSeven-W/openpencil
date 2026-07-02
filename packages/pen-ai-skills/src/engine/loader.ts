import type { SkillRegistryEntry, Phase } from './types';
import { skillRegistry as generatedRegistry } from '../_generated/skill-registry';

let registry: SkillRegistryEntry[] = generatedRegistry ?? [];

export function getSkillRegistry(): SkillRegistryEntry[] {
  return registry;
}

export function getSkillsByPhase(phase: Phase): SkillRegistryEntry[] {
  return registry.filter((entry) => entry.meta.phase.includes(phase));
}

export function getSkillByName(name: string): SkillRegistryEntry | undefined {
  return registry.find((entry) => entry.meta.name === name);
}

/** For testing: inject a custom registry */
export function setSkillRegistry(entries: SkillRegistryEntry[]): void {
  registry = entries;
}
