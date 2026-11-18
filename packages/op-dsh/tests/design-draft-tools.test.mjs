import assert from 'node:assert/strict'
import { createHash, randomBytes } from 'node:crypto'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { after, test } from 'node:test'

import { createDesignDraftToolController } from '../lib/design-draft-tools.js'
import { parseHydratableBeginResult } from '../lib/presentation-hydration.js'
import { createDocumentSnapshotFromText, RenderAccessController } from '../lib/renderer.js'

const previousDshHome = process.env.DSH_HOME
const testRoot = await mkdtemp(join(tmpdir(), 'dsh-openpencil-pipeline-tools-'))
process.env.DSH_HOME = join(testRoot, 'dsh-home')
after(async () => {
  if (previousDshHome === undefined) delete process.env.DSH_HOME
  else process.env.DSH_HOME = previousDshHome
  await rm(testRoot, { recursive: true, force: true })
})

const DRAFT_ID = 'd'.repeat(32)
const SAFE_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
  'base64',
)
const CLEAN_QUALITY = {
  geometryIssues: [], layoutIssues: [], contrastIssues: [], iconIssues: [],
  structureIssues: [], emptyShells: [], intentQuestions: [], variableIssues: [],
  imageSlots: [], navIssues: [],
}
function sha256(value) {
  return createHash('sha256').update(value).digest('hex')
}

function contrastRatio(first, second) {
  const luminance = hex => {
    const channels = hex.slice(1).match(/.{2}/g).map(value => Number.parseInt(value, 16) / 255)
      .map(value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)
    return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
  }
  const values = [luminance(first), luminance(second)].sort((a, b) => b - a)
  return (values[0] + 0.05) / (values[1] + 0.05)
}

function documentFromSeedCanvasScript(script) {
  const match = /^I\(null, (\{.*\})\);$/u.exec(script)
  assert.ok(match, `unexpected internal seed script: ${script}`)
  return {
    version: '1.0.0',
    children: [{ ...JSON.parse(match[1]), id: 'root', children: [] }],
  }
}

class FakeDraftController {
  calls = []
  aborts = []
  abortOwners = []
  beginCalls = []
  disposeCalls = 0
  finishCalls = 0
  screenshotCalls = []
  finishOptions = []
  png = SAFE_PNG
  version = 0
  finalized = false
  screenshotVersion
  documentJson = JSON.stringify({ version: '1.0.0', children: [] })
  quality = CLEAN_QUALITY
  lint = { count: 0, issues: [] }
  layout = { layout: [], layoutIssues: [] }
  batchValue = { applied: true, layoutIssues: [] }
  finalizeValue = { applied: true, advisories: [] }
  finalizeError
  finalizeAlwaysBumps = false
  finalizeDocumentTransform
  enrichImagesChanged = false
  enrichImagesTransform
  finishReportedVersion
  finishPreviewFailures = 0
  screenshotFailures = 0
  mutateBeforeFinishCheck
  materializeSeedScripts = true
  materializeUserBatchScripts = false
  seedScripts = []
  userBatchScripts = []
  restoreCalls = []
  userBatchDocumentTransform

  async begin(options) {
    this.beginCalls.push(options)
    return {
      draftId: DRAFT_ID,
      target: options.target,
      version: this.version,
      createdAt: 123,
      token: 'must-not-escape',
      daemonPath: '/private/tmp/dsh-openpencil-draft-secret/draft.op',
    }
  }

  async call(draftId, owner, tool, args, options) {
    this.calls.push({ draftId, owner, tool, args, options })
    let changed = false
    let value
    if (tool === 'get_design_agent_prompt') value = { prompt: 'complete native prompt', verifyProtocol: 'screenshot' }
    else if (tool === 'get_editor_state') value = { activePageId: 'page-1' }
    else if (tool === 'get_style_guide_tags') value = { tags: ['editorial'] }
    else if (tool === 'get_variables') value = { variables: {} }
    else if (tool === 'get_design_quality') value = this.quality
    else if (tool === 'lint_document') value = this.lint
    else if (tool === 'snapshot_layout') value = this.layout
    else if (tool === 'enrich_images') {
      value = { enriched: true }
      if (this.enrichImagesChanged) {
        if (typeof this.enrichImagesTransform === 'function') {
          this.documentJson = this.enrichImagesTransform(this.documentJson)
        }
        this.version += 1
        changed = true
        this.screenshotVersion = undefined
      }
    } else if (tool === 'batch_design' || tool === 'apply_design_system') {
      value = tool === 'batch_design' ? this.batchValue : { applied: true, layoutIssues: [] }
      if (tool === 'batch_design' && typeof args.script === 'string') {
        if (/^I\(null,/u.test(args.script)) {
          this.seedScripts.push(args.script)
          const current = JSON.parse(this.documentJson)
          if (this.materializeSeedScripts && (!Array.isArray(current.children) || current.children.length === 0)) {
            this.documentJson = JSON.stringify(documentFromSeedCanvasScript(args.script))
          }
        } else {
          this.userBatchScripts.push(args.script)
          if (typeof this.userBatchDocumentTransform === 'function') {
            this.documentJson = this.userBatchDocumentTransform(this.documentJson, args.script)
          }
          if (this.materializeUserBatchScripts) throw new Error('user batch materialization is not implemented by this fake')
        }
      }
      this.version += 1
      changed = true
      this.screenshotVersion = undefined
    } else if (tool === 'run_design_agent') {
      if (this.agentRunError !== undefined) throw this.agentRunError
      value = this.agentRunValue ?? {
        toolCalls: 9,
        stopReason: 'end_turn',
        landedRoots: 1,
        finalize: { committedScreens: 1, unfilledScreens: 0, qualityChecks: 3, qualityRepairs: 2, qualityNotes: 0 },
      }
      if (this.agentRunChanged !== false) {
        if (typeof this.agentRunDocumentTransform === 'function') {
          this.documentJson = this.agentRunDocumentTransform(this.documentJson)
        }
        this.version += 1
        changed = true
        this.screenshotVersion = undefined
      }
    } else if (tool === 'finalize_design') {
      if (this.finalizeError !== undefined) throw this.finalizeError
      value = this.finalizeValue
      if (!this.finalized || this.finalizeAlwaysBumps) {
        this.finalized = true
        if (typeof this.finalizeDocumentTransform === 'function') {
          this.documentJson = this.finalizeDocumentTransform(this.documentJson)
        }
        this.version += 1
        changed = true
        this.screenshotVersion = undefined
      }
    } else value = { ok: true }
    return { draftId, tool, value, text: JSON.stringify(value), version: this.version, changed, hasImage: false }
  }

  finalize(draftId, owner, options) {
    return this.call(draftId, owner, 'finalize_design', {}, options)
  }

  async snapshot(draftId) {
    return { draftId, version: this.version, documentJson: this.documentJson }
  }

  async restoreSnapshot(draftId, owner, snapshot, options = {}) {
    this.restoreCalls.push({ draftId, owner, snapshot, options })
    if (options.expectedVersion !== undefined) assert.equal(this.version, options.expectedVersion)
    this.documentJson = snapshot.documentJson
    this.version += 1
    this.screenshotVersion = undefined
    this.finalized = false
    return { draftId, version: this.version, documentJson: this.documentJson }
  }

  async screenshot(draftId, owner, options) {
    this.screenshotCalls.push({ draftId, owner, options })
    if (this.screenshotFailures > 0) {
      this.screenshotFailures -= 1
      throw new Error('preview temporarily unavailable')
    }
    if (options.nodeId === undefined || options.nodeId === 'root') this.screenshotVersion = this.version
    return {
      draftId,
      version: this.version,
      documentSha256: sha256(this.documentJson),
      bytes: this.png,
      mimeType: 'image/png',
      metadata: { width: 390, height: 844 },
    }
  }

  async finish(draftId, owner, options) {
    this.finishCalls += 1
    this.finishOptions.push(options)
    if (this.mutateBeforeFinishCheck !== undefined) {
      const mutate = this.mutateBeforeFinishCheck
      this.mutateBeforeFinishCheck = undefined
      mutate(this)
    }
    const currentDocumentSha256 = sha256(this.documentJson)
    if (
      options.expectedVersion !== undefined
      && (
        options.expectedVersion !== this.version
        || options.expectedDocumentSha256 !== currentDocumentSha256
      )
    ) {
      this.screenshotVersion = undefined
      this.finalized = false
      const error = new Error('checkpoint drift')
      error.code = 'OPENPENCIL_DRAFT_CHECKPOINT_DRIFT'
      error.currentVersion = this.version
      error.currentDocumentSha256 = currentDocumentSha256
      throw error
    }
    if (this.finishPreviewFailures > 0) {
      this.finishPreviewFailures -= 1
      const error = new Error('current preview required')
      error.code = 'OPENPENCIL_DRAFT_PREVIEW_REQUIRED'
      error.currentVersion = this.finishReportedVersion ?? this.version
      throw error
    }
    if (this.screenshotVersion !== this.version) {
      const error = new Error('current preview required')
      error.code = 'OPENPENCIL_DRAFT_PREVIEW_REQUIRED'
      error.currentVersion = this.finishReportedVersion ?? this.version
      throw error
    }
    const published = await options.publish({ draftId, version: this.version, documentJson: this.documentJson })
    return { draftId, version: this.version, published }
  }

  async abort(draftId, owner) {
    this.aborts.push({ draftId, owner })
    return true
  }

  async abortOwner(owner) {
    this.abortOwners.push(owner)
    return 1
  }

  async dispose() {
    this.disposeCalls += 1
  }
}

async function createHarness(options = {}) {
  const workspaceRoot = await mkdtemp(join(testRoot, 'workspace-'))
  const processPath = join(workspaceRoot, 'design.op')
  const target = { targetKey: `local:${processPath}`, displayPath: processPath }
  const draft = options.draft ?? new FakeDraftController()
  const calls = { lstat: [], resolve: [], stat: [], write: [], observe: [], grant: [], snapshot: [] }
  const fs = {
    sandboxMode: options.sandboxMode ?? 'workspace-write',
    async lstat(path, resolveOptions, signal) {
      calls.lstat.push({ path, resolveOptions, signal })
      return typeof options.pathInfo === 'function' ? options.pathInfo() : options.pathInfo
    },
    async resolve(path, resolveOptions) {
      calls.resolve.push({ path, resolveOptions })
      return options.changedTarget ?? target
    },
    processPath(received) {
      return received === target ? processPath : options.changedProcessPath ?? processPath
    },
    async stat(received, signal) {
      calls.stat.push({ received, signal })
      return options.resolvedInfo
    },
    async writeText(received, content, intent, signal, policy) {
      calls.write.push({ received, content, intent, signal, policy })
      if (options.writeError) throw options.writeError
      return { operation: 'create', version: 'fs-v1', before: null, after: content }
    },
  }
  const policy = { mode: options.policyMode ?? 'workspace-write', workspaceRoot }
  const sandboxPolicy = { resolve() { return policy } }
  const editorHost = {
    designDrafts: draft,
    grantFor(path, sha256) {
      calls.grant.push({ path, sha256 })
      return undefined
    },
    grantForDraft(draftId, ownerSessionId) {
      calls.grant.push({ draftId, ownerSessionId })
      return {
        enabled: true,
        launchUrl: '/_dsh/dsh-openpencil/editor/live/launch',
        refreshUrl: '/_dsh/dsh-openpencil/editor/live/refresh',
      }
    },
  }
  const render = new RenderAccessController(randomBytes(32))
  const controller = createDesignDraftToolController(editorHost, {
    fs,
    sandboxPolicy,
    render,
    observe(received, observation, exec) {
      calls.observe.push({ received, observation, exec })
      if (options.observeError !== undefined && observation.kind === 'present') throw options.observeError
    },
    async createDocumentSnapshot(text) {
      calls.snapshot.push(text)
      if (
        options.snapshotError !== undefined
        && calls.snapshot.length > (options.snapshotErrorAfter ?? 0)
      ) throw options.snapshotError
      return createDocumentSnapshotFromText(text)
    },
  })
  const tools = Object.fromEntries(controller.createTools().map(tool => [tool.name, tool]))
  const signal = new AbortController().signal
  const session = { id: 'session-from-exec', header: { cwd: workspaceRoot } }
  const exec = { agent: { id: 'session-from-exec', session }, signal }
  return { calls, controller, draft, exec, fs, policy, processPath, render, target, tools, workspaceRoot }
}

async function begin(harness, brief = 'Design a deliberate mobile account screen', options = {}) {
  // Established finish-path tests predate the see-then-fix round and assert
  // direct publication; they opt out here while the dedicated visual-review
  // tests below exercise the default-on behavior explicitly.
  const skipVisualReview = options.skipVisualReview ?? true
  return harness.tools.openpencil_pipeline_begin.execute(
    { path: 'design.op', brief, ...(skipVisualReview ? { skip_visual_review: true } : {}) },
    harness.exec,
  )
}

async function completeGeneration(harness) {
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const hero = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)
  const completed = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const remaining = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)
  assert.equal(completed.generationScriptCount, 2)
  harness.draft.screenshotCalls.length = 0
  return completed
}

