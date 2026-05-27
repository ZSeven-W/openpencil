import type {
  CodegenQualityIssue,
  CodegenQualityReport,
  CodegenQualityStatus,
  Framework,
} from '@zseven-w/pen-types';
import type { SaveCodegenFileInput } from '@/types/cloud';
import { validateCodegenFiles } from './codegen-files';
import type { CodegenDesignIR } from './codegen-design-ir';
import { collectDesignIRText } from './codegen-design-ir';

const PLACEHOLDER_PATTERNS = [
  /\blorem ipsum\b/i,
  /\bplaceholder\b/i,
  /\btodo\b/i,
  /\bexample\.com\b/i,
  /在此处|占位|示例文本/i,
];

const DANGEROUS_PATTERNS = [
  /<script\b[^>]*>[\s\S]*?\b(eval|Function)\s*\(/i,
  /\bdocument\.write\s*\(/i,
  /\binnerHTML\s*=\s*[^=]/i,
  /\bjavascript:/i,
];

const VOID_HTML_TAGS = new Set([
  'area',
  'base',
  'br',
  'col',
  'embed',
  'hr',
  'img',
  'input',
  'link',
  'meta',
  'param',
  'source',
  'track',
  'wbr',
]);

function issue(input: {
  code: string;
  severity?: CodegenQualityIssue['severity'];
  message: string;
  filePath?: string;
}): CodegenQualityIssue {
  return {
    severity: input.severity ?? 'error',
    code: input.code,
    message: input.message,
    filePath: input.filePath,
  };
}

function allContent(files: SaveCodegenFileInput[]): string {
  return files.map((file) => file.content).join('\n\n');
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, ' ').trim().toLowerCase();
}

function splitTextSegments(value: string): string[] {
  const raw = value.trim();
  if (!raw) return [];
  const normalized = normalizeText(value);
  const parts = raw
    .split(/\s{2,}|[•·|/]+/)
    .map((part) => part.replace(/\s+/g, ' ').trim())
    .filter(Boolean);
  return parts.length > 1 ? parts : [normalized];
}

function isMeaningfulText(value: string): boolean {
  return normalizeText(value).length >= 2;
}

function hasDesignText(content: string, text: string): boolean {
  const normalizedContent = normalizeText(content);
  const normalizedText = normalizeText(text);
  if (!normalizedText) return true;
  if (normalizedContent.includes(normalizedText)) return true;

  const segments = splitTextSegments(text).filter(isMeaningfulText);
  if (segments.length <= 1) return false;
  return segments.every((segment) => normalizedContent.includes(normalizeText(segment)));
}

function missingDesignTexts(ir: CodegenDesignIR | undefined, content: string): string[] {
  if (!ir) return [];
  return collectDesignIRText(ir)
    .filter(isMeaningfulText)
    .filter((text) => !hasDesignText(content, text))
    .slice(0, 20);
}

function assetRefVariants(ref: string): string[] {
  const normalized = ref.trim().replace(/\\/g, '/');
  const withoutDot = normalized.replace(/^\.\//, '');
  const withDot = withoutDot.startsWith('assets/') ? `./${withoutDot}` : withoutDot;
  return [...new Set([normalized, withoutDot, withDot])].filter(Boolean);
}

function missingAssetRefs(ir: CodegenDesignIR | undefined, content: string): string[] {
  if (!ir) return [];
  const expected = new Set<string>();
  const visit = (nodes: typeof ir.nodes) => {
    for (const node of nodes) {
      for (const ref of node.assetRefs) expected.add(ref);
      if (node.children) visit(node.children);
    }
  };
  visit(ir.nodes);
  return [...expected].filter((ref) => !assetRefVariants(ref).some((variant) => content.includes(variant)));
}

function stripComments(value: string): string {
  return value
    .replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '')
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, '');
}

