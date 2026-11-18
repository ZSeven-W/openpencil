/**
 * `openpencil_render` — render a `.op` design document to PNG without a
 * window. OpenPencil's own exporter is the exact primary path; Jian is an
 * explicitly-labelled runtime-preview fallback when that binary is absent.
 *
 * The tool returns plain JSON (never an `ImageBlock` — the DeepSeek
 * adapter rejects image blocks anywhere in a request). A browser-only
 * envelope with a signed preview URL rides `output.presentationMeta`
 * into `ToolCallBlock.meta`, where the keyed `tool.call.toolview`
 * component recognizes it and renders `<img>`.
 * @module dsh-openpencil/tool
 */

import { defineTool, type ToolRunContext } from '@deepseek-ai/dsh-tools'
import type { JsonValue } from './json-value.js'
import { basename } from 'node:path'
import {
  RenderAccessController,
  RendererBinaryMissingError,
  createDocumentSnapshot,
  createRenderOutput,
  findOpenPencilBinary,
  findJianBinary,
  projectRenderGrant,
  resolveInputFile,
  runOpenPencilRender,
  runJianRender,
  verifyRenderOutput,
  type RenderFrame,
  type RenderResult,
} from './renderer.js'
import type { ViewerAssetController } from './viewer-assets.js'
import type { EditorHostController } from './editor-host.js'
import { OPENPENCIL_RENDER_TOOL_NAME } from './tool-names.js'

/** Session workspace the caller resolves paths against (mirrors first-party tools). */
function sessionWorkspace(exec: ToolRunContext): string {
  return exec.agent?.session.header.cwd ?? process.cwd()
}

const renderJson = (_args: unknown, value: unknown): [{ type: 'text'; text: string }] => [{
  type: 'text',
  text: JSON.stringify(value, null, 2),
}]

/** Pure, replayable presentation projection — the browser-only envelope. */
function makePresentationMeta(
  controller: RenderAccessController,
  viewerAssets?: ViewerAssetController,
  editorHost?: EditorHostController,
) {
  return (_args: unknown, value: JsonValue): JsonValue => {
    const result = value as unknown as RenderResult
    const editor = result.editable === true
      ? editorHost?.grantFor(result.sourcePath, result.document?.sha256)
      : undefined
    return projectRenderGrant(value, controller, viewerAssets?.viewerGrant, editor)
  }
}

export interface DesignRenderArgs {
  path: string
  width?: number
  height?: number
  scale?: number
  /** Explicitly expose the original source to the managed sidebar editor. */
  editable?: boolean
  /** Expand the editor once for this live render result. */
  autoOpen?: boolean
}

