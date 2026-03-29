import { describe, it, expect } from 'vitest';
import { indent } from '../shared/indentation';
import { kebabToPascal } from '../shared/naming';
import { getTextContent } from '../shared/text';
import type { TextNode } from '@zseven-w/pen-types';

describe('indent', () => {
  it('returns empty string for depth 0', () => {
    expect(indent(0)).toBe('');
  });
  it('defaults to 2-space indent', () => {
    expect(indent(2)).toBe('    ');
  });
  it('supports custom indent string', () => {
    expect(indent(2, '    ')).toBe('        ');
  });
});

describe('kebabToPascal', () => {
  it('converts kebab-case to PascalCase', () => {
    expect(kebabToPascal('my-component')).toBe('MyComponent');
  });
  it('handles single word', () => {
    expect(kebabToPascal('button')).toBe('Button');
  });
});

describe('getTextContent', () => {
  it('returns string content directly', () => {
    const node = { type: 'text', content: 'hello' } as unknown as TextNode;
    expect(getTextContent(node)).toBe('hello');
  });
  it('joins styled segments', () => {
    const node = {
      type: 'text',
      content: [{ text: 'hello ' }, { text: 'world' }],
    } as unknown as TextNode;
    expect(getTextContent(node)).toBe('hello world');
  });
});
