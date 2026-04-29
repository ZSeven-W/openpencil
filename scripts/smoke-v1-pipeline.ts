/**
 * Smoke test: v1 system mode + design system ripple
 *
 * Validates the full pipeline:
 *   createEmptyDocument
 *     → applySemanticPalette           (seeds 56 tokens + Light/Dark theme axis)
 *     → buildHeadingV1/buildSettingRowV1/etc. with theme:'system'  (emit $X refs)
 *     → resolveNodeForCanvas(doc, { Mode: 'Light' })  (resolve to light hex)
 *     → resolveNodeForCanvas(doc, { Mode: 'Dark' })   (resolve to dark hex)
 *
 * Run: bun scripts/smoke-v1-pipeline.ts
 *
 * Known gaps surfaced by this test (do NOT fix here — report to controller):
 *   GAP-1: resolveNodeForCanvas does not resolve `fontWeight` on text nodes
 *          (only fontSize/lineHeight/letterSpacing are in the key list at resolve.ts:281)
 *   GAP-2: resolveNodeForCanvas early-exits when `variables === {}` (line 217),
 *          so DEFAULT_PALETTE_FALLBACK is unreachable for un-seeded documents
 */

import {
  createEmptyDocument,
  applySemanticPalette,
  resolveNodeForCanvas,
  buildHeadingV1,
  buildBodyTextV1,
  buildSectionHeaderV1,
  buildSettingRowV1,
} from '@zseven-w/pen-core';
import type { PenNode } from '@zseven-w/pen-types';

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

type Result = 'PASS' | 'FAIL' | 'KNOWN_GAP';

interface Assertion {
  name: string;
  result: Result;
  detail?: string;
}

const assertions: Assertion[] = [];

function assert(name: string, condition: boolean, detail?: string): void {
  assertions.push({ name, result: condition ? 'PASS' : 'FAIL', detail });
}

/** Document a known gap without failing the test. */
function knownGap(name: string, detail: string): void {
  assertions.push({ name, result: 'KNOWN_GAP', detail });
}

/** Get all fill entries from the top-level `fill` array of a node (not recursive). */
function getTopFills(node: Record<string, unknown>): Array<Record<string, unknown>> {
  const fills = node.fill as Array<Record<string, unknown>> | undefined;
  if (!Array.isArray(fills)) return [];
  return fills;
}

/** Recursively find a named child by role. */
function findByRole(
  node: Record<string, unknown>,
  role: string,
): Record<string, unknown> | undefined {
  if (node.role === role) return node;
  const children = node.children as Array<Record<string, unknown>> | undefined;
  if (!Array.isArray(children)) return undefined;
  for (const child of children) {
    const found = findByRole(child, role);
    if (found) return found;
  }
  return undefined;
}

/** Walk all string values in a node tree; return any that start with '$'. */
function collectRefs(node: Record<string, unknown>): string[] {
  const refs: string[] = [];
  for (const val of Object.values(node)) {
    if (typeof val === 'string' && val.startsWith('$')) refs.push(val);
    if (Array.isArray(val)) {
      for (const item of val) {
        if (item && typeof item === 'object')
          refs.push(...collectRefs(item as Record<string, unknown>));
        if (typeof item === 'string' && item.startsWith('$')) refs.push(item);
      }
    }
    if (val && typeof val === 'object' && !Array.isArray(val)) {
      refs.push(...collectRefs(val as Record<string, unknown>));
    }
  }
  return refs;
}

function hasNoRefs(node: Record<string, unknown>): boolean {
  return collectRefs(node).length === 0;
}

// ─────────────────────────────────────────────────────────────────────────────
// Step 1: Build v1 system-mode nodes
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n══════════════════════════════════════════════════════');
console.log(' STEP 1: Build v1 nodes with theme:"system"');
console.log('══════════════════════════════════════════════════════');

const sectionHeader = buildSectionHeaderV1({
  title: 'Notifications',
  theme: 'system',
}) as unknown as Record<string, unknown>;
const heading = buildHeadingV1({
  content: 'Account',
  level: 'h2',
  theme: 'system',
}) as unknown as Record<string, unknown>;
const bodyText = buildBodyTextV1({
  content: 'Manage your account preferences',
  theme: 'system',
}) as unknown as Record<string, unknown>;

// settingRow with switch (on:true) — no leading_icon so switch is the first child with a fill
const settingRow1 = buildSettingRowV1({
  title: 'Email',
  trailing: { kind: 'switch', on: true },
  theme: 'system',
}) as unknown as Record<string, unknown>;

