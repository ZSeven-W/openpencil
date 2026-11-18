import assert from 'node:assert/strict'
import { createHash, randomBytes } from 'node:crypto'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { after, test } from 'node:test'

const { createDesignNewTool } = await import('../lib/new-tool.js')
const { RenderAccessController } = await import('../lib/renderer.js')

const previousDshHome = process.env.DSH_HOME
const testRoot = await mkdtemp(join(tmpdir(), 'dsh-openpencil-new-tool-'))
const SIMPLE_SCRIPT = 'const root = I(null, { type: "frame", name: "Forage", width: 390, height: 844 });\nI(root, { type: "text", name: "Title", content: "Forage", width: "fill_container", height: 44, fontFamily: "Inter, system-ui, sans-serif", fontSize: 32, lineHeight: 1.2 });'
process.env.DSH_HOME = join(testRoot, 'dsh-home')
after(async () => {
  if (previousDshHome === undefined) delete process.env.DSH_HOME
  else process.env.DSH_HOME = previousDshHome
  await rm(testRoot, { recursive: true, force: true })
})

function createHarness(options = {}) {
  const workspaceRoot = '/workspace/project'
  const requestedPath = options.requestedPath ?? 'designs/forage.op'
  // The tool runs a REAL lstat on the parent directory of the resolved
  // process path, so the default must be a directory that exists on every
  // platform (`/private/tmp` is macOS-only and breaks Linux CI).
  const processPath = options.processPath ?? join(tmpdir(), 'forage.op')
  const target = { targetKey: `local:${processPath}`, displayPath: requestedPath }
  const policy = { mode: 'workspace-write', workspaceRoot }
  const documentJson = options.documentJson ?? '{"version":"1.0.0","children":[{"id":"home"}]}\n'
  const calls = {
    policy: [],
    lstat: [],
    resolve: [],
    processPath: [],
    batch: [],
    grant: [],
    write: [],
    observe: [],
  }
  const session = { id: 'session-new-tool', header: { cwd: workspaceRoot } }
  const signal = new AbortController().signal
  const exec = { agent: { id: 'agent-new-tool', session }, signal }
  const writeVersion = 'version-after-create'

  const fs = {
    sandboxMode: options.sandboxMode,
    async lstat(path, resolveOptions, receivedSignal) {
      calls.lstat.push({ path, options: resolveOptions, signal: receivedSignal })
      return options.pathInfo
    },
    async resolve(path, resolveOptions) {
      calls.resolve.push({ path, options: resolveOptions })
      return target
    },
    async stat() {
      return options.resolvedInfo
    },
    processPath(receivedTarget) {
      calls.processPath.push(receivedTarget)
      return processPath
    },
    async writeText(receivedTarget, content, intent, receivedSignal, sandboxPolicy) {
      calls.write.push({ target: receivedTarget, content, intent, signal: receivedSignal, sandboxPolicy })
      if (options.writeError !== undefined) throw options.writeError
      return {
        operation: 'create',
        version: writeVersion,
        before: null,
        after: options.writtenText ?? content,
      }
    },
  }
  const sandboxPolicy = {
    resolve(request) {
      calls.policy.push(request)
      return policy
    },
  }
  const editorHost = {
    async createDocumentBatch(batchOptions) {
      calls.batch.push(batchOptions)
      if (options.batchError !== undefined) throw options.batchError
      return {
        documentJson,
        result: options.batchResult ?? { applied: true, inserted: 1 },
      }
    },
    grantFor(sourcePath, sourceSha256) {
      calls.grant.push({ sourcePath, sourceSha256 })
      return {
        enabled: true,
        launchUrl: '/_dsh/dsh-openpencil/editor/live/launch',
        refreshUrl: '/_dsh/dsh-openpencil/editor/live/refresh',
      }
    },
  }
  const render = new RenderAccessController(randomBytes(32))
  render.attachRoute()
  const tool = createDesignNewTool(editorHost, {
    fs,
    sandboxPolicy,
    render,
    observe(receivedTarget, observation, actor) {
      calls.observe.push({ target: receivedTarget, observation, actor })
    },
  })
  return { calls, documentJson, exec, policy, processPath, requestedPath, signal, target, tool, writeVersion }
}

