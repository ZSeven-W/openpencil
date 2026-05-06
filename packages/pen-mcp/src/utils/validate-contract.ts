import type { ChunkResult, ContractValidationResult } from '@zseven-w/pen-types';

export function validateContract(result: ChunkResult): ContractValidationResult {
  const issues: string[] = [];
  const { contract, code } = result;

  // 1. componentName 必须是有效的 PascalCase 标识符（如果提供）
  if (contract.componentName && !/^[A-Z][a-zA-Z0-9]*$/.test(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" is not a valid PascalCase identifier`);
  }

  // 2. componentName 应出现在代码中（跳过名称隐式的 SFC 框架） Svelte/Vue SFC 可能有 <script>、<template> 或只有
  // <style> 以及 HTML
  const isSFC = code.includes('<script') || code.includes('<template') || code.includes('<style');
  if (contract.componentName && !isSFC && !code.includes(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" not found in generated code`);
  }

  return { valid: issues.length === 0, issues };
}
