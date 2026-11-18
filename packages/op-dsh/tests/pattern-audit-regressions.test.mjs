// Regression tests for the PATTERN-AUDIT (2026-09-06) confirmed defect.
// Driven live: a real one-shot design task rolled a text nav-link into a
// terminal contradiction — the touch-target diagnostic said "44px minimum"
// without the minWidth escape hatch, the model set width:44, and the
// container-flow check then rejected 44 < padding + label terminally.
import assert from 'node:assert/strict'
import { test } from 'node:test'

const { inspectGeneratedDesignQualityReport } = await import('../lib/design-quality.js')

const navLink = {
  type: 'frame',
  id: 'navlink-1',
  name: 'Menu link',
  role: 'nav link',
  layout: 'horizontal',
  width: 'fit_content',
  minWidth: 44,
  height: 44,
  padding: [0, 16, 0, 16],
  children: [
    { type: 'text', id: 'navlink-1-text', content: 'Menu', fontSize: 16 },
  ],
}

const page = {
  type: 'frame',
  id: 'page',
  name: 'Page',
  width: 1440,
  height: 900,
  layout: 'vertical',
  children: [navLink],
}

test('fit_content + minWidth 44 nav-link satisfies BOTH touch-target and container-flow (the escape hatch stays open)', () => {
  const report = inspectGeneratedDesignQualityReport(JSON.stringify(page))
  const rules = new Set(report.diagnostics.map((line) => line.split(':')[1]?.trim().split(' ')[0]))
  assert.ok(!rules.has('authored'), `no touch-target diagnostic expected, got: ${JSON.stringify(report.diagnostics)}`)
  assert.ok(
    report.diagnostics.every((line) => !/container.*width cannot contain/.test(line)),
    `no container-flow diagnostic expected, got: ${JSON.stringify(report.diagnostics)}`,
  )
})

test('the touch-target diagnostic names minWidth/minHeight instead of inviting a fixed 44px width', () => {
  const bad = JSON.parse(JSON.stringify(page))
  delete bad.children[0].minWidth
  const report = inspectGeneratedDesignQualityReport(JSON.stringify(bad))
  const touch = report.diagnostics.find((line) => line.includes('44px minimum'))
  assert.ok(touch !== undefined, 'the touch-target diagnostic must fire for the unsized hitbox')
  assert.match(touch, /minWidth\/minHeight/, 'the hint must point at minWidth/minHeight, not a fixed width')
  assert.match(touch, /fit_content/, 'the hint must say the fit_content sizing stays')
  const repair = report.repairTargets.find((target) => target.rule === 'touch-target')
  assert.ok(repair !== undefined && repair.patch.minWidth === 44, 'repair target must use minWidth, not width')
})

test('a fixed 44px hitbox with padding + text child still trips container-flow (the contradiction is real, the hint must route around it)', () => {
  const bad = JSON.parse(JSON.stringify(page))
  bad.children[0].width = 44
  delete bad.children[0].minWidth
  const report = inspectGeneratedDesignQualityReport(JSON.stringify(bad))
  assert.ok(
    report.diagnostics.some((line) => /cannot contain its padding, gap, and resolvable child flow/.test(line)),
    'container-flow must reject the fixed 44px hitbox',
  )
})