// settingRow with value trailing
const settingRow3 = buildSettingRowV1({
  title: 'SMS',
  trailing: { kind: 'value', value: 'Off' },
  theme: 'system',
}) as unknown as Record<string, unknown>;

// settingRow with leading_icon (for breadth of testing)
const settingRow4 = buildSettingRowV1({
  title: 'Marketing',
  leading_icon: 'mail',
  trailing: { kind: 'chevron' },
  theme: 'system',
}) as unknown as Record<string, unknown>;

const settingRow2 = buildSettingRowV1({
  title: 'Push Notifications',
  trailing: { kind: 'switch', on: false },
  theme: 'system',
}) as unknown as Record<string, unknown>;

const allV1Nodes = [
  sectionHeader,
  settingRow1,
  settingRow2,
  settingRow3,
  settingRow4,
  heading,
  bodyText,
];

// Inspect raw v1 output using role-based search (precise, not depth-first first-fill)
const headingFill = getTopFills(heading);
const switchNode = findByRole(settingRow1, 'setting-row-switch');
const switchFill = switchNode ? getTopFills(switchNode) : [];
const valueFill = findByRole(settingRow3, 'setting-row-value');

console.log('\nRaw heading node:');
console.log(`  fontSize:   ${String(heading.fontSize)}`);
console.log(`  fontWeight: ${String(heading.fontWeight)}`);
console.log(`  lineHeight: ${String(heading.lineHeight)}`);
console.log(`  fill:       ${JSON.stringify(headingFill)}`);

console.log('\nRaw settingRow1 switch fill (role=setting-row-switch):');
console.log(`  fill: ${JSON.stringify(switchFill)}`);

console.log('\nRaw settingRow3 trailing value fill (role=setting-row-value):');
console.log(`  fill: ${JSON.stringify(getTopFills(valueFill ?? {}))}`);

// Collect all $refs across v1 nodes
const allRefs = allV1Nodes.flatMap((n) => collectRefs(n));
const uniqueRefs = [...new Set(allRefs)].sort();
console.log(`\nAll unique $refs found across v1 nodes (${uniqueRefs.length}):`);
for (const ref of uniqueRefs) console.log(`  ${ref}`);

// ── Assertions: Step 1 ──────────────────────────────────────────────────────

assert(
  'heading.fontSize is $type-h2-size ref (system mode)',
  heading.fontSize === '$type-h2-size',
  `got: ${String(heading.fontSize)}`,
);

assert(
  'heading.fontWeight is $type-h2-weight ref (system mode)',
  heading.fontWeight === '$type-h2-weight',
  `got: ${String(heading.fontWeight)}`,
);

assert(
  'heading.lineHeight is $type-h2-line-height ref (system mode)',
  heading.lineHeight === '$type-h2-line-height',
  `got: ${String(heading.lineHeight)}`,
);

const headingFillColor = headingFill[0]?.color as string | undefined;
assert(
  'heading fill[0].color is $color-text-primary ref',
  headingFillColor === '$color-text-primary',
  `got: ${String(headingFillColor)}`,
);

const switchFillColor = switchFill[0]?.color as string | undefined;
assert(
  'settingRow1 switch fill[0].color is $color-accent ref (on:true)',
  switchFillColor === '$color-accent',
  `got: ${String(switchFillColor)}`,
);

const valueFillColor = valueFill
  ? (getTopFills(valueFill)[0]?.color as string | undefined)
  : undefined;
assert(
  'settingRow3 trailing value fill[0].color is $color-text-muted ref',
  valueFillColor === '$color-text-muted',
  `got: ${String(valueFillColor)}`,
);

assert(
  'v1 nodes contain multiple $refs (system mode active)',
  allRefs.length > 5,
  `found ${allRefs.length} total refs`,
);

assert(
  'bodyText.fontSize is literal number 16 (body not yet using system typography tokens)',
  typeof bodyText.fontSize === 'number' && bodyText.fontSize === 16,
  `got: ${typeof bodyText.fontSize} ${String(bodyText.fontSize)}`,
);

// ─────────────────────────────────────────────────────────────────────────────
// Step 2: Mount into doc + seed palette
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n══════════════════════════════════════════════════════');
console.log(' STEP 2: Mount nodes into doc + applySemanticPalette');
console.log('══════════════════════════════════════════════════════');

const rawDoc = createEmptyDocument();
const seededDoc = applySemanticPalette(rawDoc);

// Inject nodes into the active page's root frame children
const activePage = seededDoc.pages?.[0];
if (!activePage) throw new Error('No active page in document');
const rootFrame = activePage.children[0] as Record<string, unknown>;
rootFrame.children = allV1Nodes;

