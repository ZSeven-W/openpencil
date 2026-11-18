import assert from 'node:assert/strict'
import { test } from 'node:test'

const {
  assertGeneratedDesignQuality,
  inspectGeneratedDesignQuality,
  inspectGeneratedDesignQualityReport,
  inspectGeneratedDraftStructureReport,
} = await import('../lib/design-quality.js')

const readableText = {
  fontFamily: 'system-ui',
  fontSize: 16,
  lineHeight: 1.5,
}

function solid(color) {
  return [{ type: 'solid', color }]
}

function documentWith(children) {
  return JSON.stringify({ version: '1.0.0', children })
}

test('accepts an empty unpublished starter draft', () => {
  const starter = documentWith([])
  assert.deepEqual(inspectGeneratedDesignQuality(starter), [])
  assert.doesNotThrow(() => assertGeneratedDesignQuality(starter))
})

test('flags undersized and non-filling native controls in a login.op-style form', () => {
  const documentJson = documentWith([{
    type: 'frame',
    name: 'Login',
    children: [{
      type: 'frame',
      name: 'Form card',
      children: [
        { type: 'text_input', name: 'Email', width: 116, height: 36 },
        { type: 'text_input', name: 'Password', secure: true, width: 'fill_container', height: 'fit_content' },
        { type: 'select', name: 'Account type', width: 'fit_content', height: 72 },
        { type: 'text_area', name: 'Private biography', width: 'fill_container', height: 48 },
      ],
    }],
  }])

  const issues = inspectGeneratedDesignQuality(documentJson)
  assert.equal(issues.length, 6)
  assert.equal(issues.filter(issue => /width/.test(issue)).length, 2)
  assert.equal(issues.filter(issue => /height/.test(issue)).length, 4)
  assert.ok(issues.every(issue => !issue.includes('Email') && !issue.includes('Password')))
  assert.throws(
    () => assertGeneratedDesignQuality(documentJson),
    error => error instanceof Error
      && /failed quality checks \(6 issues\)/.test(error.message)
      && /width to "fill_container"/.test(error.message)
      && /44px through 52px/.test(error.message)
      && /multiline height from 88px through 240px/.test(error.message)
      && !error.message.includes('Account type')
      && !error.message.includes('Private biography'),
  )
})

test('accepts healthy native controls in auth and sign-up forms', () => {
  const documentJson = documentWith([{
    type: 'frame',
    role: 'auth-form',
    children: [
      { type: 'text_input', width: 'fill_container', height: 44 },
      { type: 'text_area', width: 'fill_container', height: 112 },
      { type: 'select', width: 'fill_container', height: 48 },
    ],
  }, {
    type: 'frame',
    name: 'Sign-up',
    children: [{ type: 'text_input', width: 'fill_container', height: 52 }],
  }])

  assert.deepEqual(inspectGeneratedDesignQuality(documentJson), [])
  assert.doesNotThrow(() => assertGeneratedDesignQuality(documentJson))
})

test('allows a fixed-width control in a horizontal newsletter action row', () => {
  const documentJson = documentWith([{
    type: 'frame',
    name: 'Newsletter form',
    layout: 'horizontal',
    children: [
      { type: 'text_input', id: 'email', width: 320, height: 48 },
      {
        type: 'frame', role: 'button', width: 120, height: 48,
        children: [{ type: 'text', content: 'Subscribe', ...readableText }],
      },
    ],
  }])

  const report = inspectGeneratedDesignQualityReport(documentJson)
  assert.deepEqual(report.diagnostics, [])
  assert.deepEqual(report.repairTargets, [])
})

test('returns exact safe repair targets for malformed vertical form controls', () => {
  const documentJson = documentWith([{
    type: 'frame',
    name: 'Login form',
    children: [
      { type: 'text_input', id: 'email', width: 120, height: 32 },
      { type: 'text_input', id: 'password', name: 'Password', width: 120, height: 80 },
      { type: 'text_area', id: 'bio', width: 120, height: 40 },
    ],
  }])

  const report = inspectGeneratedDesignQualityReport(documentJson)
  assert.deepEqual(report.repairTargetSummary, { total: 3, returned: 3, omitted: 0 })
  assert.deepEqual([...report.repairTargets].sort((a, b) => a.nodeId.localeCompare(b.nodeId)), [{
    nodeId: 'bio', operation: 'U', rule: 'form-control',
    patch: { width: 'fill_container', height: 120 },
  }, {
    nodeId: 'email', operation: 'U', rule: 'form-control',
    patch: { width: 'fill_container', height: 44 },
  }, {
    nodeId: 'password', operation: 'U', rule: 'form-control',
    patch: { secure: true, width: 'fill_container', height: 44 },
  }])
})

test('does not apply form sizing rules outside forms or to compact search and OTP controls', () => {
  const documentJson = documentWith([{
    type: 'frame',
    name: 'Dashboard',
    children: [{ type: 'text_input', name: 'Loose filter', width: 160, height: 32 }],
  }, {
    type: 'frame',
    name: 'Login form',
    children: [
      { type: 'text_input', role: 'searchbox', width: 180, height: 32 },
      { type: 'frame', name: 'OTP verification code', children: [
        { type: 'text_input', width: 40, height: 40 },
      ] },
    ],
  }])

  assert.deepEqual(inspectGeneratedDesignQuality(documentJson), [])
})

test('flags emoji in icon fields and emoji-only text icon nodes', () => {
  const documentJson = documentWith([{
    type: 'frame',
    leadingIcon: '🔍',
    children: [
      { type: 'text_input', trailingIcon: { glyph: '👁️' } },
      { type: 'text', role: 'icon', content: '🔒', ...readableText },
      { type: 'text', name: 'Search icon', content: '🔎  🔍', ...readableText },
      { type: 'text', role: 'icon', content: 'Secure 🔒', ...readableText },
      { type: 'icon_font', icon: 'search', width: 20, height: 20 },
    ],
  }])

  const issues = inspectGeneratedDesignQuality(documentJson)
  assert.equal(issues.length, 4)
  assert.equal(issues.filter(issue => /icon interface field/.test(issue)).length, 2)
  assert.equal(issues.filter(issue => /text node used as an icon/.test(issue)).length, 2)
  assert.throws(
    () => assertGeneratedDesignQuality(documentJson),
    error => error instanceof Error
      && /use icon_font or a component icon/.test(error.message)
      && !error.message.includes('🔒')
      && !error.message.includes('🔍'),
  )
})

test('returns safe repair targets for icon glyphs without render dimensions', () => {
  const report = inspectGeneratedDesignQualityReport(documentWith([{
    type: 'frame',
    children: [
      { type: 'icon_font', id: 'missing', iconFontName: 'search' },
      { type: 'icon_font', id: 'one-axis', iconFontName: 'menu', width: 18 },
      { type: 'icon_font', id: 'healthy', iconFontName: 'heart', width: 20, height: 22 },
    ],
  }]))

  assert.match(report.diagnostics.join(' '), /2 icon nodes.*positive numeric width and height/i)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'missing', operation: 'U', rule: 'icon-size', patch: { width: 24, height: 24 },
  }, {
    nodeId: 'one-axis', operation: 'U', rule: 'icon-size', patch: { height: 18 },
  }])
  assert.deepEqual(report.repairTargetSummary, { total: 2, returned: 2, omitted: 0 })
})

test('fails closed on parse errors and bounds diagnostics without reflecting document text', () => {
  assert.deepEqual(
    inspectGeneratedDesignQuality('{ definitely not JSON'),
    ['OpenPencil generated document is not valid JSON.'],
  )
  assert.throws(
    () => assertGeneratedDesignQuality('{ secret-invalid-document'),
    error => error instanceof Error && !error.message.includes('secret-invalid-document'),
  )

  const children = Array.from({ length: 80 }, (_, index) => ({
    type: 'text',
    role: 'icon',
    name: `private-${index}-${'x'.repeat(500)}`,
    content: '🚫',
    ...readableText,
  }))
  const issues = inspectGeneratedDesignQuality(documentWith(children))
  assert.equal(issues.length, 4)
  assert.ok(issues.every(issue => issue.length <= 200))
  assert.ok(issues.every(issue => !issue.includes('private-')))
  assert.throws(
    () => assertGeneratedDesignQuality(documentWith(children)),
    error => error instanceof Error
      && error.message.length <= 1_600
      && !error.message.includes('private-')
      && !error.message.includes('🚫'),
  )
})

test('aggregates every safe missing-typography node into one bounded repair batch', () => {
  const children = Array.from({ length: 7 }, (_, index) => ({
    id: `text-${index}`,
    type: 'text',
    name: `private-name-${index}`,
    content: `private-copy-${index}`,
    fill: solid('#111'),
  }))
  const report = inspectGeneratedDesignQualityReport(documentWith(children))
  assert.equal(report.diagnostics.filter(issue => /explicitly set fontFamily/.test(issue)).length, 1)
  assert.match(report.diagnostics.join(' '), /7 authored text nodes/i)
  assert.deepEqual(report.repairTargetSummary, { total: 7, returned: 7, omitted: 0 })
  assert.deepEqual(
    report.repairTargets.map(target => target.nodeId).sort(),
    children.map(child => child.id).sort(),
  )
  assert.ok(report.repairTargets.every(target => (
    target.operation === 'U'
    && target.rule === 'typography'
    && target.patch.fontFamily === 'Inter, system-ui, sans-serif'
    && target.patch.fontSize === 16
    && target.patch.lineHeight === 1.5
  )))
  assert.doesNotMatch(JSON.stringify(report), /private-name|private-copy/)

  const bounded = inspectGeneratedDesignQualityReport(documentWith(Array.from({ length: 600 }, (_, index) => ({
    id: `bounded-text-${index}`,
    type: 'text',
    content: `secret-${index}`,
  }))))
  assert.deepEqual(bounded.repairTargetSummary, { total: 600, returned: 512, omitted: 88 })
  assert.equal(bounded.repairTargets.length, 512)
  assert.doesNotMatch(JSON.stringify(bounded), /secret-/)
})

test('repairs bare Inter with a portable stack and accepts Inter with a generic sans fallback', () => {
  const report = inspectGeneratedDesignQualityReport(documentWith([{
    id: 'bare-inter',
    type: 'text',
    content: 'Bare Inter',
    fontFamily: 'Inter',
    fontSize: 16,
    lineHeight: 1.5,
  }, {
    id: 'portable-inter',
    type: 'text',
    content: 'Portable Inter',
    fontFamily: 'Inter, sans-serif',
    fontSize: 16,
    lineHeight: 1.5,
  }, {
    id: 'portable-system-stack',
    type: 'text',
    content: 'Portable system stack',
    fontFamily: 'Inter, system-ui, sans-serif',
    fontSize: 16,
    lineHeight: 1.5,
  }]))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 1)
  assert.match(report.diagnostics[0], /Inter without a portable generic fallback/)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'bare-inter', operation: 'U', rule: 'typography',
    patch: { fontFamily: 'Inter, system-ui, sans-serif' },
  }])
  const repaired = {
    id: 'bare-inter', type: 'text', content: 'Bare Inter', fontSize: 16, lineHeight: 1.5,
    ...report.repairTargets[0].patch,
  }
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([repaired])).diagnostics, [])
})