test('openpencil_new publishes one completed QuickJS build through guarded DSH filesystem services', async () => {
  const harness = createHarness()
  const script = SIMPLE_SCRIPT

  const result = await harness.tool.execute({
    path: harness.requestedPath,
    script,
    canvasWidth: 390,
  }, harness.exec)

  assert.deepEqual(harness.calls.policy, [{ session: harness.exec.agent.session }])
  assert.deepEqual(harness.calls.lstat, [{
    path: harness.requestedPath,
    options: { cwd: harness.policy.workspaceRoot },
    signal: harness.signal,
  }])
  assert.equal(harness.calls.resolve.length, 1)
  assert.equal(harness.calls.resolve[0].path, harness.requestedPath)
  assert.deepEqual(harness.calls.resolve[0].options, {
    cwd: harness.policy.workspaceRoot,
    signal: harness.signal,
  })
  assert.deepEqual(harness.calls.processPath, [harness.target])
  assert.deepEqual(harness.calls.batch, [{
    script,
    canvasWidth: 390,
    signal: harness.signal,
  }])
  assert.deepEqual(harness.calls.write, [{
    target: harness.target,
    content: harness.documentJson,
    intent: { kind: 'createIfAbsent' },
    signal: harness.signal,
    sandboxPolicy: harness.policy,
  }])
  assert.deepEqual(harness.calls.observe, [
    { target: harness.target, observation: { kind: 'absent' }, actor: harness.exec },
    { target: harness.target, observation: { kind: 'present', version: harness.writeVersion }, actor: harness.exec },
  ])

  const expectedText = harness.documentJson
  assert.deepEqual(result, {
    path: harness.processPath,
    filename: 'forage.op',
    bytes: Buffer.byteLength(expectedText),
    sha256: createHash('sha256').update(expectedText).digest('hex'),
    created: true,
    applied: true,
    saved: true,
    sourceTool: 'openpencil_new',
    previewIntent: 'document',
    editable: true,
    autoOpenEditor: true,
    document: {
      path: join(process.env.DSH_HOME, 'cache', 'dsh-openpencil', 'snapshots', `${createHash('sha256').update(expectedText).digest('hex')}.op`),
      filename: `${createHash('sha256').update(expectedText).digest('hex')}.op`,
      mimeType: 'application/json',
      bytes: Buffer.byteLength(expectedText),
      sha256: createHash('sha256').update(expectedText).digest('hex'),
    },
    result: { applied: true, inserted: 1 },
    note: `Created and saved ${harness.processPath}; DSH requests the managed OpenPencil editor to open automatically when the editor surface is idle.`,
  })
})

test('openpencil_new hashes the filesystem-authoritative written text', async () => {
  const normalized = '{"version":"1.0.0","children":[]}\n'
  const harness = createHarness({ writtenText: normalized })

  const result = await harness.tool.execute({
    path: harness.requestedPath,
    script: SIMPLE_SCRIPT,
  }, harness.exec)

  assert.equal(result.bytes, Buffer.byteLength(normalized))
  assert.equal(result.sha256, createHash('sha256').update(normalized).digest('hex'))
  assert.equal(await readFile(result.document.path, 'utf8'), normalized)
  assert.notEqual(normalized, harness.documentJson, 'the authoritative filesystem result differs from the daemon proposal')
})

test('openpencil_new rejects high-confidence design defects before publishing the document', async () => {
  const unsafeDesign = JSON.stringify({
    version: '1.0.0',
    children: [{
      type: 'frame',
      name: 'Login',
      children: [{
        type: 'frame',
        name: 'Form card',
        children: [{
          type: 'text_input',
          name: 'Password',
          height: 48,
          trailingIcon: '👁',
        }],
      }],
    }],
  })
  const harness = createHarness({ documentJson: unsafeDesign })

  await assert.rejects(
    harness.tool.execute({ path: harness.requestedPath, script: SIMPLE_SCRIPT }, harness.exec),
    error => error instanceof Error
      && /failed quality checks \(3 issues\)/.test(error.message)
      && /width to "fill_container"/.test(error.message)
      && /password text input must explicitly set secure to true/.test(error.message)
      && /use icon_font or a component icon/.test(error.message)
      && !error.message.includes('Password')
      && !error.message.includes('👁'),
  )

  assert.equal(harness.calls.batch.length, 1, 'quality inspection runs on the finalized transient document')
  assert.equal(harness.calls.write.length, 0, 'a rejected design is never published through DSH fs')
  assert.deepEqual(harness.calls.observe, [
    { target: harness.target, observation: { kind: 'absent' }, actor: harness.exec },
  ])
})