console.log(`\nDoc themes after applySemanticPalette: ${JSON.stringify(seededDoc.themes)}`);
console.log(`Doc variables count: ${Object.keys(seededDoc.variables ?? {}).length}`);

assert(
  'doc has Mode theme axis after applySemanticPalette',
  Array.isArray(seededDoc.themes?.Mode) &&
    (seededDoc.themes?.Mode as string[]).includes('Light') &&
    (seededDoc.themes?.Mode as string[]).includes('Dark'),
  `got: ${JSON.stringify(seededDoc.themes)}`,
);

assert(
  'doc has 56 semantic palette variables',
  Object.keys(seededDoc.variables ?? {}).length === 56,
  `got: ${Object.keys(seededDoc.variables ?? {}).length}`,
);

// ─────────────────────────────────────────────────────────────────────────────
// Step 3: Resolve in Light theme
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n══════════════════════════════════════════════════════');
console.log(' STEP 3: Resolve in Light theme');
console.log('══════════════════════════════════════════════════════');

const vars = seededDoc.variables ?? {};
const lightTheme = { Mode: 'Light' };
const darkTheme = { Mode: 'Dark' };

const lightHeading = resolveNodeForCanvas(
  heading as unknown as PenNode,
  vars,
  lightTheme,
) as unknown as Record<string, unknown>;
const lightRow1 = resolveNodeForCanvas(
  settingRow1 as unknown as PenNode,
  vars,
  lightTheme,
) as unknown as Record<string, unknown>;

const lightHeadingFontSize = lightHeading.fontSize;
const lightHeadingFill = getTopFills(lightHeading)[0]?.color as string | undefined;
const lightSwitchNode = findByRole(lightRow1, 'setting-row-switch');
const lightSwitchFill = lightSwitchNode
  ? (getTopFills(lightSwitchNode)[0]?.color as string | undefined)
  : undefined;

console.log(`\nLight-resolved heading.fontSize: ${String(lightHeadingFontSize)}`);
console.log(`Light-resolved heading.fontWeight: ${String(lightHeading.fontWeight)}`);
console.log(`Light-resolved heading.lineHeight: ${String(lightHeading.lineHeight)}`);
console.log(`Light-resolved heading fill[0].color: ${String(lightHeadingFill)}`);
console.log(`Light-resolved switch fill[0].color: ${String(lightSwitchFill)}`);

assert(
  'Light: heading.fontSize resolves to number 20 ($type-h2-size)',
  typeof lightHeadingFontSize === 'number' && lightHeadingFontSize === 20,
  `got: ${typeof lightHeadingFontSize} ${String(lightHeadingFontSize)}`,
);

assert(
  'Light: heading.lineHeight resolves to number 1.25 ($type-h2-line-height)',
  typeof lightHeading.lineHeight === 'number' && lightHeading.lineHeight === 1.25,
  `got: ${typeof lightHeading.lineHeight} ${String(lightHeading.lineHeight)}`,
);

assert(
  'Light: heading fill[0].color resolves to #0F172A (light text-primary)',
  lightHeadingFill === '#0F172A',
  `got: ${String(lightHeadingFill)}`,
);

assert(
  'Light: switch fill[0].color resolves to #2563EB (light accent)',
  lightSwitchFill === '#2563EB',
  `got: ${String(lightSwitchFill)}`,
);

// GAP-1: fontWeight not resolved — still a $ref after resolveNodeForCanvas
const fontWeightAfterLight = lightHeading.fontWeight;
knownGap(
  'GAP-1: heading.fontWeight not resolved by resolveNodeForCanvas (still a $ref)',
  `fontWeight after resolve = ${String(fontWeightAfterLight)}; ` +
    'fix: add "fontWeight" to key list in resolve.ts:281',
);

// The heading still has $type-h2-weight unresolved → refs remain
const lightHeadingRefs = collectRefs(lightHeading);
console.log(`\nRemaining $refs in light-resolved heading: ${JSON.stringify(lightHeadingRefs)}`);
assert(
  'Light: fill and fontSize refs resolved (no $color-* or $type-h*-size refs remain)',
  !lightHeadingRefs.some((r) => r.startsWith('$color-') || r === '$type-h2-size'),
  `remaining: ${JSON.stringify(lightHeadingRefs)}`,
);

// ─────────────────────────────────────────────────────────────────────────────
// Step 4: Resolve in Dark theme
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n══════════════════════════════════════════════════════');
console.log(' STEP 4: Resolve in Dark theme');
console.log('══════════════════════════════════════════════════════');