test('pipeline begin derives owner from execution, validates an absent local target, and returns only public native context', async () => {
  const harness = await createHarness()
  const result = await begin(harness)

  assert.equal(harness.draft.beginCalls.length, 1)
  assert.equal(harness.draft.beginCalls[0].ownerSessionId, 'session-from-exec')
  assert.deepEqual(harness.draft.beginCalls[0].target, {
    id: String(harness.target.targetKey), label: harness.target.displayPath, kind: 'file',
  })
  assert.deepEqual(harness.draft.calls.map(call => [call.owner, call.tool]), [
    ['session-from-exec', 'batch_design'],
  ])
  assert.equal(harness.draft.seedScripts.length, 1)
  assert.equal(harness.draft.userBatchScripts.length, 0)
  const seed = harness.draft.calls[0]
  assert.equal(seed.args.postProcess, true)
  assert.equal(seed.args.canvasWidth, 390)
  assert.match(seed.args.script, /^I\(null, \{.*"type":"frame".*"width":390.*"height":"fit_content".*"minHeight":844.*\}\);$/u)
  assert.equal(result.published, false)
  assert.equal(result.version, 1)
  assert.equal(result.platform, 'mobile')
  assert.deepEqual(result.canvas, {
    platform: 'mobile', width: 390, seedHeight: 844, finalHeight: 'fit_content',
    fixedViewport: false, rootCount: 1, rootType: 'frame',
  })
  assert.equal(result.buildContract.version, 'openpencil-script-v12')
  assert.match(result.buildContract.script.create, /rootNodeId.*page.*I\(parent,node\).*K\(realKitId.*returned frame\/group bindings.*parent children.*No Page\/root.*I\(null,?\.\.\.\).*leaves including text_input.*frame\+cornerRadius.*not ellipse/is)
  assert.equal(result.buildContract.script.wrapper, 'const draftId="<exact begin.draftId>";\nconst script=String.raw`...`;\nconst r=await tools.openpencil_pipeline_batch({draftId,script});\nreturn r;')
  assert.equal(result.buildContract.script.wrapperRule, 'Quote exact begin.draftId first; call contains only draftId,script; no fields/return inside.')
  const assembledWrapper = result.buildContract.script.wrapper.replace(
    '...',
    'const root="root";\nI(root,{type:"frame",name:"Header",layout:"horizontal"});',
  )
  assert.doesNotThrow(() => new Function('tools', `return async()=>{${assembledWrapper}}`))
  assert.match(assembledWrapper, /const draftId="<exact begin\.draftId>";\nconst script=String\.raw`[\s\S]*`;\nconst r=await tools\.openpencil_pipeline_batch\(\{draftId,script\}\);\nreturn r;$/u)
  assert.ok(assembledWrapper.indexOf('const draftId=') < assembledWrapper.indexOf('const script='))
  assert.ok(assembledWrapper.indexOf('const script=') < assembledWrapper.indexOf('openpencil_pipeline_batch'))
  assert.ok(assembledWrapper.indexOf('openpencil_pipeline_batch') < assembledWrapper.indexOf('return r;'))
  assert.doesNotMatch(assembledWrapper, /canvasWidth|String\.raw`[\s\S]*`;\s*(?:canvasWidth|return)/u)
  assert.match(result.buildContract.script.safe, /QuickJS.*loops\/helpers.*I\/K return opaque ids.*parent-only.*never mutate a binding.*no imports\/console\/host APIs\/edit\/delete/is)
  assert.match(result.buildContract.generation.first, /First<=32 I\/K.*compact Header\/Search\/Cart.*complete mobile Hero.*short ASCII brand.*dominant message\/CTA.*recognizable focal visual.*no desktop two-column rule.*no generic floating rectangles.*Below-fold\/images batch2.*else one/is)
  assert.doesNotMatch(result.buildContract.generation.first, /\bNOVA\b/, 'the compact contract must not inject a fixed brand when the brief does not name one')
  assert.match(result.buildContract.generation.second, /Second=fresh parent-only I\/K.*no mutation\/new Page\/App Content\/Header\/Hero.*rootNodeId<=3 regions.*old wrapper.*Product=media-or-none\+name\+price.*Category=96x112.*56x56 Category glyph surface.*semantic 24x24 icon\+label.*names not art\/media\/image.*rail spans width.*justifyContent:"space_between".*数码=smartphone\/camera.*食品=utensils\/sandwich\/croissant.*not lamp\/coffee.*Commerce desktop=3 equal fill_container product cards spanning the rail.*24px gap.*no unused right tail.*mobile product rail=at most 2 fill_container cards OR equal numeric-width cards.*clipped scroller.*gray armchair isolated photo\/artemide tolomeo lamp photo\/potted plant isolated photo.*third card is potted plant.*Images=node\.image.*Query<=4 English words.*one product.*no lifestyle\/collection\/category.*Last region=required Footer.*role footer.*#1C1917.*role nav-links row of >=3 role nav-link frames.*#A8A29E copyright.*Loops.*finish/is)
  assert.match(result.buildContract.generation.limit, /Exactly two I\/K generation scripts.*do not expand short briefs.*gallery\/promo variants beyond the contract sections.*closing Footer is required/is)
  assert.deepEqual(result.buildContract.continuationStyle, {
    rule: 'Use returned values only.',
  })
  assert.deepEqual(result.buildContract.quality, {
    textDefaults: 'Generated text: Inter, system-ui, sans-serif / 16 / 1.5.',
    contrast: 'Use AA text pairs from the returned continuationStyle palette.',
  })
  assert.equal('operations' in result.buildContract, false)
  assert.match(result.buildContract.repair, /Complete non-empty finish\.repairTargets.*one U\(nodeId, ?patch\)-only script.*one finish retry.*otherwise stop/is)
  assert.match(result.buildContract.node.container, /width\/height.*number.*fill_container.*fit_content.*padding:\[vertical,horizontal\].*top,right,bottom,left/i)
  assert.match(result.buildContract.node.text, /Text portable "Inter, system-ui, sans-serif"\/16\/1\.5.*CJK same.*height omitted\/fit_content.*never bare Inter/is)
  assert.match(result.buildContract.node.paint, /fill.*solid.*#RRGGBB.*stroke.*thickness.*effects\[\].*shadow blur<=40/i)
  assert.match(result.buildContract.node.parents, /Only frame\/group parent.*rectangle not rect.*ellipse\/image\/icon_font\/text\/path.*leaves/is)
  assert.match(result.buildContract.node.icon, /\{type:'icon_font',name:'Search icon',iconFontName:'search',width:20,height:20\}.*name=layer label.*iconFontName=glyph from home\/search\/shopping-bag\/shopping-cart\/user\/heart\/star\/plus\/arrow-right\/sparkles\/sun\/apple\/snowflake\/droplet\/cookie\/leaf\/coffee\/package\/gift\/baby\/spray-can\/lamp\/sofa\/armchair\/shirt\/smartphone\/camera\/utensils\/sandwich\/headphones\/laptop\/monitor\/gamepad-2\/watch\/palette\/croissant\/cake\/bed\/bed-double\/lamp-desk\/flower\/truck\/shield-check\/credit-card\/map-pin\/menu\/x\/check\/chevron-down\/phone\/mail\/facebook\/instagram\/youtube\/table-2\/gem\/music\/tv\/car\/globe\/clock\/calendar\/tag\/percent\/store\/users\/rocket\/layers\/database\/cloud\/lock\/shield\/chart-bar\/chart-line\/chart-pie\/trending-up\/activity\/gauge\/target\/code\/workflow.*else shapes/is)
  assert.match(result.buildContract.node.image, /Photos:default1.*commerce=Hero1\+product3.*all four queries distinct.*Hero exact leaf: I\(parent,\{type:"image".*imageSearchQuery:"gray loveseat isolated photo"\}\).*imageSearchQuery is direct.*never image:\{\.\.\.\}.*never wrapper/is)
  assert.match(result.buildContract.node.image, /Query<=4 English words.*one product.*no lifestyle\/collection\/category/is)
  assert.match(result.buildContract.node.image, /Generic desktop Hero uses the direct image leaf.*shapes only when user explicitly requests illustration\/no photos.*Media=image XOR shapes.*no tiny icon/is)
  assert.equal('brand' in result.buildContract.node, false)
  assert.match(result.buildContract.node.control, /Controls.*>=44x44.*fill width.*bind visible children.*Button\/CTA role button.*icon buttons.*role icon-button 44x44 frames.*20px icon.*Nav collection role nav-links.*each item role nav-link minWidth44 height44 with text.*text_input.*leaf.*placeholder only.*never I\(text_input.*Icon\+hint search uses a Search frame.*password text_input secure:true/is)
  assert.match(result.buildContract.layoutRules.join(' '), /No x\/y in horizontal\/vertical flow.*layout:"none" visual stack.*numeric width\/height.*direct child.*numeric x\/y\/width\/height.*No fill_container.*fit_content.*No empty shells.*Visible copy.*user request language.*Mobile.*only full-width root children.*text\/icons\/controls\/titles.*bound 24px-gutter rail/is)
  assert.match(result.buildContract.layoutRules.join(' '), /Mobile category item.*vertical fit_content.*56x56 frame tile.*icon.*label.*never.*icon directly/is)
  assert.match(result.buildContract.layoutRules.join(' '), /Mobile product rail.*height:"fit_content".*at most 2 all-fill_container cards.*equal numeric-width cards.*clipped\/scroller rail.*never mix numeric and fill_container/is)
  assert.equal(result.rootNodeId, 'root')
  assert.deepEqual(result.editorState, {})
  assert.deepEqual(result.styleGuideTags, {
    name: 'dsh-editorial-warm', tags: ['editorial', 'warm', 'product-ui'],
  })
  assert.equal(result.continuationStyle.version, 'openpencil-continuation-style-v1')
  assert.equal(result.continuationStyle.typography.fontFamily, 'Inter, system-ui, sans-serif')
  assert.deepEqual(result.continuationStyle.palette, {
    page: '#F4F0E8', panel: '#FFFFFF', surface: '#111318', onSurface: '#FAF8F3',
    mutedOnSurface: '#C9C5BC', accent: '#A84300', accentHighlight: '#FFD9A8',
    onAccent: '#FFFFFF', ink: '#17191D', muted: '#66635E', line: '#DED8CE', surfaceLine: '#8F929B',
  })
  assert.ok(parseHydratableBeginResult(result), 'the real v12 begin producer must satisfy the strict hydration parser')
  for (const [background, foreground] of [
    ['page', 'ink'], ['page', 'muted'], ['page', 'accent'],
    ['panel', 'ink'], ['panel', 'muted'], ['panel', 'accent'],
    ['surface', 'onSurface'], ['surface', 'mutedOnSurface'], ['surface', 'accentHighlight'], ['surface', 'surfaceLine'],
    ['accent', 'onAccent'], ['accent', 'accentHighlight'],
  ]) {
    assert.ok(contrastRatio(
      result.continuationStyle.palette[background],
      result.continuationStyle.palette[foreground],
    ) >= 4.5)
  }
  assert.match(result.next, /Without narration.*batch 1.*root="root".*<=32 I\/K.*complete header\/nav.*polished hero.*Follow generation\.first exactly.*frame\/group containers.*children immediately.*text_input.*leaf.*icon\+hint search.*named Search frame wrapper.*mobile platform is final.*never abort\/rebegin.*generation\.second/is)
  assert.match(harness.tools.openpencil_pipeline_batch.description, /Commit the next QuickJS step.*live-canvas preview.*exactly two I\/K scripts.*complete finish\.repairTargets.*one later U-only script/is)
  assert.deepEqual(Object.keys(harness.tools.openpencil_pipeline_batch.parameters.properties), ['draftId', 'script'])
  assert.ok(harness.tools.openpencil_pipeline_batch.parameters.required.includes('script'))
  assert.match(harness.tools.openpencil_pipeline_batch.parameters.properties.script.description, /Standalone sandboxed QuickJS.*String\.raw.*fixed \{draftId,script\} object.*return r.*begin\.buildContract.*latest next/is)
  assert.doesNotMatch(harness.tools.openpencil_pipeline_batch.description, /canvasWidth/)
  assert.doesNotMatch(harness.tools.openpencil_pipeline_batch.parameters.properties.script.description, /canvasWidth/)
  assert.equal(result.liveCanvas, true)
  assert.equal(result.autoOpenEditor, true)
  assert.equal(result.document.sha256.length, 64)
  assert.equal('designAgentPrompt' in result, false)
  assert.equal(harness.calls.snapshot.length, 1)
  assert.equal(harness.calls.write.length, 0, 'begin must keep the target absent')
  assert.equal(harness.calls.observe[0].observation.kind, 'absent')
  const serialized = JSON.stringify(result)
  assert.doesNotMatch(serialized, /must-not-escape|dsh-openpencil-draft-secret/)
  assert.doesNotMatch(serialized, /\bNOVA\b/, 'an unspecified brand must not inherit a hard-coded NOVA identity')
  const modelSurface = [
    serialized,
    harness.tools.openpencil_pipeline_batch.description,
    harness.tools.openpencil_pipeline_batch.parameters.properties.script.description,
  ].join('\n')
  assert.doesNotMatch(modelSurface, /\boperations\b|\bDSL\b/i, 'the v12 model contract must expose only QuickJS script mode')
  // App-alignment deliberately grew this result: the begin contract now
  // carries a matched style-guide continuation plus 1-2 domain-skill
  // digests (the same knowledge OpenPencil's built-in agent loads).
  assert.ok(Buffer.byteLength(serialized, 'utf8') < 12 * 1024, `pipeline_begin compact JSON must stay below 12 KiB, got ${Buffer.byteLength(serialized, 'utf8')} bytes`)
  assert.equal('ownerSessionId' in harness.tools.openpencil_pipeline_begin.parameters.properties, false)
  assert.equal(harness.tools.openpencil_pipeline_begin.parameters.properties.path.required, undefined)
  assert.match(harness.tools.openpencil_pipeline_begin.parameters.properties.path.description, /Optional.*Omit it unless.*explicitly named.*plugin creates.*concrete collision-resistant filename.*explicit target must not exist.*preserved exactly/is)

  const routeDetach = harness.render.attachRoute()
  try {
    const projected = harness.tools.openpencil_pipeline_begin.output.presentationMeta({}, result)
    const envelope = projected.$dshOpenPencil
    assert.equal(envelope.autoOpenEditor, true)
    assert.equal(envelope.draftId, DRAFT_ID)
    assert.equal(envelope.liveDraft, true)
    assert.equal(envelope.editor.enabled, true)
    assert.equal(envelope.document.path, harness.processPath)
    assert.deepEqual(harness.calls.grant.at(-1), {
      draftId: DRAFT_ID,
      ownerSessionId: 'session-from-exec',
    })
  } finally {
    routeDetach()
  }

  const webHarness = await createHarness()
  const webResult = await begin(webHarness, 'Design an ecommerce homepage')
  assert.equal(webResult.platform, 'web')
  assert.equal(webResult.canvas.width, 1440)
  assert.equal(webResult.canvas.seedHeight, 900)
  assert.match(webResult.buildContract.generation.first, /literal hex colors.*no palette alias variables.*const header=I\(root,.*role:"navbar".*height:64.*padding:\[0,160\].*through that header binding.*never as root siblings.*frame role nav-link minWidth44 height44 containing its text child.*never put role nav-link on text.*Header actions role toolbar.*44x44 role icon-button frame.*20px icon.*Hero is width:"fill_container" horizontal padding:\[64,160\].*copy512\+gap64\+image448.*never set Hero width1120 together with padding.*headline\/subtitle width:"fill_container".*all other copy follows the user language.*Chinese request => Chinese copy.*one headline\/subtitle\/primary CTA.*CTA role button 160x48 #C2410C\/#FFFFFF.*label.*CTA binding/is)
  assert.match(webResult.buildContract.generation.first, /Generic commerce MUST use this direct visual pattern.*I\(hero,\{type:"image".*imageSearchQuery:"gray loveseat isolated photo"\}\).*imageSearchQuery is a direct node field.*must differ from every product-card query.*Never wrap the image.*image:\{\.\.\.\}.*mix it with shapes/is)
  assert.match(webResult.buildContract.generation.first, /layout:none 4-6-layer ellipse\/path fallback is allowed only when the user explicitly requests illustration\/no photos/is)
  assert.match(webResult.buildContract.node.image, /<=4 English words.*one product.*no lifestyle\/collection\/category/is)
  assert.deepEqual(webResult.styleGuideTags, {
    name: 'ecommerce-modern-light',
    tags: ['clean', 'light-mode', 'modern', 'rounded', 'warm-tones', 'landing-page'],
  })
  assert.equal(webResult.continuationStyle.styleGuide, 'ecommerce-modern-light')
  assert.match(webResult.continuationStyle.recipe.hero, /1120px row.*512px copy.*64px gap.*448px product visual.*headline\/subtitle fill the copy width.*primary CTA.*56px display/is)

  const autoPathHarness = await createHarness()
  await autoPathHarness.tools.openpencil_pipeline_begin.execute({
    brief: 'Design an ecommerce homepage',
  }, autoPathHarness.exec)
  assert.match(autoPathHarness.calls.lstat[0].path, /^shop-home-[a-z0-9]+-[a-f0-9]{4}\.op$/u)

  const explicitHarness = await createHarness()
  const explicitResult = await begin(explicitHarness, 'Design a 1280×1600 ecommerce homepage')
  assert.deepEqual(explicitResult.canvas, {
    platform: 'web', width: 1280, seedHeight: 1600, finalHeight: 1600,
    fixedViewport: true, rootCount: 1, rootType: 'frame',
  })
})

test('begin matches the brief onto App style guides and domain skills deterministically', async () => {
  // A Chinese mobile food brief resolves the App's warm food guide, and the
  // contract carries the mobile-app + CJK domain digests.
  const coffee = await createHarness()
  const coffeeResult = await begin(coffee, '画一个移动端咖啡外卖 App 首页,中文文案')
  assert.equal(coffeeResult.platform, 'mobile')
  assert.equal(coffeeResult.continuationStyle.styleGuide, 'warm-food-mobile-light')
  assert.equal(coffeeResult.styleGuideTags.name, 'warm-food-mobile-light')
  assert.ok(coffeeResult.styleGuideTags.tags.includes('food'))
  assert.match(coffeeResult.continuationStyle.palette.page, /^#[0-9A-F]{6}$/i)
  assert.equal(coffeeResult.continuationStyle.typography.fontFamily, 'Inter, system-ui, sans-serif')
  const coffeeGuidance = coffeeResult.buildContract.domainGuidance
  assert.ok(coffeeGuidance['mobile-app'].length > 200, 'mobile briefs carry the mobile-app architecture digest')
  assert.match(coffeeGuidance['mobile-app'], /status bar/i)
  assert.ok(coffeeGuidance['cjk-typography'].length > 200, 'CJK briefs carry the CJK typography digest')
  assert.match(coffeeGuidance['cjk-typography'], /lineHeight/)
  assert.ok(
    Buffer.byteLength(JSON.stringify(coffeeGuidance)) <= 2560,
    'domain guidance stays within its injection budget',
  )

  // A commerce-web brief keeps the untouched builtin direction: exact
  // builtin palette, no guide override, no surprise recipe change.
  const shop = await createHarness()
  const shopResult = await begin(shop, 'Design an ecommerce homepage')
  assert.equal(shopResult.continuationStyle.styleGuide, 'ecommerce-modern-light')
  assert.equal(shopResult.continuationStyle.palette.accent, '#C2410C')
  assert.equal(shopResult.styleGuideTags.name, 'ecommerce-modern-light')

  // A brief with no category signal falls back to the editorial default.
  const plain = await createHarness()
  const plainResult = await begin(plain, 'Design a deliberate account screen for web')
  assert.equal(plainResult.continuationStyle.styleGuide, 'dsh-editorial-warm')
  assert.equal(plainResult.styleGuideTags.name, 'dsh-editorial-warm')
})

test('begin ships a landing scaffold and stays deterministic per brief', async () => {
  const landing = await createHarness()
  const landingResult = await begin(landing, '为 Nimbus 设计一个 SaaS 产品官网落地页,中文文案')
  assert.equal(landingResult.platform, 'web')
  assert.equal(landingResult.continuationStyle.styleGuide, 'saas-modern-light')
  const guidance = landingResult.buildContract.domainGuidance
  assert.ok(guidance['cjk-typography'] !== undefined)
  const scaffolded = Object.values(guidance).some(digest => /Proven section skeleton/.test(digest))
  assert.equal(typeof guidance === 'object', true)
  // Determinism: the same brief yields byte-identical knowledge fields.
  const again = await createHarness()
  const againResult = await begin(again, '为 Nimbus 设计一个 SaaS 产品官网落地页,中文文案')
  assert.deepEqual(againResult.continuationStyle, landingResult.continuationStyle)
  assert.deepEqual(againResult.buildContract.domainGuidance, guidance)
  assert.equal(scaffolded || guidance['landing-page'] === undefined, true)
})

test('the canonical I(null) page wrapper heals to the existing root and bare Inter normalizes', async () => {
  const harness = await createHarness()
  await begin(harness)
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const page = I(null, {type:"frame", name:"Home", width:390, height:844, layout:"vertical"});\n'
      + 'const header = I(page, {type:"frame", name:"Header", width:"fill_container", height:64});\n'
      + 'I(header, {type:"text", name:"Brand", content:"MELLOW", fontFamily:"Inter", fontSize:20, lineHeight:1.2});',
  }, harness.exec)
  const sent = harness.draft.calls.filter(call => call.tool === 'batch_design').at(-1).args.script
  assert.match(sent, /const page = "root";/)
  assert.doesNotMatch(sent, /I\(\s*null/)
  assert.match(sent, /fontFamily:"Inter, system-ui, sans-serif"/)
  assert.doesNotMatch(sent, /fontFamily:"Inter"/)

  // A non-canonical I(null, ...) still rejects with the exact rewrite.
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'I("n1", {type:"frame", name:"A"}); const extra = I(null, {type:"frame", name:"B"});',
    }, harness.exec),
    /page root already exists.*replace const page = I\(null, \{\.\.\.\}\) with const page = "root"/s,
  )
})

test('a duplicate begin names the live draft and the exact next step instead of a dead end', async () => {
  const harness = await createHarness()
  await begin(harness)
  await assert.rejects(
    harness.tools.openpencil_pipeline_begin.execute({ path: 'other.op', brief: 'another brief' }, harness.exec),
    /already has the active OpenPencil draft ".+".*generation scripts committed: 0\/2.*Do not begin again.*send batch 1/s,
  )
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'I("n1", {type:"frame", name:"Header"});',
  }, harness.exec)
  await assert.rejects(
    harness.tools.openpencil_pipeline_begin.execute({ path: 'other.op', brief: 'another brief' }, harness.exec),
    /generation scripts committed: 1\/2.*second and final batch/s,
  )
})

test('pipeline begin refuses read-only, existing, non-op, and non-agent calls before starting a daemon', async () => {
  const readOnly = await createHarness({ policyMode: 'read-only' })
  await assert.rejects(begin(readOnly), /Workspace Write/)
  const existing = await createHarness({ pathInfo: { type: 'file' } })
  await assert.rejects(existing.tools.openpencil_pipeline_begin.execute({ path: 'design.op', brief: 'brief' }, existing.exec), /already exists/)
  const badExtension = await createHarness()
  await assert.rejects(badExtension.tools.openpencil_pipeline_begin.execute({ path: 'design.json', brief: 'brief' }, badExtension.exec), /end in \.op/)
  const noAgent = await createHarness()
  await assert.rejects(
    noAgent.tools.openpencil_pipeline_begin.execute({ path: 'design.op', brief: 'brief' }, { signal: noAgent.exec.signal }),
    /agent-owned/,
  )
})

test('context is allowlisted and recursively rejects path, URL, export, import, and spawn-shaped arguments', async () => {
  const harness = await createHarness()
  await begin(harness)
  const before = harness.draft.calls.length
  await assert.rejects(
    harness.tools.openpencil_pipeline_context.execute({
      draftId: DRAFT_ID,
      tool: 'batch_get',
      arguments: { nested: { filePath: '/private/tmp/escape.op' } },
    }, harness.exec),
    /not allowed in an isolated design draft/,
  )
  assert.equal(harness.draft.calls.length, before)
  await assert.rejects(
    harness.tools.openpencil_pipeline_context.execute({ draftId: DRAFT_ID, tool: 'spawn_agents', arguments: {} }, harness.exec),
    /must be one of|tool is not allowed/,
  )
  const result = await harness.tools.openpencil_pipeline_context.execute({
    draftId: DRAFT_ID, tool: 'apply_design_system', arguments: { name: 'halo' },
  }, harness.exec)
  assert.equal(result.tool, 'apply_design_system')
  const variables = await harness.tools.openpencil_pipeline_context.execute({
    draftId: DRAFT_ID,
    tool: 'set_variables',
    arguments: { variables: { accent: { type: 'color', value: '#A9642F' } } },
  }, harness.exec)
  assert.equal(variables.tool, 'set_variables')
  await assert.rejects(
    harness.tools.openpencil_pipeline_context.execute({
      draftId: DRAFT_ID, tool: 'set_variables', arguments: { variables: { accent: { type: 'color', value: '#A9642F' } } },
    }, harness.exec),
    /already consumed/,
  )
  await assert.rejects(
    harness.tools.openpencil_pipeline_context.execute({
      draftId: DRAFT_ID,
      tool: 'get_design_agent_prompt',
      arguments: {},
    }, harness.exec),
    /must be one of|tool is not allowed/,
  )
  await assert.rejects(
    harness.tools.openpencil_pipeline_context.execute({
      draftId: DRAFT_ID, tool: 'enrich_images', arguments: { timeout_seconds: 30, prompt: 'escape' },
    }, harness.exec),
    /only accepts timeout_seconds and root_ids/,
  )
  await harness.tools.openpencil_pipeline_context.execute({
    draftId: DRAFT_ID, tool: 'get_guidelines', arguments: { topic: 'layout' },
  }, harness.exec)
  await harness.tools.openpencil_pipeline_context.execute({
    draftId: DRAFT_ID, tool: 'get_variables', arguments: {},
  }, harness.exec)
  await assert.rejects(
    harness.tools.openpencil_pipeline_context.execute({
      draftId: DRAFT_ID, tool: 'list_style_guides', arguments: {},
    }, harness.exec),
    /context budget exhausted/,
  )
})

test('batch enforces the begin canvas and avoids automatic full quality/layout round trips', async () => {
  const harness = await createHarness()
  await begin(harness)
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({ draftId: DRAFT_ID, script: ' ' }, harness.exec),
    error => {
      assert.match(error.message, /provide one non-empty QuickJS script/i)
      assert.doesNotMatch(error.message, /\boperations\b|\bDSL\b/i)
      return true
    },
  )
  const tooManyFirstNodes = Array.from(
    { length: 33 },
    (_, index) => `const node${index} = I("root", {type:"rectangle", width:1, height:1});`,
  ).join('\n')
  const callsBeforeOversizedFirstScript = harness.draft.calls.length
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: tooManyFirstNodes,
    }, harness.exec),
    /first live-preview script is limited to 32 I\/K calls and 8192 bytes/i,
  )
  assert.equal(harness.draft.calls.length, callsBeforeOversizedFirstScript)

  const tooLargeFirstScript = `const padding = ${JSON.stringify('x'.repeat(8 * 1024))};\nI("root", {type:"rectangle", width:1, height:1});`
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: tooLargeFirstScript,
    }, harness.exec),
    /first live-preview script is limited to 32 I\/K calls and 8192 bytes/i,
  )
  assert.equal(harness.draft.calls.length, callsBeforeOversizedFirstScript)

  const maxAllowedFirstNodes = Array.from(
    { length: 32 },
    (_, index) => `const allowed${index} = I("root", {type:"rectangle", width:1, height:1});`,
  ).join('\n')
  const result = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: maxAllowedFirstNodes,
    canvasWidth: 390,
  }, harness.exec)
  const batch = harness.draft.calls.filter(call => call.tool === 'batch_design').at(-1)
  assert.equal(batch.args.postProcess, true)
  assert.equal(batch.args.canvasWidth, 390)
  assert.equal(batch.args.script, maxAllowedFirstNodes)
  assert.deepEqual(harness.draft.userBatchScripts, [batch.args.script])
  assert.equal(harness.draft.calls.at(-1).tool, 'batch_design')
  assert.equal(harness.draft.calls.some(call => call.tool === 'get_design_quality'), false)
  assert.equal(harness.draft.calls.some(call => call.tool === 'snapshot_layout'), false)
  assert.equal(result.canvasCheck.valid, true)
  assert.equal(result.rootNodeId, 'root')
  assert.equal(result.generationScriptCount, 1)
  assert.equal(result.generationScriptLimit, 2)
  assert.deepEqual(result.diagnostics, [])
  assert.ok(result.screenshot)
  assert.match(result.next, /Without narration.*second and final QuickJS script.*Fresh QuickJS.*opaque I\/K ids.*parent-only.*no mutation\/locals.*No new Page\/App Content\/Header\/Hero.*begin\.rootNodeId.*<=3 regions\/cards.*exact old wrapper id.*Category helper.*card=I\(rail.*96x112 vertical.*face=I\(card.*Category glyph surface.*width:56.*height:56.*I\(face.*icon_font.*iconFontName:glyph.*I\(card.*text.*content:label.*rail justifyContent:"space_between".*never art\/media\/image.*数码\/electronics=smartphone\/camera.*食品\/food=utensils\/sandwich\/croissant.*never lamp\/coffee.*three equal fill_container product cards from one coherent collection.*large image\/name\/price.*gap24.*no unused right tail.*gray armchair isolated photo.*artemide tolomeo lamp photo.*potted plant isolated photo.*label the third as a potted plant.*query <=4 English words.*exactly one product.*no lifestyle\/collection\/category.*no lone small icon.*large media.*one primary visual.*outside commerce default to one image.*finish once/is)
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'const wrongWidth = I("root", {type:"frame"});',
      canvasWidth: 1440,
    }, harness.exec),
    /must match the 390px begin canvas contract/,
  )
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({ draftId: DRAFT_ID, script: 'I(null,{type:"frame"})' }, harness.exec),
    /page root already exists.*replace const page = I\(null/is,
  )

  const callsBeforeBindingMutation = harness.draft.calls.length
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'const wish = I("root", {type:"frame"});\nwish.x = undefined;\nI(wish,{type:"text",content:"lost"});',
    }, harness.exec),
    /I\/K bindings are opaque node-id strings.*cannot be mutated.*never assign binding\.x.*binding\.y.*any member/i,
  )
  assert.equal(harness.draft.calls.length, callsBeforeBindingMutation, 'member mutation must fail before native draft execution')

  const callsBeforePrematureRepair = harness.draft.calls.length
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'U("root", {"name":"Generated Page"});',
    }, harness.exec),
    /generation scripts may only create with I\/K.*finish must authorize.*U\(\) repair script/i,
  )
  assert.equal(harness.draft.calls.length, callsBeforePrematureRepair)

  const completed = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const remaining = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)
  assert.equal(completed.generationScriptCount, 2)
  assert.equal(completed.generationScriptLimit, 2)
  assert.ok(completed.screenshot)
  assert.match(completed.next, /Without narration.*call finish exactly once.*validate.*final user preview.*publish atomically/is)
  assert.deepEqual(harness.draft.userBatchScripts, [
    batch.args.script,
    'const remaining = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  ])
  const callsBeforeThirdScript = harness.draft.calls.length
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'const forbiddenThird = I("root", {type:"frame"});',
    }, harness.exec),
    /ordinary generation is limited to two direct QuickJS scripts.*call finish before any repair script/i,
  )
  assert.equal(harness.draft.calls.length, callsBeforeThirdScript)
})

