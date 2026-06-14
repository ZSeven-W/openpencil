/* eslint-disable no-console */
// Drive `computeLayoutPositions` directly on the 5-card horizontal
// frame from /tmp/op-figma-ts.json and print the children x/y AFTER
// layout, so we can compare with authored values.

import { readFileSync } from 'node:fs';
import { computeLayoutPositions } from '../packages/pen-core/src/layout/engine';
import type { PenNode } from '../packages/pen-types/src';

const doc = JSON.parse(readFileSync('/tmp/op-figma-ts.json', 'utf8'));

function find5CardFrame(node: PenNode): PenNode | null {
  if (
    node.type === 'frame' &&
    (node as any).layout === 'horizontal' &&
    Array.isArray((node as any).children)
  ) {
    const flat: string[] = [];
    function go(x: any): void {
      if (x?.type === 'text') {
        const t =
          typeof x.content === 'string'
            ? x.content
            : Array.isArray(x.content)
              ? x.content.map((s: any) => s.text).join('')
              : '';
        flat.push(t);
      }
      if (x?.children) for (const c of x.children) go(c);
    }
    for (const c of (node as any).children) go(c);
    if (flat.includes('一级高风险') && flat.includes('其他')) return node;
  }
  if ('children' in (node as any)) {
    for (const c of (node as any).children ?? []) {
      const r = find5CardFrame(c);
      if (r) return r;
    }
  }
  return null;
}

let parent: PenNode | null = null;
for (const p of doc.pages ?? []) {
  for (const c of p.children) {
    if (!parent) parent = find5CardFrame(c);
  }
}
if (!parent) {
  console.error('not found');
  process.exit(1);
}
const kids = (parent as any).children;
console.log('AUTHORED:');
kids.forEach((c: any, i: number) => {
  const t = (() => {
    const out: string[] = [];
    function go(x: any): void {
      if (x?.type === 'text') {
        out.push(
          typeof x.content === 'string'
            ? x.content
            : Array.isArray(x.content)
              ? x.content.map((s: any) => s.text).join('')
              : '',
        );
      }
      if (x?.children) for (const c of x.children) go(c);
    }
    go(c);
    return out[0] ?? '';
  })();
  console.log(
    `  [${i}] ${c.type} x=${c.x} y=${c.y} w=${JSON.stringify((c as any).width)} text=${JSON.stringify(t)}`,
  );
});

console.log('\nAFTER computeLayoutPositions:');
const after = computeLayoutPositions(parent, kids);
after.forEach((c: any, i: number) => {
  const t = (() => {
    const out: string[] = [];
    function go(x: any): void {
      if (x?.type === 'text') {
        out.push(
          typeof x.content === 'string'
            ? x.content
            : Array.isArray(x.content)
              ? x.content.map((s: any) => s.text).join('')
              : '',
        );
      }
      if (x?.children) for (const c of x.children) go(c);
    }
    go(c);
    return out[0] ?? '';
  })();
  console.log(
    `  [${i}] ${c.type} x=${c.x} y=${c.y} w=${JSON.stringify((c as any).width)} text=${JSON.stringify(t)}`,
  );
});
