/** Bundled DSH playbook for creating deliberate OpenPencil designs. */

import { readFileSync } from 'node:fs'
import type { Context } from '@deepseek-ai/cordis'

export const OPENPENCIL_DESIGN_SKILL_NAME = 'openpencil-design'

export const OPENPENCIL_DESIGN_SKILL_DESCRIPTION =
  'Optional troubleshooting/reference for the DSH OpenPencil pipeline. Ordinary creation starts from '
  + 'openpencil_pipeline_begin and must not load this skill.'

export const OPENPENCIL_DESIGN_SKILL_WHEN_TO_USE =
  'Only when the user explicitly asks to load openpencil-design or requests pipeline troubleshooting; '
  + 'never for an ordinary design request.'

export const OPENPENCIL_DESIGN_SKILL_CONTENT = readFileSync(
  new URL('./assets/openpencil-design/SKILL.md', import.meta.url),
  'utf8',
)

export const OPENPENCIL_DESIGN_GUIDANCE_SECTION = {
  name: 'openpencil:design-skill-guidance',
  order: 130,
  text: () => 'An ordinary new OpenPencil design is one strict transaction: `openpencil_pipeline_begin` -> batch 1 -> batch 2 -> `openpencil_pipeline_finish`. Only when the user explicitly asks for the high-fidelity App-identical engine, begin may take engine:"app-agent": then the transaction is begin -> one `openpencil_pipeline_agent_run` -> finish, with no generation batches. Call begin exactly once and immediately; never load `openpencil-design` first. Omit begin.path unless the user explicitly named a file: the plugin chooses the concrete collision-resistant `.op` filename. Preserve an explicitly named path exactly. Pass the user\'s direct request as begin.brief without adding a platform, viewport, or device they did not name. Begin fixes the path, root, platform, and canvas for the whole transaction: default to web/desktop unless the user explicitly requested mobile, then never reinterpret, switch, or rebuild them. After every successful begin/batch call, make the next required call without narration, planning, comparison, inspection, or any unrelated tool call. A thrown error or any result with `canContinue:false` is terminal: report it once and do not retry, inspect, context-read, abort, or start a replacement draft. In run_code use exactly:\n'
    + 'const draftId = "<exact begin.draftId>";\n'
    + 'const script = String.raw`...`;\n'
    + 'const r = await tools.openpencil_pipeline_batch({ draftId, script });\n'
    + 'return r;\n'
    + 'Quote the exact begin.draftId into the standalone `draftId` string first. The fixed call object contains only `draftId` and `script`; return only `r`. Batch 1 and batch 2 are fresh-scope direct I/K scripts against begin.rootNodeId. I/K return opaque node-id strings: use a returned binding only as a later I/K parent, put every property in the node object passed to I/K, and never assign `binding.x`, `binding.y`, or any other member. '
    + 'A category rail gives every distinct label a distinct matching icon tile named `<label> icon tile`; never name category tiles art/media/image or give them image queries. Match 数码/electronics to `smartphone` or `camera`, and 食品/food to `utensils`, `sandwich`, or `croissant`; never use `lamp` or `coffee` for those categories. Every `iconFontName` must come from the verified glyph list in begin\'s `buildContract.node.icon`; an off-catalog name renders as an invisible blank. Complete each repeated product card\'s media (or intentionally omit it), name, and price before starting the next card. Batch 2 always ends by attaching the required Footer as the last root region: a role footer `#1C1917` full-width band (padding `[48,160]` desktop, `[40,24]` mobile) holding the brand text in `#FFFFFF`, one role nav-links row of minWidth-44 height-44 role nav-link frames with `#D6D3D1` labels, and a 13px `#A8A29E` copyright line — a page without its Footer is unfinished. Bind every semantic container (Header, Nav, Actions, section, card, Footer) to a const and insert its children through that binding, never as root siblings; an empty role container fails the batch. '
    + 'Every generated text node explicitly uses `fontFamily:"Inter, system-ui, sans-serif"`: desktop keeps its bundled Inter while the web editor has a portable generic fallback and never depends on a locally installed Inter face. Visible copy follows the user request language consistently; a Chinese request means Chinese copy except an optional short ASCII brand. The begin result already selects the concrete style guide; use its palette, typography, spacing, and recipe without another context/tool call. In generation scripts use literal hex colors directly in node objects, not palette alias variables. Default to one image outside commerce. For desktop commerce, batch 1 uses the bundled ecommerce-modern-light direction: clean white base, warm-tinted section rhythm, 1120px centered content, 56px Hero display, and a balanced horizontal two-column split row inside an optional warm section wrapper. Header role navbar, height 64, `padding:[0,160]`; Nav role nav-links contains 44px role nav-link frames, each containing its text child; never put nav-link on text. Header actions role toolbar contains 44x44 role icon-button frames. CTA role button is 160x48 with #C2410C/#FFFFFF and its label inside the CTA binding. Hero is full-width with `padding:[64,160]`, copy width 512 + gap 64 + image width 448; headline and subtitle each use `width:"fill_container"`. Generic commerce creates the product image directly under Hero: `I(hero,{type:"image",name:"Hero product image",width:448,height:360,imageSearchQuery:"gray loveseat isolated photo"})`; `imageSearchQuery` is a direct field, never `image:{...}`, never a wrapper, never mixed with shapes, and never reused by a product card. Use a 4-6 layer layout:none ellipse/path composition only when the user explicitly requests illustration/no photos. '
    + 'Desktop commerce uses exactly three equal-width fill_container product cards from one coherent collection spanning the rail, with 24px gaps and no unused right tail; a mobile product rail instead uses at most 2 fill_container cards or equal numeric-width cards inside an explicit clipped scroller — never three fill_container cards. Badge text on the #C2410C accent must be >=15px semibold, or use #C2410C text on #FFF7ED. A generic home collection uses the exact validated queries `gray armchair isolated photo`, `artemide tolomeo lamp photo`, and `potted plant isolated photo`; the third card is a potted plant, not a vase. That trio is only for a generic home brief — in any other vertical every card query must name the exact product in that card visible name (咖啡 => `latte cup photo`), never the home trio. Every commerce `imageSearchQuery` uses no more than four English words, names exactly one product, and never uses a broad lifestyle, collection, or category query. Never put a lone small icon inside large fixed product media. Each Hero/Product/Art/Media frame has exactly one primary visual: one image leaf with a concrete English `imageSearchQuery`, or one substantial composed-shape visual, never an image plus an icon placeholder. The host enriches committed commerce images before each live preview, while finish retains one canonical post-final fallback. '
    + 'After batch 2, call finish once. `published:true` is terminal success. A result with `canContinue:true` continues by doing exactly what its `next` says: `stage:"needs_preview"` or `stage:"needs_refinalization"` means call finish exactly once more (never inspect or rerun generation); `stage:"needs_visual_review"` presents the final preview digest plus a checklist — either call finish exactly once more unchanged to accept and publish, or send exactly ONE bounded correction batch (I/K/U only, at most 16 calls and 6 KiB, never rebuilding Header or Hero) and then finish; and `stage:"needs_correction"` requires `canContinue:true` and one complete non-empty `repairTargets` array whose `checks.dsh.repairTargetSummary.omitted` is `0` and whose every item contains `operation:"U"`, an exact non-empty `nodeId`, and a non-empty `patch`: apply every item together in exactly one U-only batch, then call finish exactly once more, with no narration between them. The host bounds repair rounds at two: repair again ONLY when the new finish result itself presents another complete repairTargets array. Any other non-published finish result is terminal; stop on anything except `published:true` and never retry, inspect, read an image/context, abort, or rebuild. '
    + 'When the user explicitly wants to choose a style guide before generating, the optional pre-generation flow is `openpencil_style_plan` -> designer choice -> `openpencil_style_confirm` -> `openpencil_pipeline_begin` with style_selection set to the returned handle and the user\'s brief otherwise unchanged; this flow is opt-in only and never inserts itself into an ordinary transaction. '
} as const

