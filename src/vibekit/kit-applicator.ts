/**
 * Applies a Vibe Kit to the current document.
 *
 * Uses existing document-store CRUD in a history batch so the entire
 * kit swap is a single undo entry.
 */

import type { VibeKit } from '@/types/vibekit'
import type { VariableDefinition } from '@/types/variables'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { useVibeKitStore } from '@/stores/vibekit-store'
import { VIBE_KIT_SCHEMA } from './schema'

/**
 * Apply a Vibe Kit to the current document.
 *
 * 1. Wraps all mutations in a history batch (single undo entry)
 * 2. Sets themes from the kit
 * 3. Sets all variables from the kit, filling missing ones with schema fallbacks
 * 4. Updates the active kit in the vibekit store
 *
 * Canvas re-renders automatically — use-canvas-sync detects variable changes.
 */
export function applyKit(kit: VibeKit): void {
  const doc = useDocumentStore.getState().document
  const { startBatch, endBatch } = useHistoryStore.getState()
  const { setVariable, setThemes } = useDocumentStore.getState()

  startBatch(doc)

  // Apply themes if the kit defines them
  if (kit.themes) {
    setThemes(kit.themes)
  }

  // Apply all kit variables
  for (const [name, def] of Object.entries(kit.variables)) {
    setVariable(name, def)
  }

  // Fill missing schema variables with fallbacks
  for (const [name, entry] of Object.entries(VIBE_KIT_SCHEMA)) {
    if (!(name in kit.variables)) {
      const fallbackDef: VariableDefinition = {
        type: entry.type,
        value: entry.fallback,
      }
      setVariable(name, fallbackDef)
    }
  }

  endBatch()

  // Track the active kit
  useVibeKitStore.getState().setActiveKit(kit.id)
}

/**
 * Build a VibeKit from the current document's variables.
 * Useful for exporting the current state as a reusable kit.
 */
export function extractKitFromDocument(name: string): VibeKit {
  const doc = useDocumentStore.getState().document
  const variables = doc.variables ?? {}
  const themes = doc.themes ?? {}

  // Only include variables that are part of the schema
  const kitVariables: Record<string, VariableDefinition> = {}
  for (const varName of Object.keys(VIBE_KIT_SCHEMA)) {
    if (varName in variables) {
      kitVariables[varName] = variables[varName]
    }
  }

  return {
    id: `kit-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    name,
    version: '1.0.0',
    variables: kitVariables,
    themes,
    assets: {},
    metadata: {
      createdAt: new Date().toISOString(),
      generatedBy: 'manual',
    },
  }
}