const darkHeading = resolveNodeForCanvas(
  heading as unknown as PenNode,
  vars,
  darkTheme,
) as unknown as Record<string, unknown>;
const darkRow1 = resolveNodeForCanvas(
  settingRow1 as unknown as PenNode,
  vars,
  darkTheme,
) as unknown as Record<string, unknown>;

const darkHeadingFontSize = darkHeading.fontSize;
const darkHeadingFill = getTopFills(darkHeading)[0]?.color as string | undefined;
const darkSwitchNode = findByRole(darkRow1, 'setting-row-switch');
const darkSwitchFill = darkSwitchNode
  ? (getTopFills(darkSwitchNode)[0]?.color as string | undefined)
  : undefined;

console.log(`\nDark-resolved heading.fontSize: ${String(darkHeadingFontSize)}`);
console.log(`Dark-resolved heading fill[0].color: ${String(darkHeadingFill)}`);
console.log(`Dark-resolved switch fill[0].color: ${String(darkSwitchFill)}`);

// Side-by-side comparison table
console.log('\n┌─────────────────────────────┬──────────────────┬──────────────────┐');
console.log('│ Field                       │ Light            │ Dark             │');
console.log('├─────────────────────────────┼──────────────────┼──────────────────┤');
console.log(
  `│ heading.fontSize            │ ${String(lightHeadingFontSize).padEnd(16)} │ ${String(darkHeadingFontSize).padEnd(16)} │`,
);
console.log(
  `│ heading fill (text-primary) │ ${String(lightHeadingFill).padEnd(16)} │ ${String(darkHeadingFill).padEnd(16)} │`,
);
console.log(
  `│ switch fill (accent)        │ ${String(lightSwitchFill).padEnd(16)} │ ${String(darkSwitchFill).padEnd(16)} │`,
);
console.log('└─────────────────────────────┴──────────────────┴──────────────────┘');

assert(
  'Dark: heading.fontSize resolves to number 20 (typography is theme-agnostic)',
  typeof darkHeadingFontSize === 'number' && darkHeadingFontSize === 20,
  `got: ${typeof darkHeadingFontSize} ${String(darkHeadingFontSize)}`,
);

assert(
  'Dark: heading fill[0].color resolves to #F1F5F9 (dark text-primary)',
  darkHeadingFill === '#F1F5F9',
  `got: ${String(darkHeadingFill)}`,
);

assert(
  'Dark: switch fill[0].color resolves to #60A5FA (dark accent)',
  darkSwitchFill === '#60A5FA',
  `got: ${String(darkSwitchFill)}`,
);

assert(
  'Light vs Dark heading fill.color are DIFFERENT (color theme ripple works)',
  lightHeadingFill !== darkHeadingFill,
  `light=${String(lightHeadingFill)}, dark=${String(darkHeadingFill)}`,
);

assert(
  'Light vs Dark heading.fontSize are SAME (numeric is theme-agnostic)',
  lightHeadingFontSize === darkHeadingFontSize,
  `light=${String(lightHeadingFontSize)}, dark=${String(darkHeadingFontSize)}`,
);

assert(
  'Light vs Dark switch fill.color are DIFFERENT (accent ripple works)',
  lightSwitchFill !== darkSwitchFill,
  `light=${String(lightSwitchFill)}, dark=${String(darkSwitchFill)}`,
);

// ─────────────────────────────────────────────────────────────────────────────
// Step 5: Fallback path (un-seeded doc)
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n══════════════════════════════════════════════════════');
console.log(' STEP 5: Fallback path — un-seeded doc (no palette)');
console.log('══════════════════════════════════════════════════════');

// GAP-2: resolveNodeForCanvas exits early when variables === {} (line 217)
// The DEFAULT_PALETTE_FALLBACK is never reached. We verify this is the actual
// behavior and document it as a known gap.
const unseededDoc = createEmptyDocument();
const unseededVars = unseededDoc.variables ?? {};

const fallbackLightHeading = resolveNodeForCanvas(heading as unknown as PenNode, unseededVars, {
  Mode: 'Light',
}) as unknown as Record<string, unknown>;
const fallbackLightFill = getTopFills(fallbackLightHeading)[0]?.color as string | undefined;
const fallbackLightSize = fallbackLightHeading.fontSize;

console.log(`\nUn-seeded vars count: ${Object.keys(unseededVars).length}`);
console.log(`Fallback Light heading fill[0].color: ${String(fallbackLightFill)}`);
console.log(`Fallback Light heading.fontSize: ${String(fallbackLightSize)}`);
console.log(`(Expected: early exit → $refs remain unresolved due to GAP-2)`);

