import type { PenNode } from '@zseven-w/pen-types'

// === Canonical framework type ===

export type Framework = 'react' | 'vue' | 'svelte' | 'html' | 'flutter' | 'swiftui' | 'compose' | 'react-native'

export const FRAMEWORKS: Framework[] = ['react', 'vue', 'svelte', 'html', 'flutter', 'swiftui', 'compose', 'react-native']

// === Step 1 output: AI planner returns this (no node data, minimal tokens) ===

export interface PlannedChunk {
  id: string
  name: string
  nodeIds: string[]
  role: string
  suggestedComponentName: string
  dependencies: string[]
  exposedSlots?: string[]
}

export interface CodePlanFromAI {
  chunks: PlannedChunk[]
  sharedStyles: { name: string; description: string }[]
  rootLayout: { direction: string; gap: number; responsive: boolean }
}

// === Runtime: hydrated with node data + execution order ===

export interface ExecutableChunk extends PlannedChunk {
  nodes: PenNode[]
  order: number
  depContracts: ChunkContract[]
}

export interface CodeExecutionPlan {
  chunks: ExecutableChunk[]
  sharedStyles: { name: string; description: string }[]
  rootLayout: { direction: string; gap: number; responsive: boolean }
}

// === Chunk contract: structured metadata output from each chunk ===

export interface ChunkContract {
  chunkId: string
  componentName: string
  exportedProps: PropDef[]
  slots: SlotDef[]
  cssClasses: string[]
  cssVariables: string[]
  imports: ImportDef[]
}

export interface PropDef {
  name: string
  type: string
  required: boolean
}

export interface SlotDef {
  name: string
  description: string
}

export interface ImportDef {
  source: string
  specifiers: string[]
}

// === Chunk generation output ===

export interface ChunkResult {
  chunkId: string
  code: string
  contract: ChunkContract
}

// === Progress events ===

export type ChunkStatus = 'pending' | 'running' | 'done' | 'degraded' | 'failed' | 'skipped'

export type CodeGenProgress =
  | { step: 'planning'; status: 'running' | 'done' | 'failed'; plan?: CodePlanFromAI; error?: string }
  | { step: 'chunk'; chunkId: string; name: string; status: ChunkStatus; result?: ChunkResult; error?: string }
  | { step: 'assembly'; status: 'running' | 'done' | 'failed'; error?: string }
  | { step: 'complete'; finalCode: string; degraded: boolean }
  | { step: 'error'; message: string; chunkId?: string }

// === Contract validation ===

export interface ContractValidationResult {
  valid: boolean
  issues: string[]
}

export function validateContract(result: ChunkResult): ContractValidationResult {
  const issues: string[] = []
  const { contract, code } = result

  // 1. componentName must be a valid PascalCase identifier (if provided)
  if (contract.componentName && !/^[A-Z][a-zA-Z0-9]*$/.test(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" is not a valid PascalCase identifier`)
  }

  // 2. componentName should appear in code (skip for SFC frameworks where name is implicit)
  // Svelte/Vue SFC may have <script>, <template>, or just <style> with HTML
  const isSFC = code.includes('<script') || code.includes('<template') || code.includes('<style')
  if (contract.componentName && !isSFC && !code.includes(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" not found in generated code`)
  }

  return { valid: issues.length === 0, issues }
}