test('repairs only strictly reversible UTF-8-as-Windows-1252 mojibake text', () => {
  const children = [{
    id: 'brand-mojibake', type: 'text', content: 'M\u00c3\u00a9ridien', ...readableText,
  }, {
    id: 'quote-mojibake', type: 'text', content: 'It\u00e2\u20ac\u2122s ready', ...readableText,
  }, {
    id: 'normal-accent', type: 'text', content: 'M\u00e9ridien', ...readableText,
  }, {
    id: 'normal-chinese', type: 'text', content: '正常中文', ...readableText,
  }, {
    id: 'normal-ascii', type: 'text', content: 'Plain ASCII', ...readableText,
  }, {
    id: 'mixed-unsafe', type: 'text', content: '\u00e9 plus M\u00c3\u00a9', ...readableText,
  }]
  const report = inspectGeneratedDesignQualityReport(documentWith(children))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 1)
  assert.match(report.diagnostics[0], /2 text nodes contain reversible UTF-8-as-Windows-1252 mojibake/)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'brand-mojibake', operation: 'U', rule: 'canvas-contract', patch: { content: 'M\u00e9ridien' },
  }, {
    nodeId: 'quote-mojibake', operation: 'U', rule: 'canvas-contract', patch: { content: 'It\u2019s ready' },
  }])

  const repaired = structuredClone(children)
  for (const target of report.repairTargets) {
    Object.assign(repaired.find(node => node.id === target.nodeId), target.patch)
  }
  const repairedReport = inspectGeneratedDesignQualityReport(documentWith(repaired))
  assert.deepEqual(repairedReport.diagnostics, [])
  assert.deepEqual(repairedReport.repairTargets, [])
})

test('rejects the realistic v2 audit shape across every hard release gate', () => {
  const v2AuditDocument = documentWith([{
    type: 'frame',
    name: 'private-root-name',
    width: 390,
    height: 480,
    layout: 'vertical',
    gap: 16,
    padding: [20, 20, 28, 20],
    clipContent: true,
    fill: solid('#14201B'),
    children: [
      {
        type: 'frame',
        role: 'status-bar',
        width: 'fill_container',
        height: 62,
        children: [{ type: 'text', content: '9:41', fill: solid('#FFF') }],
      },
      {
        type: 'frame',
        role: 'icon-button',
        width: 36,
        height: 36,
        children: [{ type: 'icon_font', iconFontName: 'arrow-left', width: 18, height: 18 }],
      },
      { type: 'text', content: 'private missing typography', fill: solid('#FFF') },
      {
        type: 'text',
        content: '欢迎回来',
        fontFamily: 'Inter',
        fontSize: 16,
        lineHeight: 1.2,
        fill: solid('#FFF'),
      },
      {
        type: 'frame',
        role: 'form-group',
        width: 'fill_container',
        height: 'fit_content',
        children: [{
          type: 'text_input',
          name: 'private password',
          width: 'fill_container',
          height: 48,
        }],
      },
      {
        type: 'frame',
        role: 'checkbox-row',
        layout: 'horizontal',
        width: 'fill_container',
        height: 24,
        children: [{ type: 'checkbox', width: 18, height: 18 }],
      },
      {
        type: 'frame',
        role: 'button',
        width: 'fill_container',
        height: 48,
        children: [
          { type: 'icon_font', iconFontName: 'message-circle', width: 18, height: 18 },
          { type: 'text', content: '微信登录', ...readableText, fill: solid('#FFF') },
        ],
      },
      {
        type: 'frame',
        role: 'button',
        width: 'fill_container',
        height: 48,
        children: [
          { type: 'icon_font', iconFontName: 'apple', width: 18, height: 18 },
          {
            type: 'text',
            content: 'Apple sign in',
            fontFamily: 'Private Serif Brand',
            fontSize: 16,
            lineHeight: 1.5,
            fill: solid('#FFF'),
          },
        ],
      },
      {
        type: 'frame',
        width: 'fill_container',
        height: 'fit_content',
        fill: solid('#FFF'),
        children: [{
          type: 'text',
          content: 'private low contrast copy',
          fontFamily: 'Private Mono Brand',
          fontSize: 16,
          lineHeight: 1.5,
          fill: solid('#777'),
        }],
      },
      { type: 'frame', width: 'fill_container', height: 100, children: [] },
      { type: 'frame', width: 'fill_container', height: 'fill_container', children: [] },
    ],
  }])

  const issues = inspectGeneratedDesignQuality(v2AuditDocument)
  const diagnostic = issues.join('\n')
  assert.match(diagnostic, /34px of bottom safe-area/)
  assert.match(diagnostic, /44px minimum/)
  assert.match(diagnostic, /password text input must explicitly set secure/)
  assert.match(diagnostic, /checkbox interaction row must provide a 44px wrapper/)
  assert.match(diagnostic, /approved WeChat brand icon mapping/)
  assert.match(diagnostic, /explicitly set fontFamily, fontSize, and lineHeight/)
  assert.match(diagnostic, /CJK text node must use a lineHeight of at least 1.3/)
  assert.match(diagnostic, /no more than two normalized font families/)
  assert.match(diagnostic, /WCAG AA contrast/)
  assert.match(diagnostic, /approved Apple brand icon mapping/)
  assert.ok(!diagnostic.includes('private'))
  assert.throws(() => assertGeneratedDesignQuality(v2AuditDocument))
})

test('rejects the shop-home-v4 empty header, root-edge content, and collapsed search flow', () => {
  const documentJson = documentWith([{
    type: 'frame',
    name: 'Generated Page private-root',
    width: 390,
    height: 'fit_content',
    layout: 'vertical',
    padding: 0,
    gap: 16,
    children: [
      {
        type: 'frame',
        name: 'Header private-empty-shell',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'horizontal',
        padding: [12, 24, 4, 24],
        children: [],
      },
      {
        type: 'text',
        name: 'Brand private-edge-copy',
        content: 'Nook private-authored-copy',
        width: 'fit_content',
        height: 'fit_content',
        ...readableText,
      },
      {
        type: 'frame',
        name: 'Search Content Rail private-safe-wrapper',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        padding: [0, 24, 0, 24],
        children: [{
          id: 'shop-home-v4-search-row',
          type: 'frame',
          name: 'Search row private-collapsed',
          width: 'fill_container',
          height: 18,
          layout: 'horizontal',
          padding: [8, 10, 8, 12],
          gap: 8,
          children: [
            { type: 'icon_font', name: 'Search icon private-glyph', width: 18, height: 18 },
            {
              type: 'text',
              name: 'Search placeholder private-copy',
              content: '搜索商品 private-authored-copy',
              width: 'fit_content',
              height: 'fit_content',
              ...readableText,
            },
          ],
        }],
      },
      {
        type: 'frame',
        name: 'Cart button private-edge-control',
        width: 40,
        height: 40,
        layout: 'horizontal',
        children: [{ type: 'icon_font', width: 20, height: 20 }],
      },
      {
        type: 'text',
        name: 'Section title private-edge-copy',
        content: '快速分类 private-authored-copy',
        width: 'fit_content',
        height: 'fit_content',
        ...readableText,
      },
      {
        type: 'text',
        name: 'Featured title private-edge-copy',
        content: '今日主推 private-authored-copy',
        width: 'fit_content',
        height: 'fit_content',
        ...readableText,
      },
      {
        type: 'frame',
        name: 'Bottom nav private-chrome',
        width: 'fill_container',
        height: 72,
        layout: 'horizontal',
        padding: [8, 16, 8, 16],
        children: [{ type: 'icon_font', width: 20, height: 20 }],
      },
    ],
  }])

  const report = inspectGeneratedDesignQualityReport(documentJson)
  const diagnostic = report.diagnostics.join('\n')
  assert.match(diagnostic, /^0\.0: frame semantic header, navigation, footer, or toolbar container/m)
  assert.match(diagnostic, /^0\.1: text mobile vertical root must wrap direct text/m)
  assert.match(diagnostic, /^0\.3: frame mobile vertical root must wrap direct text/m)
  assert.match(diagnostic, /^0\.4: text mobile vertical root must wrap direct text/m)
  assert.match(diagnostic, /^0\.5: text mobile vertical root must wrap direct text/m)
  assert.match(diagnostic, /^0\.2\.0: frame fixed horizontal container height cannot contain/m)
  assert.doesNotMatch(diagnostic, /private-|Nook|搜索商品|快速分类|今日主推/)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'shop-home-v4-search-row',
    operation: 'U',
    rule: 'flow-size',
    patch: { height: 40 },
  }])
  assert.throws(() => assertGeneratedDesignQuality(documentJson))
})

test('accepts a standard zero-root-padding mobile header and content rail', () => {
  const documentJson = documentWith([{
    type: 'frame',
    width: 390,
    height: 'fit_content',
    layout: 'vertical',
    padding: 0,
    gap: 16,
    children: [
      {
        type: 'frame',
        role: 'header',
        width: 'fill_container',
        height: 64,
        layout: 'horizontal',
        padding: [0, 24, 0, 24],
        gap: 12,
        children: [
          { type: 'text', content: 'Nook', width: 'fit_content', height: 'fit_content', ...readableText },
          {
            type: 'frame',
            role: 'icon-button',
            width: 44,
            height: 44,
            children: [{ type: 'icon_font', width: 20, height: 20 }],
          },
        ],
      },
      {
        type: 'frame',
        name: 'Content Rail',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        padding: [0, 24, 0, 24],
        gap: 12,
        children: [
          { type: 'text', content: 'Featured', width: 'fit_content', height: 'fit_content', ...readableText },
          {
            type: 'frame',
            width: 'fill_container',
            height: 48,
            layout: 'horizontal',
            padding: [12, 16, 12, 16],
            children: [{ type: 'text', content: 'Search', width: 'fit_content', height: 'fit_content', ...readableText }],
          },
        ],
      },
      {
        type: 'frame',
        role: 'footer',
        width: 'fill_container',
        height: 64,
        layout: 'horizontal',
        padding: [0, 24, 0, 24],
        children: [{ type: 'icon_font', width: 20, height: 20 }],
      },
    ],
  }])

  assert.deepEqual(inspectGeneratedDesignQuality(documentJson), [])
  assert.doesNotThrow(() => assertGeneratedDesignQuality(documentJson))
})

test('exempts explicit overlays and floating actions from the mobile content rail rule', () => {
  const documentJson = documentWith([{
    type: 'frame',
    width: 390,
    height: 'fit_content',
    layout: 'vertical',
    padding: 0,
    children: [
      {
        type: 'frame',
        role: 'overlay',
        width: 40,
        height: 40,
        children: [{ type: 'icon_font', width: 20, height: 20 }],
      },
      {
        type: 'frame',
        semantics: { role: 'floating-action-button' },
        width: 56,
        height: 56,
        children: [{ type: 'icon_font', width: 24, height: 24 }],
      },
      {
        type: 'text',
        content: 'Overlay label',
        width: 'fit_content',
        height: 'fit_content',
        x: 24,
        y: 96,
        constraints: { h: 'left', v: 'top' },
        ...readableText,
      },
    ],
  }])

  assert.deepEqual(inspectGeneratedDesignQuality(documentJson), [])
})