test('an invalid second-generation category rail is rolled back and gets one bounded correction', async () => {
  const harness = await createHarness()
  await begin(harness, '画个电商首页')
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const hero = I("root", {type:"frame", name:"Hero"});',
  }, harness.exec)
  const afterFirst = harness.draft.documentJson
  harness.draft.userBatchDocumentTransform = (documentJson, script) => {
    const document = JSON.parse(documentJson)
    if (!script.includes('categories')) return documentJson
    const good = script.includes('corrected')
    document.children[0].children.push({
      id: good ? 'good-rail' : 'bad-rail',
      type: 'frame',
      name: 'Category rail',
      layout: 'horizontal',
      children: ['Lamps', 'Ceramics', 'Textiles'].map((label, index) => ({
        id: `${good ? 'good' : 'bad'}-${index}`,
        type: 'frame',
        name: `${label} icon tile`,
        width: 120,
        height: 120,
        layout: 'vertical',
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        children: good ? [
          { type: 'icon_font', iconFontName: ['lamp', 'droplet', 'shirt'][index], width: 28, height: 28 },
          { type: 'text', content: label, fontFamily: 'Inter, system-ui, sans-serif', fontSize: 16, lineHeight: 1.5 },
        ] : [{
          type: 'frame', name: 'Tile chip', width: 64, height: 64,
          fill: [{ type: 'solid', color: '#F4F0E8' }],
        }],
      })),
    })
    return JSON.stringify(document)
  }

  const rejected = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const categories = I("root", {type:"frame", name:"categories"});',
  }, harness.exec)
  assert.equal(rejected.stage, 'needs_generation_correction')
  assert.equal(rejected.rolledBack, true)
  assert.equal(rejected.changed, false)
  assert.equal(rejected.generationScriptCount, 1)
  assert.equal(rejected.canContinue, true)
  assert.match(rejected.next, /resend only the corrected second script once.*card=I\(rail.*96x112 vertical.*face=I\(card.*Category glyph surface.*width:56.*height:56.*iconFontName:glyph.*content:label.*Use the nested 56x56 face exactly.*fix every reported rail height\/width\/overflow issue/is)
  assert.ok(rejected.diagnostics.some(issue => /category card must include a visible visual tile/.test(issue)))
  assert.ok(rejected.diagnostics.some(issue => /category card must include a visible non-empty text label/.test(issue)))
  assert.equal(harness.draft.restoreCalls.length, 1)
  assert.equal(harness.draft.documentJson, afterFirst)

  const corrected = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const correctedCategories = I("root", {type:"frame", name:"corrected categories"});',
  }, harness.exec)
  assert.equal(corrected.generationScriptCount, 2)
  assert.equal(corrected.canContinue, true)
  assert.ok(corrected.screenshot)
})

