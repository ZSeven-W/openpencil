/**
 * @zseven-w/dsh-openpencil — preview `.op` design documents in DSH.
 *
 * Plugin lifecycle: register the model-facing tool plus signed routes for
 * exact PNGs, immutable document snapshots, and the optional read-only Web
 * SDK canvas. Everything is
 * registered through `ctx.effect` (or a returned disposer) so unloading the
 * plugin removes every contribution.
 *
 * The `openpencil_render` tool never returns an ImageBlock — the DeepSeek
 * adapter rejects image blocks anywhere in a request. The browser-only
 * envelope rides `output.presentationMeta` into `ToolCallBlock.meta`, and
 * the keyed `tool.call.toolview` client component renders a PNG-first card
 * and lazily mounts the OpenPencil canvas on demand.
 * @module @zseven-w/dsh-openpencil
 */

import type { Context } from '@deepseek-ai/cordis'
import type FileSystem from '@deepseek-ai/dsh-fs'
import type { FsObservation, FsTarget } from '@deepseek-ai/dsh-fs'
import type SandboxPolicyService from '@deepseek-ai/dsh-sandbox-policy'
import type ToolRegistry from '@deepseek-ai/dsh-tools'
import type { ToolExecution, ToolExecutionResult, ToolRunContext } from '@deepseek-ai/dsh-tools'
import type { Session, SessionStore } from '@deepseek-ai/dsh-session'
import type WebServer from '@deepseek-ai/dsh-host-webserver'
import type { IncomingMessage } from 'node:http'
import {
  RENDER_ROUTE_PREFIX,
  RenderAccessController,
  prepareRenderAccessKey,
} from './renderer.js'
import { createDesignRenderTool } from './tool.js'
import {
  createDesignCreateTool,
  createDesignEditTool,
  createDesignSelectionTool,
} from './design-tools.js'
import { createDesignNewTool } from './new-tool.js'
import {
  VIEWER_ASSET_ROUTE_PREFIX,
  prepareViewerAssets,
} from './viewer-assets.js'
import {
  EDITOR_ROUTE_PREFIX,
  EditorHostController,
} from './editor-host.js'
import {
  OPENPENCIL_CREATE_TOOL_NAME,
  OPENPENCIL_EDIT_TOOL_NAME,
  OPENPENCIL_NEW_TOOL_NAME,
  OPENPENCIL_RENDER_TOOL_NAME,
  OPENPENCIL_SELECTION_TOOL_NAME,
  OPENPENCIL_TOOL_NAMES,
} from './tool-names.js'
import { createDesignDraftToolController } from './design-draft-tools.js'
import {
  PRESENTATION_HYDRATION_ROUTE,
  PresentationHydrationController,
} from './presentation-hydration.js'
import {
  registerOpenPencilDesignGuidance,
  registerOpenPencilDesignSkill,
} from './design-skill.js'

export {
  LEGACY_DESIGN_RENDER_TOOL_NAME,
  OPENPENCIL_CREATE_TOOL_NAME,
  OPENPENCIL_EDIT_TOOL_NAME,
  OPENPENCIL_NEW_TOOL_NAME,
  OPENPENCIL_PIPELINE_ABORT_TOOL_NAME,
  OPENPENCIL_PIPELINE_BATCH_TOOL_NAME,
  OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME,
  OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME,
  OPENPENCIL_PIPELINE_FINISH_TOOL_NAME,
  OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME,
  OPENPENCIL_PIPELINE_TOOL_NAMES,
  OPENPENCIL_RENDER_TOOL_NAME,
  OPENPENCIL_SELECTION_TOOL_NAME,
  OPENPENCIL_TOOL_NAMES,
} from './tool-names.js'
export {
  OPENPENCIL_DESIGN_GUIDANCE_SECTION,
  OPENPENCIL_DESIGN_SKILL_CONTENT,
  OPENPENCIL_DESIGN_SKILL_DESCRIPTION,
  OPENPENCIL_DESIGN_SKILL_NAME,
  OPENPENCIL_DESIGN_SKILL_WHEN_TO_USE,
  registerOpenPencilDesignGuidance,
  registerOpenPencilDesignSkill,
} from './design-skill.js'
export {
  DesignDraftToolController,
  createDesignDraftToolController,
  type DesignDraftToolServices,
} from './design-draft-tools.js'

/** Stable plugin name (the loader entry id in cordis.patch.yml). */
export const name = '@zseven-w/dsh-openpencil'