test('accepts a healthy mobile fixture with brand mappings, safe geometry, and normalized fonts', () => {
  const documentJson = documentWith([{
    type: 'frame',
    width: 390,
    height: 844,
    layout: 'vertical',
    gap: 12,
    padding: [20, 20, 34, 20],
    clipContent: true,
    fill: solid('#FFF'),
    children: [
      {
        type: 'frame',
        role: 'status-bar',
        width: 'fill_container',
        height: 44,
        children: [{ type: 'text', content: '9:41', fill: solid('#111') }],
      },
      {
        type: 'frame',
        role: 'icon-button',
        width: 44,
        height: 44,
        children: [{ type: 'icon_font', iconFontName: 'arrow-left', width: 18, height: 18 }],
      },
      {
        type: 'text',
        content: 'Welcome',
        fontFamily: 'system-ui, -apple-system, sans-serif',
        fontSize: 28,
        lineHeight: 1.3,
        fill: solid('#111'),
      },
      {
        type: 'frame',
        role: 'auth-form',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        gap: 12,
        children: [{
          type: 'text_input',
          semantics: { label: 'Password' },
          secure: true,
          width: 'fill_container',
          height: 48,
        }, {
          type: 'frame',
          role: 'checkbox-row',
          width: 'fill_container',
          height: 44,
          layout: 'horizontal',
          children: [{ type: 'checkbox', width: 18, height: 18 }],
        }],
      },
      {
        type: 'frame',
        role: 'button',
        width: 'fill_container',
        height: 44,
        children: [
          { type: 'path', iconId: 'simple-icons:wechat', d: 'M1 1h22v22H1z', width: 18, height: 18 },
          { type: 'text', content: '微信登录', ...readableText, fill: solid('#111') },
        ],
      },
      {
        type: 'frame',
        role: 'button',
        width: 'fill_container',
        height: 44,
        children: [
          { type: 'icon_font', iconFontFamily: 'simple-icons', iconFontName: 'apple', width: 18, height: 18 },
          {
            type: 'text',
            content: 'Apple sign in',
            fontFamily: 'Inter, sans-serif',
            fontSize: 16,
            lineHeight: 1.5,
            fill: solid('#111'),
          },
        ],
      },
      { type: 'checkbox', width: 18, height: 18 },
      { type: 'frame', width: 'fill_container', height: 'fill_container', children: [] },
    ],
  }])

  assert.deepEqual(inspectGeneratedDesignQuality(documentJson), [])
  assert.doesNotThrow(() => assertGeneratedDesignQuality(documentJson))
})

test('accepts explicit bottom-nav padding or provable trailing flow space but not clip-only claims', () => {
  const withBottomNav = documentWith([{
    type: 'frame',
    width: 390,
    height: 600,
    layout: 'vertical',
    children: [
      { type: 'frame', role: 'status-bar', width: 'fill_container', height: 44, children: [] },
      { type: 'frame', width: 'fill_container', height: 'fill_container', children: [] },
      {
        type: 'frame',
        role: 'tab-bar',
        width: 'fill_container',
        height: 80,
        padding: [8, 8, 34, 8],
        children: [],
      },
    ],
  }])
  const withTrailingSpace = documentWith([{
    type: 'frame',
    width: 390,
    height: 600,
    layout: 'vertical',
    children: [
      { type: 'frame', role: 'status-bar', width: 'fill_container', height: 44, children: [] },
      { type: 'frame', width: 'fill_container', height: 100, children: [] },
    ],
  }])
  const clipOnly = documentWith([{
    type: 'frame',
    width: 390,
    height: 600,
    layout: 'vertical',
    clipContent: false,
    children: [
      { type: 'frame', role: 'status-bar', width: 'fill_container', height: 44, children: [] },
      { type: 'frame', width: 'fill_container', height: 'fill_container', children: [] },
      { type: 'frame', width: 'fill_container', height: 30, children: [] },
    ],
  }])

  assert.deepEqual(inspectGeneratedDesignQuality(withBottomNav), [])
  assert.deepEqual(inspectGeneratedDesignQuality(withTrailingSpace), [])
  assert.match(inspectGeneratedDesignQuality(clipOnly).join('\n'), /34px of bottom safe-area/)
})

test('requires 44px authored hit boxes for button, icon-button, and nav-link roles', () => {
  const documentJson = documentWith([{
    type: 'frame',
    children: [
      {
        id: 'n-button', type: 'frame', role: 'button', width: 43, height: 44,
        children: [{ type: 'text', content: 'Go', ...readableText }],
      },
      {
        id: 'n-icon', type: 'frame', semantics: { role: 'icon-button' }, width: 44, height: 43,
        children: [{ type: 'icon_font', iconFontName: 'menu', width: 20, height: 20 }],
      },
      {
        id: 'n-nav', type: 'frame', role: 'nav-link', width: 'fit_content', height: 44,
        children: [{ type: 'text', content: 'Home', ...readableText }],
      },
    ],
  }])

  const report = inspectGeneratedDesignQualityReport(documentJson)
  const issues = report.diagnostics.filter(issue => /44px minimum/.test(issue))
  assert.equal(issues.length, 3)
  assert.ok(issues.some(issue => /^0\.0: frame node n-button .*width/.test(issue)))
  assert.ok(issues.some(issue => /^0\.1: frame node n-icon .*height/.test(issue)))
  assert.ok(issues.some(issue => /^0\.2: frame node n-nav .*width/.test(issue)))
  assert.deepEqual(report.repairTargets, [
    { nodeId: 'n-button', operation: 'U', rule: 'touch-target', patch: { width: 44 } },
    { nodeId: 'n-icon', operation: 'U', rule: 'touch-target', patch: { height: 44 } },
    { nodeId: 'n-nav', operation: 'U', rule: 'touch-target', patch: { minWidth: 44 } },
  ])

  const repaired = JSON.parse(documentJson)
  for (const target of report.repairTargets) {
    const node = repaired.children[0].children.find(candidate => candidate.id === target.nodeId)
    Object.assign(node, target.patch)
  }
  assert.deepEqual(inspectGeneratedDesignQualityReport(JSON.stringify(repaired)).diagnostics, [])
})

test('computes fixed vertical and horizontal flow minima recursively while fill spacers cost zero', () => {
  const vertical = documentWith([{
    type: 'frame',
    width: 200,
    height: 90,
    layout: 'vertical',
    gap: 5,
    padding: 10,
    clipContent: false,
    children: [
      { type: 'frame', width: 'fill_container', height: 40, children: [] },
      {
        type: 'frame',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        children: [{ type: 'text', content: 'two lines are conservatively measured', width: 80, ...readableText }],
      },
      { type: 'frame', width: 'fill_container', height: 'fill_container', children: [] },
    ],
  }])
  const horizontal = documentWith([{
    type: 'frame',
    width: 80,
    height: 100,
    layout: 'horizontal',
    gap: 5,
    padding: 10,
    clipContent: false,
    children: [
      { type: 'frame', width: 40, height: 'fill_container', children: [] },
      { type: 'text', content: 'Wide', width: 'fit_content', ...readableText },
      { type: 'frame', width: 'fill_container', height: 'fill_container', children: [] },
    ],
  }])

  assert.match(inspectGeneratedDesignQuality(vertical).join('\n'), /fixed vertical container height/)
  assert.match(inspectGeneratedDesignQuality(horizontal).join('\n'), /fixed horizontal container width/)
})

test('exempts clipped and semantic scroll containers on the main axis only', () => {
  const report = inspectGeneratedDesignQualityReport(documentWith([{
    id: 'clipped-carousel',
    type: 'frame',
    width: 60,
    height: 44,
    layout: 'horizontal',
    clipContent: true,
    padding: 8,
    gap: 8,
    children: [
      { type: 'frame', width: 40, height: 20, children: [] },
      { type: 'frame', width: 40, height: 20, children: [] },
    ],
  }, {
    id: 'semantic-carousel',
    type: 'frame',
    name: 'Product carousel',
    width: 60,
    height: 44,
    layout: 'horizontal',
    padding: 8,
    gap: 8,
    children: [
      { type: 'frame', width: 40, height: 20, children: [] },
      { type: 'frame', width: 40, height: 20, children: [] },
    ],
  }, {
    id: 'semantic-listbox',
    type: 'frame',
    role: 'listbox',
    width: 100,
    height: 40,
    layout: 'vertical',
    padding: 8,
    gap: 8,
    children: [
      { type: 'frame', width: 80, height: 32, children: [] },
      { type: 'frame', width: 80, height: 32, children: [] },
    ],
  }]))

  assert.deepEqual(report.diagnostics, [])
  assert.deepEqual(report.repairTargets, [])
  assert.deepEqual(report.repairTargetSummary, { total: 0, returned: 0, omitted: 0 })

  const crossAxisReport = inspectGeneratedDesignQualityReport(documentWith([{
    id: 'cross-axis-overflow',
    type: 'frame',
    width: 60,
    height: 30,
    layout: 'horizontal',
    clipContent: true,
    padding: 8,
    gap: 8,
    children: [
      { type: 'frame', width: 40, height: 20, children: [] },
      { type: 'frame', width: 40, height: 20, children: [] },
    ],
  }]))
  assert.equal(crossAxisReport.diagnostics.length, 1)
  assert.match(crossAxisReport.diagnostics[0], /fixed horizontal container height/)
  assert.deepEqual(crossAxisReport.repairTargets, [{
    nodeId: 'cross-axis-overflow',
    operation: 'U',
    rule: 'flow-size',
    patch: { height: 36 },
  }])
})

test('returns a merged fixed-flow size repair and becomes clean after the equivalent U patch', () => {
  const overflow = {
    id: 'compact-flow',
    type: 'frame',
    width: 50,
    height: 18,
    layout: 'horizontal',
    padding: [8, 10, 8, 12],
    gap: 8,
    children: [
      { type: 'icon_font', width: 18, height: 18 },
      { type: 'text', content: 'Go', width: 20, height: 20, ...readableText },
    ],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([overflow]))

  assert.equal(report.diagnostics.filter(issue => /fixed horizontal container/.test(issue)).length, 2)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'compact-flow',
    operation: 'U',
    rule: 'flow-size',
    patch: { width: 68, height: 36 },
  }])
  assert.deepEqual(report.repairTargetSummary, { total: 1, returned: 1, omitted: 0 })

  const repaired = structuredClone(overflow)
  Object.assign(repaired, report.repairTargets[0].patch)
  const repairedReport = inspectGeneratedDesignQualityReport(documentWith([repaired]))
  assert.deepEqual(repairedReport.diagnostics, [])
  assert.deepEqual(repairedReport.repairTargets, [])
  assert.deepEqual(repairedReport.repairTargetSummary, { total: 0, returned: 0, omitted: 0 })

  const withoutId = structuredClone(overflow)
  delete withoutId.id
  const withoutIdReport = inspectGeneratedDesignQualityReport(documentWith([withoutId]))
  assert.equal(withoutIdReport.diagnostics.filter(issue => /fixed horizontal container/.test(issue)).length, 2)
  assert.deepEqual(withoutIdReport.repairTargets, [])
  assert.deepEqual(withoutIdReport.repairTargetSummary, { total: 0, returned: 0, omitted: 0 })
})

