import assert from 'node:assert/strict'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

test('plugin mounts its HTTP routes through the rc.2 webServer service', async () => {
  const root = await mkdtemp(join(tmpdir(), 'dsh-openpencil-host-api-'))
  const previousDshHome = process.env.DSH_HOME
  process.env.DSH_HOME = join(root, 'dsh-home')

  const routeRegistrations = []
  const routeRemovals = []
  const injectedServices = []
  const registeredTools = []
  const emittedEvents = []
  const eventListeners = []
  const disposedEventListeners = []
  let disposeInjectedRoutes
  let disposeInjectedAuthentication
  let registeredAuthentication
  const authenticatedRequest = {}
  let releaseEditorHostDispose
  const editorHostDisposeBarrier = new Promise(resolve => { releaseEditorHostDispose = resolve })
  const { EditorHostController } = await import('../lib/editor-host.js')
  const { DesignDraftController } = await import('../lib/design-draft-controller.js')
  const { DesignDraftToolController } = await import('../lib/design-draft-tools.js')
  const originalEditorHostDispose = EditorHostController.prototype.dispose
  const originalAttachAuthentication = EditorHostController.prototype.attachRequestAuthentication
  EditorHostController.prototype.attachRequestAuthentication = function (authenticate) {
    registeredAuthentication = authenticate
    return originalAttachAuthentication.call(this, authenticate)
  }
  const originalAbortOwner = DesignDraftController.prototype.abortOwner
  const originalHasActiveDraft = DesignDraftToolController.prototype.hasActiveDraft
  let editorHostDisposeCalls = 0
  const abortedDraftOwners = []
  const activeDraftOwners = new Set()
  let requestWaterfallNextResolved = false
  EditorHostController.prototype.dispose = function disposeWithBarrier() {
    editorHostDisposeCalls += 1
    return editorHostDisposeBarrier
  }
  DesignDraftController.prototype.abortOwner = async function abortOwnerForHostTest(owner) {
    abortedDraftOwners.push(owner)
    return 0
  }
  DesignDraftToolController.prototype.hasActiveDraft = function hasActiveDraftForHostTest(owner) {
    assert.equal(requestWaterfallNextResolved, true, 'agent/request must resolve next() before inspecting draft state')
    return activeDraftOwners.has(owner)
  }

  const webServer = {
    register(route) {
      routeRegistrations.push(route)
      return () => { routeRemovals.push(route.path) }
    },
  }
  const ctx = {
    sessions: { get() { return undefined } },
    fs: {
      async lstat() { return undefined },
      async resolve(path) { return { targetKey: `test:${path}`, displayPath: path } },
      processPath(target) { return String(target.targetKey).replace(/^test:/, '') },
      async writeText(_target, content) {
        return { operation: 'create', version: 'test-version', before: null, after: content }
      },
    },
    sandboxPolicy: {
      resolve() { return { mode: 'workspace-write', workspaceRoot: root } },
    },
    get() { return undefined },
    tools: {
      register(tool) {
        registeredTools.push(tool)
        return () => {}
      },
    },
    effect(install) {
      return install()
    },
    on(name, listener) {
      const registration = { name, listener }
      eventListeners.push(registration)
      return () => { disposedEventListeners.push(registration) }
    },
    emit(...args) {
      emittedEvents.push(args)
    },
    inject(services, install) {
      injectedServices.push([...services])
      if (services.includes('connection')) {
        install({
          connection: { requestRejection(req) { return req === authenticatedRequest ? undefined : 401 } },
          effect(mount) { disposeInjectedAuthentication = mount(); return disposeInjectedAuthentication },
        })
        return
      }
      install({
        webServer,
        effect(mount) {
          disposeInjectedRoutes = mount()
          return disposeInjectedRoutes
        },
      })
    },
    logger: { info() {} },
  }

  try {
    const { apply, inject } = await import(`../lib/index.js?host-api=${Date.now()}`)
    const disposePlugin = await apply(ctx)

    assert.deepEqual(inject, ['tools', 'sessions', 'fs', 'sandboxPolicy'])
    assert.deepEqual(injectedServices, [['skills'], ['systemPrompt'], ['connection'], ['webServer']])
    // 14 = 5 legacy/compat tools + 7 pipeline tools (begin/context/batch/agent_run/inspect/finish/abort)
    //   + 2 opt-in style planner tools (style_plan/style_confirm).
    assert.equal(registeredTools.length, 14)
    assert.deepEqual(registeredTools.map(tool => tool.name), [
      'openpencil_render',
      'openpencil_selection',
      'openpencil_new',
      'openpencil_create',
      'openpencil_edit',
      'openpencil_pipeline_begin',
      'openpencil_pipeline_context',
      'openpencil_pipeline_batch',
      'openpencil_pipeline_agent_run',
      'openpencil_pipeline_inspect',
      'openpencil_pipeline_finish',
      'openpencil_pipeline_abort',
      'openpencil_style_plan',
      'openpencil_style_confirm',
    ])
    assert.equal(registeredTools.some(tool => tool.name === 'design_render'), false, 'legacy render alias must remain client-only')
    const registeredByName = Object.fromEntries(registeredTools.map(tool => [tool.name, tool]))
    assert.match(
      registeredByName.openpencil_render.description,
      /ordinary new design.*openpencil_pipeline_begin.*openpencil_new only.*explicitly requests.*simple one-shot.*existing \.op.*explicit user request for a PNG.*not a model completion gate/is,
    )
    assert.match(
      registeredByName.openpencil_create.description,
      /ordinary new design.*openpencil_pipeline_begin.*openpencil_new.*only.*explicitly requested simple one-shot/is,
    )
    assert.equal(registeredTools[0].output.schema.properties.sourceTool.const, 'openpencil_render')
    assert.deepEqual([...registeredTools[2].parameters.required].sort(), ['path', 'script'])
    assert.equal(registeredTools[2].output.schema.properties.created.const, true)
    assert.equal(registeredTools[2].output.schema.properties.sourceTool.const, 'openpencil_new')
    assert.equal(registeredTools[2].output.schema.properties.previewIntent.const, 'document')
    assert.equal(typeof registeredTools[2].output.presentationMeta, 'function')
    assert.deepEqual([...registeredTools[5].parameters.required].sort(), ['brief'])
    assert.deepEqual([...registeredTools[6].parameters.required].sort(), ['draftId', 'tool'])
    assert.deepEqual([...registeredTools[7].parameters.required].sort(), ['draftId', 'script'])
    assert.deepEqual([...registeredTools[8].parameters.required].sort(), ['draftId'])
    assert.equal(typeof registeredTools[8].output.presentationMeta, 'function')
    assert.deepEqual([...registeredTools[9].parameters.required].sort(), ['draftId', 'kind'])
    assert.equal(typeof registeredTools[9].output.presentationMeta, 'function')
    assert.deepEqual([...registeredTools[10].parameters.required].sort(), ['draftId'])
    assert.deepEqual([...registeredTools[11].parameters.required].sort(), ['draftId'])
    assert.equal(typeof registeredTools[10].output.presentationMeta, 'function')
    assert.deepEqual(emittedEvents, [], 'registration alone must not claim a filesystem observation')
    const agentRequestEvents = eventListeners.filter(event => event.name === 'agent/request')
    assert.equal(agentRequestEvents.length, 1, 'plugin must install one scoped request waterfall')
    const requestListener = agentRequestEvents[0].listener
    const request = async (agentId, config, sessionId = agentId, turn = 1) => {
      requestWaterfallNextResolved = false
      return requestListener({ agent: { id: agentId, session: { id: sessionId } }, turn, step: 1, signal: AbortSignal.abort() }, async () => {
        requestWaterfallNextResolved = true
        return config
      })
    }
    const baseDeepSeekConfig = Object.freeze({
      provider: 'deepseek-official',
      model: 'deepseek-v4-flash',
      reasoningEffort: 'high',
      temperature: 0.2,
      maxTokens: 8192,
      stop: ['END'],
      adapterPrivate: { keep: true },
    })
    assert.equal(
      await request('design-owner', baseDeepSeekConfig),
      baseDeepSeekConfig,
      'before pipeline_begin the effective request config must remain untouched',
    )
    activeDraftOwners.add('design-owner')
    assert.equal(
      await request('other-agent', baseDeepSeekConfig),
      baseDeepSeekConfig,
      'another agent must not inherit the design owner reasoning override',
    )
    const nonDeepSeekConfig = Object.freeze({
      ...baseDeepSeekConfig,
      provider: 'openai',
      model: 'gpt-test',
    })
    assert.equal(
      await request('design-owner', nonDeepSeekConfig),
      nonDeepSeekConfig,
      'non-DeepSeek providers must keep their configured reasoning effort',
    )
    assert.deepEqual(
      await request('design-owner', baseDeepSeekConfig),
      { ...baseDeepSeekConfig, reasoningEffort: 'off' },
      'an active OpenPencil draft disables DeepSeek reasoning without dropping adapter fields',
    )
    assert.deepEqual(
      await request('transport-agent-id', baseDeepSeekConfig, 'design-owner'),
      { ...baseDeepSeekConfig, reasoningEffort: 'off' },
      'draft ownership follows the stable agent session id instead of the transport agent id',
    )
    activeDraftOwners.delete('design-owner')
    assert.deepEqual(
      await request('design-owner', baseDeepSeekConfig),
      { ...baseDeepSeekConfig, reasoningEffort: 'off' },
      'the post-finish response in the same turn must stay on the fast design path',
    )
    assert.equal(
      await request('design-owner', baseDeepSeekConfig, 'design-owner', 2),
      baseDeepSeekConfig,
      'a later user turn without an active draft restores the configured reasoning effort',
    )

    // DSH >= 0.2.2 locks reasoningEffort during step preparation from agent.model.current().
    let modelSelection = { provider: 'deepseek-official', model: 'deepseek-v4-flash', reasoningEffort: 'high' }
    const modelControl = {
      current() { return modelSelection },
      select(next) { modelSelection = { ...next } },
    }
    const modelAgent = { id: 'model-owner', session: { id: 'model-owner' }, model: modelControl }
    requestWaterfallNextResolved = false
    await requestListener({ agent: modelAgent, turn: 3, step: 1, signal: AbortSignal.abort() }, async () => {
      requestWaterfallNextResolved = true
      return { ...baseDeepSeekConfig, ...modelSelection }
    })
    activeDraftOwners.add('model-owner')
    for (const event of eventListeners.filter(event => event.name === 'tools/result')) {
      event.listener({ name: 'openpencil_pipeline_begin', callId: 'call-1', agent: modelAgent }, { isError: false, value: {} })
    }
    assert.equal(modelSelection.reasoningEffort, 'off', 'tools/result switches agent.model before the next step preparation')
    const stepPreparedConfig = Object.freeze({ ...baseDeepSeekConfig, ...modelSelection })
    requestWaterfallNextResolved = false
    const proposedForLockedStep = await requestListener({ agent: modelAgent, turn: 3, step: 2, signal: AbortSignal.abort() }, async () => {
      requestWaterfallNextResolved = true
      return stepPreparedConfig
    })
    assert.equal(proposedForLockedStep, stepPreparedConfig, 'agent/request must not mutate a step-prepared model selection')
    activeDraftOwners.delete('model-owner')
    for (const event of eventListeners.filter(event => event.name === 'session/event')) {
      event.listener({ id: 'model-owner' }, { type: 'turn/end' })
    }
    assert.equal(modelSelection.reasoningEffort, 'high', 'turn/end restores the pre-draft reasoning effort')
    assert.deepEqual(
      routeRegistrations.map(route => ({ kind: route.kind, path: route.path })),
      [
        { kind: 'prefix', path: '/_dsh/dsh-openpencil/render' },
        { kind: 'exact', path: '/_dsh/dsh-openpencil/presentation' },
        { kind: 'prefix', path: '/_dsh/dsh-openpencil/viewer-assets' },
        { kind: 'prefix', path: '/_dsh/dsh-openpencil/editor' },
      ],
    )
    assert.equal(typeof disposeInjectedAuthentication, 'function')
    assert.equal(registeredAuthentication(authenticatedRequest), true)
    assert.equal(registeredAuthentication({}), false)
    disposeInjectedAuthentication()
    assert.equal(typeof disposeInjectedRoutes, 'function')

    for (const event of eventListeners.filter(event => event.name === 'session/disposed')) {
      event.listener({ id: 'disposed-draft-owner' })
    }
    await new Promise(resolve => setImmediate(resolve))
    assert.deepEqual(abortedDraftOwners, ['disposed-draft-owner'])

    const routeDisposal = disposeInjectedRoutes()
    const pluginDisposal = disposePlugin()
    let routeDisposed = false
    let pluginDisposed = false
    void routeDisposal.then(() => { routeDisposed = true })
    void pluginDisposal.then(() => { pluginDisposed = true })
    await new Promise(resolve => setImmediate(resolve))

    assert.equal(editorHostDisposeCalls, 1, 'route and plugin cleanup must join one editor-host disposal')
    assert.equal(routeDisposed, false, 'route cleanup must await the editor-host teardown')
    assert.equal(pluginDisposed, false, 'plugin cleanup must await the editor-host teardown')
    assert.equal(
      disposedEventListeners.includes(agentRequestEvents[0]),
      true,
      'plugin cleanup must detach the agent/request waterfall before awaiting host teardown',
    )
    assert.deepEqual(routeRemovals.sort(), routeRegistrations.map(route => route.path).sort())

    releaseEditorHostDispose()
    await Promise.all([routeDisposal, pluginDisposal])
  } finally {
    releaseEditorHostDispose?.()
    EditorHostController.prototype.dispose = originalEditorHostDispose
    EditorHostController.prototype.attachRequestAuthentication = originalAttachAuthentication
    DesignDraftController.prototype.abortOwner = originalAbortOwner
    DesignDraftToolController.prototype.hasActiveDraft = originalHasActiveDraft
    if (previousDshHome === undefined) delete process.env.DSH_HOME
    else process.env.DSH_HOME = previousDshHome
    await rm(root, { recursive: true, force: true })
  }
})

