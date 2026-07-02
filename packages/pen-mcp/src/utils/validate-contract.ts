import type { ChunkResult, ContractValidationResult } from '@zseven-w/pen-types';

export function validateContract(result: ChunkResult): ContractValidationResult {
  const issues: string[] = [];
  const { contract, code } = result;

  // 1. componentName must be a valid PascalCase identifier (if provided)
  if (contract.componentName && !/^[A-Z][a-zA-Z0-9]*$/.test(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" is not a valid PascalCase identifier`);
  }

  // 2. componentName should appear in code (skip for SFC frameworks where name is implicit)
  // Svelte/Vue SFC may have <script>, <template>, or just <style> with HTML
  const isSFC = code.includes('<script') || code.includes('<template') || code.includes('<style');
  if (contract.componentName && !isSFC && !code.includes(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" not found in generated code`);
  }

  return { valid: issues.length === 0, issues };
}