test('repairs excessive repeated-card rail height and mixed fixed/fill card widths in one pass', () => {
  const rail = {
    id: 'n31',
    type: 'frame',
    name: 'Product rail',
    width: 'fill_container',
    height: 696,
    layout: 'horizontal',
    gap: 12,
    padding: [0, 20, 0, 20],
    children: [{
      id: 'n32', type: 'frame', name: 'Product card 0', role: 'card', width: 168, height: 232, children: [],
    }, {
      id: 'n37', type: 'frame', name: 'Product card 1', role: 'card', width: 'fill_container', height: 232, children: [],
    }, {
      id: 'n42', type: 'frame', name: 'Product card 2', role: 'product-card', width: 'fill_container', height: 232, children: [],
    }],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([rail]))
  const draftReport = inspectGeneratedDraftStructureReport(documentWith([rail]))

  assert.equal(report.diagnostics.length, 2)
  assert.deepEqual(draftReport.diagnostics, report.diagnostics)
  assert.match(report.diagnostics[0], /repeated-card rail height leaves excessive empty space/)
  assert.match(report.diagnostics[1], /must not mix one consistent fixed card width with fill-container/)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'n31', operation: 'U', rule: 'flow-size', patch: { height: 232 },
  }, {
    nodeId: 'n37', operation: 'U', rule: 'flow-size', patch: { width: 168 },
  }, {
    nodeId: 'n42', operation: 'U', rule: 'flow-size', patch: { width: 168 },
  }])
  assert.deepEqual(report.repairTargetSummary, { total: 3, returned: 3, omitted: 0 })

  const repaired = structuredClone(rail)
  for (const target of report.repairTargets) {
    const node = target.nodeId === repaired.id
      ? repaired
      : repaired.children.find(candidate => candidate.id === target.nodeId)
    Object.assign(node, target.patch)
  }
  const repairedReport = inspectGeneratedDesignQualityReport(documentWith([repaired]))
  assert.deepEqual(repairedReport.diagnostics, [])
  assert.deepEqual(repairedReport.repairTargets, [])
  assert.deepEqual(repairedReport.repairTargetSummary, { total: 0, returned: 0, omitted: 0 })
})

test('draft gate blocks compact product rails that overflow without explicit scroller intent', () => {
  const card = id => ({
    id, type: 'frame', role: 'product-card', width: 168, height: 232,
    children: [{ type: 'text', content: id, ...readableText }],
  })
  const root = {
    id: 'mobile-root', type: 'frame', width: 390, height: 'fit_content', layout: 'vertical',
    children: [{
      id: 'product-rail', type: 'frame', name: 'Product rail', width: 'fill_container',
      height: 'fit_content', layout: 'horizontal', gap: 12, padding: [0, 20],
      children: [card('one'), card('two'), card('three')],
    }],
  }

  const blocked = inspectGeneratedDraftStructureReport(documentWith([root]))
  assert.equal(blocked.unrepairableDiagnosticCount, 1)
  assert.match(blocked.diagnostics[0], /must fit the viewport or declare explicit clipped scroller intent/)

  root.children[0].children[1].width = 'fill_container'
  root.children[0].children[2].width = 'fill_container'
  const mixed = inspectGeneratedDraftStructureReport(documentWith([root]))
  assert.equal(mixed.diagnostics.length, 2)
  assert.match(mixed.diagnostics[0], /must not mix one consistent fixed card width with fill-container/)
  assert.match(mixed.diagnostics[1], /must fit the viewport or declare explicit clipped scroller intent/)

  root.children[0].clipContent = true
  const clipped = inspectGeneratedDraftStructureReport(documentWith([root]))
  assert.equal(clipped.diagnostics.length, 1)
  assert.match(clipped.diagnostics[0], /must not mix one consistent fixed card width with fill-container/)
})

test('does not apply repeated-card rail repairs to ordinary headers, buttons, or hero rows', () => {
  const report = inspectGeneratedDesignQualityReport(documentWith([{
    id: 'header',
    type: 'frame',
    role: 'header',
    width: 'fill_container',
    height: 696,
    layout: 'horizontal',
    children: [{
      id: 'primary-action', type: 'frame', role: 'button', width: 168, height: 232, fill: solid('#111'),
      children: [{ type: 'text', content: 'Primary', ...readableText }],
    }, {
      id: 'secondary-action', type: 'frame', role: 'button', width: 'fill_container', height: 232, fill: solid('#EEE'),
      children: [{ type: 'text', content: 'Secondary', ...readableText }],
    }],
  }, {
    id: 'hero',
    type: 'frame',
    name: 'Hero',
    width: 'fill_container',
    height: 696,
    layout: 'horizontal',
    children: [{
      id: 'hero-copy', type: 'frame', role: 'card', width: 168, height: 232, children: [],
    }, {
      id: 'hero-visual', type: 'frame', role: 'card', width: 'fill_container', height: 232, children: [],
    }],
  }]))

  assert.deepEqual(report.diagnostics, [])
  assert.deepEqual(report.repairTargets, [])
  assert.deepEqual(report.repairTargetSummary, { total: 0, returned: 0, omitted: 0 })
})

test('hides unresolved searched images and collapses their fixed card and repeated-card rail', () => {
  const rail = {
    id: 'product-rail',
    type: 'frame',
    width: 'fill_container',
    height: 696,
    layout: 'horizontal',
    gap: 12,
    children: [{
      id: 'product-card-0',
      type: 'frame',
      role: 'card',
      width: 168,
      height: 232,
      layout: 'vertical',
      children: [{
        id: 'product-image-0',
        type: 'image',
        src: 'placeholder://image-search-failed',
        imageSearchQuery: 'private product query',
        width: 'fill_container',
        height: 140,
      }],
    }, {
      id: 'product-card-1',
      type: 'frame',
      role: 'product-card',
      width: 168,
      height: 232,
      children: [],
    }],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([rail]))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 2)
  assert.match(report.diagnostics[0], /repeated-card rail height leaves excessive empty space/)
  assert.match(report.diagnostics[1], /unresolved search source must be hidden and collapsed/)
  assert.doesNotMatch(JSON.stringify(report), /private product query|placeholder:\/\//)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'product-rail', operation: 'U', rule: 'flow-size', patch: { height: 'fit_content' },
  }, {
    nodeId: 'product-image-0', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }, {
    nodeId: 'product-card-0', operation: 'U', rule: 'flow-size', patch: { height: 'fit_content' },
  }])

  const repaired = structuredClone(rail)
  const pending = [repaired]
  while (pending.length > 0) {
    const node = pending.pop()
    const target = report.repairTargets.find(candidate => candidate.nodeId === node.id)
    if (target !== undefined) Object.assign(node, target.patch)
    if (Array.isArray(node.children)) pending.push(...node.children)
  }
  const repairedReport = inspectGeneratedDesignQualityReport(documentWith([repaired]))
  assert.deepEqual(repairedReport.diagnostics, [])
  assert.deepEqual(repairedReport.repairTargets, [])
  assert.deepEqual(repairedReport.repairTargetSummary, { total: 0, returned: 0, omitted: 0 })
})

test('hides blank searched images but accepts resolved remote and data image sources', () => {
  const blank = {
    id: 'blank-image',
    type: 'image',
    src: '   ',
    imageSearchQuery: 'private blank query',
    width: 160,
    height: 120,
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([blank, {
    id: 'remote-image',
    type: 'image',
    src: 'https://example.test/product.png',
    imageSearchQuery: 'private remote query',
    width: 160,
    height: 120,
  }, {
    id: 'data-image',
    type: 'image',
    src: 'data:image/png;base64,AA==',
    imageSearchQuery: 'private data query',
    width: 160,
    height: 120,
  }]))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 1)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'blank-image', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }])
  const repaired = { ...blank, ...report.repairTargets[0].patch }
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([repaired])).diagnostics, [])
})

test('collapses a commerce rail symmetrically when any requested product image failed enrichment', () => {
  const imageCard = (id, src) => ({
    id, type: 'frame', role: 'product-card', width: 168, height: 232,
    children: [{
      id: `${id}-image`, type: 'image', src, imageSearchQuery: `private ${id}`,
      width: 'fill_container', height: 140,
    }, { type: 'text', content: id, ...readableText }, { type: 'text', content: '$99', ...readableText }],
  })
  const rail = {
    id: 'product-rail', type: 'frame', name: 'Featured grid', layout: 'horizontal',
    width: 'fill_container', height: 'fit_content', gap: 12,
    children: [imageCard('one', ''), imageCard('two', 'placeholder://image-search-failed')],
  }

  const blocked = inspectGeneratedDesignQualityReport(documentWith([rail]))
  assert.equal(blocked.unrepairableDiagnosticCount, 0)
  assert.ok(blocked.diagnostics.some(issue => /lost one or more requested product images/iu.test(issue)))
  assert.doesNotMatch(JSON.stringify(blocked), /private one|private two/)
  for (const nodeId of ['one-image', 'two-image']) {
    const target = blocked.repairTargets.find(candidate => candidate.nodeId === nodeId)
    assert.deepEqual(target?.patch, { visible: false, width: 0, height: 0 })
  }

  rail.children[0].children[0].src = 'https://example.test/resolved.png'
  const partiallyResolved = inspectGeneratedDesignQualityReport(documentWith([rail]))
  assert.equal(partiallyResolved.unrepairableDiagnosticCount, 0)
  assert.ok(partiallyResolved.diagnostics.some(issue => /lost one or more requested product images/iu.test(issue)))
  assert.ok(['one-image', 'two-image'].every(nodeId => (
    partiallyResolved.repairTargets.some(candidate => candidate.nodeId === nodeId)
  )), 'every searched rail image collapses together so the cards stay symmetric')

  rail.children[1].children[0].src = 'https://example.test/also-resolved.png'
  const fullyResolved = inspectGeneratedDesignQualityReport(documentWith([rail]))
  assert.ok(fullyResolved.diagnostics.every(issue => !/lost one or more requested product images/iu.test(issue)))

  // A rail whose failed images were already collapsed by the repair pass is
  // settled — the diagnostic must not re-raise after the single repair.
  rail.children[0].children[0].src = ''
  rail.children[1].children[0].src = 'placeholder://image-search-failed'
  for (const card of rail.children) {
    Object.assign(card.children[0], { visible: false, width: 0, height: 0 })
  }
  const collapsed = inspectGeneratedDesignQualityReport(documentWith([rail]))
  assert.ok(collapsed.diagnostics.every(issue => !/lost one or more requested product images/iu.test(issue)))
})

