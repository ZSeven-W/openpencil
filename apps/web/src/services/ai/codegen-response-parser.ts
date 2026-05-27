import type { ChunkContract, ChunkResult, ContractValidationResult } from '@zseven-w/pen-types';

export function validateContract(result: ChunkResult): ContractValidationResult {
  const issues: string[] = [];
  const { contract, code } = result;
  if (contract.componentName && !/^[A-Z][a-zA-Z0-9]*$/.test(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" is not a valid PascalCase identifier`);
  }
  const isSFC = code.includes('<script') || code.includes('<template') || code.includes('<style');
  if (contract.componentName && !isSFC && !code.includes(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" not found in generated code`);
  }
  return { valid: issues.length === 0, issues };
}

export function parseChunkResponse(response: string, chunkId: string): ChunkResult {
  const separator = '---CONTRACT---';
  const sepIdx = response.indexOf(separator);
  if (sepIdx !== -1) {
    const code = cleanCode(response.slice(0, sepIdx));
    const contractStr = response.slice(sepIdx + separator.length).trim();
    const contract = tryParseContract(contractStr, chunkId);
    if (contract) return { chunkId, code, contract };
  }

  const contractJsonMatch = response.match(/```json\s*\n([\s\S]*?)\n\s*```/);
  if (contractJsonMatch) {
    const jsonStr = contractJsonMatch[1].trim();
    if (jsonStr.includes('"componentName"')) {
      const contract = tryParseContract(jsonStr, chunkId);
      if (contract) {
        const jsonBlockStart = response.indexOf(contractJsonMatch[0]);
        const code = cleanCode(response.slice(0, jsonBlockStart));
        return { chunkId, code, contract };
      }
    }
  }

  const lastJsonMatch = response.match(/(\{[^{}]*"componentName"[^{}]*\})\s*$/);
  if (lastJsonMatch) {
    const contract = tryParseContract(lastJsonMatch[1], chunkId);
    if (contract) {
      const jsonStart = response.lastIndexOf(lastJsonMatch[1]);
      const code = cleanCode(response.slice(0, jsonStart));
      return { chunkId, code, contract };
    }
  }

  const code = cleanCode(response);
  const inferredContract = inferContractFromCode(code, chunkId);
  return { chunkId, code, contract: inferredContract };
}

function tryParseContract(str: string, chunkId: string): ChunkContract | null {
  try {
    const cleaned = str
      .replace(/^```\w*\n?/gm, '')
      .replace(/```\s*$/gm, '')
      .trim();
    const parsed = JSON.parse(cleaned) as ChunkContract;
    if (parsed.componentName) {
      parsed.chunkId = chunkId;
      parsed.exportedProps = parsed.exportedProps ?? [];
      parsed.slots = parsed.slots ?? [];
      parsed.cssClasses = parsed.cssClasses ?? [];
      parsed.cssVariables = parsed.cssVariables ?? [];
      parsed.imports = parsed.imports ?? [];
      parsed.outputFiles = Array.isArray(parsed.outputFiles)
        ? parsed.outputFiles.filter((file): file is string => typeof file === 'string')
        : undefined;
      return parsed;
    }
  } catch {
    /* invalid JSON */
  }
  return null;
}

function inferContractFromCode(code: string, chunkId: string): ChunkContract {
  const isSFC = code.includes('<script') || code.includes('<template') || code.includes('<style');
  const exportMatch =
    code.match(/export\s+default\s+function\s+(\w+)/) ??
    code.match(/export\s+function\s+([A-Z]\w*)/) ??
    (!isSFC ? code.match(/export\s+default\s+class\s+(\w+)/) : null) ??
    code.match(/fun\s+([A-Z]\w*)\s*\(/) ??
    code.match(/struct\s+(\w+)\s*:\s*View/) ??
    code.match(/class\s+(\w+)\s+extends/);
  const componentName = exportMatch?.[1] ?? '';
  const importMatches = [...code.matchAll(/import\s+.*?from\s+['"](.+?)['"]/g)];

  return {
    chunkId,
    componentName,
    exportedProps: [],
    slots: [],
    cssClasses: [],
    cssVariables: [],
    imports: importMatches.map((match) => ({ source: match[1], specifiers: [] })),
    outputFiles: [],
  };
}

export function cleanCode(raw: string): string {
  return raw
    .replace(/^```\w*\n?/gm, '')
    .replace(/```\s*$/gm, '')
    .trim();
}