function findUnbalancedHtmlTag(content: string): string | null {
  const stack: string[] = [];
  const tagPattern = /<\/?([a-zA-Z][\w:-]*)(?:\s[^<>]*)?>/g;
  const stripped = stripComments(content);
  let match: RegExpExecArray | null;

  while ((match = tagPattern.exec(stripped))) {
    const raw = match[0];
    const tagName = match[1].toLowerCase();
    if (raw.startsWith('<!') || raw.startsWith('<?') || VOID_HTML_TAGS.has(tagName)) continue;
    if (raw.endsWith('/>')) continue;

    if (raw.startsWith('</')) {
      const last = stack.pop();
      if (last !== tagName) return tagName;
      continue;
    }
    stack.push(tagName);
  }

  return stack.pop() ?? null;
}

function getTagBlock(content: string, tagName: string): string | null {
  const pattern = new RegExp(`<${tagName}\\b[^>]*>([\\s\\S]*?)<\\/${tagName}>`, 'i');
  return pattern.exec(content)?.[1] ?? null;
}

function countOpeningTags(content: string, tagName: string): number {
  const pattern = new RegExp(`<${tagName}\\b[^>]*>`, 'gi');
  return content.match(pattern)?.length ?? 0;
}

function countClosingTags(content: string, tagName: string): number {
  const pattern = new RegExp(`</${tagName}>`, 'gi');
  return content.match(pattern)?.length ?? 0;
}

function hasTextOutsideTags(content: string): boolean {
  return content.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim().length > 0;
}

function validateHtmlStructure(file: SaveCodegenFileInput): CodegenQualityIssue[] {
  const issues: CodegenQualityIssue[] = [];
  const content = file.content;

  if (!/<head[\s>]/i.test(content)) {
    issues.push(
      issue({
        code: 'html_missing_head',
        message: 'HTML output is missing a <head> section.',
        filePath: file.path,
      }),
    );
  }
  if (!/<body[\s>]/i.test(content)) {
    issues.push(
      issue({
        code: 'html_missing_body',
        message: 'HTML output is missing a <body> section.',
        filePath: file.path,
      }),
    );
  }
  const unbalancedTag = findUnbalancedHtmlTag(content);
  if (unbalancedTag) {
    issues.push(
      issue({
        code: 'html_unbalanced_tag',
        message: `HTML output has an unbalanced <${unbalancedTag}> tag.`,
        filePath: file.path,
      }),
    );
  }
  const body = getTagBlock(content, 'body');
  if (body !== null && !hasTextOutsideTags(body) && !/<(img|svg|canvas|video|picture)\b/i.test(body)) {
    issues.push(
      issue({
        code: 'html_empty_body',
        message: 'HTML output has an empty <body> section.',
        filePath: file.path,
      }),
    );
  }

  return issues;
}

function validateVueSfcStructure(file: SaveCodegenFileInput): CodegenQualityIssue[] {
  const issues: CodegenQualityIssue[] = [];
  const content = file.content;

  for (const tagName of ['template', 'script', 'style']) {
    const openCount = countOpeningTags(content, tagName);
    const closeCount = countClosingTags(content, tagName);
    if (openCount !== closeCount) {
      issues.push(
        issue({
          code: 'vue_unbalanced_block',
          message: `Vue SFC has unbalanced <${tagName}> blocks.`,
          filePath: file.path,
        }),
      );
    }
  }

  const template = getTagBlock(content, 'template');
  if (template !== null && !hasTextOutsideTags(template) && !/<(img|svg|canvas|component)\b/i.test(template)) {
    issues.push(
      issue({
        code: 'vue_empty_template',
        message: 'Vue SFC has an empty <template> block.',
        filePath: file.path,
      }),
    );
  }

  return issues;
}