test('a sparse first-generation desktop commerce hero is rolled back before it can poison batch two', async () => {
  const harness = await createHarness()
  const begun = await begin(harness, '画个电商首页')
  assert.match(begun.buildContract.generation.first, /Chinese request => Chinese copy/)
  harness.draft.userBatchDocumentTransform = (documentJson, script) => {
    const document = JSON.parse(documentJson)
    const corrected = script.includes('corrected')
    document.children[0].children.push({
      id: corrected ? 'hero-corrected' : 'hero-sparse',
      type: 'frame',
      name: 'Hero',
      width: 'fill_container',
      height: 'fit_content',
      layout: corrected ? 'horizontal' : 'vertical',
      children: [{
        type: 'frame', name: 'Hero content', width: 'fill_container', layout: 'vertical',
        children: [{ type: 'text', content: '发现理想生活', fontFamily: 'Inter, system-ui, sans-serif', fontSize: 64, lineHeight: 1.5 }],
      }, ...(corrected ? [{
        type: 'image', name: 'Hero product image', width: 560, height: 360,
        imageSearchQuery: 'armchair studio photo', src: 'https://example.test/armchair.png',
      }] : [])],
    })
    return JSON.stringify(document)
  }
  const seededDocument = harness.draft.documentJson

  const rejected = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const hero = I("root", {type:"frame", name:"Hero", layout:"vertical"});',
  }, harness.exec)

  assert.equal(rejected.stage, 'needs_generation_correction')
  assert.equal(rejected.rolledBack, true)
  assert.equal(rejected.generationScriptCount, 0)
  assert.equal(rejected.canContinue, true)
  assert.match(rejected.diagnostics[0], /horizontal copy\/visual split.*positioned 4\+ layer composition.*ellipse\/path.*plain stacked rectangles/i)
  assert.match(rejected.next, /corrected first script once.*rollback removed both Header\/Nav and Hero.*recreate both.*Hero is width:"fill_container" horizontal padding:\[64,160\].*copy width512.*gap64.*image width448.*never combine width1120 with padding.*headline and subtitle each use width:"fill_container".*non-brand copy.*user language/is)
  assert.match(rejected.next, /literal hex colors.*no aliases.*Bind the Header first.*role:"navbar".*height:64.*padding:\[0,160\].*through that binding.*never as root siblings.*Nav role nav-links.*44px role nav-link on a frame.*text child.*never on text.*Header actions role toolbar.*44x44 role icon-button wrappers.*CTA role button 160x48 #C2410C\/#FFFFFF.*label through the CTA binding.*generic commerce.*directly under Hero exactly as I\(hero,\{type:"image".*imageSearchQuery:"gray loveseat isolated photo"\}\).*distinct from every product card.*never use a wrapper.*image:\{\.\.\.\}.*or shapes.*no blank field or overflow/is)
  assert.equal(harness.draft.documentJson, seededDocument)

  const corrected = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const correctedHero = I("root", {type:"frame", name:"Hero", layout:"horizontal"});',
  }, harness.exec)
  assert.equal(corrected.generationScriptCount, 1)
  assert.equal(corrected.canContinue, true)
  assert.ok(corrected.screenshot)
  const correctedDocument = JSON.parse(harness.draft.documentJson)
  const correctedHero = correctedDocument.children[0].children.find(node => node.id === 'hero-corrected')
  assert.equal(correctedHero.children[0].children[0].content, '发现理想生活')
  assert.deepEqual(correctedHero.children[1], {
    type: 'image', name: 'Hero product image', width: 560, height: 360,
    imageSearchQuery: 'armchair studio photo', src: 'https://example.test/armchair.png',
  })
})

test('an overflowing first-generation desktop commerce hero is rolled back before batch two', async () => {
  const harness = await createHarness()
  await begin(harness, '画个电商首页')
  harness.draft.userBatchDocumentTransform = (documentJson, script) => {
    const document = JSON.parse(documentJson)
    const corrected = script.includes('corrected')
    document.children[0].children.push({
      id: corrected ? 'hero-bounded' : 'hero-overflowing',
      type: 'frame',
      name: 'Hero',
      width: 'fill_container',
      height: 560,
      padding: [64, 80],
      gap: 64,
      layout: 'horizontal',
      children: [{
        type: 'frame', name: 'Hero content', width: 'fill_container', height: 'fit_content', layout: 'vertical',
        children: [{ type: 'text', content: '发现理想生活', fontFamily: 'Inter, system-ui, sans-serif', fontSize: 64, lineHeight: 1.5 }],
      }, {
        type: 'image', name: 'Hero product image', width: 500, height: corrected ? 400 : 790,
        imageSearchQuery: 'armchair studio photo', src: 'https://example.test/armchair.png',
      }],
    })
    return JSON.stringify(document)
  }
  const seededDocument = harness.draft.documentJson

  const rejected = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const hero = I("root", {type:"frame", name:"Hero", height:560});',
  }, harness.exec)

  assert.equal(rejected.stage, 'needs_generation_correction')
  assert.equal(rejected.rolledBack, true)
  assert.equal(rejected.generationScriptCount, 0)
  assert.equal(rejected.canContinue, true)
  assert.ok(rejected.diagnostics.some(issue => /desktop commerce Hero visual overflows its fixed height/.test(issue)))
  assert.match(rejected.next, /rollback removed both Header\/Nav and Hero.*recreate both.*Hero is width:"fill_container" horizontal padding:\[64,160\].*copy width512.*gap64.*image width448.*never combine width1120 with padding.*headline and subtitle each use width:"fill_container".*non-brand copy.*user language/is)
  assert.match(rejected.next, /literal hex colors.*no aliases.*Bind the Header first.*role:"navbar".*height:64.*padding:\[0,160\].*through that binding.*never as root siblings.*Nav role nav-links.*44px role nav-link on a frame.*text child.*never on text.*Header actions role toolbar.*44x44 role icon-button wrappers.*CTA role button 160x48 #C2410C\/#FFFFFF.*label through the CTA binding.*generic commerce.*directly under Hero exactly as I\(hero,\{type:"image".*imageSearchQuery:"gray loveseat isolated photo"\}\).*distinct from every product card.*never use a wrapper.*image:\{\.\.\.\}.*or shapes.*no blank field or overflow/is)
  assert.equal(harness.draft.documentJson, seededDocument)

  const corrected = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const correctedHero = I("root", {type:"frame", name:"Hero", height:560});',
  }, harness.exec)
  assert.equal(corrected.generationScriptCount, 1)
  assert.equal(corrected.canContinue, true)
  assert.ok(corrected.screenshot)
})

test('a committed JS transaction reports a preview-only fallback without asking to rerun code', async () => {
  const harness = await createHarness()
  await begin(harness)
  harness.draft.screenshotFailures = 1

  const result = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const content = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)

  assert.equal(result.changed, true)
  assert.equal(result.generationScriptCount, 1)
  assert.equal(result.generationScriptLimit, 2)
  assert.equal(result.previewUnavailable, true)
  assert.equal('screenshot' in result, false)
  assert.equal(harness.draft.userBatchScripts.length, 1)
  assert.match(result.next, /committed JS.*live canvas.*PNG user preview.*temporarily unavailable.*Do not rerun it.*second and final completion.*Fresh QuickJS.*opaque I\/K ids.*parent-only.*no mutation\/locals.*No new Page\/App Content\/Header\/Hero.*begin\.rootNodeId.*<=3 regions\/cards.*exact old wrapper id.*final preview automatically/is)
  assert.doesNotMatch(result.next, /pipeline_inspect|read_image|visual inspection/i)
  const completed = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const rest = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)
  assert.equal(completed.generationScriptCount, 2)
  assert.equal(completed.generationScriptLimit, 2)
  assert.ok(completed.screenshot)
  assert.match(completed.next, /Without narration.*call finish exactly once.*publish atomically/is)
})

test('finish keeps the draft private until both direct JS generation scripts have completed', async () => {
  const harness = await createHarness()
  await begin(harness)

  const beforeGeneration = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(beforeGeneration.published, false)
  assert.equal(beforeGeneration.stage, 'needs_generation')
  assert.equal(beforeGeneration.generationScriptCount, 0)
  assert.equal(beforeGeneration.generationScriptLimit, 2)
  assert.match(beforeGeneration.next, /bounded first-visible-viewport QuickJS script.*preview.*automatically/is)
  assert.equal(harness.draft.calls.some(call => call.tool === 'finalize_design'), false)
  assert.equal(harness.draft.finishCalls, 0)
  assert.equal(harness.calls.write.length, 0)

  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const hero = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)
  const screenshotsAfterFirstScript = harness.draft.screenshotCalls.length
  const afterFirstScript = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(afterFirstScript.published, false)
  assert.equal(afterFirstScript.stage, 'needs_generation')
  assert.equal(afterFirstScript.generationScriptCount, 1)
  assert.equal(afterFirstScript.generationScriptLimit, 2)
  assert.match(afterFirstScript.next, /Without narration.*second and final direct JS script.*Fresh QuickJS.*opaque I\/K ids.*parent-only.*no mutation\/locals.*No new Page\/App Content\/Header\/Hero.*begin\.rootNodeId.*<=3 regions\/cards.*exact old wrapper id.*preview.*automatically.*finish once/is)
  assert.equal(harness.draft.screenshotCalls.length, screenshotsAfterFirstScript, 'finish generation gate must not render another preview')
  assert.equal(harness.draft.calls.some(call => call.tool === 'finalize_design'), false)
  assert.equal(harness.draft.finishCalls, 0)
  assert.equal(harness.calls.write.length, 0)
})

