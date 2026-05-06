/**
 * 设计代码生成器（视觉参考管道的第一阶段）。
 *
 * 这里会调用模型最强的视觉/排版能力，先产出一份自包含的 HTML/CSS。
 * 这份代码不是最终产物，而是后续生成 PenNode 时使用的视觉参考。
 * 同时会注入设计原则，尽量保证视觉质量稳定。
 */

import type { DesignSystem } from './ai-types';
import type { AIProviderType } from '@/types/agent-settings';
import { generateCompletion } from './ai-service';
import { getSkillByName } from '@zseven-w/pen-ai-skills';
import { designSystemToPromptContext } from './design-system-generator';

interface CodeGenOptions {
  width: number;
  height: number;
  model?: string;
  provider?: AIProviderType;
}

/**
 * 为设计请求生成一份自包含的 HTML/CSS 参考代码。
 * 这份代码会作为后续画布生成流程的视觉蓝图。
 */
export async function generateDesignCode(
  prompt: string,
  designSystem: DesignSystem,
  options: CodeGenOptions,
): Promise<string> {
  const designCodeSkill = getSkillByName('design-code')?.content ?? '';
  const principles = getSkillByName('design-principles')?.content ?? '';

  // 构造系统提示，并把设计原则拼进去
  const systemPrompt = principles ? `${designCodeSkill}\n\n${principles}` : designCodeSkill;

  // 基于设计系统上下文构造用户提示
  const dsContext = designSystemToPromptContext(designSystem);
  const userPrompt = buildCodeGenUserPrompt(prompt, dsContext, options.width, options.height);

  const response = await generateCompletion(
    systemPrompt,
    userPrompt,
    options.model,
    options.provider,
  );

  return extractHtmlFromResponse(response);
}

/**
 * 从 AI 响应里提取 HTML 内容。
 * 兼容代码围栏、Markdown 包裹或裸 HTML 这几种返回形式。
 */
function extractHtmlFromResponse(response: string): string {
  const trimmed = response.trim();

  // 先检查是否是代码围栏包裹的 HTML
  const fenceMatch = trimmed.match(/```(?:html)?\s*\n?([\s\S]*?)\n?```/);
  if (fenceMatch) {
    const content = fenceMatch[1].trim();
    if (content.includes('<!DOCTYPE') || content.includes('<html')) {
      return content;
    }
  }

  // 再检查响应本身是否直接以 HTML 开头
  if (trimmed.startsWith('<!DOCTYPE') || trimmed.startsWith('<html')) {
    return trimmed;
  }

  // 尝试在响应正文中提取完整的 HTML 文档
  const htmlMatch = trimmed.match(/(<!DOCTYPE[\s\S]*<\/html>)/i);
  if (htmlMatch) {
    return htmlMatch[1];
  }

  // 最后兜底：把裸内容包成一个最小可用的 HTML 文档
  return `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Design</title></head>
<body>${trimmed}</body>
</html>`;
}

/**
 * 从 HTML 中提取结构摘要，供子代理作为参考。
 * 输出一段简洁的文本描述，用来概括页面结构。
 */
export function extractStructureSummary(html: string): string {
  const lines: string[] = ['DESIGN REFERENCE STRUCTURE:'];

  // 提取 section 级别的结构元素
  const sectionPattern =
    /<(?:section|header|footer|nav|main|div)\s+[^>]*(?:class|id)="([^"]*)"[^>]*>/gi;
  let match: RegExpExecArray | null;
  while ((match = sectionPattern.exec(html)) !== null) {
    const classOrId = match[1];
    if (classOrId && !classOrId.includes('__')) {
      lines.push(`- Section: ${classOrId}`);
    }
  }

  // 提取标题内容，补充结构提示
  const headingPattern = /<h([1-6])[^>]*>([\s\S]*?)<\/h\1>/gi;
  while ((match = headingPattern.exec(html)) !== null) {
    const level = match[1];
    const content = match[2]
      .replace(/<[^>]+>/g, '')
      .trim()
      .slice(0, 60);
    if (content) {
      lines.push(`- H${level}: "${content}"`);
    }
  }

  // 提取按钮 / CTA 文本
  const buttonPattern =
    /<(?:button|a)\s+[^>]*class="[^"]*(?:btn|button|cta)[^"]*"[^>]*>([\s\S]*?)<\/(?:button|a)>/gi;
  while ((match = buttonPattern.exec(html)) !== null) {
    const text = match[1]
      .replace(/<[^>]+>/g, '')
      .trim()
      .slice(0, 30);
    if (text) {
      lines.push(`- CTA: "${text}"`);
    }
  }

  // 如果提取不到结构信息，就退回到通用摘要
  if (lines.length <= 1) {
    lines.push('(HTML structure extracted — use as visual layout reference)');
  }

  return lines.join('\n');
}

/**
 * 提取与某个子任务标签最相关的 HTML 片段。
 * 这里会用启发式规则匹配 section/div 的 id、class 和标题内容。
 */
export function extractHtmlSection(html: string, subtaskLabel: string): string | null {
  const labelLower = subtaskLabel.toLowerCase();

  // 先根据常见关键词尝试定位对应的 section
  const keywords = labelLower
    .replace(/[（(].+[)）]/g, '')
    .split(/[\s,/]+/)
    .filter((w) => w.length > 2);

  if (keywords.length === 0) return null;

  // 构造一个用于匹配容器节点的正则
  const keywordPattern = keywords.map((k) => k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|');
  const sectionRegex = new RegExp(
    `<(?:section|div|header|footer|nav)[^>]*(?:class|id)="[^"]*(?:${keywordPattern})[^"]*"[^>]*>[\\s\\S]*?(?=<(?:section|div|header|footer|nav)[^>]*(?:class|id)="|$)`,
    'i',
  );

  const match = sectionRegex.exec(html);
  if (match) {
    // 截断到适合放进上下文的长度
    const section = match[0].slice(0, 1500);
    return `HTML reference for "${subtaskLabel}":\n${section}`;
  }

  return null;
}

/**
 * 为 HTML/CSS 代码生成构造用户提示。
 * 其中会包含设计系统 token 和视口约束。
 */
function buildCodeGenUserPrompt(
  userPrompt: string,
  designSystemContext: string,
  width: number,
  height: number,
): string {
  const heightInstruction =
    height > 0
      ? `Height: ${height}px (fixed viewport).`
      : `Height: auto (content determines height, estimate based on sections).`;

  return `Design request: ${userPrompt}

Viewport: Width ${width}px. ${heightInstruction}

${designSystemContext}

Generate the complete HTML file now.`;
}