/** Services this plugin's root fiber requires. */
export const inject = ['tools', 'sessions', 'fs', 'sandboxPolicy']

/**
 * rc.2 source worktrees augmented the legacy `cordis` package name while the
 * published rc line augments `@deepseek-ai/cordis`. Keep this plugin's build
 * structural so the same source type-checks against both without changing its
 * runtime service contract.
 */
type HostContext = Context & {
  tools: ToolRegistry
  sessions: SessionStore
  fs: FileSystem
  sandboxPolicy: SandboxPolicyService
}

/** Public Host authentication subset; optional on older or headless profiles. */
type ConnectionContext = Context & {
  connection: { requestRejection(req: IncomingMessage): number | undefined }
}

/** Structural subset of DSH's agent and provider call contract. */
type AgentModelSelection = {
  provider: string
  model: string
  reasoningEffort?: string
  [key: string]: unknown
}

type AgentModelControl = {
  current(): AgentModelSelection | undefined
  select(selection: AgentModelSelection): void
}

type AgentRequestPayload = {
  agent: {
    readonly id: string | number
    readonly session: {
      readonly id: string | number
    }
    readonly model?: AgentModelControl
  }
  readonly turn: number
}

type AgentRequestConfig = {
  provider: string
  model: string
  reasoningEffort?: string
  [key: string]: unknown
}

type HostEventContext = Context & {
  on(name: 'agent/request', listener: (payload: AgentRequestPayload, next: () => Promise<AgentRequestConfig>) => Promise<AgentRequestConfig>): () => void
  on(name: 'tools/result', listener: (exec: Readonly<ToolExecution>, result: Readonly<ToolExecutionResult>) => void): () => void
  on(name: 'session/event', listener: (session: Session, event: { readonly type: string }) => void): () => void
  on(name: 'session/disposed', listener: (session: Session) => void): () => void
  emit(name: 'fs/observed', target: FsTarget, observation: FsObservation, actor: ToolRunContext): void
}

function isDeepSeekProvider(provider: string): boolean {
  return provider.toLowerCase().startsWith('deepseek')
}

/** Read the optional bind-time trust snapshot without making Web-only runtime glue a hard peer. */
function webRuntimeTrustedHosts(ctx: Context): readonly string[] {
  const get = (ctx as unknown as { get?: (name: string) => unknown }).get
  const runtime = typeof get === 'function' ? get.call(ctx, 'webRuntime') : undefined
  if (typeof runtime !== 'object' || runtime === null || !('trustedHosts' in runtime)) return []
  const trustedHosts = (runtime as { trustedHosts?: unknown }).trustedHosts
  return Array.isArray(trustedHosts) && trustedHosts.every(value => typeof value === 'string')
    ? trustedHosts
    : []
}