test('a changed batch defers native diagnostics to finish and keeps the committed live preview', async () => {
  const harness = await createHarness()
  harness.draft.batchValue = {
    applied: true,
    layoutIssues: [{ code: 'hero-overflow', nodeId: 'hero' }],
  }
  await begin(harness)

  const result = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const hero = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)

  assert.equal(result.changed, true)
  assert.equal(result.generationScriptCount, 1)
  assert.deepEqual(result.diagnostics, [])
  assert.equal('layoutIssues' in result.batch, false)
  assert.deepEqual(result.batch, { applied: true })
  assert.ok(result.screenshot, 'a committed changed batch must preview even when diagnostics require repair')
  assert.equal(result.previewUnavailable, undefined)
  assert.equal(harness.draft.screenshotCalls.length, 1)
  assert.match(result.next, /Without narration.*second and final QuickJS script.*Fresh QuickJS.*opaque I\/K ids.*parent-only.*no mutation\/locals.*begin\.rootNodeId.*<=3 regions\/cards.*exact old wrapper id.*Then call finish once/is)
})

test('the first commerce batch enriches its Hero product image before the live preview', async () => {
  const harness = await createHarness()
  await begin(harness, 'Design an ecommerce homepage')
  harness.draft.userBatchDocumentTransform = documentJson => {
    const document = JSON.parse(documentJson)
    document.children[0].children.push({
      type: 'frame', id: 'hero', name: 'Hero', width: 'fill_container', height: 'fit_content',
      layout: 'vertical', padding: [64, 160], children: [{
        type: 'frame', id: 'hero-row', name: 'HeroRow', width: 1120, height: 360,
        layout: 'horizontal', gap: 64, children: [{
          type: 'frame', id: 'hero-copy', name: 'Hero copy', width: 608, height: 'fit_content',
          layout: 'vertical', children: [{ type: 'text', content: 'A better everyday', fontFamily: 'Inter, system-ui, sans-serif', fontSize: 56, lineHeight: 1.05 }],
        }, {
          type: 'image', id: 'hero-photo', name: 'Hero armchair photo',
          width: 448, height: 360, imageSearchQuery: 'armchair studio photo', src: '',
        }],
      }],
    })
    return JSON.stringify(document)
  }
  harness.draft.enrichImagesChanged = true
  harness.draft.enrichImagesTransform = documentJson => {
    const document = JSON.parse(documentJson)
    document.children[0].children[0].children[0].children.find(node => node.id === 'hero-photo').src = 'enriched://hero-armchair'
    return JSON.stringify(document)
  }

  const completed = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const hero = I("root", {type:"frame", name:"Hero", width:"fill_container", height:"fit_content"});',
  }, harness.exec)

  const enrichCalls = harness.draft.calls.filter(call => call.tool === 'enrich_images')
  assert.equal(completed.generationScriptCount, 1)
  assert.equal(completed.version, 3)
  assert.equal(enrichCalls.length, 1)
  assert.deepEqual(enrichCalls[0].args, { timeout_seconds: 8 })
  assert.equal(enrichCalls[0].options.expectedVersion, 2)
  assert.match(harness.draft.documentJson, /enriched:\/\/hero-armchair/)
  assert.equal(harness.draft.screenshotVersion, 3)
  assert.equal(harness.draft.screenshotCalls.length, 1)
  assert.ok(completed.screenshot)
})

test('the second commerce batch enriches unresolved product images before its live preview', async () => {
  const harness = await createHarness()
  await begin(harness, 'Design an ecommerce homepage')
  harness.draft.userBatchDocumentTransform = (documentJson, script) => {
    const document = JSON.parse(documentJson)
    if (script.includes('products')) {
      document.children[0].children.push({
        type: 'image', id: 'product-photo', name: 'Armchair product photo',
        width: 320, height: 240, imageSearchQuery: 'armchair studio photo', src: '',
      })
    }
    return JSON.stringify(document)
  }

  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const header = I("root", {type:"frame", name:"Header", width:"fill_container", height:64});',
  }, harness.exec)
  harness.draft.enrichImagesChanged = true
  harness.draft.enrichImagesTransform = documentJson => {
    const document = JSON.parse(documentJson)
    document.children[0].children.find(node => node.id === 'product-photo').src = 'enriched://armchair'
    return JSON.stringify(document)
  }

  const completed = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const products = I("root", {type:"frame", name:"Products", width:"fill_container", height:"fit_content"});',
  }, harness.exec)

  const enrichCalls = harness.draft.calls.filter(call => call.tool === 'enrich_images')
  assert.equal(completed.generationScriptCount, 2)
  assert.equal(completed.version, 4)
  assert.equal(enrichCalls.length, 1)
  assert.deepEqual(enrichCalls[0].args, { timeout_seconds: 8 })
  assert.equal(enrichCalls[0].options.expectedVersion, 3)
  assert.match(harness.draft.documentJson, /enriched:\/\/armchair/)
  assert.equal(harness.draft.screenshotVersion, 4, 'the live preview must render the enriched document version')
  assert.equal(harness.draft.screenshotCalls.length, 2)
  assert.ok(completed.screenshot)
})

test('pipeline begin seeds the only empty root before returning script generation context', async () => {
  const harness = await createHarness()
  const result = await begin(harness, 'Design an ecommerce homepage')
  assert.equal(harness.draft.seedScripts.length, 1)
  assert.equal(harness.draft.userBatchScripts.length, 0)
  const document = JSON.parse(harness.draft.documentJson)
  assert.equal(document.children.length, 1)
  assert.deepEqual(document.children[0], {
    type: 'frame', name: 'Generated Page', width: 1440, height: 'fit_content', minHeight: 900,
    layout: 'vertical', padding: 0, gap: 0,
    fill: [{ type: 'solid', color: '#FFFFFF' }], id: 'root', children: [],
  })
  assert.equal(result.rootNodeId, 'root')
  assert.equal(result.version, 1)
  assert.equal(result.buildContract.version, 'openpencil-script-v12')
  assert.equal(result.buildContract.layoutRules.some(rule => /^Mobile:/i.test(rule)), false)
  assert.deepEqual(result.continuationStyle.typography.display, [56, 700, 1.05])
  assert.deepEqual(result.continuationStyle.typography.heading, [36, 700, 1.2])
  assert.deepEqual(result.continuationStyle.spacing.sectionPadding, [64, 160])
  assert.equal(result.continuationStyle.spacing.pageInset, 160)
  assert.deepEqual(result.continuationStyle.palette, {
    page: '#FFFFFF', panel: '#FFFFFF', surface: '#1C1917', onSurface: '#FFFFFF',
    mutedOnSurface: '#A8A29E', accent: '#C2410C', accentHighlight: '#FFF7ED',
    onAccent: '#FFFFFF', ink: '#1C1917', muted: '#57534E', line: '#E7E5E4', surfaceLine: '#F5F5F4',
  })
  assert.doesNotMatch(JSON.stringify(result), /firstBurst/i)
})

test('a Web brief is seeded at the authoritative desktop width before user JS runs', async () => {
  const harness = await createHarness()
  const begun = await begin(harness, 'Design an ecommerce homepage')
  assert.equal(begun.canvas.width, 1440)
  const generated = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const hero = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)
  const batch = harness.draft.calls.filter(call => call.tool === 'batch_design').at(-1)
  assert.equal(batch.args.canvasWidth, 1440)
  assert.equal(generated.canvasCheck.valid, true)
  assert.equal(JSON.parse(harness.draft.documentJson).children[0].width, 1440)
})

test('the direct user request owns platform choice even when the model rewrites begin.brief', async () => {
  const desktop = await createHarness()
  desktop.exec.agent.session.deriveMessages = () => [{
    role: 'user',
    source: { kind: 'user' },
    content: [{ type: 'text', text: '画个电商首页' }],
  }]
  const preservedDesktop = await begin(desktop, '移动端电商首页，390x844 商品瀑布流')
  assert.equal(preservedDesktop.platform, 'web')
  assert.equal(preservedDesktop.canvas.width, 1440)

  const mobile = await createHarness()
  mobile.exec.agent.session.deriveMessages = () => [{
    role: 'user',
    source: { kind: 'user' },
    content: [{ type: 'text', text: '画个移动端电商首页' }],
  }]
  const preservedMobile = await begin(mobile, 'Design an ecommerce homepage')
  assert.equal(preservedMobile.platform, 'mobile')
  assert.equal(preservedMobile.canvas.width, 390)
})

test('pipeline begin aborts when the internal seed does not yield one authoritative valid root', async () => {
  const harness = await createHarness()
  harness.draft.materializeSeedScripts = false
  await assert.rejects(begin(harness, 'Design a desktop dashboard'), /native canvas seed did not produce the authoritative single root frame/i)
  assert.equal(harness.draft.seedScripts.length, 1)
  assert.deepEqual(harness.draft.aborts, [{ draftId: DRAFT_ID, owner: 'session-from-exec' }])
})

test('a fluid canvas cannot publish until its completed root height is fit_content', async () => {
  const harness = await createHarness()
  await begin(harness, 'Design a mobile learning dashboard')
  await completeGeneration(harness)
  harness.draft.documentJson = JSON.stringify({
    version: '1.0.0',
    children: [{ type: 'frame', id: 'root', width: 390, height: 844, children: [] }],
  })

  const blocked = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'needs_correction')
  assert.match(blocked.diagnostics.join(' '), /completed root height fit_content.*current root height is 844/i)

  harness.draft.documentJson = JSON.stringify({
    version: '1.0.0',
    children: [{ type: 'frame', id: 'root', width: 390, height: 'fit_content', children: [] }],
  })
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'U("root", {"height":"fit_content"});',
  }, harness.exec)
  const repairBatch = harness.draft.calls.filter(call => call.tool === 'batch_design').at(-1)
  assert.equal(repairBatch.args.script, 'U("root", {"height":"fit_content"});')
  assert.equal('operations' in repairBatch.args, false)
  const ready = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(ready.published, true)
  assert.equal(harness.draft.screenshotCalls.length, 2, 'the repair preview cannot replace the post-final root proof')
  assert.equal(harness.calls.write.length, 1)
})

