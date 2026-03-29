import type { TextNode } from '@zseven-w/pen-types';

export function getTextContent(node: TextNode): string {
  if (typeof node.content === 'string') return node.content;
  return node.content.map((s) => s.text).join('');
}
