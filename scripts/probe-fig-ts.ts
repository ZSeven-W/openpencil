/* eslint-disable no-console */
// Drive the TS pen-figma import on a .fig file, then dump a structural
// digest so we can diff against the Rust probe_fig output.
//
// Usage: bun run scripts/probe-fig-ts.ts <path.fig>

import { readFileSync } from 'node:fs';
import { basename } from 'node:path';
import { parseFigFile } from '../packages/pen-figma/src/fig-parser';
import {
  figmaAllPagesToPenDocument,
  getFigmaPages,
} from '../packages/pen-figma/src/figma-node-mapper';
import { resolveImageBlobs } from '../packages/pen-figma/src/figma-image-resolver';
import type { PenNode } from '../packages/pen-types/src';

const path = process.argv[2];
if (!path) {
  console.error('usage: bun run scripts/probe-fig-ts.ts <path.fig>');
  process.exit(2);
}

const bytes = readFileSync(path);
const ab = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;

const decoded = parseFigFile(ab);
const pages = getFigmaPages(decoded);
const name = basename(path).replace(/\.fig$/, '');
const result = figmaAllPagesToPenDocument(decoded, name, 'openpencil');
resolveImageBlobs(
  result.document.pages?.flatMap((p) => p.children) ?? [],
  result.imageBlobs,
  decoded.imageFiles,
);

if (process.env.OP_DUMP_JSON) {
  const fs = await import('node:fs');
  fs.writeFileSync(process.env.OP_DUMP_JSON, JSON.stringify(result.document, null, 2));
  console.error(`dumped PenDocument JSON to ${process.env.OP_DUMP_JSON}`);
}

console.log(`input: ${path} (${bytes.length} bytes)`);
console.log(`pages: ${pages.length}`);
for (let i = 0; i < pages.length; i++) {
  const p = pages[i];
  const root = result.document.pages?.[i];
  console.log(`  [${i}] name=${JSON.stringify(p.name)} children=${root?.children.length ?? 0}`);
  if (root) {
    const counts = tally(root.children);
    if (Object.keys(counts).length > 0) {
      const parts = Object.entries(counts)
        .sort()
        .map(([k, v]) => `${k}=${v}`)
        .join(' ');
      console.log(`        top-level tally: ${parts}`);
    }
  }
}
console.log(`warnings: ${result.warnings.length}`);
for (const w of result.warnings.slice(0, 8)) console.log(`  - ${w}`);

// Walk the first page in dfs and print a hash of (id, type, x, y, w, h) so we can compare.
// Optional drill into a named subframe via env var OP_PROBE_DRILL.
const drillNeedle = process.env.OP_PROBE_DRILL;
const firstPage = result.document.pages?.[0];
if (firstPage && drillNeedle) {
  for (const root of firstPage.children) {
    if (!('children' in root) || !Array.isArray(root.children)) continue;
    for (const c of root.children) {
      if ((c.name ?? '').includes(drillNeedle)) {
        console.log(`── drilling into ${JSON.stringify(c.name)} ──`);
        console.log(
          `width=${JSON.stringify((c as any).width)} height=${JSON.stringify((c as any).height)} layout=${JSON.stringify((c as any).layout)} gap=${JSON.stringify((c as any).gap)}`,
        );
        const kids = (c as any).children ?? [];
        for (let i = 0; i < kids.length; i++) {
          const k = kids[i];
          console.log(
            `   [${i}] ${(k.name ?? '').slice(0, 24).padEnd(24)} ${k.type} x=${k.x ?? 0} y=${k.y ?? 0} w=${JSON.stringify((k as any).width)} h=${JSON.stringify((k as any).height)}`,
          );
        }
      }
    }
  }
}
if (firstPage) {
  const digest = digestTree(firstPage.children);
  console.log(`first-page digest: ${digest}`);
  const total = countDeep(firstPage.children);
  console.log(`first-page deep node count: ${total}`);
  const textNodes = collectText(firstPage.children);
  console.log(`first-page text node count: ${textNodes.length}`);
  const overlaps = findCoLocatedTexts(textNodes);
  console.log(`first-page co-located-text clusters (≥2 nodes sharing x,y): ${overlaps.length}`);
  for (const c of overlaps.slice(0, 10)) {
    console.log(
      `   - at (${c.x}, ${c.y}): ${c.texts
        .slice(0, 4)
        .map((t) => JSON.stringify(t))
        .join(' | ')}${c.texts.length > 4 ? ' …' : ''}`,
    );
  }
}

function tally(nodes: PenNode[]): Record<string, number> {
  const r: Record<string, number> = {};
  for (const n of nodes) r[n.type] = (r[n.type] ?? 0) + 1;
  return r;
}

function countDeep(nodes: PenNode[]): number {
  let n = nodes.length;
  for (const c of nodes) {
    if ('children' in c && Array.isArray(c.children)) n += countDeep(c.children);
  }
  return n;
}

function digestTree(nodes: PenNode[]): string {
  let h = 0;
  function go(arr: PenNode[]) {
    for (const c of arr) {
      const sig = `${c.type}|${c.x ?? 0}|${c.y ?? 0}|${'width' in c ? JSON.stringify(c.width) : ''}|${'height' in c ? JSON.stringify(c.height) : ''}`;
      for (let i = 0; i < sig.length; i++) {
        h = (h * 31 + sig.charCodeAt(i)) | 0;
      }
      if ('children' in c && Array.isArray(c.children)) go(c.children);
    }
  }
  go(nodes);
  return (h >>> 0).toString(16);
}

function collectText(nodes: PenNode[]): Array<{ x: number; y: number; text: string }> {
  const out: Array<{ x: number; y: number; text: string }> = [];
  function go(arr: PenNode[]) {
    for (const c of arr) {
      if (c.type === 'text') {
        const text =
          typeof c.content === 'string'
            ? c.content
            : Array.isArray(c.content)
              ? c.content.map((s) => s.text).join('')
              : '';
        out.push({ x: c.x ?? 0, y: c.y ?? 0, text });
      }
      if ('children' in c && Array.isArray(c.children)) go(c.children);
    }
  }
  go(nodes);
  return out;
}

function findCoLocatedTexts(
  texts: Array<{ x: number; y: number; text: string }>,
): Array<{ x: number; y: number; texts: string[] }> {
  const m = new Map<string, string[]>();
  for (const t of texts) {
    const k = `${Math.round(t.x)},${Math.round(t.y)}`;
    if (!m.has(k)) m.set(k, []);
    m.get(k)!.push(t.text);
  }
  const out: Array<{ x: number; y: number; texts: string[] }> = [];
  for (const [k, v] of m.entries()) {
    if (v.length >= 2) {
      const [xs, ys] = k.split(',');
      out.push({ x: parseInt(xs), y: parseInt(ys), texts: v });
    }
  }
  return out;
}