/** Create the `openpencil_render` tool definition bound to one controller. */
export function createDesignRenderTool(
  controller: RenderAccessController,
  viewerAssets?: ViewerAssetController,
  editorHost?: EditorHostController,
) {
  return defineTool({
    name: OPENPENCIL_RENDER_TOOL_NAME,
    description: 'Render an existing OpenPencil .op design document exactly as the design canvas, '
      + 'then show a PNG and an optional interactive read-only canvas in the conversation. '
      + 'Give the absolute path to a .op file (or a path relative to the session workspace). '
      + 'For an ordinary new design when no .op file exists, call openpencil_pipeline_begin and follow its two live-preview JS batches plus deterministic finish contract. Use openpencil_new only when the user explicitly requests its simple one-shot compatibility path. Render is for an existing .op document or an explicit user request for a PNG, not a model completion gate. '
      + 'The image appears directly in the chat; the file path is returned for further use. '
      + 'Set editable=true when the user asks to edit an existing design, and set autoOpen=true only when this render result should open its editor immediately. '
      + 'When rendering a design generated in the current turn, editable=true is mandatory so its production preview always retains the Edit canvas/sidebar action; use autoOpen=true unless the user asked to keep the editor closed. '
      + 'Leave width/height unset for design-accurate output. Width/height are only supported '
      + 'by the lower-fidelity Jian runtime fallback.',
    parameters: {
      path: { type: 'string', required: true, description: 'Path to the .op design document to render.' },
      width: { type: 'number', description: 'Explicit logical viewport width in pixels. Omit to use the document size.' },
      height: { type: 'number', description: 'Explicit logical viewport height in pixels. Omit to use the document size.' },
      scale: { type: 'number', description: 'Pixel scale factor applied to the output (device-pixel ratio). Default 1.' },
      editable: { type: 'boolean', description: 'Expose an Edit in sidebar action for the original .op source. Default false.' },
      autoOpen: { type: 'boolean', description: 'Automatically expand the editor once for this live editable render result. Default false.' },
    },
    output: {
      schema: {
        type: 'object',
        additionalProperties: false,
        properties: {
          path: { type: 'string', required: true },
          filename: { type: 'string', required: true },
          mimeType: { type: 'string', const: 'image/png', required: true },
          kind: { type: 'string', const: 'image', required: true },
          description: { type: 'string', required: true },
          sourceTool: { type: 'string', const: OPENPENCIL_RENDER_TOOL_NAME, required: true },
          previewIntent: { type: 'string', const: 'image', required: true },
          bytes: { type: 'integer', required: true },
          width: { type: 'integer' },
          height: { type: 'integer' },
          sha256: { type: 'string' },
          sourcePath: { type: 'string' },
          renderer: { type: 'string' },
          rendererBinary: { type: 'string' },
          fidelity: { type: 'string' },
          warnings: { type: 'array', items: { type: 'string' } },
          frames: {
            type: 'array',
            items: {
              type: 'object',
              additionalProperties: false,
              properties: {
                path: { type: 'string', required: true },
                filename: { type: 'string', required: true },
                mimeType: { type: 'string', const: 'image/png', required: true },
                bytes: { type: 'integer', required: true },
                width: { type: 'integer', required: true },
                height: { type: 'integer', required: true },
                sha256: { type: 'string', required: true },
                id: { type: 'string' },
                name: { type: 'string' },
                index: { type: 'integer' },
              },
            },
          },
          frameCount: { type: 'integer' },
          editable: { type: 'boolean' },
          autoOpenEditor: { type: 'boolean' },
          document: {
            type: 'object',
            additionalProperties: false,
            properties: {
              path: { type: 'string', required: true },
              filename: { type: 'string', required: true },
              mimeType: { type: 'string', const: 'application/json', required: true },
              bytes: { type: 'integer', required: true },
              sha256: { type: 'string', required: true },
            },
          },
          note: { type: 'string' },
        },
      },
      render: renderJson,
      presentationMeta: makePresentationMeta(controller, viewerAssets, editorHost),
    },
    async execute(args: DesignRenderArgs, exec): Promise<RenderResult> {
      const input = await resolveInputFile(args.path, sessionWorkspace(exec))
      const document = await createDocumentSnapshot(input)
      const warnings: string[] = []
      let out: string
      let renderer: 'openpencil' | 'jian'
      let rendererBinary: string
      let fidelity: 'exact' | 'runtime-preview'
      let exactFrames: Awaited<ReturnType<typeof runOpenPencilRender>>['frames'] | undefined
      const openPencil = findOpenPencilBinary()
      if (openPencil !== undefined) {
        if (args.width !== undefined || args.height !== undefined) {
          throw new Error(`${OPENPENCIL_RENDER_TOOL_NAME}: width/height are not supported by the exact OpenPencil renderer; omit them and use scale`)
        }
        try {
          const exact = await runOpenPencilRender({
            binary: openPencil,
            input: document.path,
            scale: args.scale,
            signal: exec.signal,
          })
          out = exact.png
          exactFrames = exact.frames
          warnings.push(...exact.warnings)
          renderer = 'openpencil'
          rendererBinary = openPencil
          fidelity = 'exact'
        } catch (error) {
          if (!(error instanceof RendererBinaryMissingError)) throw error
          const fallback = await createRenderOutput()
          const jian = findJianBinary()
          await runJianRender({
            binary: jian, input: document.path, out: fallback,
            width: args.width, height: args.height, scale: args.scale, signal: exec.signal,
          })
          out = fallback
          renderer = 'jian'
          rendererBinary = jian
          fidelity = 'runtime-preview'
          warnings.push('OpenPencil exact renderer was unavailable; using lower-fidelity Jian runtime preview.')
        }
      } else {
        const fallback = await createRenderOutput()
        const jian = findJianBinary()
        await runJianRender({
          binary: jian, input: document.path, out: fallback,
          width: args.width, height: args.height, scale: args.scale, signal: exec.signal,
        })
        out = fallback
        renderer = 'jian'
        rendererBinary = jian
        fidelity = 'runtime-preview'
        warnings.push('OpenPencil exact renderer was not found; using lower-fidelity Jian runtime preview.')
      }
      const verified = await verifyRenderOutput(out)
      const frames: RenderFrame[] | undefined = exactFrames === undefined
        ? undefined
        : await Promise.all(exactFrames.map(async frame => {
            const frameVerified = frame.png === out ? verified : await verifyRenderOutput(frame.png)
            return {
              path: frame.png,
              filename: basename(frame.png),
              mimeType: 'image/png' as const,
              bytes: frameVerified.bytes,
              width: frameVerified.width,
              height: frameVerified.height,
              sha256: frameVerified.sha256,
              ...(frame.id === undefined ? {} : { id: frame.id }),
              ...(frame.name === undefined ? {} : { name: frame.name }),
              index: frame.index,
            }
          }))
      const result: RenderResult = {
        path: out,
        filename: basename(out),
        mimeType: 'image/png',
        kind: 'image',
        description: `Rendered ${input} with ${renderer} (${fidelity})`,
        sourceTool: OPENPENCIL_RENDER_TOOL_NAME,
        previewIntent: 'image',
        bytes: verified.bytes,
        width: verified.width,
        height: verified.height,
        sha256: verified.sha256,
        sourcePath: input,
        renderer,
        rendererBinary,
        fidelity,
        warnings,
        ...(frames === undefined ? {} : { frames, frameCount: frames.length }),
        editable: args.editable === true,
        ...(args.editable === true && args.autoOpen === true ? { autoOpenEditor: true } : {}),
        document,
      }
      return result
    },
    presentCall: (args: DesignRenderArgs) => ({
      card: 'generic',
      title: `Render ${args.path}`,
      kind: 'execute',
      locations: [{ path: args.path }],
    }),
  })
}
