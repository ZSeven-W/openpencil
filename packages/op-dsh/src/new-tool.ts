/** Create a brand-new OpenPencil document from one transactional QuickJS build. */

import { lstat } from 'node:fs/promises'
import { basename, dirname, extname, isAbsolute } from 'node:path'
import type FileSystem from '@deepseek-ai/dsh-fs'
import type { FsObservation, FsTarget } from '@deepseek-ai/dsh-fs'
import type SandboxPolicyService from '@deepseek-ai/dsh-sandbox-policy'
import { defineTool, type ToolRunContext } from '@deepseek-ai/dsh-tools'
import type { JsonValue } from './json-value.js'
import type { EditorHostController } from './editor-host.js'
import { assertGeneratedDesignQuality } from './design-quality.js'
import {
  type DocumentSnapshot,
  RenderAccessController,
  createDocumentSnapshotFromText,
  projectDocumentGrant,
} from './renderer.js'
import { OPENPENCIL_NEW_TOOL_NAME } from './tool-names.js'

const MAX_SCRIPT_LENGTH = 256 * 1024
const SCRIPT_EXAMPLE = 'const root = I(null, { type: "frame", name: "Home", width: 390, height: 844, layout: "vertical", gap: 24, padding: 24, fill: [{ type: "solid", color: "#F7F1E7" }] });\nconst card = I(root, { type: "frame", name: "Welcome card", width: "fill_container", height: "fit_content", layout: "vertical", gap: 12, padding: 20, cornerRadius: 16, fill: [{ type: "solid", color: "#FFFFFF" }], stroke: { thickness: 1, fill: [{ type: "solid", color: "#E7DFD2" }] } });\nI(card, { type: "text", name: "Title", content: "Forage", width: "fill_container", height: 44, fontFamily: "Inter, system-ui, sans-serif", fontSize: 32, fontWeight: 700, lineHeight: 1.2, textAlign: "left", fill: [{ type: "solid", color: "#173C2B" }] });'

export interface DesignNewArgs {
  path: string
  script: string
  canvasWidth?: number
}

export interface DesignNewServices {
  fs: FileSystem
  sandboxPolicy: SandboxPolicyService
  render: RenderAccessController
  observe(target: FsTarget, observation: FsObservation, exec: ToolRunContext): void
}

export interface DesignNewResult {
  path: string
  filename: string
  bytes: number
  sha256: string
  created: true
  applied: true
  saved: true
  sourceTool: typeof OPENPENCIL_NEW_TOOL_NAME
  previewIntent: 'document'
  editable: true
  autoOpenEditor: true
  document: DocumentSnapshot
  result?: Record<string, JsonValue>
  note: string
}

function renderJson(_args: unknown, value: unknown): [{ type: 'text'; text: string }] {
  return [{ type: 'text', text: JSON.stringify(value, null, 2) }]
}

function isRecord(value: unknown): value is Record<string, JsonValue> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function makePresentationMeta(
  editorHost: EditorHostController,
  controller: RenderAccessController,
) {
  return (_args: unknown, value: JsonValue): JsonValue => {
    const result = value as unknown as DesignNewResult
    const editor = editorHost.grantFor(result.path, result.document?.sha256)
    return projectDocumentGrant(value, controller, editor)
  }
}

/**
 * Build and atomically publish a new `.op` document. The QuickJS build runs in
 * a private managed daemon, so no existing file or browser-owned sidebar is
 * required and a failed design never leaves an empty target behind.
 */