test('runOpenPencilRender renders frames through the bundled op-host-web-server --mcp binary', async () => {
  const { tryResolveEditorRuntime } = await import('../lib/editor-runtime.js')
  const runtime = tryResolveEditorRuntime()
  if (runtime === undefined) return
  const root = await mkdtemp(join(tmpdir(), 'dsh-openpencil-mcp-render-'))
  const previousDshHome = process.env.DSH_HOME
  process.env.DSH_HOME = join(root, 'dsh-home')
  try {
    const docPath = join(root, 'design.op')
    await writeFile(docPath, JSON.stringify({
      version: '1.0.0',
      children: [
        { id: 'hero-frame', type: 'frame', name: 'Hero', x: 0, y: 0, width: 120, height: 80, fill: [{ type: 'solid', color: '#3366ff' }], children: [] },
        { id: 'card-frame', type: 'frame', name: 'Card', x: 160, y: 0, width: 60, height: 40, fill: [{ type: 'solid', color: '#22aa55' }], children: [] },
      ],
    }))
    const { runOpenPencilRender, verifyRenderOutput } = await import('../lib/renderer.js')
    const rendered = await runOpenPencilRender({
      binary: runtime.binary,
      input: docPath,
      scale: 1,
      signal: new AbortController().signal,
    })
    assert.equal(rendered.frames.length, 2)
    assert.deepEqual(rendered.frames.map(frame => ({ id: frame.id, name: frame.name, index: frame.index })), [
      { id: 'hero-frame', name: 'Hero', index: 0 },
      { id: 'card-frame', name: 'Card', index: 1 },
    ])
    const first = await verifyRenderOutput(rendered.frames[0].png)
    const second = await verifyRenderOutput(rendered.frames[1].png)
    assert.equal(first.width, 120)
    assert.equal(first.height, 80)
    assert.equal(second.width, 60)
    assert.equal(second.height, 40)
  } finally {
    if (previousDshHome === undefined) delete process.env.DSH_HOME
    else process.env.DSH_HOME = previousDshHome
    await rm(root, { recursive: true, force: true })
  }
})