test('blocks commerce Hero and product cards that reuse an image query or resolved asset', () => {
  const imageCard = (id, query, src) => ({
    id, type: 'frame', role: 'product-card', width: 'fill_container', height: 'fit_content',
    layout: 'vertical', children: [
      { id: `${id}-image`, type: 'image', src, imageSearchQuery: query, width: 'fill_container', height: 300 },
      { type: 'text', content: id, ...readableText },
      { type: 'text', content: '$99', ...readableText },
    ],
  })
  const hero = {
    id: 'hero', type: 'frame', name: 'Hero', role: 'hero', layout: 'horizontal',
    width: 1120, height: 360, children: [{
      id: 'hero-image', type: 'image', src: 'data:image/png;base64,AA==',
      imageSearchQuery: 'gray loveseat isolated photo', width: 448, height: 360,
    }],
  }
  const rail = {
    id: 'product-rail', type: 'frame', name: 'Product rail', layout: 'horizontal',
    width: 1120, height: 'fit_content', gap: 24, children: [
      imageCard('one', 'gray loveseat isolated photo', 'data:image/png;base64,BB=='),
      imageCard('two', 'artemide tolomeo lamp photo', 'data:image/png;base64,AA=='),
    ],
  }

  const duplicated = inspectGeneratedDraftStructureReport(documentWith([hero, rail]))
  assert.equal(duplicated.unrepairableDiagnosticCount >= 1, true)
  assert.ok(duplicated.diagnostics.some(issue => /must use distinct image queries and distinct resolved assets/iu.test(issue)))
  assert.doesNotMatch(JSON.stringify(duplicated), /gray loveseat|tolomeo lamp|base64/iu)

  rail.children[0].children[0].imageSearchQuery = 'gray armchair isolated photo'
  rail.children[1].children[0].src = 'data:image/png;base64,CC=='
  const distinct = inspectGeneratedDraftStructureReport(documentWith([hero, rail]))
  assert.ok(distinct.diagnostics.every(issue => !/must use distinct image queries and distinct resolved assets/iu.test(issue)))
})

test('collapses a fixed hero media wrapper around an already-hidden unresolved image', () => {
  const hero = {
    id: 'hero',
    type: 'frame',
    role: 'hero',
    width: 'fill_container',
    height: 600,
    layout: 'horizontal',
    padding: [72, 80],
    children: [{
      id: 'hero-copy',
      type: 'frame',
      width: 'fill_container',
      height: 'fit_content',
      children: [{ type: 'text', content: 'Summer collection', ...readableText }],
    }, {
      id: 'hero-media',
      type: 'frame',
      name: 'HeroImage',
      width: 560,
      height: 640,
      fill: solid('#DDD'),
      children: [{
        id: 'hero-image',
        type: 'image',
        visible: false,
        src: 'placeholder://image-search-failed',
        imageSearchQuery: 'private hero query',
        width: 0,
        height: 0,
      }],
    }],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([hero]))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 2)
  assert.match(report.diagnostics[0], /fixed horizontal container height cannot contain/)
  assert.match(report.diagnostics[1], /fixed media wrapper whose sole visible asset is an unresolved image/)
  assert.doesNotMatch(JSON.stringify(report), /private hero query|placeholder:\/\//)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'hero', operation: 'U', rule: 'flow-size', patch: { height: 'fit_content' },
  }, {
    nodeId: 'hero-media', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }])

  const repaired = structuredClone(hero)
  const pending = [repaired]
  while (pending.length > 0) {
    const node = pending.pop()
    const target = report.repairTargets.find(candidate => candidate.nodeId === node.id)
    if (target !== undefined) Object.assign(node, target.patch)
    if (Array.isArray(node.children)) pending.push(...node.children)
  }
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([repaired])).diagnostics, [])
})

test('hides empty search, button, and CTA shells while preserving nonempty actions and structural containers', () => {
  const documentJson = documentWith([{
    type: 'frame',
    children: [{
      id: 'empty-search', type: 'frame', name: 'Search', width: 240, height: 48, fill: solid('#EEE'), children: [],
    }, {
      id: 'empty-button', type: 'frame', role: 'button', width: 120, height: 48, fill: solid('#111'), children: [],
    }, {
      id: 'empty-cta',
      type: 'frame',
      name: 'CTA',
      role: 'cta-section',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      padding: [1, 0, 1, 1],
      children: [{
        id: 'empty-primary', type: 'frame', role: 'button', width: 120, height: 48, fill: solid('#111'), children: [],
      }, {
        id: 'empty-secondary', type: 'frame', role: 'button', width: 120, height: 48, fill: solid('#EEE'), children: [],
      }],
    }, {
      id: 'icon-button',
      type: 'frame',
      role: 'button',
      width: 48,
      height: 48,
      children: [{ type: 'icon_font', iconFontName: 'search', width: 20, height: 20 }],
    }, {
      id: 'header',
      type: 'frame',
      role: 'header',
      children: [{ type: 'text', content: 'Store', ...readableText }],
    }],
  }])
  const report = inspectGeneratedDesignQualityReport(documentJson)

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 4)
  assert.ok(report.diagnostics.every(issue => /no visible text, icon, image, or control descendant/.test(issue)))
  assert.deepEqual(report.repairTargets.map(target => target.nodeId), [
    'empty-search', 'empty-button', 'empty-cta', 'empty-primary', 'empty-secondary',
  ])
  assert.ok(report.repairTargets.every(target => (
    target.rule === 'canvas-contract'
    && target.patch.visible === false
    && target.patch.width === 0
    && target.patch.height === 0
  )))

  const repaired = JSON.parse(documentJson)
  const targets = new Map(report.repairTargets.map(target => [target.nodeId, target.patch]))
  const pending = [...repaired.children]
  while (pending.length > 0) {
    const node = pending.pop()
    const target = targets.get(node.id)
    if (target !== undefined) Object.assign(node, target)
    if (Array.isArray(node.children)) pending.push(...node.children)
  }
  assert.deepEqual(inspectGeneratedDesignQualityReport(JSON.stringify(repaired)).diagnostics, [])
})

test('collapses every media wrapper when distinct repeated product cards reuse any icon-only artwork', () => {
  const productCard = (id, name, iconFontName = 'shirt') => ({
    id,
    type: 'frame',
    name,
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    children: [{
      id: `${id}-media`,
      type: 'frame',
      name: 'visual',
      width: 'fill_container',
      height: 200,
      layout: 'horizontal',
      fill: solid('#DDD'),
      children: [{ type: 'icon_font', iconFontName, width: 36, height: 36 }],
    }, {
      type: 'text', content: name, ...readableText,
    }, {
      type: 'text', content: '$ 89.00', ...readableText,
    }],
  })
  const rail = {
    id: 'product-rail',
    type: 'frame',
    name: 'Rail',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    gap: 24,
    children: [
      productCard('linen-shirt', 'Linen Shirt'),
      productCard('wool-coat', 'Wool Coat'),
      productCard('suede-tote', 'Suede Tote', 'lamp'),
    ],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([rail]))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 1)
  assert.match(report.diagnostics[0], /reuse an icon glyph as fixed media art/)
  assert.deepEqual(report.repairTargets.map(target => target.nodeId), [
    'linen-shirt-media', 'wool-coat-media', 'suede-tote-media',
  ])

  const repaired = structuredClone(rail)
  const targets = new Map(report.repairTargets.map(target => [target.nodeId, target.patch]))
  const pending = [repaired]
  while (pending.length > 0) {
    const node = pending.pop()
    const target = targets.get(node.id)
    if (target !== undefined) Object.assign(node, target)
    if (Array.isArray(node.children)) pending.push(...node.children)
  }
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([repaired])).diagnostics, [])

  const distinctIcons = structuredClone(rail)
  distinctIcons.children[1].children[0].children[0].iconFontName = 'watch'
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([distinctIcons])).diagnostics, [])
})

test('contrast failures emit deterministic recolor repairs when ink or white rescues the pair', () => {
  const page = {
    id: 'page', type: 'frame', width: 1200, height: 'fit_content', layout: 'vertical',
    fill: solid('#FFFFFF'),
    children: [{
      id: 'hero', type: 'frame', width: 'fill_container', height: 200, layout: 'vertical',
      fill: solid('#FF5A1F'),
      children: [{
        id: 'headline', type: 'text', content: '一杯暖意', fontSize: 16, lineHeight: 1.5,
        fontFamily: 'Inter, system-ui, sans-serif', fill: solid('#FFFFFF'),
      }],
    }, {
      id: 'ghost', type: 'text', content: '看不见的字', fontSize: 16, lineHeight: 1.5,
      fontFamily: 'Inter, system-ui, sans-serif', fill: solid('#FFFFFF'),
    }],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([page]))
  const headline = report.repairTargets.find(target => target.nodeId === 'headline')
  assert.deepEqual(headline?.patch, { fill: [{ type: 'solid', color: '#1C1917' }] })
  const ghost = report.repairTargets.find(target => target.nodeId === 'ghost')
  assert.deepEqual(ghost?.patch, { fill: [{ type: 'solid', color: '#1C1917' }] })
  assert.equal(report.unrepairableDiagnosticCount, 0)

  // A mid-gray backdrop defeats both rescue candidates and stays terminal.
  const gray = structuredClone(page)
  gray.children[0].fill = solid('#7A7A7A')
  gray.children[0].children[0].fill = solid('#8A8A8A')
  gray.children.pop()
  const terminal = inspectGeneratedDesignQualityReport(documentWith([gray]))
  assert.ok(terminal.unrepairableDiagnosticCount >= 1)
  assert.ok(!terminal.repairTargets.some(target => target.nodeId === 'headline'))
})

function categoryCard(id, label, iconFontName, options = {}) {
  const children = []
  if (options.visual !== false) {
    children.push({
      id: `${id}-tile`,
      type: 'frame',
      name: `${label} visual tile`,
      width: 'fill_container',
      height: 160,
      layout: 'horizontal',
      fill: solid('#EEE'),
      children: [{ type: 'icon_font', iconFontName, width: 28, height: 28 }],
    })
  }
  if (options.label !== false) {
    children.push({ type: 'text', content: label, ...readableText })
  }
  return {
    id,
    type: 'frame',
    name: `${label} category card`,
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 12,
    children,
  }
}

function categorySection(cards) {
  return {
    id: 'category-section',
    type: 'frame',
    name: 'Shop by category',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    children: [{
      id: 'category-rail',
      type: 'frame',
      name: 'Category cards',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      gap: 24,
      children: cards,
    }],
  }
}

test('accepts complete category cards with visible tiles, labels, and distinct sole glyphs', () => {
  const documentJson = documentWith([categorySection([
    categoryCard('living', 'Living', 'sofa'),
    categoryCard('lighting', 'Lighting', 'lamp'),
    categoryCard('dining', 'Dining', 'utensils'),
  ])])

  assert.deepEqual(inspectGeneratedDesignQualityReport(documentJson), {
    diagnostics: [],
    unrepairableDiagnosticCount: 0,
    repairTargets: [],
    repairTargetSummary: { total: 0, returned: 0, omitted: 0 },
  })
})