function validateFrameworkShape(framework: Framework, files: SaveCodegenFileInput[]): CodegenQualityIssue[] {
  const issues: CodegenQualityIssue[] = [];
  if (!['html', 'vue', 'uniapp'].includes(framework)) {
    return issues;
  }

  if (framework === 'html') {
    const html = files.find((file) => file.path.endsWith('.html')) ?? files[0];
    if (!html?.content.trim()) {
      issues.push(issue({ code: 'html_empty', message: 'HTML output is empty.' }));
    } else {
      if (!/<html[\s>]/i.test(html.content)) {
        issues.push(
          issue({
            code: 'html_missing_document',
            severity: 'warning',
            message: 'HTML output should include a complete document shell.',
            filePath: html.path,
          }),
        );
      }
      issues.push(...validateHtmlStructure(html));
      if (!/<style[\s>]/i.test(html.content) && !/\.css/.test(html.content)) {
        issues.push(
          issue({
            code: 'html_missing_styles',
            severity: 'warning',
            message: 'HTML output should include CSS for visual fidelity.',
            filePath: html.path,
          }),
        );
      }
    }
  }

  if (framework === 'vue') {
    const vueFiles = files.filter((file) => file.path.endsWith('.vue'));
    if (vueFiles.length === 0) {
      issues.push(issue({ code: 'vue_missing_sfc', message: 'Vue output is missing a .vue file.' }));
    }
    for (const file of vueFiles) {
      if (!/<template[\s>]/i.test(file.content)) {
        issues.push(
          issue({
            code: 'vue_missing_template',
            message: 'Vue SFC is missing a <template> block.',
            filePath: file.path,
          }),
        );
      }
      issues.push(...validateVueSfcStructure(file));
      if (!/<style[\s>]/i.test(file.content)) {
        issues.push(
          issue({
            code: 'vue_missing_style',
            severity: 'warning',
            message: 'Vue SFC should include styles for visual fidelity.',
            filePath: file.path,
          }),
        );
      }
    }
  }

  return issues;
}

export function buildCodegenQualityReport(input: {
  framework: Framework;
  files: SaveCodegenFileInput[];
  designIR?: CodegenDesignIR;
  repaired?: boolean;
}): CodegenQualityReport {
  const issues: CodegenQualityIssue[] = [];
  const content = allContent(input.files);

  if (input.files.length === 0 || !content.trim()) {
    issues.push(issue({ code: 'empty_output', message: 'Generated output is empty.' }));
  }

  for (const file of input.files) {
    if (file.content.trim().length < 12) {
      issues.push(
        issue({
          code: 'file_too_short',
          message: 'Generated file is too short to be useful.',
          filePath: file.path,
        }),
      );
    }
    for (const pattern of PLACEHOLDER_PATTERNS) {
      if (pattern.test(file.content)) {
        issues.push(
          issue({
            code: 'placeholder_text',
            severity: 'warning',
            message: 'Generated output contains placeholder text.',
            filePath: file.path,
          }),
        );
        break;
      }
    }
    for (const pattern of DANGEROUS_PATTERNS) {
      if (pattern.test(file.content)) {
        issues.push(
          issue({
            code: 'dangerous_code',
            message: 'Generated output contains unsafe browser code.',
            filePath: file.path,
          }),
        );
        break;
      }
    }
  }

  const fileValidation = validateCodegenFiles(input.framework, input.files);
  for (const validationIssue of fileValidation.issues) {
    issues.push(issue({ code: 'file_structure', message: validationIssue }));
  }

  issues.push(...validateFrameworkShape(input.framework, input.files));

  for (const text of missingDesignTexts(input.designIR, content)) {
    issues.push(
      issue({
        code: 'missing_text',
        message: `Generated output is missing design text: ${text}`,
      }),
    );
  }

  for (const asset of missingAssetRefs(input.designIR, content)) {
    issues.push(
      issue({
        code: 'missing_asset',
        message: `Generated output is missing asset reference: ${asset}`,
      }),
    );
  }

  const errorCount = issues.filter((item) => item.severity === 'error').length;
  const warningCount = issues.filter((item) => item.severity === 'warning').length;
  const status: CodegenQualityStatus =
    errorCount > 0 ? 'failed' : input.repaired ? 'repaired' : 'passed';

  return {
    status,
    framework: input.framework,
    issues,
    checkedAt: new Date().toISOString(),
    summary: {
      fileCount: input.files.length,
      errorCount,
      warningCount,
      missingTextCount: issues.filter((item) => item.code === 'missing_text').length,
      missingAssetCount: issues.filter((item) => item.code === 'missing_asset').length,
    },
  };
}