/** Plugin entry: mount every model-facing contribution. */
export async function apply(ctx: Context): Promise<() => Promise<void>> {
  const hostCtx = ctx as HostContext
  const eventCtx = ctx as HostEventContext
  const disposers: Array<() => void | Promise<void>> = []
  const designReasoningTurns = new Map<string, number>()
  const lastSeenTurns = new Map<string, number>()
  const agentModelsByOwner = new Map<string, AgentModelControl>()
  const savedModelSelections = new Map<string, { model: AgentModelControl; selection: AgentModelSelection }>()

  const applyDesignModelSelection = (owner: string, model: AgentModelControl | undefined): void => {
    if (model === undefined || typeof model.current !== 'function' || typeof model.select !== 'function') return
    agentModelsByOwner.set(owner, model)
    if (savedModelSelections.has(owner)) return
    const current = model.current()
    if (current === undefined || !isDeepSeekProvider(current.provider)) return
    savedModelSelections.set(owner, { model, selection: { ...current } })
    if (current.reasoningEffort !== 'off') {
      model.select({ ...current, reasoningEffort: 'off' })
    }
  }

  const restoreDesignModelSelection = (owner: string): void => {
    const saved = savedModelSelections.get(owner)
    if (saved === undefined) return
    savedModelSelections.delete(owner)
    try {
      const current = saved.model.current()
      if (
        current !== undefined
        && current.provider === saved.selection.provider
        && current.model === saved.selection.model
        && current.reasoningEffort === 'off'
      ) {
        saved.model.select({ ...saved.selection })
      }
    } catch {
      // Agent may already be disposed during teardown.
    }
  }
  const accessKey = await prepareRenderAccessKey()
  const controller = new RenderAccessController(accessKey)
  const viewerAssets = await prepareViewerAssets()
  const editorHost = new EditorHostController(accessKey)
  const designDraftTools = createDesignDraftToolController(editorHost, {
    fs: hostCtx.fs,
    sandboxPolicy: hostCtx.sandboxPolicy,
    render: controller,
    observe: (target, observation, exec) => eventCtx.emit('fs/observed', target, observation, exec),
  })
  const presentationHydration = new PresentationHydrationController({
    sessions: hostCtx.sessions,
    render: controller,
    viewer: viewerAssets,
    editor: editorHost,
    // webRuntime is provided after bind by the official Web bundle. Resolve
    // it at request time so loopback still works on older/headless hosts while
    // configured LAN authorities receive the same Host fence as DSH /api.
    trustedHosts: () => webRuntimeTrustedHosts(ctx),
  })
  let editorHostDisposePromise: Promise<void> | undefined
  let designDraftToolsDisposePromise: Promise<void> | undefined
  const disposeEditorHost = (): Promise<void> => {
    editorHostDisposePromise ??= editorHost.dispose()
    return editorHostDisposePromise
  }
  const disposeDesignDraftTools = (): Promise<void> => {
    designDraftToolsDisposePromise ??= designDraftTools.dispose()
    return designDraftToolsDisposePromise
  }

  // Optional model guidance: detailed design knowledge lives in an on-demand
  // bundled skill, with only a short load-before-generation prompt reminder.
  disposers.push(registerOpenPencilDesignSkill(ctx))
  disposers.push(registerOpenPencilDesignGuidance(ctx))

  // Tool registration: global (every agent sees it). The tool's
  // presentationMeta consults `controller.routeAvailable`, so a profile
  // without the webserver still gets a plain-JSON result — no dangling URL.
  disposers.push(ctx.effect(
    () => hostCtx.tools.register(createDesignRenderTool(controller, viewerAssets, editorHost)),
    `dsh-openpencil: ${OPENPENCIL_RENDER_TOOL_NAME} tool`,
  ))
  disposers.push(ctx.effect(
    () => hostCtx.tools.register(createDesignSelectionTool(editorHost)),
    `dsh-openpencil: ${OPENPENCIL_SELECTION_TOOL_NAME} tool`,
  ))
  disposers.push(ctx.effect(
    () => hostCtx.tools.register(createDesignNewTool(editorHost, {
      fs: hostCtx.fs,
      sandboxPolicy: hostCtx.sandboxPolicy,
      render: controller,
      observe: (target, observation, exec) => eventCtx.emit('fs/observed', target, observation, exec),
    })),
    `dsh-openpencil: ${OPENPENCIL_NEW_TOOL_NAME} tool`,
  ))
  disposers.push(ctx.effect(
    () => hostCtx.tools.register(createDesignCreateTool(editorHost)),
    `dsh-openpencil: ${OPENPENCIL_CREATE_TOOL_NAME} tool`,
  ))
  disposers.push(ctx.effect(
    () => hostCtx.tools.register(createDesignEditTool(editorHost)),
    `dsh-openpencil: ${OPENPENCIL_EDIT_TOOL_NAME} tool`,
  ))
  for (const tool of designDraftTools.createTools()) {
    disposers.push(ctx.effect(
      () => hostCtx.tools.register(tool),
      `dsh-openpencil: ${tool.name} tool`,
    ))
  }
  disposers.push(ctx.effect(
    () => eventCtx.on('agent/request', async (payload, next) => {
      // This is a waterfall: resolve the effective provider config first, then
      // replace only its reasoning knob on legacy hosts. On DSH >= 0.2.2 step
      // preparation locks reasoningEffort from `agent.model.current()`, so the
      // selection is switched before step preparation via `agent.model.select()`
      // and restored when the design turn finishes.
      const config = await next()
      const owner = String(payload.agent.session.id)
      const model = payload.agent.model
      lastSeenTurns.set(owner, payload.turn)
      if (model !== undefined) agentModelsByOwner.set(owner, model)
      if (designDraftTools.hasActiveDraft(owner)) {
        designReasoningTurns.set(owner, payload.turn)
        applyDesignModelSelection(owner, model)
      } else if (designReasoningTurns.get(owner) !== payload.turn) {
        designReasoningTurns.delete(owner)
        restoreDesignModelSelection(owner)
      }
      if (designReasoningTurns.get(owner) !== payload.turn || !isDeepSeekProvider(config.provider)) {
        return config
      }
      if (typeof model?.select === 'function') {
        return config
      }
      return { ...config, reasoningEffort: 'off' }
    }),
    'dsh-openpencil: active design draft reasoning override',
  ))
  disposers.push(ctx.effect(
    () => eventCtx.on('tools/result', (exec, result) => {
      presentationHydration.observeToolResult(exec, result)
      if (exec.agent === undefined) return
      const owner = String(exec.agent.session.id)
      const model = (exec.agent as { readonly model?: AgentModelControl }).model ?? agentModelsByOwner.get(owner)
      if (model !== undefined && designDraftTools.hasActiveDraft(owner)) {
        const turn = lastSeenTurns.get(owner)
        if (turn !== undefined) designReasoningTurns.set(owner, turn)
        applyDesignModelSelection(owner, model)
      }
    }),
    'dsh-openpencil: nested presentation result observer',
  ))
  disposers.push(ctx.effect(
    () => eventCtx.on('session/event', (session, event) => {
      if (event?.type !== 'turn/end') return
      const owner = String(session.id)
      designReasoningTurns.delete(owner)
      restoreDesignModelSelection(owner)
    }),
    'dsh-openpencil: design turn reasoning restore',
  ))
  disposers.push(ctx.effect(
    () => eventCtx.on('session/disposed', session => presentationHydration.forgetSession(String(session.id))),
    'dsh-openpencil: nested presentation session cleanup',
  ))
  disposers.push(ctx.effect(
    () => eventCtx.on('session/disposed', session => {
      const owner = String(session.id)
      designReasoningTurns.delete(owner)
      lastSeenTurns.delete(owner)
      restoreDesignModelSelection(owner)
      agentModelsByOwner.delete(owner)
      void designDraftTools.abortOwner(owner).catch(error => {
        ctx.logger.warn(`dsh-openpencil draft cleanup failed: ${error instanceof Error ? error.message : String(error)}`)
      })
    }),
    'dsh-openpencil: design draft session cleanup',
  ))

  ctx.inject(['connection'], connectionCtx => connectionCtx.effect(() => {
    const connection = (connectionCtx as ConnectionContext).connection
    return editorHost.attachRequestAuthentication(req => connection.requestRejection(req) === undefined)
  }, 'dsh-openpencil: authenticated Desktop editor requests'))

  // Optional Web routes: only mounted when a webServer service exists
  // (headless profiles never attach, and `routeAvailable` stays false).
  // The inject fiber is parent-scoped and tears itself down with this ctx;
  // the inner effect's disposer is the route removal.
  ctx.inject(['webServer'], (webCtx) => webCtx.effect(() => {
    const webServer = (webCtx as Context & { webServer: WebServer }).webServer
    const detach = controller.attachRoute()
    const disposeRoute = webServer.register({
      kind: 'prefix',
      path: RENDER_ROUTE_PREFIX,
      handler: (req, res) => controller.handle(req, res),
    })
    const disposePresentationRoute = webServer.register({
      kind: 'exact',
      path: PRESENTATION_HYDRATION_ROUTE,
      handler: (req, res) => presentationHydration.handle(req, res),
    })
    const disposeViewerRoute = viewerAssets.available
      ? (() => {
          const detachViewer = viewerAssets.attachRoute()
          const disposeViewer = webServer.register({
            kind: 'prefix',
            path: VIEWER_ASSET_ROUTE_PREFIX,
            handler: (req, res) => viewerAssets.handle(req, res),
          })
          return () => {
            disposeViewer()
            detachViewer()
          }
        })()
      : undefined
    const detachEditor = editorHost.attachRoute()
    const disposeEditorRoute = webServer.register({
      kind: 'prefix',
      path: EDITOR_ROUTE_PREFIX,
      handler: (req, res) => editorHost.handle(req, res),
    })
    return async () => {
      disposeEditorRoute()
      detachEditor()
      disposeViewerRoute?.()
      disposePresentationRoute()
      disposeRoute()
      detach()
      await disposeEditorHost()
    }
  }, 'dsh-openpencil: render route'))

  ctx.logger.info(`dsh-openpencil mounted (${OPENPENCIL_TOOL_NAMES.join(' + ')}; viewer assets: ${viewerAssets.available ? 'ready' : 'unavailable'}; editor: ${editorHost.available ? 'ready' : 'unavailable'})`)
  return async () => {
    for (const dispose of disposers.reverse()) await dispose()
    await Promise.all([disposeDesignDraftTools(), disposeEditorHost()])
  }
}