test('repairs excessive fixed category rail height while accepting authored content height', () => {
  const section = categorySection([
    compactCategoryCard('lamps', 'Lamps', 'lamp'),
    compactCategoryCard('ceramics', 'Ceramics', 'droplet'),
    compactCategoryCard('textiles', 'Textiles', 'shirt'),
  ])
  const rail = section.children[0]
  rail.padding = [32, 80]
  rail.height = 800

  const excessive = inspectGeneratedDesignQualityReport(documentWith([section]))
  assert.ok(excessive.diagnostics.some(issue => /category rail height leaves excessive empty space/iu.test(issue)))
  assert.deepEqual(excessive.repairTargets.find(target => target.nodeId === 'category-rail'), {
    nodeId: 'category-rail',
    operation: 'U',
    rule: 'flow-size',
    patch: { height: 'fit_content' },
  })

  rail.height = 232
  const authored = inspectGeneratedDesignQualityReport(documentWith([section]))
  assert.ok(authored.diagnostics.every(issue => !/category rail height leaves excessive empty space/iu.test(issue)))
  assert.ok(authored.repairTargets.every(target => target.nodeId !== 'category-rail'))
})

function compactCategoryCard(id, label, visual, options = {}) {
  const children = []
  if (visual === 'placeholder') {
    children.push({
      type: 'image', src: 'placeholder://image-search-failed', width: 28, height: 28,
    })
  } else if (visual === 'shape') {
    children.push({ type: 'ellipse', width: 28, height: 28, fill: solid('#A84300') })
  } else if (visual !== undefined) {
    children.push({ type: 'icon_font', iconFontName: visual, width: 28, height: 28 })
  }
  if (options.label !== false) children.push({ type: 'text', content: label, ...readableText })
  return {
    id,
    type: 'frame',
    name: `${label} icon tile`,
    width: 120,
    height: 120,
    layout: 'vertical',
    fill: options.surface === false ? undefined : solid('#FFF'),
    children,
  }
}

test('accepts compact category cards whose painted card is the tile with a direct visual leaf and label', () => {
  const documentJson = documentWith([categorySection([
    compactCategoryCard('lamps', 'Lamps', 'lamp'),
    compactCategoryCard('ceramics', 'Ceramics', 'droplet'),
    compactCategoryCard('textiles', 'Textiles', 'shape'),
  ])])

  assert.deepEqual(inspectGeneratedDesignQualityReport(documentJson).diagnostics, [])
})

test('does not misclassify a horizontal category section wrapper as another category rail', () => {
  const rail = categorySection([
    compactCategoryCard('lamps', 'Lamps', 'lamp'),
    compactCategoryCard('ceramics', 'Ceramics', 'droplet'),
    compactCategoryCard('textiles', 'Textiles', 'shirt'),
  ]).children[0]
  const documentJson = documentWith([{
    id: 'category-section-horizontal',
    type: 'frame',
    name: 'Shop by category',
    layout: 'horizontal',
    children: [{
      id: 'category-heading-copy',
      type: 'frame',
      name: 'Category heading copy',
      layout: 'vertical',
      children: [
        { type: 'text', content: 'Shop by category', ...readableText },
        { type: 'text', content: 'Everything, in one place', ...readableText },
      ],
    }, rail],
  }])

  assert.deepEqual(inspectGeneratedDesignQualityReport(documentJson).diagnostics, [])
  assert.deepEqual(inspectGeneratedDraftStructureReport(documentJson).diagnostics, [])
})

test('keeps compact category label-only, placeholder, and unpainted icon cards blocked', () => {
  const documentJson = documentWith([categorySection([
    compactCategoryCard('label-only', 'Label only'),
    compactCategoryCard('placeholder', 'Placeholder', 'placeholder'),
    compactCategoryCard('no-surface', 'No surface', 'lamp', { surface: false }),
  ])])
  const report = inspectGeneratedDesignQualityReport(documentJson)

  assert.equal(report.unrepairableDiagnosticCount, 3)
  assert.equal(report.diagnostics.length, 3)
  assert.ok(report.diagnostics.every(issue => /category card must include a visible visual tile/.test(issue)))
  assert.deepEqual(report.repairTargets, [])
})

test('blocks category cards missing a visible visual tile or text label without guessing repairs', () => {
  const documentJson = documentWith([categorySection([
    categoryCard('living', 'Living', 'sofa', { visual: false }),
    categoryCard('lighting', 'Lighting', 'lamp', { label: false }),
    categoryCard('dining', 'Dining', 'utensils'),
  ])])
  const report = inspectGeneratedDesignQualityReport(documentJson)

  assert.equal(report.unrepairableDiagnosticCount, 2)
  assert.equal(report.diagnostics.length, 2)
  assert.match(report.diagnostics[0], /category card must include a visible visual tile/)
  assert.match(report.diagnostics[1], /category card must include a visible non-empty text label/)
  assert.deepEqual(report.repairTargets, [])
  const draftReport = inspectGeneratedDraftStructureReport(documentJson)
  assert.equal(draftReport.unrepairableDiagnosticCount, 2)
  assert.deepEqual(draftReport.diagnostics, report.diagnostics)
})

test('blocks distinct category labels that reuse the same sole icon glyph', () => {
  const documentJson = documentWith([categorySection([
    categoryCard('living', 'Living', 'box'),
    categoryCard('lighting', 'Lighting', 'box'),
    categoryCard('dining', 'Dining', 'utensils'),
  ])])
  const report = inspectGeneratedDesignQualityReport(documentJson)

  assert.equal(report.unrepairableDiagnosticCount, 1)
  assert.equal(report.diagnostics.length, 1)
  assert.match(report.diagnostics[0], /distinct category labels reuse the same sole icon glyph/)
  assert.deepEqual(report.repairTargets, [])
  assert.throws(() => assertGeneratedDesignQuality(documentJson), /publication must stop/)
})

test('does not treat category navigation, button groups, or product rails as category card rails', () => {
  const nav = {
    type: 'frame',
    name: 'Category navigation',
    role: 'nav',
    layout: 'horizontal',
    children: [
      { type: 'text', content: 'Living', ...readableText },
      { type: 'text', content: 'Lighting', ...readableText },
    ],
  }
  const buttons = {
    type: 'frame',
    name: 'Shop by category actions',
    layout: 'horizontal',
    children: ['Living', 'Lighting'].map(label => ({
      type: 'frame',
      role: 'button',
      width: 120,
      height: 48,
      children: [
        { type: 'icon_font', iconFontName: 'box', width: 20, height: 20 },
        { type: 'text', content: label, ...readableText },
      ],
    })),
  }
  const productRail = {
    id: 'product-rail',
    type: 'frame',
    name: 'Product rail',
    layout: 'horizontal',
    children: ['Chair', 'Lamp'].map((label, index) => ({
      id: `product-${index}`,
      type: 'frame',
      role: 'product-card',
      width: 220,
      height: 'fit_content',
      layout: 'vertical',
      children: [{
        id: `product-${index}-visual`,
        type: 'frame',
        name: 'Product visual',
        width: 'fill_container',
        height: 180,
        children: [{ type: 'icon_font', iconFontName: 'box', width: 28, height: 28 }],
      }, {
        type: 'text', content: label, ...readableText,
      }, {
        type: 'text', content: '$120', ...readableText,
      }],
    })),
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([nav, buttons, productRail]))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.ok(report.diagnostics.every(issue => !/category card|category labels/.test(issue)))
  assert.ok(report.repairTargets.every(target => target.nodeId.endsWith('-visual')))

  const hiddenAlternative = categorySection([
    categoryCard('hidden-living', 'Living', 'box', { visual: false }),
    categoryCard('hidden-lighting', 'Lighting', 'box', { label: false }),
  ])
  hiddenAlternative.visible = false
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([hiddenAlternative])).diagnostics, [])
})

test('accepts substantive composed-shape media but still rejects ordinary painted empty shells', () => {
  const composedHero = {
    id: 'hero-visual',
    type: 'frame',
    name: 'Hero visual',
    width: 520,
    height: 420,
    layout: 'none',
    children: [
      {
        type: 'frame', name: 'Big tile', width: 300, height: 340, cornerRadius: 24,
        fill: solid('#A84300'),
        children: [{
          type: 'frame', name: 'Big tile inner', width: 220, height: 220, cornerRadius: 16,
          fill: solid('#FFD9A8'),
          children: [{
            type: 'frame', name: 'Big tile dot', width: 72, height: 72, cornerRadius: 36,
            fill: solid('#FFF'), children: [],
          }],
        }],
      },
      {
        type: 'frame', name: 'Tall tile', width: 220, height: 400, cornerRadius: 24,
        fill: solid('#8F929B'),
        children: [{
          type: 'frame', name: 'Tall tile inner', width: 140, height: 200, cornerRadius: 16,
          fill: solid('#FFF'),
          children: [{
            type: 'frame', name: 'Tall tile dot', width: 48, height: 48, cornerRadius: 24,
            fill: solid('#17191D'), children: [],
          }],
        }],
      },
    ],
  }
  const page = {
    type: 'frame', width: 1440, height: 'fit_content', layout: 'vertical', padding: 80,
    children: [composedHero],
  }
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([page])).diagnostics, [])
  assert.deepEqual(inspectGeneratedDraftStructureReport(documentWith([page])).diagnostics, [])

  const ordinaryShell = structuredClone(composedHero)
  ordinaryShell.id = 'ordinary-media'
  ordinaryShell.name = 'Product image'
  ordinaryShell.children = [
    { type: 'frame', width: 72, height: 72, fill: solid('#FFF'), children: [] },
    { type: 'frame', width: 40, height: 40, fill: solid('#FFF'), children: [] },
  ]
  const report = inspectGeneratedDesignQualityReport(documentWith([{
    ...page, children: [ordinaryShell],
  }]))
  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 1)
  assert.match(report.diagnostics[0], /fixed media, art, photo, or image wrapper has no visible/)
  assert.deepEqual(report.repairTargets.map(target => target.nodeId), ['ordinary-media'])
})