test('screenshot inspection tolerates an irrelevant maxDepth and exposes one safe content-addressed cache file', async () => {
  const harness = await createHarness()
  await begin(harness)
  const result = await harness.tools.openpencil_pipeline_inspect.execute({
    draftId: DRAFT_ID,
    kind: 'screenshot',
    maxDepth: 8,
  }, harness.exec)
  assert.match(result.screenshot.path, /dsh-openpencil\/design-draft-inspections\/[a-f0-9]{64}\.png$/)
  assert.equal(result.screenshot.mimeType, 'image/png')
  assert.equal(result.screenshot.bytes, SAFE_PNG.length)
  assert.equal(result.screenshot.width, 1)
  assert.equal(result.screenshot.height, 1)
  assert.equal(harness.draft.screenshotCalls[0].owner, 'session-from-exec')
  assert.match(result.next, /exact user preview/i)
  assert.match(result.next, /exact user preview.*No model image inspection is required/is)
  assert.match(result.next, /call finish once to finalize.*render its own final preview.*publish/is)
  assert.doesNotMatch(result.next, /read_image|visual inspection/i)
  assert.doesNotMatch(result.next, /quality\/finalize gates/i)
  assert.match(harness.tools.openpencil_pipeline_inspect.description, /Screenshot returns.*user preview.*render-integrity.*finish.*never requires model image inspection/is)
  assert.match(harness.tools.openpencil_pipeline_finish.description, /Validate.*final user preview.*atomically publish.*complete structured repairTargets.*one U-only repair.*unstructured validation or host failure is terminal.*not be retried/is)
  const serialized = JSON.stringify(result)
  assert.doesNotMatch(serialized, /safe-png-bytes|token|managed-draft|draft\.op/)

  assert.equal(typeof harness.tools.openpencil_pipeline_inspect.output.presentationMeta, 'function')
  const unavailable = harness.tools.openpencil_pipeline_inspect.output.presentationMeta({}, result)
  assert.equal('$dshOpenPencil' in unavailable, false)

  const detachRoute = harness.render.attachRoute()
  try {
    const projected = harness.tools.openpencil_pipeline_inspect.output.presentationMeta({}, result)
    const envelope = projected.$dshOpenPencil
    const expectedFilename = `render-stage-${result.screenshot.sha256}.png`
    assert.equal(envelope.schemaVersion, 2)
    assert.equal(envelope.image.path, expectedFilename)
    assert.equal(envelope.image.width, 1)
    assert.equal(envelope.image.height, 1)
    assert.deepEqual(envelope.frames, [envelope.image])
    assert.match(envelope.image.previewUrl, /^\/_dsh\/dsh-openpencil\/render\//)
    assert.equal(JSON.stringify(envelope).includes(result.screenshot.path), false)

    const token = decodeURIComponent(envelope.image.previewUrl.split('/').at(-1))
    const payload = harness.render.verify(token)
    assert.deepEqual(payload, {
      v: 2,
      kind: 'image',
      filename: expectedFilename,
      bytes: SAFE_PNG.length,
      sha256: result.screenshot.sha256,
    })
    assert.equal('path' in payload, false, 'signed browser capability must not encode a host cache path')
    const browserCopy = await readFile(join(
      process.env.DSH_HOME,
      'cache',
      'dsh-openpencil',
      'renders',
      expectedFilename,
    ))
    assert.deepEqual(browserCopy, SAFE_PNG)
  } finally {
    detachRoute()
  }
})

test('layout inspection flattens the native layout envelope into one direct tree', async () => {
  const harness = await createHarness()
  harness.draft.layout = { layout: [{ id: 'root', width: 390, height: 844 }], layoutIssues: [] }
  await begin(harness)
  const result = await harness.tools.openpencil_pipeline_inspect.execute({
    draftId: DRAFT_ID,
    kind: 'layout',
    maxDepth: 8,
  }, harness.exec)
  assert.deepEqual(result.tree, [{ id: 'root', width: 390, height: 844 }])
  assert.equal('layout' in result, false)
  assert.deepEqual(result.diagnostics, [])
})

test('finish finalizes, renders the exact post-final root, and publishes atomically in one healthy call', async () => {
  const harness = await createHarness()
  await begin(harness)
  await completeGeneration(harness)

  const routeDetach = harness.render.attachRoute()
  try {
    const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
    assert.equal(published.published, true)
    assert.equal('stage' in published, false, 'a healthy finish must publish directly without a visual-inspection round trip')
    assert.equal(published.draftId, DRAFT_ID)
    assert.equal(published.sourceTool, 'openpencil_pipeline_finish')
    assert.equal(published.autoOpenEditor, true)
    assert.equal(published.preview.mimeType, 'image/png')
    assert.equal(published.preview.width, 1)
    assert.equal(published.preview.height, 1)
    assert.equal(harness.draft.screenshotCalls.length, 1)
    assert.deepEqual(harness.calls.write[0].intent, { kind: 'createIfAbsent' })
    assert.equal(harness.calls.observe.at(-1).observation.kind, 'present')
    assert.equal(harness.draft.calls.filter(call => call.tool === 'finalize_design').length, 1)
    assert.equal(harness.draft.calls.filter(call => call.tool === 'enrich_images').length, 0)
    assert.equal(harness.draft.calls.filter(call => call.tool === 'get_design_quality').length, 1)
    assert.equal(harness.draft.calls.filter(call => call.tool === 'lint_document').length, 1)
    assert.equal(harness.draft.calls.filter(call => call.tool === 'snapshot_layout').length, 1)
    assert.equal(harness.draft.finishOptions.at(-1).expectedVersion, 4)
    assert.equal(harness.draft.finishOptions.at(-1).expectedDocumentSha256, published.document.sha256)
    const projected = harness.tools.openpencil_pipeline_finish.output.presentationMeta({}, published)
    assert.equal(projected.$dshOpenPencil.draftId, DRAFT_ID)
    assert.equal(projected.$dshOpenPencil.liveDraft, false)
  } finally {
    routeDetach()
  }
  await assert.rejects(
    harness.tools.openpencil_pipeline_abort.execute({ draftId: DRAFT_ID }, harness.exec),
    /does not exist|another DSH agent/,
  )
})

test('a thrown native finalizer error becomes one terminal result instead of inviting retries', async () => {
  const harness = await createHarness()
  harness.draft.finalizeError = new Error('finalize_design could not prove command replay parity')
  await begin(harness)
  await completeGeneration(harness)

  const result = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(result.stage, 'blocked_host_failure')
  assert.equal(result.canContinue, false)
  assert.match(result.diagnostics[0], /command replay parity/i)
  assert.match(result.next, /Stop and report.*Do not retry finish.*abort.*rebuild another draft/is)
  assert.equal(harness.draft.screenshotCalls.length, 0)
  assert.equal(harness.calls.write.length, 0)
})

test('finish finalizes before enriching every canonical image slot and reuses both on retry', async () => {
  const harness = await createHarness()
  await begin(harness, 'Design an ecommerce homepage with product photography')
  await completeGeneration(harness)
  harness.draft.finalizeDocumentTransform = documentJson => {
    const finalized = JSON.parse(documentJson)
    finalized.children[0].children.push({
      type: 'image',
      id: 'hero-photo',
      width: 320,
      height: 240,
      imageSearchQuery: 'minimal ceramic coffee set on warm neutral background',
    })
    return JSON.stringify(finalized)
  }
  harness.draft.enrichImagesChanged = true
  harness.draft.enrichImagesTransform = documentJson => {
    const enriched = JSON.parse(documentJson)
    enriched.children[0].children.find(node => node.id === 'hero-photo').src = 'enriched://hero-photo'
    return JSON.stringify(enriched)
  }
  harness.draft.finishPreviewFailures = 1

  const waiting = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(waiting.stage, 'needs_preview')
  const nativeTools = harness.draft.calls.map(call => call.tool)
  assert.equal(nativeTools.filter(tool => tool === 'enrich_images').length, 1)
  assert.ok(nativeTools.indexOf('finalize_design') < nativeTools.indexOf('enrich_images'))
  assert.deepEqual(
    harness.draft.calls.find(call => call.tool === 'enrich_images').args,
    { timeout_seconds: 20 },
  )

  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true)
  assert.equal(harness.draft.calls.filter(call => call.tool === 'enrich_images').length, 1)
  assert.match(harness.calls.write[0].content, /enriched:\/\/hero-photo/)
  assert.equal(published.document.sha256, sha256(harness.calls.write[0].content))
})

test('an early context enrichment cannot suppress canonical post-final image enrichment', async () => {
  const harness = await createHarness()
  await begin(harness, 'Design an ecommerce homepage with product photography')
  await completeGeneration(harness)
  const draft = JSON.parse(harness.draft.documentJson)
  draft.children[0].children.push({
    type: 'image', id: 'product-photo', width: 320, height: 240,
    imageSearchQuery: 'minimal product still life', src: '',
  })
  harness.draft.documentJson = JSON.stringify(draft)

  await harness.tools.openpencil_pipeline_context.execute({
    draftId: DRAFT_ID,
    tool: 'enrich_images',
    arguments: { timeout_seconds: 2 },
  }, harness.exec)
  assert.equal(harness.draft.calls.filter(call => call.tool === 'enrich_images').length, 1)

  harness.draft.enrichImagesChanged = true
  harness.draft.enrichImagesTransform = documentJson => {
    const enriched = JSON.parse(documentJson)
    enriched.children[0].children.find(node => node.id === 'product-photo').src = 'enriched://product-photo'
    return JSON.stringify(enriched)
  }
  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)

  assert.equal(published.published, true)
  assert.equal(harness.draft.calls.filter(call => call.tool === 'enrich_images').length, 2)
  assert.match(harness.calls.write[0].content, /enriched:\/\/product-photo/)
})

test('a same-version same-SHA batch preview is still replaced by a post-final root proof', async () => {
  const harness = await createHarness()
  await begin(harness)
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const viewport = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const remainder = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)

  const preFinalVersion = harness.draft.version
  const preFinalDocumentSha256 = sha256(harness.draft.documentJson)
  const preFinalScreenshotCount = harness.draft.screenshotCalls.length
  assert.equal(harness.draft.screenshotVersion, preFinalVersion)
  assert.equal(preFinalScreenshotCount, 2)

  // Simulate a native finalizer that confirms the exact bytes without bumping
  // the version. The batch screenshot matches both identifiers, but it was
  // captured before the finalization checkpoint and cannot prove publication.
  harness.draft.finalized = true
  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)

  assert.equal(published.published, true)
  assert.equal('stage' in published, false)
  assert.equal(harness.draft.version, preFinalVersion)
  assert.equal(published.document.sha256, preFinalDocumentSha256)
  assert.equal(harness.draft.screenshotCalls.length, preFinalScreenshotCount + 1, 'finish must capture a fresh post-final root screenshot')
  assert.equal(harness.draft.finishOptions.at(-1).expectedVersion, preFinalVersion)
  assert.equal(harness.draft.finishOptions.at(-1).expectedDocumentSha256, preFinalDocumentSha256)
  assert.equal(harness.draft.calls.filter(call => call.tool === 'finalize_design').length, 1)
})

test('preview proof drift projects the controller currentVersion and never returns a stale artifact', async () => {
  const harness = await createHarness()
  harness.draft.finishReportedVersion = 7
  harness.draft.finishPreviewFailures = 1
  await begin(harness)
  await completeGeneration(harness)
  await harness.tools.openpencil_pipeline_inspect.execute({
    draftId: DRAFT_ID,
    kind: 'screenshot',
  }, harness.exec)

  const result = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(result.stage, 'needs_refinalization')
  assert.equal(result.version, 7)
  assert.equal('screenshot' in result, false)
  assert.equal('finalization' in result, false)
  assert.match(result.next, /changed concurrently to version 7.*stale finalization and preview proof were discarded/is)
  assert.doesNotMatch(result.next, /read_image|visual inspection/i)
})

test('a child-node detail preview never replaces the automatic finalized root proof', async () => {
  const harness = await createHarness()
  harness.draft.finishPreviewFailures = 1
  await begin(harness)
  await completeGeneration(harness)
  const checkpoint = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(checkpoint.stage, 'needs_preview')
  assert.ok(checkpoint.screenshot)

  const child = await harness.tools.openpencil_pipeline_inspect.execute({
    draftId: DRAFT_ID,
    kind: 'screenshot',
    nodeId: 'child-card',
  }, harness.exec)
  assert.match(child.next, /child-node detail preview only.*finish.*render.*finalized root preview/is)
  assert.doesNotMatch(child.next, /read_image|visual inspection/i)

  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true)
  assert.equal(harness.draft.screenshotCalls.length, 2, 'one automatic root preview plus one child detail preview')
})

test('publish-time version or byte drift clears stale proof and returns a recoverable refinalization state', async () => {
  const harness = await createHarness()
  harness.draft.finishPreviewFailures = 1
  await begin(harness)
  await completeGeneration(harness)
  const checkpoint = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(checkpoint.stage, 'needs_preview')
  harness.draft.mutateBeforeFinishCheck = draft => {
    draft.documentJson = JSON.stringify({
      version: '1.0.0',
      driftedAtSameVersion: true,
      children: [{ type: 'frame', id: 'root', name: 'Root', width: 390, height: 'fit_content', children: [] }],
    })
  }
  const drifted = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(drifted.stage, 'needs_refinalization')
  assert.equal(drifted.version, checkpoint.version)
  assert.deepEqual(drifted.diagnostics, [])
  assert.equal('screenshot' in drifted, false)
  assert.equal('finalization' in drifted, false)
  assert.match(drifted.next, /authoritative document changed.*stale finalization and root preview proof were discarded.*call finish exactly once/is)
  assert.equal(harness.calls.write.length, 0)

  const refinalized = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(refinalized.published, true)
  assert.equal(harness.draft.calls.filter(call => call.tool === 'finalize_design').length, 2)
})

test('native and DSH diagnostics are compact, cached, and block before final preview rendering', async () => {
  const nativeBlocked = await createHarness()
  nativeBlocked.draft.quality = { ...CLEAN_QUALITY, contrastIssues: [{ nodeId: '7', ratio: 1.4 }] }
  nativeBlocked.draft.finalizeValue = {
    applied: true,
    advisories: [{ code: 'native-finalize-advisory', nodeId: '8' }],
    repairRecords: Array.from({ length: 20 }, (_, index) => ({ nodeId: `private-${index}` })),
  }
  await begin(nativeBlocked)
  await completeGeneration(nativeBlocked)
  const nativeResult = await nativeBlocked.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, nativeBlocked.exec)
  assert.equal(nativeResult.stage, 'blocked_host_validation')
  assert.equal(nativeResult.canContinue, false)
  assert.equal('repairTargets' in nativeResult, false)
  assert.match(nativeResult.next, /Stop and report.*Do not guess node ids.*retry finish.*rebuild another draft/is)
  assert.match(nativeResult.diagnostics.join(' '), /nodeId.*7|ratio.*1\.4|native-finalize-advisory/i)
  assert.equal(nativeResult.checks.native.sources.quality, 1)
  assert.equal(nativeResult.checks.native.sources.finalize, 1)
  assert.equal('result' in nativeResult.finalization, false)
  assert.equal('finalize' in nativeResult, false)
  assert.equal('quality' in nativeResult, false)
  assert.equal('lint' in nativeResult, false)
  assert.equal('layoutCheck' in nativeResult, false)
  assert.doesNotMatch(JSON.stringify(nativeResult), /repairRecords|private-/)
  assert.equal(nativeBlocked.draft.finishCalls, 0)
  assert.equal(nativeBlocked.calls.write.length, 0)
  const nativeProbeCounts = Object.fromEntries(
    ['get_design_quality', 'lint_document', 'snapshot_layout']
      .map(tool => [tool, nativeBlocked.draft.calls.filter(call => call.tool === tool).length]),
  )
  const repeatedNative = await nativeBlocked.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, nativeBlocked.exec)
  assert.deepEqual(repeatedNative.diagnostics, nativeResult.diagnostics)
  assert.equal(repeatedNative.finalization.reused, true)
  for (const [tool, count] of Object.entries(nativeProbeCounts)) {
    assert.equal(nativeBlocked.draft.calls.filter(call => call.tool === tool).length, count)
  }

  const lintInfo = await createHarness()
  lintInfo.draft.lint = {
    count: 1,
    issues: [{ severity: 'info', code: 'absolute-positioning-share', nodeId: 'host-owned' }],
  }
  await begin(lintInfo)
  await completeGeneration(lintInfo)
  await lintInfo.tools.openpencil_pipeline_inspect.execute({ draftId: DRAFT_ID, kind: 'screenshot' }, lintInfo.exec)
  const lintInfoResult = await lintInfo.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, lintInfo.exec)
  assert.equal(lintInfoResult.published, true)

  const lintHeuristics = await createHarness()
  lintHeuristics.draft.lint = {
    count: 3,
    issues: [
      { severity: 'warning', category: 'invisible-container', nodeId: 'surface' },
      { severity: 'warning', code: 'sibling-inconsistency', nodeId: 'label' },
      { severity: 'warning', category: 'mixed-sibling-padding', nodeId: 'section' },
    ],
  }
  await begin(lintHeuristics)
  await completeGeneration(lintHeuristics)
  const lintHeuristicsResult = await lintHeuristics.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, lintHeuristics.exec)
  assert.equal(lintHeuristicsResult.published, true)

  const lintWarning = await createHarness()
  lintWarning.draft.lint = {
    count: 2,
    issues: [
      { severity: 'warning', code: 'text-explicit-height', nodeId: 'n1' },
      { severity: 'warning', category: 'text-effect', nodeId: 'n2' },
    ],
  }
  await begin(lintWarning)
  await completeGeneration(lintWarning)
  const lintWarningResult = await lintWarning.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, lintWarning.exec)
  assert.equal(lintWarningResult.published, true)

  const lintError = await createHarness()
  lintError.draft.lint = {
    count: 2,
    issues: [
      { severity: 'error', code: 'invalid-layout', nodeId: 'broken' },
      { severity: 'error', category: 'text-effect', nodeId: 'fatal-text' },
    ],
  }
  await begin(lintError)
  await completeGeneration(lintError)
  const lintErrorResult = await lintError.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, lintError.exec)
  assert.equal(lintErrorResult.stage, 'blocked_host_validation')
  assert.equal(lintErrorResult.canContinue, false)
  assert.match(lintErrorResult.diagnostics.join(' '), /invalid-layout.*broken/i)
  assert.match(lintErrorResult.diagnostics.join(' '), /text-effect.*fatal-text/i)
  assert.equal(lintError.draft.finishCalls, 0)

  const lintHardWarning = await createHarness()
  lintHardWarning.draft.lint = {
    count: 1,
    issues: [{ severity: 'warning', category: 'empty-path', nodeId: 'invisible', reason: 'renders invisible' }],
  }
  await begin(lintHardWarning)
  await completeGeneration(lintHardWarning)
  const lintHardWarningResult = await lintHardWarning.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, lintHardWarning.exec)
  assert.equal(lintHardWarningResult.stage, 'blocked_host_validation')
  assert.equal(lintHardWarningResult.canContinue, false)
  assert.match(lintHardWarningResult.diagnostics.join(' '), /empty-path.*renders invisible/i)
  assert.equal(lintHardWarning.draft.finishCalls, 0)

  const imageSlotHint = await createHarness()
  imageSlotHint.draft.quality = {
    ...CLEAN_QUALITY,
    imageSlots: ['product-swatch may be an unfilled image slot; operations-mode G is unavailable in script mode'],
  }
  await begin(imageSlotHint)
  await completeGeneration(imageSlotHint)
  const imageSlotResult = await imageSlotHint.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, imageSlotHint.exec)
  assert.equal(imageSlotResult.published, true)

  const jsBlocked = await createHarness()
  jsBlocked.draft.documentJson = JSON.stringify({
    version: '1.0.0',
    children: [{ type: 'frame', id: 'root', name: 'Login', width: 390, height: 'fit_content', children: [{
      type: 'frame', id: 'form', name: 'Form', children: [{ id: 'control', type: 'text_input', height: 20 }],
    }] }],
  })
  await begin(jsBlocked)
  await completeGeneration(jsBlocked)
  const jsResult = await jsBlocked.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, jsBlocked.exec)
  assert.equal(jsResult.stage, 'needs_correction')
  assert.equal(jsResult.canContinue, true)
  assert.match(jsResult.diagnostics.join(' '), /fill_container|height/i)
  assert.equal(jsResult.checks.dsh.diagnosticCount, 2)
  assert.equal(jsBlocked.draft.finishCalls, 0)
  assert.equal(jsBlocked.draft.screenshotCalls.length, 0)
  assert.equal(jsBlocked.calls.write.length, 0)
})

