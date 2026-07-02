import { describe, it, expect } from 'vitest';
import {
  buildHeading,
  buildBodyText,
  buildListRow,
  buildFormField,
  buildFaqItem,
  buildComment,
} from '../element-builders/index.js';

/**
 * Unicode normalization invariant: builders must PRESERVE the
 * caller's byte representation verbatim. A string passed as NFC
 * (composed) must come out as NFC; NFD (decomposed) must come
 * out as NFD. No silent normalization.
 *
 * Why this matters:
 *   - macOS file-paths often arrive as NFD (HFS+ legacy convention)
 *   - Windows / Linux / most web APIs use NFC
 *   - Copy-paste between apps can carry any of the 4 normal forms
 *   - Some CJK inputs emit precomposed characters, others emit
 *     decomposed base + combining marks
 *
 * If the AI generates a design tree where the same visible label
 * appears in two different normal forms (header vs list row), we
 * WANT that to survive end-to-end — a downstream validator that
 * does NFC normalization will produce consistent trees, but the
 * BUILDER should not pre-empt that decision. Otherwise we get:
 *
 *   1. The user types "café" (NFC: `café`), sees it in the
 *      app, copies it elsewhere, gets "café" unexpectedly
 *   2. Round-trip through the builder breaks exact-match lookups
 *      in external systems (e.g. emoji-less fallback keys)
 *   3. String-length counts diverge silently (NFC "é" is 1 code
 *      point; NFD is 2) — affects text-measurement.
 *
 * This test fuzzes the same visible string across 4 normal forms
 * (NFC, NFD, NFKC, NFKD) and asserts the builder's output string
 * bytes match the input bytes exactly.
 */

const VISIBLE = 'café'; // composed e-acute looks the same as decomposed
const NFC = VISIBLE.normalize('NFC');
const NFD = VISIBLE.normalize('NFD');
const NFKC = VISIBLE.normalize('NFKC');
const NFKD = VISIBLE.normalize('NFKD');

// Sanity: the four forms aren't all byte-identical.
const FORMS = { NFC, NFD, NFKC, NFKD } as const;
type FormName = keyof typeof FORMS;

function getTextByName(
  node: { name?: string; content?: string; children?: unknown[] },
  name: string,
): string | undefined {
  if (node.name === name && typeof node.content === 'string') return node.content;
  const kids = (node.children ?? []) as Array<{
    name?: string;
    content?: string;
    children?: unknown[];
  }>;
  for (const c of kids) {
    const hit = getTextByName(c, name);
    if (hit !== undefined) return hit;
  }
  return undefined;
}

describe('element builders — unicode normalization preservation', () => {
  // Sanity: we're really testing 4 distinct byte sequences.
  it('test fixtures actually differ byte-wise across normal forms', () => {
    expect(NFC).not.toBe(NFD);
    expect(NFKC).not.toBe(NFKD);
    // NFC === NFKC for "café" (no compatibility remapping), and
    // NFD === NFKD same reason — that's fine, the interesting
    // distinction is NFC vs NFD.
    expect(NFC.length).toBeLessThan(NFD.length);
  });

  for (const formName of Object.keys(FORMS) as FormName[]) {
    const form = FORMS[formName];

    it(`buildHeading preserves ${formName} verbatim`, () => {
      const h = buildHeading({ content: form });
      expect((h as { content: string }).content).toBe(form);
      // Byte-for-byte equality (=== on strings is already byte-compare
      // for a given encoding; code-point length is the easy check).
      expect((h as { content: string }).content.length).toBe(form.length);
    });

    it(`buildBodyText preserves ${formName} verbatim`, () => {
      const b = buildBodyText({ content: form });
      expect((b as { content: string }).content).toBe(form);
    });

    it(`buildListRow preserves ${formName} in title + subtitle`, () => {
      const r = buildListRow({ title: form, subtitle: form });
      const title = getTextByName(r, 'Title');
      const subtitle = getTextByName(r, 'Subtitle');
      expect(title).toBe(form);
      expect(subtitle).toBe(form);
    });

    it(`buildFormField preserves ${formName} in label + placeholder`, () => {
      const f = buildFormField({ label: form, placeholder: form });
      const label = getTextByName(f, 'Label');
      const placeholder = getTextByName(f, 'Placeholder');
      expect(label).toBe(form);
      expect(placeholder).toBe(form);
    });

    it(`buildFaqItem preserves ${formName} in question + answer`, () => {
      const q = buildFaqItem({ question: form, answer: form, expanded: true });
      const qText = getTextByName(q, 'Question');
      const aText = getTextByName(q, 'Answer');
      expect(qText).toBe(form);
      expect(aText).toBe(form);
    });

    it(`buildComment preserves ${formName} in author + body`, () => {
      const c = buildComment({ author: form, body: form });
      // Author text node name is 'Author', body is 'Body'
      const author = getTextByName(c, 'Author');
      const body = getTextByName(c, 'Body');
      expect(author).toBe(form);
      expect(body).toBe(form);
    });
  }

  // Zero-width / invisible characters: must also pass through unchanged
  // (no builder-side whitespace stripping beyond what was already there).
  it('preserves zero-width joiner (\\u200D) mid-word', () => {
    const input = 'emo‍ji'; // "emoji" with a zero-width joiner
    const h = buildHeading({ content: input });
    expect((h as { content: string }).content).toBe(input);
    expect((h as { content: string }).content.length).toBe(6);
  });

  it('preserves byte order mark (\\uFEFF) at string start', () => {
    const input = '﻿Hello';
    const h = buildHeading({ content: input });
    expect((h as { content: string }).content).toBe(input);
  });

  it('preserves ZWJ emoji sequence (family of 4)', () => {
    // Man + ZWJ + Woman + ZWJ + Boy + ZWJ + Girl — renders as one
    // composed family glyph. 7 code-point sequence.
    const family = '\u{1F468}‍\u{1F469}‍\u{1F466}‍\u{1F467}';
    const h = buildHeading({ content: family });
    expect((h as { content: string }).content).toBe(family);
  });

  it('preserves combining-marks cluster (Vietnamese)', () => {
    // "nước" with combining marks (NFD form) vs precomposed (NFC)
    const nfc = 'nước';
    const nfd = nfc.normalize('NFD');
    const nfcH = buildHeading({ content: nfc });
    const nfdH = buildHeading({ content: nfd });
    expect((nfcH as { content: string }).content).toBe(nfc);
    expect((nfdH as { content: string }).content).toBe(nfd);
    // And the two outputs should NOT be ===
    expect((nfcH as { content: string }).content).not.toBe((nfdH as { content: string }).content);
  });

  it('preserves CJK (no hidden NFKC collapse of halfwidth / fullwidth)', () => {
    // Fullwidth "A" (U+FF21) vs ASCII "A" (U+0041) are DIFFERENT
    // characters. NFKC would collapse fullwidth to ASCII; if the
    // builder silently ran NFKC, this would become "A" and break
    // exact-match lookups in languages that rely on halfwidth/
    // fullwidth distinction (Japanese input, some Chinese inputs).
    const fullwidth = 'Ａ';
    const ascii = 'A';
    expect(fullwidth).not.toBe(ascii);
    const hFull = buildHeading({ content: fullwidth });
    const hAscii = buildHeading({ content: ascii });
    expect((hFull as { content: string }).content).toBe(fullwidth);
    expect((hAscii as { content: string }).content).toBe(ascii);
    expect((hFull as { content: string }).content).not.toBe(
      (hAscii as { content: string }).content,
    );
  });
});