test('blocks a sparse desktop commerce hero and accepts a balanced positioned visual', () => {
  const productCard = (id, name) => ({
    id,
    type: 'frame',
    role: 'product-card',
    width: 280,
    height: 'fit_content',
    layout: 'vertical',
    children: [
      { type: 'text', content: name, ...readableText },
      { type: 'text', content: '$120', ...readableText },
    ],
  })
  const productRail = {
    id: 'product-rail',
    type: 'frame',
    name: 'Product rail',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    children: [productCard('chair', 'Lounge chair'), productCard('lamp', 'Table lamp')],
  }
  const sparseHero = {
    id: 'hero',
    type: 'frame',
    name: 'Hero',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    children: [{
      type: 'frame', name: 'Hero content', width: 'fill_container', layout: 'vertical',
      children: [{ type: 'text', content: 'Discover your ideal life', ...readableText }],
    }],
  }
  const root = {
    id: 'root', type: 'frame', width: 1440, height: 'fit_content', layout: 'vertical',
    children: [sparseHero, productRail],
  }

  const blocked = inspectGeneratedDraftStructureReport(documentWith([root]))
  assert.equal(blocked.unrepairableDiagnosticCount, 1)
  assert.match(
    blocked.diagnostics[0],
    /horizontal copy\/visual split.*positioned 4\+ layer composition containing an ellipse\/path/,
  )

  const fixed = structuredClone(root)
  const heroCopy = fixed.children[0].children[0]
  heroCopy.width = 608
  fixed.children[0].padding = [64, 160]
  fixed.children[0].children = [{
    id: 'hero-row',
    type: 'frame',
    name: 'HeroRow',
    width: 'fill_container',
    height: 360,
    layout: 'horizontal',
    gap: 64,
    children: [heroCopy, {
      type: 'frame',
      name: 'Hero visual',
      width: 448,
      height: 360,
      layout: 'none',
      children: [
        {
          type: 'frame', x: 0, y: 40, width: 300, height: 280,
          fill: solid('#A84300'), children: [],
        },
        {
          type: 'ellipse', x: 208, y: 0, width: 240, height: 240,
          fill: solid('#FFD9A8'),
        },
        {
          type: 'rectangle', x: 88, y: 180, width: 320, height: 160,
          fill: solid('#FFFFFF'),
        },
        {
          type: 'path', x: 32, y: 64, width: 160, height: 120,
          fill: solid('#17191D'),
        },
      ],
    }],
  }]

  assert.deepEqual(inspectGeneratedDraftStructureReport(documentWith([fixed])).diagnostics, [])
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([fixed])).diagnostics, [])

  const horizontalOuterWithNestedRow = structuredClone(fixed)
  horizontalOuterWithNestedRow.children[0].layout = 'horizontal'
  assert.deepEqual(
    inspectGeneratedDraftStructureReport(documentWith([horizontalOuterWithNestedRow])).diagnostics,
    [],
  )
  assert.deepEqual(
    inspectGeneratedDesignQualityReport(documentWith([horizontalOuterWithNestedRow])).diagnostics,
    [],
  )

  const imageHero = structuredClone(fixed)
  imageHero.children[0].children[0].children[1] = {
    id: 'hero-product-image',
    type: 'image',
    name: 'Hero armchair photo',
    width: 448,
    height: 360,
    imageSearchQuery: 'armchair studio photo',
    src: '',
  }
  assert.deepEqual(
    inspectGeneratedDraftStructureReport(documentWith([imageHero])).diagnostics,
    [],
    'a direct product-image leaf is the preferred compact Hero visual before enrichment',
  )

  const invalidFirstViewport = structuredClone(imageHero)
  invalidFirstViewport.children.unshift({
    id: 'header',
    type: 'frame',
    name: 'Header',
    width: 'fill_container',
    height: 64,
    layout: 'horizontal',
    padding: [16, 160],
    children: [{
      type: 'frame',
      name: 'Search',
      width: 320,
      height: 48,
      layout: 'horizontal',
      children: [{ type: 'text', content: 'Search', ...readableText }],
    }],
  })
  const firstViewportCta = {
    id: 'hero-cta',
    type: 'frame',
    name: 'Primary CTA',
    role: 'button',
    width: 160,
    height: 48,
    layout: 'horizontal',
    padding: [0, 24],
    fill: solid('#F97316'),
    children: [{
      type: 'text',
      content: 'Shop now',
      fontFamily: 'Inter, system-ui, sans-serif',
      fontSize: 16,
      fontWeight: 500,
      lineHeight: 1.5,
      fill: solid('#FFFFFF'),
    }],
  }
  invalidFirstViewport.children[1].children[0].children[0].children.push(firstViewportCta)
  const firstViewportReport = inspectGeneratedDraftStructureReport(
    documentWith([invalidFirstViewport]),
    true,
  )
  assert.ok(firstViewportReport.diagnostics.some(issue => /fixed horizontal container height/.test(issue)))
  assert.ok(firstViewportReport.diagnostics.some(issue => /WCAG AA contrast/.test(issue)))

  const repairedFirstViewport = structuredClone(invalidFirstViewport)
  repairedFirstViewport.children[0].padding = [0, 160]
  repairedFirstViewport.children[1].children[0].children[0].children.at(-1).fill = solid('#C2410C')
  assert.deepEqual(
    inspectGeneratedDraftStructureReport(documentWith([repairedFirstViewport]), true).diagnostics,
    [],
  )

  const malformedHeader = structuredClone(imageHero)
  malformedHeader.children.unshift({
    id: 'header-malformed',
    type: 'frame',
    name: 'Header',
    width: 'fill_container',
    height: 64,
    layout: 'horizontal',
    padding: [0, 160],
    justifyContent: 'space_between',
    children: [
      { type: 'text', name: 'Brand', content: 'Nook', ...readableText },
      {
        type: 'frame', name: 'Nav links', height: 44, layout: 'horizontal',
        children: [{ type: 'text', content: 'Home', ...readableText }],
      },
      {
        type: 'frame', name: 'Header actions', height: 44, layout: 'horizontal',
        children: [{ type: 'icon_font', iconFontName: 'search', width: 20, height: 20 }],
      },
    ],
  })
  const malformedHeaderReport = inspectGeneratedDraftStructureReport(
    documentWith([malformedHeader]),
    true,
  )
  assert.ok(malformedHeaderReport.diagnostics.some(issue => /role:"nav-links" collection/.test(issue)))
  assert.ok(malformedHeaderReport.diagnostics.some(issue => /bare text children/.test(issue)))
  assert.ok(malformedHeaderReport.diagnostics.some(issue => /role:"toolbar"/.test(issue)))
  assert.ok(malformedHeaderReport.diagnostics.some(issue => /44x44 role:"icon-button"/.test(issue)))

  const semanticHeader = structuredClone(imageHero)
  semanticHeader.children.unshift({
    id: 'header-semantic',
    type: 'frame',
    name: 'Header',
    role: 'navbar',
    width: 'fill_container',
    height: 64,
    layout: 'horizontal',
    padding: [0, 160],
    alignItems: 'center',
    justifyContent: 'space_between',
    children: [
      { type: 'text', name: 'Brand', content: 'Nook', ...readableText },
      {
        type: 'frame', name: 'Nav links', role: 'nav-links', height: 44,
        layout: 'horizontal', gap: 16, alignItems: 'center',
        children: [{
          type: 'frame', name: 'Home nav link', role: 'nav-link',
          width: 'fit_content', minWidth: 44, height: 44, layout: 'horizontal',
          alignItems: 'center', justifyContent: 'center',
          children: [{ type: 'text', content: 'Home', ...readableText }],
        }],
      },
      {
        type: 'frame', name: 'Header actions', role: 'toolbar', height: 44,
        layout: 'horizontal', gap: 8, alignItems: 'center',
        children: [{
          type: 'frame', name: 'Search icon button', role: 'icon-button',
          width: 44, height: 44, layout: 'horizontal',
          alignItems: 'center', justifyContent: 'center',
          children: [{ type: 'icon_font', iconFontName: 'search', width: 20, height: 20 }],
        }],
      },
    ],
  })
  assert.deepEqual(
    inspectGeneratedDraftStructureReport(documentWith([semanticHeader]), true).diagnostics,
    [],
  )

  const plainRectangles = structuredClone(fixed)
  plainRectangles.children[0].children[0].children[1].children = [
    { type: 'frame', x: 0, y: 20, width: 300, height: 300, fill: solid('#A84300'), children: [] },
    { type: 'frame', x: 128, y: 0, width: 320, height: 220, fill: solid('#FFD9A8'), children: [] },
    { type: 'rectangle', x: 80, y: 180, width: 360, height: 160, fill: solid('#FFFFFF') },
    { type: 'rectangle', x: 32, y: 72, width: 180, height: 120, fill: solid('#17191D') },
  ]
  const plainRectangleReport = inspectGeneratedDraftStructureReport(documentWith([plainRectangles]))
  assert.equal(plainRectangleReport.unrepairableDiagnosticCount, 1)
  assert.match(
    plainRectangleReport.diagnostics[0],
    /plain stacked rectangles/,
  )

  const narrowVisual = structuredClone(fixed)
  narrowVisual.children[0].children[0].children[1].width = 320
  narrowVisual.children[0].children[0].children[1].children = [
    { type: 'frame', x: 0, y: 40, width: 240, height: 260, fill: solid('#A84300'), children: [] },
    { type: 'ellipse', x: 150, y: 0, width: 160, height: 160, fill: solid('#FFD9A8') },
    { type: 'rectangle', x: 64, y: 180, width: 220, height: 140, fill: solid('#FFFFFF') },
    { type: 'path', x: 24, y: 70, width: 120, height: 90, fill: solid('#17191D') },
  ]
  const narrowVisualReport = inspectGeneratedDraftStructureReport(documentWith([narrowVisual]))
  assert.equal(narrowVisualReport.unrepairableDiagnosticCount, 1)
  assert.match(
    narrowVisualReport.diagnostics[0],
    /Hero visual must occupy about 40-50% of the inner width/,
  )

  const fullWidthText = structuredClone(fixed)
  fullWidthText.children[0].children[0].children = [
    { ...heroCopy, width: 'fill_container' },
  ]
  const fullWidthTextReport = inspectGeneratedDraftStructureReport(documentWith([fullWidthText]))
  assert.equal(fullWidthTextReport.unrepairableDiagnosticCount, 1)
  assert.match(fullWidthTextReport.diagnostics[0], /horizontal copy\/visual split/)

  const ambiguousNestedSplit = structuredClone(fixed)
  ambiguousNestedSplit.children[0].children.push({
    type: 'text', content: 'Detached full-width hero copy', ...readableText,
  })
  const ambiguousNestedReport = inspectGeneratedDraftStructureReport(documentWith([ambiguousNestedSplit]))
  assert.equal(ambiguousNestedReport.unrepairableDiagnosticCount, 1)
  assert.match(ambiguousNestedReport.diagnostics[0], /horizontal copy\/visual split/)

  const overflowing = structuredClone(fixed)
  overflowing.children[0].children[0].height = 560
  overflowing.children[0].children[0].children[1].height = 790
  const overflowReport = inspectGeneratedDraftStructureReport(documentWith([overflowing]))
  assert.ok(overflowReport.diagnostics.some(issue => /desktop commerce Hero visual overflows its fixed height/.test(issue)))

  overflowing.children[0].children[0].children[1].height = 400
  assert.deepEqual(inspectGeneratedDraftStructureReport(documentWith([overflowing])).diagnostics, [])

  const widthOverflow = structuredClone(overflowing)
  widthOverflow.children[0].children[0].gap = 72
  widthOverflow.children[0].children[0].children[0].width = 601
  const widthReport = inspectGeneratedDraftStructureReport(documentWith([widthOverflow]))
  assert.ok(widthReport.diagnostics.some(issue => /copy width \+ right visual width \+ gap \+ horizontal padding/.test(issue)))

  widthOverflow.children[0].children[0].children[0].width = 600
  assert.deepEqual(inspectGeneratedDraftStructureReport(documentWith([widthOverflow])).diagnostics, [])
})