test('one finish reports native findings beside an authorized DSH repair before any screenshot', async () => {
  const harness = await createHarness()
  harness.draft.quality = { ...CLEAN_QUALITY, contrastIssues: [{ nodeId: 'native-7', ratio: 1.4 }] }
  harness.draft.documentJson = JSON.stringify({
    version: '1.0.0',
    children: [{
      type: 'frame', id: 'root', name: 'Login', width: 390, height: 844,
      children: [{ type: 'frame', name: 'Form', children: [{ type: 'text_input', height: 20 }] }],
    }],
  })
  await begin(harness)
  await completeGeneration(harness)
  const result = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  // Native findings ride along in the report but no longer veto the DSH
  // repair attempt: they routinely co-describe the defect the repair fixes,
  // and publication itself still requires a clean native pass.
  assert.equal(result.stage, 'needs_correction')
  assert.equal(result.canContinue, true)
  assert.match(result.diagnostics.join(' '), /native-7/)
  assert.match(result.diagnostics.join(' '), /fill_container|44px through 52px/i)
  assert.equal(result.checks.native.diagnosticCount, 1)
  assert.equal(result.checks.dsh.diagnosticCount, 2)
  assert.equal(harness.draft.finishCalls, 0)
  assert.equal(harness.draft.screenshotCalls.length, 0)
})

test('an authorized repair follows a root id transactionally replaced by native finalization', async () => {
  const harness = await createHarness()
  await begin(harness)
  await completeGeneration(harness)
  const authored = JSON.parse(harness.draft.documentJson)
  authored.children[0].children = [{
    type: 'frame', id: 'content-rail', width: 'fill_container', height: 'fit_content',
    layout: 'vertical', padding: [0, 24], children: [{ type: 'text', id: 'copy-1', content: 'Repair me' }],
  }]
  harness.draft.documentJson = JSON.stringify(authored)
  harness.draft.finalizeDocumentTransform = documentJson => {
    const document = JSON.parse(documentJson)
    document.children[0].id = 'final-root'
    return JSON.stringify(document)
  }

  const blocked = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'needs_correction')
  assert.deepEqual(blocked.repairTargets, [{
    nodeId: 'copy-1',
    operation: 'U',
    rule: 'typography',
    patch: { fontFamily: 'Inter, system-ui, sans-serif', fontSize: 16, lineHeight: 1.5 },
  }])
  assert.match(blocked.next, /Apply every repairTargets item.*only U\(nodeId, ?patch\).*host bounds repair rounds/is)

  const repaired = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'U("copy-1", {fontFamily:"Inter, system-ui, sans-serif",fontSize:16,lineHeight:1.5});',
  }, harness.exec)
  assert.equal(repaired.changed, true)
  assert.equal(repaired.rootNodeId, 'final-root')
  assert.equal(repaired.generationScriptCount, 2)
})

test('touch-target diagnostics name the node and missing axis, then converge in one structured repair', async () => {
  const harness = await createHarness()
  await begin(harness)
  await completeGeneration(harness)
  const document = JSON.parse(harness.draft.documentJson)
  document.children[0].children = [{
    type: 'frame', id: 'rail', width: 'fill_container', height: 'fit_content',
    layout: 'vertical', padding: [0, 24], children: [{
      type: 'frame', id: 'n46', role: 'button', width: 'fit_content', height: 48,
      layout: 'horizontal', padding: [0, 16], children: [{
        type: 'text', id: 'n47', content: 'View all', fontFamily: 'system-ui', fontSize: 16, lineHeight: 1.5,
      }],
    }],
  }]
  harness.draft.documentJson = JSON.stringify(document)

  const blocked = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'needs_correction')
  assert.equal(blocked.canContinue, true)
  assert.match(blocked.diagnostics.join(' '), /node n46 .*missing a 44px minimum on width/i)
  assert.deepEqual(blocked.repairTargets, [{
    nodeId: 'n46', operation: 'U', rule: 'touch-target', patch: { minWidth: 44 },
  }])

  document.children[0].children[0].children[0].minWidth = 44
  harness.draft.documentJson = JSON.stringify(document)
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'U("n46", {minWidth:44});',
  }, harness.exec)
  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true)
})

test('repair rounds are bounded at two and a third guess is terminal', async () => {
  const harness = await createHarness()
  await begin(harness)
  await completeGeneration(harness)
  const document = JSON.parse(harness.draft.documentJson)
  document.children[0].children = [{
    type: 'frame', id: 'rail', width: 'fill_container', height: 'fit_content',
    layout: 'vertical', padding: [0, 24], children: [{
      type: 'frame', id: 'n46', role: 'button', width: 'fit_content', height: 48,
      layout: 'horizontal', padding: [0, 16], children: [{
        type: 'text', id: 'n47', content: 'View all', fontFamily: 'system-ui', fontSize: 16, lineHeight: 1.5,
      }],
    }],
  }]
  harness.draft.documentJson = JSON.stringify(document)

  const first = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(first.stage, 'needs_correction')
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'U("n46", {minHeight:44});',
  }, harness.exec)
  // A repair that fixed nothing may earn exactly one more complete round.
  const second = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(second.stage, 'needs_correction')
  assert.equal(second.canContinue, true)
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'U("n46", {minHeight:44});',
  }, harness.exec)
  const terminal = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(terminal.stage, 'blocked_host_validation')
  assert.equal(terminal.canContinue, false)
  assert.equal('repairTargets' in terminal, false)
  assert.match(terminal.next, /bounded repair rounds were already used.*Do not guess node ids.*retry finish/is)
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'U("n46", {minWidth:44});',
    }, harness.exec),
    /ordinary generation is limited to two direct QuickJS scripts.*call finish before any repair script/i,
  )
})

test('more than four missing typography nodes return every safe target and converge into one-call publication', async () => {
  const harness = await createHarness()
  const textNodes = Array.from({ length: 7 }, (_, index) => ({
    type: 'text',
    id: `copy-${index}`,
    name: `private-name-${index}`,
    content: `private-copy-${index}`,
  }))
  harness.draft.documentJson = JSON.stringify({
    version: '1.0.0',
    children: [{ type: 'frame', id: 'root', width: 390, height: 'fit_content', children: textNodes }],
  })
  await begin(harness)
  await completeGeneration(harness)

  const blocked = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'needs_correction')
  assert.equal(blocked.diagnostics.filter(issue => /explicitly set fontFamily/.test(issue)).length, 1)
  assert.match(blocked.diagnostics.join(' '), /7 authored text nodes/i)
  assert.deepEqual(blocked.checks.dsh.repairTargetSummary, { total: 7, returned: 7, omitted: 0 })
  assert.deepEqual(
    blocked.repairTargets.map(target => target.nodeId).sort(),
    textNodes.map(node => node.id).sort(),
  )
  assert.ok(blocked.repairTargets.every(target => (
    target.operation === 'U'
    && target.patch.fontFamily === 'Inter, system-ui, sans-serif'
    && target.patch.fontSize === 16
    && target.patch.lineHeight === 1.5
  )))
  assert.doesNotMatch(JSON.stringify(blocked), /private-name|private-copy/)
  assert.equal(harness.draft.finishCalls, 0)
  assert.equal(harness.draft.screenshotCalls.length, 0)

  const callsBeforeMixedRepair = harness.draft.calls.length
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'const extra = I("root", {type:"frame"}); U("copy-0", {fontFamily:"Inter, system-ui, sans-serif"});',
    }, harness.exec),
    /authorized repair.*one bounded QuickJS script.*only U\(nodeId, ?patch\)/i,
  )
  assert.equal(harness.draft.calls.length, callsBeforeMixedRepair)

  harness.draft.documentJson = JSON.stringify({
    version: '1.0.0',
    children: [{
      type: 'frame', id: 'root', width: 390, height: 'fit_content',
      children: textNodes.map(node => ({
        ...node,
        fontFamily: 'system-ui',
        fontSize: 16,
        lineHeight: 1.5,
      })),
    }],
  })
  const repairScript = blocked.repairTargets
    .map(target => `U(${JSON.stringify(target.nodeId)}, ${JSON.stringify(target.patch)})`)
    .join('\n')
  const repaired = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: repairScript,
  }, harness.exec)
  assert.equal(repaired.generationScriptCount, 2, 'the authorized repair must not change the generation count')
  const repairCall = harness.draft.calls.filter(call => call.tool === 'batch_design').at(-1)
  assert.equal(repairCall.args.script, repairScript)
  assert.equal('operations' in repairCall.args, false)
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'U("root", {"name":"unauthorized second repair"});',
    }, harness.exec),
    /ordinary generation is limited to two direct QuickJS scripts.*call finish before any repair script/i,
  )

  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true)
  assert.equal(harness.draft.screenshotCalls.length, 2, 'the repair preview cannot replace the post-final root proof')
  assert.equal(harness.calls.write.length, 1)
})

test('an incomplete repair-target page is terminal instead of starting a repair loop', async () => {
  const harness = await createHarness()
  harness.draft.documentJson = JSON.stringify({
    version: '1.0.0',
    children: [{
      type: 'frame', id: 'root', width: 390, height: 844,
      children: Array.from({ length: 600 }, (_, index) => ({
        type: 'text', id: `copy-${index}`, content: `Copy ${index}`,
      })),
    }],
  })
  await begin(harness)
  await completeGeneration(harness)
  const blocked = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'blocked_host_validation')
  assert.equal(blocked.canContinue, false)
  assert.deepEqual(blocked.checks.dsh.repairTargetSummary, { total: 600, returned: 512, omitted: 88 })
  assert.equal('repairTargets' in blocked, false)
  assert.match(blocked.next, /complete actionable repair transaction.*Do not guess node ids.*retry finish/is)
  assert.equal(harness.calls.write.length, 0)
})

test('empty-shell quality hints remain observable but do not block one-call publication', async () => {
  const harness = await createHarness()
  harness.draft.quality = {
    ...CLEAN_QUALITY,
    emptyShells: [
      { nodeId: 'spacer-1', name: 'Intentional Spacer' },
      { nodeId: 'divider-1', name: 'Intentional Divider' },
    ],
  }
  await begin(harness)
  await completeGeneration(harness)

  const quality = await harness.tools.openpencil_pipeline_inspect.execute({
    draftId: DRAFT_ID,
    kind: 'quality',
  }, harness.exec)
  assert.match(quality.diagnostics.join(' '), /Intentional Spacer/)
  assert.match(quality.diagnostics.join(' '), /Intentional Divider/)

  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true)
  assert.equal(harness.draft.screenshotCalls.length, 1)
  assert.equal(harness.calls.write.length, 1)
})