// Verify the early-exit behavior: empty vars → node returned unchanged
assert(
  'Un-seeded doc with empty vars returns node unchanged (early-exit confirmed)',
  fallbackLightFill === '$color-text-primary' && fallbackLightSize === '$type-h2-size',
  `fill=${String(fallbackLightFill)}, fontSize=${String(fallbackLightSize)}`,
);

knownGap(
  'GAP-2: DEFAULT_PALETTE_FALLBACK unreachable when doc.variables is {} (resolveNodeForCanvas line 217 early exit)',
  'Fix: change guard to allow empty-vars path when DEFAULT_PALETTE_FALLBACK has the key, ' +
    'OR pre-populate createEmptyDocument() vars with palette defaults',
);

// Demonstrate that fallback DOES work when vars has at least one non-palette entry
// (proving the code path itself works — the guard is the only issue)
const artificialVars = { dummy: { type: 'number' as const, value: 1 } };
const artificialFallbackHeading = resolveNodeForCanvas(
  heading as unknown as PenNode,
  artificialVars,
  { Mode: 'Light' },
) as unknown as Record<string, unknown>;
const artificialFallbackFill = getTopFills(artificialFallbackHeading)[0]?.color as
  | string
  | undefined;
const artificialFallbackSize = artificialFallbackHeading.fontSize;

console.log(`\nArtificial non-empty vars (triggers fallback path):`);
console.log(`  heading fill[0].color: ${String(artificialFallbackFill)}`);
console.log(`  heading.fontSize: ${String(artificialFallbackSize)}`);

assert(
  'DEFAULT_PALETTE_FALLBACK works when vars is non-empty (fill → #0F172A)',
  artificialFallbackFill === '#0F172A',
  `got: ${String(artificialFallbackFill)}`,
);

assert(
  'DEFAULT_PALETTE_FALLBACK works when vars is non-empty (fontSize → 20)',
  typeof artificialFallbackSize === 'number' && artificialFallbackSize === 20,
  `got: ${typeof artificialFallbackSize} ${String(artificialFallbackSize)}`,
);

// Fallback comparison table
console.log('\n┌─────────────────────────────┬──────────────────┬──────────────────┐');
console.log('│ Field                       │ Seeded           │ Artificial-FB    │');
console.log('├─────────────────────────────┼──────────────────┼──────────────────┤');
console.log(
  `│ Light heading fill.color    │ ${String(lightHeadingFill).padEnd(16)} │ ${String(artificialFallbackFill).padEnd(16)} │`,
);
console.log(
  `│ Light heading.fontSize      │ ${String(lightHeadingFontSize).padEnd(16)} │ ${String(artificialFallbackSize).padEnd(16)} │`,
);
console.log('└─────────────────────────────┴──────────────────┴──────────────────┘');

assert(
  'Seeded == Fallback for Light fill (fallback byte-equiv to seeded)',
  lightHeadingFill === artificialFallbackFill,
  `seeded=${String(lightHeadingFill)}, fallback=${String(artificialFallbackFill)}`,
);

assert(
  'Seeded == Fallback for Light fontSize (fallback byte-equiv to seeded)',
  lightHeadingFontSize === artificialFallbackSize,
  `seeded=${String(lightHeadingFontSize)}, fallback=${String(artificialFallbackSize)}`,
);

// ─────────────────────────────────────────────────────────────────────────────
// Summary
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n══════════════════════════════════════════════════════');
console.log(' ASSERTION SUMMARY');
console.log('══════════════════════════════════════════════════════');

let passed = 0;
let failed = 0;
let gaps = 0;
for (const a of assertions) {
  let icon: string;
  if (a.result === 'PASS') {
    icon = '✓';
    passed++;
  } else if (a.result === 'FAIL') {
    icon = '✗';
    failed++;
  } else {
    icon = '⚠';
    gaps++;
  }
  const detail = a.detail ? `  (${a.detail})` : '';
  console.log(`${icon} [${a.result}] ${a.name}${detail}`);
}

console.log(
  `\n${passed}/${assertions.length - gaps} assertions passed, ${failed} failed, ${gaps} known gap(s) documented`,
);

if (failed > 0) {
  console.error('\n[SMOKE TEST FAILED] — see FAIL assertions above');
  process.exit(1);
} else {
  console.log('\n[SMOKE TEST PASSED] Full pipeline OK (known gaps documented above)');
}