test('collapses the real empty product media-card-rail-section cascade and becomes clean after one repair batch', () => {
  const root = {
    id: 'root',
    type: 'frame',
    name: 'Shop home',
    width: 1440,
    height: 'fit_content',
    layout: 'vertical',
    children: [{
      id: 'n44',
      type: 'frame',
      name: 'Featured products',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      children: [{
        id: 'n45',
        type: 'frame',
        name: 'Product heading',
        width: 'fill_container',
        height: 'fit_content',
        children: [{ type: 'text', content: 'Bestsellers', ...readableText }],
      }, {
        id: 'n49',
        type: 'frame',
        name: 'Product rail',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'horizontal',
        children: [{
          id: 'n50',
          type: 'frame',
          name: 'Product Arc Floor Lamp',
          width: 'fill_container',
          height: 'fit_content',
          layout: 'vertical',
          children: [{
            id: 'n51',
            type: 'frame',
            name: 'Media Arc Floor Lamp',
            width: 'fill_container',
            height: 300,
            layout: 'none',
            stroke: solid('#DDD'),
            children: [{
              id: 'n52',
              type: 'frame',
              name: 'Arc Floor Lamp art',
              width: 72,
              height: 72,
              fill: solid('#FFF'),
              children: [],
            }, {
              id: 'n54',
              type: 'frame',
              name: 'Wishlist',
              width: 40,
              height: 40,
              fill: solid('#FFF'),
              children: [],
            }],
          }],
        }],
      }],
    }],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([root]))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 3)
  assert.match(report.diagnostics[0], /fixed media, art, photo, or image wrapper has no visible/)
  assert.match(report.diagnostics[1], /product card has no meaningful visible product name and price/)
  assert.match(report.diagnostics[2], /product rail has no valid visible product card/)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'n51', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }, {
    nodeId: 'n50', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }, {
    nodeId: 'n49', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }, {
    nodeId: 'n44', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }])

  const repaired = structuredClone(root)
  const targets = new Map(report.repairTargets.map(target => [target.nodeId, target.patch]))
  const pending = [repaired]
  while (pending.length > 0) {
    const node = pending.pop()
    const patch = targets.get(node.id)
    if (patch !== undefined) Object.assign(node, patch)
    if (Array.isArray(node.children)) pending.push(...node.children)
  }
  const repairedReport = inspectGeneratedDesignQualityReport(documentWith([repaired]))
  assert.deepEqual(repairedReport.diagnostics, [])
  assert.deepEqual(repairedReport.repairTargets, [])
  assert.deepEqual(repairedReport.repairTargetSummary, { total: 0, returned: 0, omitted: 0 })
})

test('keeps a named and priced product card while collapsing only its empty media shell', () => {
  const rail = {
    id: 'rail',
    type: 'frame',
    name: 'Product rail',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    children: [{
      id: 'card',
      type: 'frame',
      name: 'Product Linen Chair',
      width: 280,
      height: 'fit_content',
      layout: 'vertical',
      children: [{
        id: 'media',
        type: 'frame',
        name: 'Product image',
        width: 'fill_container',
        height: 240,
        fill: solid('#EEE'),
        children: [{
          type: 'frame', name: 'Blank art', width: 72, height: 72, fill: solid('#FFF'), children: [],
        }],
      }, {
        type: 'text', content: 'Linen Chair', ...readableText,
      }, {
        type: 'text', content: '$520', ...readableText,
      }],
    }],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([rail]))

  assert.equal(report.diagnostics.length, 1)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'media', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }])
  const repaired = structuredClone(rail)
  Object.assign(repaired.children[0].children[0], report.repairTargets[0].patch)
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([repaired])).diagnostics, [])

  const resolved = structuredClone(rail)
  resolved.children[0].children[0].children = [{
    type: 'image', src: 'https://example.test/linen-chair.png', width: 'fill_container', height: 240,
  }]
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([resolved])).diagnostics, [])
})

test('collapses a large product media shell whose only meaningful visual is a small icon glyph', () => {
  const rail = {
    id: 'icon-product-rail',
    type: 'frame',
    name: 'Product rail',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    children: [{
      id: 'icon-product-card',
      type: 'frame',
      role: 'product-card',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      children: [{
        id: 'icon-product-media',
        type: 'frame',
        name: 'Product image',
        width: 'fill_container',
        height: 220,
        fill: solid('#EEE'),
        children: [{
          type: 'frame',
          name: 'Icon backing',
          width: 64,
          height: 64,
          fill: solid('#FFF'),
          children: [{ type: 'icon_font', iconFontName: 'lamp', width: 32, height: 32 }],
        }],
      }, {
        type: 'text', content: 'Arc Floor Lamp', ...readableText,
      }, {
        type: 'text', content: '$149', ...readableText,
      }],
    }],
  }
  const report = inspectGeneratedDesignQualityReport(documentWith([rail]))

  assert.equal(report.unrepairableDiagnosticCount, 0)
  assert.equal(report.diagnostics.length, 1)
  assert.match(report.diagnostics[0], /sole meaningful visual is a small icon glyph/)
  assert.deepEqual(report.repairTargets, [{
    nodeId: 'icon-product-media', operation: 'U', rule: 'canvas-contract',
    patch: { visible: false, width: 0, height: 0 },
  }])

  const repaired = structuredClone(rail)
  Object.assign(repaired.children[0].children[0], report.repairTargets[0].patch)
  assert.deepEqual(inspectGeneratedDesignQualityReport(documentWith([repaired])).diagnostics, [])
})

test('keeps real or composite product art, small icon controls, and icon-only media outside product rails', () => {
  const productCard = (id, media) => ({
    id,
    type: 'frame',
    role: 'product-card',
    width: 280,
    height: 'fit_content',
    layout: 'vertical',
    children: [media, {
      type: 'text', content: `${id} chair`, ...readableText,
    }, {
      type: 'text', content: '$120', ...readableText,
    }],
  })
  const rail = {
    id: 'healthy-product-rail',
    type: 'frame',
    name: 'Product rail',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    children: [
      productCard('resolved', {
        id: 'resolved-media',
        type: 'frame',
        name: 'Product image',
        width: 'fill_container',
        height: 220,
        children: [{
          type: 'image', src: 'https://example.test/chair.png', width: 'fill_container', height: 220,
        }],
      }),
      productCard('composite', {
        id: 'composite-media',
        type: 'frame',
        name: 'Product art',
        width: 'fill_container',
        height: 220,
        children: [{
          type: 'ellipse', width: 112, height: 112, fill: solid('#DDD'),
        }, {
          type: 'icon_font', iconFontName: 'armchair', width: 32, height: 32,
        }],
      }),
      productCard('small-shell', {
        id: 'small-shell-media',
        type: 'frame',
        name: 'Product visual',
        width: 64,
        height: 64,
        children: [{ type: 'icon_font', iconFontName: 'heart', width: 24, height: 24 }],
      }),
      productCard('large-glyph', {
        id: 'large-glyph-media',
        type: 'frame',
        name: 'Product visual',
        width: 'fill_container',
        height: 220,
        children: [{ type: 'icon_font', iconFontName: 'armchair', width: 112, height: 112 }],
      }),
    ],
  }
  const nonProduct = {
    id: 'service-rail',
    type: 'frame',
    name: 'Service benefits',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    children: [{
      id: 'service-card',
      type: 'frame',
      width: 280,
      height: 'fit_content',
      layout: 'vertical',
      children: [{
        id: 'service-visual',
        type: 'frame',
        name: 'Visual',
        width: 'fill_container',
        height: 220,
        children: [{ type: 'icon_font', iconFontName: 'truck', width: 32, height: 32 }],
      }, {
        type: 'text', content: 'Free delivery', ...readableText,
      }, {
        type: 'text', content: '$0', ...readableText,
      }],
    }],
  }

  const report = inspectGeneratedDesignQualityReport(documentWith([rail, nonProduct]))
  assert.deepEqual(report.diagnostics, [])
  assert.deepEqual(report.repairTargets, [])
})

test('enforces normal and large-text WCAG thresholds and composites alpha solids', () => {
  const documentJson = documentWith([{
    type: 'frame',
    fill: solid('#FFF'),
    children: [
      { type: 'text', content: 'normal', ...readableText, fill: solid('#777') },
      { type: 'text', content: 'large', fontFamily: 'system-ui', fontSize: 24, lineHeight: 1.3, fill: solid('#777') },
      {
        type: 'frame',
        fill: solid('#00000080'),
        children: [{ type: 'text', content: 'alpha', ...readableText, fill: solid('#777') }],
      },
    ],
  }])

  const contrastIssues = inspectGeneratedDesignQuality(documentJson)
    .filter(issue => /WCAG AA contrast/.test(issue))
  assert.equal(contrastIssues.length, 2)
  assert.ok(contrastIssues.some(issue => issue.startsWith('0.0: text')))
  assert.ok(contrastIssues.some(issue => issue.startsWith('0.2.0: text')))
  assert.ok(!contrastIssues.some(issue => issue.startsWith('0.1: text')))
})

test('skips uncertain variable and gradient colors and exempts status text and visual checkboxes', () => {
  const documentJson = documentWith([{
    type: 'frame',
    width: 100,
    height: 100,
    fill: [{ type: 'linear_gradient', stops: [] }],
    children: [
      { type: 'text', content: 'uncertain contrast', ...readableText, fill: solid('#777') },
      {
        type: 'frame',
        fill: [{ type: 'solid', color: '$surface' }],
        children: [{ type: 'text', content: 'variable', ...readableText, fill: solid('#777') }],
      },
      {
        type: 'frame',
        role: 'status-bar',
        children: [{ type: 'text', content: 'status without authored typography' }],
      },
      { type: 'checkbox', width: 18, height: 18 },
      { type: 'frame', width: 'calc(100% - 8px)', height: 'fit_content', children: [] },
    ],
  }])

  assert.deepEqual(inspectGeneratedDesignQuality(documentJson), [])
})

test('checks pages trees, nested semantics, and styled text without reflecting authored data', () => {
  const documentJson = JSON.stringify({
    version: '1.0.0',
    pages: [{
      name: 'private-page',
      children: [{
        type: 'frame',
        children: [{
          type: 'text_input',
          semantics: { label: 'Password private-secret' },
          width: 'fill_container',
          height: 48,
        }, {
          type: 'text',
          role: 'icon',
          content: [{ text: '🚫' }],
          ...readableText,
        }],
      }],
    }],
  })

  const diagnostic = inspectGeneratedDesignQuality(documentJson).join('\n')
  assert.match(diagnostic, /^pages\.0\.0\.0: text_input password text input/m)
  assert.match(diagnostic, /^pages\.0\.0\.1: text text node used as an icon/m)
  assert.ok(!diagnostic.includes('private'))
  assert.ok(!diagnostic.includes('🚫'))
})