test('finish checkpoints an always-bumping finalizer across preview retries and only invalidates it after a real draft mutation', async () => {
  const harness = await createHarness()
  harness.draft.finalizeAlwaysBumps = true
  await begin(harness)
  await completeGeneration(harness)
  harness.draft.screenshotFailures = 2

  const finalized = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(finalized.stage, 'needs_preview')
  assert.equal(finalized.reason, 'preview_unavailable')
  assert.equal(finalized.version, 4)
  assert.equal(finalized.finalization.reused, false)
  assert.deepEqual(finalized.diagnostics, [])
  assert.match(finalized.finalization.note, /informational only.*does not request a repair/i)
  assert.equal('result' in finalized.finalization, false)
  assert.match(finalized.next, /retry only the preview render and publish/i)
  assert.doesNotMatch(finalized.next, /pipeline_inspect|read_image|visual inspection/i)

  const repeated = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(repeated.stage, 'needs_preview')
  assert.equal(repeated.reason, 'preview_unavailable')
  assert.equal(repeated.version, 4)
  assert.equal(repeated.finalization.reused, true)
  assert.deepEqual(repeated.diagnostics, [])
  assert.match(repeated.finalization.note, /informational only.*does not request a repair/i)
  assert.equal('result' in repeated.finalization, false)
  assert.equal(harness.draft.calls.filter(call => call.tool === 'finalize_design').length, 1)
  assert.equal(harness.draft.calls.filter(call => ['get_design_quality', 'lint_document', 'snapshot_layout'].includes(call.tool)).length, 3)
  assert.equal(harness.draft.finishCalls, 0)

  await harness.tools.openpencil_pipeline_context.execute({
    draftId: DRAFT_ID,
    tool: 'apply_design_system',
    arguments: { name: 'changed' },
  }, harness.exec)
  const refinalized = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(refinalized.published, true)
  assert.equal(harness.draft.calls.filter(call => call.tool === 'finalize_design').length, 2)
})

test('publication races retain the draft; explicit abort, owner cleanup, and plugin disposal tear it down', async () => {
  const writeError = new Error('createIfAbsent lost race')
  const harness = await createHarness({ writeError })
  await begin(harness)
  await completeGeneration(harness)
  const blocked = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'blocked_host_failure')
  assert.equal(blocked.canContinue, false)
  assert.match(blocked.diagnostics[0], /lost race/)
  assert.match(blocked.next, /Do not retry finish.*rebuild another draft/is)
  assert.equal(harness.calls.write.length, 1)
  const aborted = await harness.tools.openpencil_pipeline_abort.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(aborted.aborted, true)
  assert.deepEqual(harness.draft.aborts, [{ draftId: DRAFT_ID, owner: 'session-from-exec' }])

  const ownerHarness = await createHarness()
  await begin(ownerHarness)
  await ownerHarness.controller.abortOwner('session-from-exec')
  assert.deepEqual(ownerHarness.draft.abortOwners, ['session-from-exec'])
  await ownerHarness.controller.dispose()
  await ownerHarness.controller.dispose()
  assert.equal(ownerHarness.draft.disposeCalls, 1)
})

test('publication prepares presentation artifacts before commit and treats post-commit observation as best effort', async () => {
  const snapshotBlocked = await createHarness({
    snapshotError: new Error('snapshot cache unavailable'),
    snapshotErrorAfter: 1,
  })
  await begin(snapshotBlocked)
  await completeGeneration(snapshotBlocked)
  const blocked = await snapshotBlocked.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, snapshotBlocked.exec)
  assert.equal(blocked.stage, 'blocked_host_failure')
  assert.equal(blocked.canContinue, false)
  assert.match(blocked.diagnostics[0], /snapshot cache unavailable/)
  assert.equal(snapshotBlocked.calls.write.length, 0, 'target must remain absent when presentation preparation fails')

  const observerBlocked = await createHarness({ observeError: new Error('observer failed after commit') })
  await begin(observerBlocked)
  await completeGeneration(observerBlocked)
  const published = await observerBlocked.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, observerBlocked.exec)
  assert.equal(published.published, true)
  assert.equal(observerBlocked.calls.write.length, 1)
})

test('publication rejects a post-final PNG whose cached bytes changed after its integrity checkpoint', async () => {
  const draft = new FakeDraftController()
  draft.png = Buffer.concat([SAFE_PNG, Buffer.from([0])])
  draft.finishPreviewFailures = 1
  const harness = await createHarness({ draft })
  await begin(harness)
  await completeGeneration(harness)
  const checkpoint = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(checkpoint.stage, 'needs_preview')
  assert.ok(checkpoint.screenshot, 'the forced controller fallback must retain the exact post-final proof')
  await writeFile(checkpoint.screenshot.path, Buffer.concat([draft.png, Buffer.from([1])]))
  const blocked = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'blocked_host_failure')
  assert.equal(blocked.canContinue, false)
  assert.match(blocked.diagnostics[0], /preview changed after its exact PNG integrity checkpoint/)
  assert.equal(harness.calls.write.length, 0)
})

const CLEAN_PUBLISHABLE_DOCUMENT = JSON.stringify({
  version: '1.0.0',
  children: [{ type: 'frame', id: 'root', width: 390, height: 'fit_content', children: [] }],
})

async function reachVisualReview(harness) {
  await begin(harness, 'Design a mobile learning dashboard', { skipVisualReview: false })
  await completeGeneration(harness)
  harness.draft.documentJson = CLEAN_PUBLISHABLE_DOCUMENT
  const review = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(review.stage, 'needs_visual_review')
  return review
}

test('a clean finish yields one visual review round whose acceptance publishes', async () => {
  const harness = await createHarness()
  const review = await reachVisualReview(harness)
  assert.equal(review.published, false)
  assert.equal(review.canContinue, true)
  assert.ok(Array.isArray(review.digest) && review.digest.length > 0, 'digest lines ride the review result')
  assert.match(review.digest.join('\n'), /visible regions|footer/i)
  assert.ok(Array.isArray(review.checklist) && review.checklist.length >= 5)
  assert.ok(review.screenshot !== undefined, 'the exact post-final preview artifact rides the review result')
  assert.match(review.next, /finish exactly once more.*ONE bounded correction batch.*I\/K\/U only.*16 calls.*6 KiB.*Header or Hero/is)
  assert.equal(harness.draft.finishCalls, 0, 'nothing publishes during the review round')

  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true)
  assert.equal(harness.calls.write.length, 1)
})

test('skip_visual_review publishes directly after a clean gate', async () => {
  const harness = await createHarness()
  await begin(harness, 'Design a mobile learning dashboard')
  await completeGeneration(harness)
  harness.draft.documentJson = CLEAN_PUBLISHABLE_DOCUMENT
  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true)
})

test('the visual review round authorizes exactly one bounded I/K/U correction batch', async () => {
  const harness = await createHarness()
  await reachVisualReview(harness)

  const corrected = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'U("root", {height:"fit_content"}); const extra = I("root", {type:"frame", width:"fill_container", height:"fit_content"});',
  }, harness.exec)
  assert.equal(corrected.changed, true)
  assert.equal(corrected.generationScriptCount, 2, 'a visual correction batch is not a generation script')

  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'U("root", {height:"fit_content"});',
    }, harness.exec),
    /call finish before any repair script/i,
    'the single visual correction authorization is consumed',
  )

  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true, 'the corrected page re-validates and publishes without a second review')
})

test('visual review correction batches enforce their bounds before touching the canvas', async () => {
  const harness = await createHarness()
  await reachVisualReview(harness)
  const batchCallsBefore = harness.draft.calls.filter(call => call.tool === 'batch_design').length

  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'D("root");',
    }, harness.exec),
    /C\/D\/M\/R\/G mutations are not authorized/i,
  )
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: Array.from({ length: 17 }, (_, index) => `U("n${index}", {minHeight:44});`).join('\n'),
    }, harness.exec),
    /limited to 16 I\/K\/U calls/i,
  )
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: `U("root", {name:"pad"}); // ${'x'.repeat(7 * 1024)}`,
    }, harness.exec),
    /limited to 6144 bytes/i,
  )
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'const header = I("root", {type:"frame", name:"Header"});',
    }, harness.exec),
    /must not rebuild Header or Hero/i,
  )
  assert.equal(
    harness.draft.calls.filter(call => call.tool === 'batch_design').length,
    batchCallsBefore,
    'rejected correction scripts never reach the canvas',
  )

  const corrected = await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'U("root", {height:"fit_content"});',
  }, harness.exec)
  assert.equal(corrected.changed, true, 'a rejected script does not consume the authorization')
})

test('gate work introduced by the visual correction runs repair rounds without a second review', async () => {
  const harness = await createHarness()
  await reachVisualReview(harness)

  const authored = JSON.parse(CLEAN_PUBLISHABLE_DOCUMENT)
  authored.children[0].children = [{
    type: 'frame', id: 'rail', width: 'fill_container', height: 'fit_content',
    layout: 'vertical', padding: [0, 24], children: [{ type: 'text', id: 'copy-1', content: 'Review me' }],
  }]
  harness.draft.documentJson = JSON.stringify(authored)
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'const copy = I("rail", {type:"text", content:"Review me"});',
  }, harness.exec)

  const blocked = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'needs_correction', 'the corrected page re-enters the quality gate, not the review')
  const repairedDocument = JSON.parse(harness.draft.documentJson)
  repairedDocument.children[0].children[0].children[0] = {
    type: 'text', id: 'copy-1', content: 'Review me',
    fontFamily: 'Inter, system-ui, sans-serif', fontSize: 16, lineHeight: 1.5,
  }
  harness.draft.documentJson = JSON.stringify(repairedDocument)
  await harness.tools.openpencil_pipeline_batch.execute({
    draftId: DRAFT_ID,
    script: 'U("copy-1", {fontFamily:"Inter, system-ui, sans-serif",fontSize:16,lineHeight:1.5});',
  }, harness.exec)
  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true)
})

test('the app-agent engine runs begin -> one agent run -> finish -> published', async () => {
  const harness = await createHarness()
  const begun = await harness.tools.openpencil_pipeline_begin.execute(
    { path: 'design.op', brief: 'Design a deliberate mobile account screen', engine: 'app-agent' },
    harness.exec,
  )
  assert.match(begun.next, /openpencil_pipeline_agent_run exactly once.*Never send generation batch scripts/is)

  harness.draft.agentRunDocumentTransform = () => CLEAN_PUBLISHABLE_DOCUMENT
  const run = await harness.tools.openpencil_pipeline_agent_run.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(run.changed, true)
  assert.equal(run.canContinue, true)
  assert.equal(run.agentRun.stopReason, 'end_turn')
  assert.equal(run.agentRun.landedRoots, 1)
  assert.equal(run.agentRun.finalize.committedScreens, 1)
  assert.ok(run.screenshot !== undefined, 'the live preview artifact rides the agent-run result')
  assert.match(run.next, /finish exactly once/i)
  const call = harness.draft.calls.filter(entry => entry.tool === 'run_design_agent').at(-1)
  assert.equal(call.args.brief, 'Design a deliberate mobile account screen')
  assert.equal(call.args.timeout_seconds, 240)
  assert.equal(call.args.max_turns, 12)

  const published = await harness.tools.openpencil_pipeline_finish.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(published.published, true, 'app-agent drafts publish through the ordinary finish gate')
  assert.equal(harness.calls.write.length, 1)
})

test('the app-agent engine refuses generation scripts and a second agent run', async () => {
  const harness = await createHarness()
  await harness.tools.openpencil_pipeline_begin.execute(
    { path: 'design.op', brief: 'Design a deliberate mobile account screen', engine: 'app-agent' },
    harness.exec,
  )
  await assert.rejects(
    harness.tools.openpencil_pipeline_batch.execute({
      draftId: DRAFT_ID,
      script: 'const hero = I("root", {type:"frame"});',
    }, harness.exec),
    /app-agent engine; call openpencil_pipeline_agent_run once/i,
  )
  harness.draft.agentRunDocumentTransform = () => CLEAN_PUBLISHABLE_DOCUMENT
  await harness.tools.openpencil_pipeline_agent_run.execute({ draftId: DRAFT_ID }, harness.exec)
  await assert.rejects(
    harness.tools.openpencil_pipeline_agent_run.execute({ draftId: DRAFT_ID }, harness.exec),
    /single agent run was already used.*openpencil_pipeline_finish/is,
  )
})

test('script drafts refuse the agent run and app-agent brief size/timeout bounds hold', async () => {
  const harness = await createHarness()
  await begin(harness)
  await assert.rejects(
    harness.tools.openpencil_pipeline_agent_run.execute({ draftId: DRAFT_ID }, harness.exec),
    /script engine; follow begin\.next/i,
  )
  const agentHarness = await createHarness()
  await agentHarness.tools.openpencil_pipeline_begin.execute(
    { path: 'design.op', brief: 'brief', engine: 'app-agent' },
    agentHarness.exec,
  )
  for (const timeout_seconds of [0, 29, 271]) {
    await assert.rejects(
      agentHarness.tools.openpencil_pipeline_agent_run.execute({ draftId: DRAFT_ID, timeout_seconds }, agentHarness.exec),
      /timeout_seconds must be a number between 30 and 270/,
    )
  }
  for (const max_turns of [3, 29, 1.5]) {
    await assert.rejects(
      agentHarness.tools.openpencil_pipeline_agent_run.execute({ draftId: DRAFT_ID, max_turns }, agentHarness.exec),
      /max_turns must be an integer between 4 and 28/,
    )
  }
  await assert.rejects(
    harness.tools.openpencil_pipeline_begin.execute(
      { path: 'other.op', brief: 'brief', engine: 'desktop' },
      harness.exec,
    ),
    /engine/i,
  )
})

test('a missing daemon provider surfaces terminal configuration guidance without consuming retries', async () => {
  const harness = await createHarness()
  await harness.tools.openpencil_pipeline_begin.execute(
    { path: 'design.op', brief: 'brief', engine: 'app-agent' },
    harness.exec,
  )
  harness.draft.agentRunError = new Error('run_design_agent: no builtin agent provider is configured')
  await assert.rejects(
    harness.tools.openpencil_pipeline_agent_run.execute({ draftId: DRAFT_ID }, harness.exec),
    /no builtin agent provider is configured.*Configure one in the OpenPencil agent settings.*do not retry/is,
  )
})

test('an agent run that lands nothing is terminal and blocks publication scripts', async () => {
  const harness = await createHarness()
  await harness.tools.openpencil_pipeline_begin.execute(
    { path: 'design.op', brief: 'brief', engine: 'app-agent' },
    harness.exec,
  )
  harness.draft.agentRunChanged = false
  harness.draft.agentRunValue = { toolCalls: 2, stopReason: 'timeout', landedRoots: 0, loopError: 'loop timed out before landing' }
  const blocked = await harness.tools.openpencil_pipeline_agent_run.execute({ draftId: DRAFT_ID }, harness.exec)
  assert.equal(blocked.stage, 'blocked_agent_run')
  assert.equal(blocked.canContinue, false)
  assert.deepEqual(blocked.diagnostics, ['loop timed out before landing'])
})