export function createDesignNewTool(editorHost: EditorHostController, services: DesignNewServices) {
  return defineTool({
    name: OPENPENCIL_NEW_TOOL_NAME,
    description: 'Compatibility path for an explicitly requested simple one-shot OpenPencil .op design from one transactional QuickJS script. '
      + 'Ordinary natural-language generation uses openpencil_pipeline_begin, two live-preview JS batches, and openpencil_pipeline_finish instead. '
      + 'Do not inspect or hand-write .op JSON and do not ask the user to open a sidebar. '
      + 'Choose a concise workspace-relative .op filename when the user did not specify one. '
      + 'This is a local OpenPencil-host operation and requires the session Workspace Write permission. '
      + 'The target must not already exist and its parent directory must exist. '
      + 'The script runs in sandboxed QuickJS, not Node.js or a browser: use only the global creation primitives I(parent, node) and K(kitId, parent, overrides). '
      + 'In DSH Code Mode, put the entire program in the script string passed to tools.openpencil_new; I/K do not exist in the outer run_code runtime and must never be executed there. '
      + 'C/U/D/M/R/G are not available in script mode. Plain JavaScript const/let declarations, data arrays, and for...of loops are supported. '
      + `Start with this executable pattern: ${SCRIPT_EXAMPLE}. `
      + 'I() returns the inserted node binding: capture it in a const and pass that value as the parent of later I() calls. Never set your own id and never write I("root", ...), I("card", ...), or another name/id string as a parent. '
      + 'Create the root first, then use multiple semantic I() calls for sections and content; do not collapse the whole screen into one giant nested object. '
      + 'Use canonical OpenPencil layout properties: layout, gap, padding, justifyContent, alignItems, cornerRadius, and textAlign; never invent paddingX, paddingY, radius, strokeWidth, align, or CSS-style aliases. '
      + 'Paints use fill: [{type:"solid",color:"#RRGGBB"}]. Borders use stroke: {thickness:1,fill:[{type:"solid",color:"#RRGGBB"}]}; shadows use effects: [{type:"shadow",offsetX:0,offsetY:8,blur:24,spread:0,color:"#0000001A"}]. '
      + 'Container enums are layout vertical/horizontal/none, justifyContent start/center/end/space_between/space_around, and alignItems start/center/end. Sizing is numeric, fill_container, or fit_content. '
      + 'Use native text_input/text_area/select/switch/checkbox controls instead of drawing lookalikes; text_input accepts placeholder, value, leadingIcon, trailingIcon, fill, stroke, and cornerRadius. '
      + 'Every text_input and select stacked in a form must declare width:"fill_container" and an explicit 44-52px control height (normally 48); a stacked text_area must also be full width with an intentional multi-line height such as 96-160px. Password text_input nodes must explicitly set secure:true. Never rely on renderer defaults. '
      + 'Never use emoji or a text character as an interface icon. Use an icon_font node with a real glyph name, or instantiate a matching kit component with K(). '
      + 'Provider brand icons use both fields: iconFontFamily:"simple-icons" with iconFontName:"wechat" or "apple"; never put the collection prefix inside iconFontName and never use Lucide\'s fruit apple for Apple sign-in. '
      + 'Before writing nodes, choose one explicit style fingerprint: named visual concept, font pairing, neutral/accent palette, radius scale, elevation treatment, and one signature visual moment. Avoid the generic initial-letter logo + white form card + saturated button template unless the user requests it. '
      + 'Give every generated visible text node an explicit fontFamily:"Inter, system-ui, sans-serif" and an intentional lineHeight. Desktop uses its bundled Inter, while the Web host can use the generic fallback without a missing-font prompt; never use bare Inter or pure system-ui as the portable default. For Chinese interfaces use lineHeight at least 1.3 and zero letter spacing. Use another named family only when the editor context or user confirms it is installed. '
      + 'A complete design needs at least one fixed-size renderable root frame. Build one clear primary task with strong negative space, a deliberate type hierarchy, consistent radii/padding/shadows, and at most two saturated colors. '
      + 'For mobile, use one App Content wrapper with 16-20px horizontal padding, 20-24px vertical gaps, and keep the first useful module within 20-32px of the title/header group. Prefer fewer, stronger modules and one distinctive visual idea over a crowded generic template. '
      + 'The entire script applies transactionally, then OpenPencil runs its built-in post-processing and finalization pipeline before the document is published atomically. That pipeline repairs deterministic structure and layout defects; it does not invent the visual concept, typography, component sizing, brand treatment, or iconography for a weak script. '
      + 'Before publication, DSH rejects high-confidence form sizing and emoji-icon defects. If that quality check fails, correct the reported structural paths and retry the same target path; no file was created. '
      + 'One call creates the file and requests editor auto-open after deterministic native and DSH checks: after success DSH opens the generated document in the managed OpenPencil sidebar when that surface is idle. It never replaces an editor already owned by another session; in that case the user-facing Edit canvas action remains available. Stop after success. Do not call openpencil_render, read_image, or openpencil_pipeline_inspect as a completion gate; this compatibility tool does not require model visual self-inspection.',
    parameters: {
      path: {
        type: 'string', required: true,
        description: 'New workspace-relative or absolute .op path. Choose a useful filename without asking when the user omitted one. Existing targets are never overwritten.',
      },
      script: {
        type: 'string', required: true,
        description: `A complete JavaScript program STRING executed by OpenPencil's sandboxed QuickJS; in Code Mode do not execute I/K in the outer run_code runtime. Build with I(parent, node) or K(kitId, parent, overrides); const/let, arrays, and for...of loops are allowed, while C/U/D/M/R/G, imports, Node.js, browser, network, and filesystem APIs are unavailable. Begin with ${SCRIPT_EXAMPLE}. Every child parent MUST be the binding returned from an earlier I(), never a quoted id/name; do not set node ids yourself. Use multiple semantic I() calls and canonical layout, gap, padding, justifyContent, alignItems, cornerRadius, textAlign, fill-array, stroke-object, and effects-array properties. Every generated text node explicitly sets fontFamily:"Inter, system-ui, sans-serif". Prefer native controls over frame lookalikes; stacked text_input/select controls must use width:"fill_container" plus an explicit 44-52px height, text_area uses a deliberate multi-line height such as 96-160px, and password text_input nodes set secure:true. Use icon_font or K() components instead of emoji icons, and define an intentional font/palette/radius/elevation style fingerprint in the script. Limit: 256 KiB source.`,
      },
      canvasWidth: { type: 'number', description: 'Optional canvas-width hint for OpenPencil post-processing.' },
    },
    output: {
      schema: {
        type: 'object', additionalProperties: false,
        properties: {
          path: { type: 'string', required: true },
          filename: { type: 'string', required: true },
          bytes: { type: 'integer', required: true },
          sha256: { type: 'string', required: true },
          created: { type: 'boolean', const: true, required: true },
          applied: { type: 'boolean', const: true, required: true },
          saved: { type: 'boolean', const: true, required: true },
          sourceTool: { type: 'string', const: OPENPENCIL_NEW_TOOL_NAME, required: true },
          previewIntent: { type: 'string', const: 'document', required: true },
          editable: { type: 'boolean', const: true, required: true },
          autoOpenEditor: { type: 'boolean', const: true, required: true },
          document: {
            type: 'object', required: true, additionalProperties: false,
            properties: {
              path: { type: 'string', required: true },
              filename: { type: 'string', required: true },
              mimeType: { type: 'string', const: 'application/json', required: true },
              bytes: { type: 'integer', required: true },
              sha256: { type: 'string', required: true },
            },
          },
          result: { type: 'object', additionalProperties: true },
          note: { type: 'string', required: true },
        },
      },
      render: renderJson,
      presentationMeta: makePresentationMeta(editorHost, services.render),
    },
    async execute(args: DesignNewArgs, exec) {
      const requestedPath = args.path.trim()
      if (requestedPath.length === 0) throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: path is required`)
      if (extname(requestedPath).toLowerCase() !== '.op') {
        throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: path must end in .op`)
      }
      if (args.script.trim().length === 0) throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: script must not be empty`)
      if (args.script.length > MAX_SCRIPT_LENGTH) throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: script is too large`)
      if (args.canvasWidth !== undefined && (!Number.isFinite(args.canvasWidth) || args.canvasWidth <= 0 || args.canvasWidth > 16_384)) {
        throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: canvasWidth must be greater than 0 and at most 16384`)
      }

      const sandboxPolicy = services.sandboxPolicy.resolve({ session: exec.agent?.session })
      if (services.fs.sandboxMode !== undefined && sandboxPolicy.mode === 'read-only') {
        throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: creating a design requires Workspace Write access; switch this session from Read Only to Workspace Write and retry`)
      }
      const resolveOptions = { cwd: sandboxPolicy.workspaceRoot, signal: exec.signal }
      const pathInfo = await services.fs.lstat(requestedPath, { cwd: sandboxPolicy.workspaceRoot }, exec.signal)
      if (pathInfo !== undefined) throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: target already exists: ${requestedPath}`)
      const target = await services.fs.resolve(requestedPath, resolveOptions)
      const processPath = services.fs.processPath(target)
      if (extname(processPath).toLowerCase() !== '.op') {
        throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: resolved target must end in .op`)
      }
      if (!isAbsolute(processPath)) throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: the DSH filesystem did not provide an absolute host path`)
      let parentInfo
      try {
        parentInfo = await lstat(dirname(processPath))
      } catch {
        throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: the target parent is not available to the local OpenPencil host; use an existing directory in a local DSH workspace`)
      }
      if (!parentInfo.isDirectory()) {
        throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: target parent must be a local directory`)
      }
      const resolvedInfo = await services.fs.stat(target, exec.signal)
      if (resolvedInfo !== undefined) {
        services.observe(target, { kind: 'present', version: resolvedInfo.version }, exec)
        throw new Error(`${OPENPENCIL_NEW_TOOL_NAME}: target already exists: ${requestedPath}`)
      }
      services.observe(target, { kind: 'absent' }, exec)

      const batch = await editorHost.createDocumentBatch({
        script: args.script,
        ...(args.canvasWidth === undefined ? {} : { canvasWidth: args.canvasWidth }),
        signal: exec.signal,
      })
      // The managed daemon has already applied post-processing and finalization,
      // but those passes intentionally do not make aesthetic decisions. Refuse
      // a deliberately small set of unambiguous regressions while the document
      // is still transient, so a model can correct its script without publishing
      // a narrow form control or an emoji masquerading as an interface icon.
      assertGeneratedDesignQuality(batch.documentJson)
      const outcome = await services.fs.writeText(
        target,
        batch.documentJson,
        { kind: 'createIfAbsent' },
        exec.signal,
        sandboxPolicy,
      )
      services.observe(target, { kind: 'present', version: outcome.version }, exec)
      const document = await createDocumentSnapshotFromText(outcome.after)
      const bytes = document.bytes
      const sha256 = document.sha256
      return {
        path: processPath,
        filename: basename(processPath),
        bytes,
        sha256,
        created: true as const,
        applied: true as const,
        saved: true as const,
        sourceTool: OPENPENCIL_NEW_TOOL_NAME,
        previewIntent: 'document' as const,
        editable: true as const,
        autoOpenEditor: true as const,
        document,
        ...(isRecord(batch.result) ? { result: batch.result } : {}),
        note: `Created and saved ${processPath}; DSH requests the managed OpenPencil editor to open automatically when the editor surface is idle.`,
      }
    },
    presentCall: (args: DesignNewArgs) => ({
      card: 'generic', title: `Create ${args.path}`, kind: 'execute', locations: [{ path: args.path }],
    }),
  })
}