interface SkillService {
  register(skill: {
    name: string
    description: string
    whenToUse?: string
    content: string
    source: string
  }): () => void
}

interface SystemPromptService {
  section(section: {
    name: string
    order: number
    text: () => string
  }): () => void
}

interface DisposableFiber {
  dispose(): unknown
}

function disposeFiber(fiber: unknown): void {
  if (typeof fiber !== 'object' || fiber === null || !('dispose' in fiber)) return
  const dispose = (fiber as DisposableFiber).dispose
  if (typeof dispose === 'function') void dispose.call(fiber)
}

/** Register the playbook only when the profile provides DSH's skill service. */
export function registerOpenPencilDesignSkill(ctx: Context): () => void {
  const fiber = ctx.inject(['skills'], skillCtx => {
    const skills = (skillCtx as Context & { skills?: SkillService }).skills
    // Defensive for lightweight host mocks that invoke every optional inject
    // callback with the same partial context.
    if (typeof skills?.register !== 'function') return
    skillCtx.effect(() => skills.register({
      name: OPENPENCIL_DESIGN_SKILL_NAME,
      description: OPENPENCIL_DESIGN_SKILL_DESCRIPTION,
      whenToUse: OPENPENCIL_DESIGN_SKILL_WHEN_TO_USE,
      content: OPENPENCIL_DESIGN_SKILL_CONTENT,
      source: 'bundled',
    }), 'dsh-openpencil:design-skill')
  })
  return () => disposeFiber(fiber)
}

/** Add a short reminder; the detailed contract remains in the on-demand skill. */
export function registerOpenPencilDesignGuidance(ctx: Context): () => void {
  const fiber = ctx.inject(['systemPrompt'], promptCtx => {
    const systemPrompt = (promptCtx as Context & { systemPrompt?: SystemPromptService }).systemPrompt
    if (typeof systemPrompt?.section !== 'function') return
    promptCtx.effect(
      () => systemPrompt.section(OPENPENCIL_DESIGN_GUIDANCE_SECTION),
      'dsh-openpencil:design-skill-guidance',
    )
  })
  return () => disposeFiber(fiber)
}