test('openpencil_new projects a signed document-only grant and auto-opens the editor', async () => {
  const harness = createHarness()
  const result = await harness.tool.execute({
    path: harness.requestedPath,
    script: SIMPLE_SCRIPT,
  }, harness.exec)

  const projected = harness.tool.output.presentationMeta({}, result)
  const envelope = projected.$dshOpenPencil
  assert.equal(envelope.schemaVersion, 2)
  assert.equal('image' in envelope, false)
  assert.equal(envelope.document.path, harness.processPath)
  assert.equal(envelope.document.sha256, result.document.sha256)
  assert.match(envelope.document.url, /^\/_dsh\/dsh-openpencil\/render\//)
  assert.equal(envelope.document.downloadUrl, `${envelope.document.url}?download=1`)
  assert.equal(envelope.editor.launchUrl, '/_dsh/dsh-openpencil/editor/live/launch')
  assert.equal(envelope.autoOpenEditor, true)
  assert.deepEqual(harness.calls.grant, [{
    sourcePath: harness.processPath,
    sourceSha256: result.document.sha256,
  }])
})

test('openpencil_new rejects an existing target before starting a design daemon', async () => {
  const harness = createHarness({
    pathInfo: { type: 'file', version: 'existing-version', size: 12 },
  })

  await assert.rejects(
    harness.tool.execute({
      path: harness.requestedPath,
      script: SIMPLE_SCRIPT,
    }, harness.exec),
    /target already exists: designs\/forage\.op/,
  )
  assert.equal(harness.calls.resolve.length, 0)
  assert.equal(harness.calls.batch.length, 0)
  assert.equal(harness.calls.write.length, 0)
  assert.deepEqual(harness.calls.observe, [])
})

test('openpencil_new rejects a resolved target that appeared after the no-follow probe', async () => {
  const harness = createHarness({
    resolvedInfo: { type: 'file', version: 'raced-version', size: 24 },
  })

  await assert.rejects(
    harness.tool.execute({
      path: harness.requestedPath,
      script: SIMPLE_SCRIPT,
    }, harness.exec),
    /target already exists: designs\/forage\.op/,
  )
  assert.equal(harness.calls.batch.length, 0)
  assert.equal(harness.calls.write.length, 0)
  assert.deepEqual(harness.calls.observe, [
    { target: harness.target, observation: { kind: 'present', version: 'raced-version' }, actor: harness.exec },
  ])
})

test('openpencil_new fails before daemon startup in a read-only sandbox', async () => {
  const harness = createHarness({ sandboxMode: 'read-only' })
  harness.policy.mode = 'read-only'

  await assert.rejects(
    harness.tool.execute({
      path: harness.requestedPath,
      script: SIMPLE_SCRIPT,
    }, harness.exec),
    /requires Workspace Write access/,
  )
  assert.equal(harness.calls.lstat.length, 0)
  assert.equal(harness.calls.batch.length, 0)
  assert.equal(harness.calls.write.length, 0)
})

test('openpencil_new preserves create-if-absent publication failures', async () => {
  const race = new Error('competitor created the target')
  const harness = createHarness({ writeError: race })

  await assert.rejects(
    harness.tool.execute({
      path: harness.requestedPath,
      script: SIMPLE_SCRIPT,
    }, harness.exec),
    error => error === race,
  )
  assert.equal(harness.calls.batch.length, 1)
  assert.equal(harness.calls.write.length, 1)
  assert.deepEqual(harness.calls.write[0].intent, { kind: 'createIfAbsent' })
  assert.deepEqual(harness.calls.observe, [
    { target: harness.target, observation: { kind: 'absent' }, actor: harness.exec },
  ])
})

test('openpencil_new never publishes a target when the transactional design batch fails', async () => {
  const batchFailure = new Error('batch_design rejected the QuickJS script')
  const harness = createHarness({ batchError: batchFailure })

  await assert.rejects(
    harness.tool.execute({
      path: harness.requestedPath,
      script: SIMPLE_SCRIPT,
    }, harness.exec),
    error => error === batchFailure,
  )
  assert.equal(harness.calls.batch.length, 1)
  assert.equal(harness.calls.write.length, 0)
  assert.deepEqual(harness.calls.observe, [
    { target: harness.target, observation: { kind: 'absent' }, actor: harness.exec },
  ])
})

test('openpencil_new validates paths and programs before acquiring filesystem or daemon capabilities', async () => {
  const harness = createHarness()
  const tooLarge = 'x'.repeat(256 * 1024 + 1)
  for (const [args, pattern] of [
    [{ path: '   ', script: SIMPLE_SCRIPT }, /path is required/],
    [{ path: 'design.json', script: SIMPLE_SCRIPT }, /path must end in \.op/],
    [{ path: 'design.op', script: '   ' }, /script must not be empty/],
    [{ path: 'design.op', script: tooLarge }, /script is too large/],
    [{ path: 'design.op', script: SIMPLE_SCRIPT, canvasWidth: 0 }, /canvasWidth must be greater than 0/],
    [{ path: 'design.op', script: SIMPLE_SCRIPT, canvasWidth: 16385 }, /canvasWidth must be greater than 0/],
  ]) {
    await assert.rejects(harness.tool.execute(args, harness.exec), pattern)
  }
  assert.equal(harness.calls.policy.length, 0)
  assert.equal(harness.calls.lstat.length, 0)
  assert.equal(harness.calls.batch.length, 0)
  assert.equal(harness.calls.write.length, 0)
})

test('openpencil_new refuses a provider-resolved non-op process path', async () => {
  const harness = createHarness({ processPath: '/workspace/project/designs/forage.json' })

  await assert.rejects(
    harness.tool.execute({
      path: harness.requestedPath,
      script: SIMPLE_SCRIPT,
    }, harness.exec),
    /resolved target must end in \.op/,
  )
  assert.equal(harness.calls.batch.length, 0)
  assert.equal(harness.calls.write.length, 0)
  assert.deepEqual(harness.calls.observe, [])
})

test('openpencil_new exposes a strict creation schema and output contract', () => {
  const harness = createHarness()
  assert.equal(harness.tool.name, 'openpencil_new')
  assert.deepEqual([...harness.tool.parameters.required].sort(), ['path', 'script'])
  assert.deepEqual(Object.keys(harness.tool.parameters.properties).sort(), ['canvasWidth', 'path', 'script'])
  assert.equal('operations' in harness.tool.parameters.properties, false)
  assert.equal('postProcess' in harness.tool.parameters.properties, false)
  assert.equal(harness.tool.output.schema.additionalProperties, false)
  assert.equal(harness.tool.output.schema.properties.created.const, true)
  assert.equal(harness.tool.output.schema.properties.applied.const, true)
  assert.equal(harness.tool.output.schema.properties.saved.const, true)
  assert.equal(harness.tool.output.schema.properties.sourceTool.const, 'openpencil_new')
  assert.equal(harness.tool.output.schema.properties.previewIntent.const, 'document')
  assert.equal(harness.tool.output.schema.properties.editable.const, true)
  assert.equal(harness.tool.output.schema.properties.autoOpenEditor.const, true)
  assert.deepEqual(harness.tool.presentCall({
    path: harness.requestedPath,
    script: SIMPLE_SCRIPT,
  }), {
    card: 'generic',
    title: `Create ${harness.requestedPath}`,
    kind: 'execute',
    locations: [{ path: harness.requestedPath }],
  })

  const decisionContract = `${harness.tool.description}\n${harness.tool.parameters.properties.script.description}`
  assert.doesNotMatch(decisionContract, /\{\.\.\.\}/, 'model-facing examples must be executable rather than schematic')
  assert.match(decisionContract, /explicitly requested simple one-shot/i)
  assert.match(decisionContract, /ordinary natural-language generation uses openpencil_pipeline_begin/i)
  assert.match(decisionContract, /do not ask the user to open a sidebar/i)
  assert.doesNotMatch(decisionContract, /load the bundled openpencil-design skill/i)
  assert.match(decisionContract, /sandboxed QuickJS/i)
  assert.match(decisionContract, /script (?:program )?string.*outer run_code runtime/i)
  assert.match(decisionContract, /I\/K do not exist in the outer run_code runtime/i)
  assert.match(decisionContract, /I\(parent, node\).*K\(kitId, parent, overrides\)/i)
  assert.match(decisionContract, /const\/let.*arrays.*for\.\.\.of/i)
  assert.match(decisionContract, /C\/U\/D\/M\/R\/G.*not available/i)
  assert.match(decisionContract, /const root = I\(null,/)
  assert.match(decisionContract, /const card = I\(root,/)
  assert.match(decisionContract, /type: "text".*fontFamily: "Inter, system-ui, sans-serif".*lineHeight: 1\.2/s)
  assert.match(decisionContract, /binding returned from an earlier I\(\)/i)
  assert.match(decisionContract, /never (?:write )?I\("root",/i)
  assert.match(decisionContract, /do not set node ids yourself/i)
  assert.match(decisionContract, /multiple semantic I\(\) calls/i)
  assert.match(decisionContract, /layout, gap, padding, justifyContent, alignItems, cornerRadius, and textAlign/i)
  assert.match(decisionContract, /fill: \[\{type:"solid",color:/i)
  assert.match(decisionContract, /stroke: \{thickness:1,fill:/i)
  assert.match(decisionContract, /justifyContent start\/center\/end\/space_between\/space_around/i)
  assert.match(decisionContract, /native text_input/i)
  assert.match(decisionContract, /text_input and select.*width:"fill_container".*44-52px/i)
  assert.match(decisionContract, /text_area.*multi-line height.*96-160px/i)
  assert.match(decisionContract, /never use emoji.*interface icon/i)
  assert.match(decisionContract, /icon_font.*real glyph name/i)
  assert.match(decisionContract, /style fingerprint/i)
  assert.match(decisionContract, /generic initial-letter logo.*white form card.*saturated button/i)
  assert.match(decisionContract, /every generated visible text node.*fontFamily:"Inter, system-ui, sans-serif"/i)
  assert.match(decisionContract, /Desktop uses its bundled Inter.*Web host.*generic fallback.*never use bare Inter or pure system-ui/is)
  assert.match(decisionContract, /Chinese interfaces use lineHeight at least 1\.3/i)
  assert.match(decisionContract, /another named family.*confirms it is installed/i)
  assert.doesNotMatch(decisionContract, /default to (?:pure )?system-ui/i)
  assert.match(decisionContract, /never invent paddingX, paddingY, radius, strokeWidth, align/i)
  assert.match(decisionContract, /negative space/i)
  assert.match(decisionContract, /at most two saturated colors/i)
  assert.match(decisionContract, /16-20px horizontal padding/i)
  assert.match(decisionContract, /post-processing and finalization pipeline/i)
  assert.match(decisionContract, /repairs deterministic structure and layout defects/i)
  assert.match(decisionContract, /does not invent the visual concept, typography, component sizing/i)
  assert.match(decisionContract, /rejects high-confidence form sizing and emoji-icon defects/i)
  assert.match(decisionContract, /retry the same target path.*no file was created/i)
  assert.match(decisionContract, /one call creates the file and requests editor auto-open/i)
  assert.match(decisionContract, /opens.*sidebar when that surface is idle/i)
  assert.match(decisionContract, /never replaces an editor already owned by another session/i)
  assert.match(decisionContract, /Stop after success/i)
  assert.match(decisionContract, /Do not call openpencil_render, read_image, or openpencil_pipeline_inspect as a completion gate/i)
  assert.doesNotMatch(decisionContract, /visual QA|vision.*available/i)
  assert.doesNotMatch(decisionContract, /call openpencil_render.*editable=true/i)
})
