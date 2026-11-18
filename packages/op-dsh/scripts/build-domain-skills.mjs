import { readFile, mkdir, rename, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')

export const DEFAULT_SKILLS_DIR = resolve(
  root,
  '../../crates/op-ai-skills/skills/domains',
)
export const DEFAULT_TEMPLATES_DIR = resolve(
  root,
  '../../crates/op-editor-core/assets/scene_templates',
)
export const DEFAULT_OUTPUT_PATH = resolve(root, 'lib/assets/openpencil-design/domain-skills.json')

/** The seven domain skills the begin contract can inject. */
const SKILL_FILES = [
  'mobile-app',
  'landing-page',
  'dashboard',
  'web-app',
  'form-ui',
  'cards',
  'cjk-typography',
]

/**
 * Shipped page-shaped scene templates whose section skeletons double as
 * layout scaffolds. Deck/infographic templates are deliberately excluded:
 * `use_scene_template` adopts whole boards, which fights the pipeline's
 * single-root two-batch transaction, so only the outline travels.
 */
const TEMPLATE_FILES = ['product-landing-light.op', 'saas-landing-orange.op']

/** Byte budget for one skill digest (JSON string bytes of the text). */
const MAX_SKILL_BYTES = 1000

function fail(message) {
  throw new Error(`build-domain-skills: ${message}`)
}

/**
 * Keep only executable substance: bullet rules and lines that carry numbers,
 * hex colors, or hard directives. Narrative sentences are dropped first, so
 * what survives the budget is the part a generation script can obey.
 */
/**
 * Rules that conflict with the DSH pipeline contract are dropped per skill:
 * DSH text nodes must keep the portable "Inter, system-ui, sans-serif" stack
 * (the render hosts own CJK fallback), so cjk-typography's font-family
 * mandates would only fight the typography quality gate.
 */
const SKILL_DROP_PATTERNS = {
  // The App's size-banded CJK lineHeight (1.02-1.25 for display sizes)
  // conflicts with DSH's hard CJK gate (every text >= 1.3), so that band
  // is replaced by a DSH-aligned rule below.
  'cjk-typography': /Noto Sans|Body family|font pairing|lineHeight bands by FONT SIZE/,
}

/** DSH-aligned rules appended after the vendored digest per skill. */
const SKILL_APPEND_RULES = {
  'cjk-typography':
    'lineHeight (DSH gate): every CJK text keeps lineHeight >= 1.3 — display/headline 1.3, body 1.7-1.8, captions 1.45-1.5; never below 1.3.',
}

function digestSkillMarkdown(markdown, skillName) {
  const drop = SKILL_DROP_PATTERNS[skillName]
  const body = markdown.replace(/^---\n[\s\S]*?\n---\n/, '')
  const lines = []
  for (const raw of body.split('\n')) {
    const line = raw
      .replace(/^#+\s*/, '')
      .replace(/\*\*/g, '')
      .replace(/`/g, '')
      .trim()
    if (line.length < 8) continue
    const isBullet = /^[-*]\s/.test(raw.trim()) || /^\d+\)/.test(line)
    const hasSubstance = /\d|#[0-9A-Fa-f]{3,8}|\b(?:MUST|NEVER|DO NOT|ALWAYS|never|always|only|forbidden)\b/.test(line)
    if (!isBullet && !hasSubstance) continue
    if (drop !== undefined && drop.test(line)) continue
    // Cap one rule's budget share so a long lead bullet cannot starve the
    // rest of the skill; cut at a clause boundary where one exists.
    let rule = line.replace(/^[-*]\s+/, '')
    if (rule.length > 170) {
      const clause = rule.slice(0, 170).lastIndexOf(';')
      rule = rule.slice(0, clause > 80 ? clause : 167).trimEnd()
    }
    lines.push(rule)
  }
  const appended = SKILL_APPEND_RULES[skillName]
  if (appended !== undefined) lines.unshift(appended)
  const seen = new Set()
  let digest = ''
  for (const line of lines) {
    const key = line.slice(0, 60)
    if (seen.has(key)) continue
    seen.add(key)
    const next = digest.length === 0 ? line : `${digest} • ${line}`
    if (Buffer.byteLength(next) > MAX_SKILL_BYTES) break
    digest = next
  }
  return digest
}

function parseKeywords(markdown) {
  const front = /^---\n([\s\S]*?)\n---\n/.exec(markdown)?.[1] ?? ''
  const keywords = []
  const push = value => {
    const trimmed = value.trim().replace(/^["']|["']$/g, '').trim()
    if (trimmed.length > 0 && !trimmed.startsWith('/') && !trimmed.startsWith('#')) {
      keywords.push(trimmed)
    }
  }
  const inline = /keywords:\s*\[([^\]]*)\]/.exec(front)?.[1]
  if (inline) {
    for (const keyword of inline.split(',')) push(keyword)
  } else {
    const block = /keywords:\n((?:\s+(?:- [^\n]*|#[^\n]*)\n?)+)/.exec(front)?.[1] ?? ''
    for (const line of block.split('\n')) {
      const item = /^\s*- (.+)$/.exec(line)?.[1]
      if (item !== undefined) push(item)
    }
  }
  return keywords
}

/** Top-level section outline of a page template: `Name(height)` per region. */
function templateSkeleton(name, documentJson) {
  const doc = JSON.parse(documentJson)
  const rootNode = (doc.children ?? [])[0]
  if (!rootNode || !Array.isArray(rootNode.children)) fail(`${name} has no root sections`)
  const sections = rootNode.children
    .filter(child => typeof child.name === 'string' && child.name.length > 0)
    .map(child => {
      const height = typeof child.height === 'number' ? `${Math.round(child.height)}px` : child.height ?? 'fit'
      return `${child.name}(${height})`
    })
  if (sections.length < 4) fail(`${name} yields fewer than 4 sections`)
  return {
    name: name.replace(/\.op$/, ''),
    width: rootNode.width,
    sections: sections.slice(0, 14),
  }
}

export function createDomainSkillCatalog(skillSources, templateSources) {
  const skills = {}
  for (const [name, markdown] of skillSources) {
    const digest = digestSkillMarkdown(markdown, name)
    if (Buffer.byteLength(digest) < 200) fail(`${name} digest is implausibly small`)
    skills[name] = { keywords: parseKeywords(markdown), digest }
  }
  for (const required of SKILL_FILES) {
    if (skills[required] === undefined) fail(`missing required skill ${required}`)
  }
  if (!skills['cjk-typography'].digest.includes('lineHeight')) {
    fail('cjk-typography digest lost its lineHeight bands')
  }
  if (!/status bar|STATUS BAR/i.test(skills['mobile-app'].digest)) {
    fail('mobile-app digest lost the status-bar contract')
  }
  const templates = templateSources.map(([name, json]) => templateSkeleton(name, json))
  return { skills, templates }
}

export async function buildDomainSkills(options = {}) {
  const skillsDir = resolve(options.skillsDir ?? DEFAULT_SKILLS_DIR)
  const templatesDir = resolve(options.templatesDir ?? DEFAULT_TEMPLATES_DIR)
  const outputPath = resolve(options.outputPath ?? DEFAULT_OUTPUT_PATH)
  const skillSources = []
  for (const name of SKILL_FILES) {
    skillSources.push([name, await readFile(join(skillsDir, `${name}.md`), 'utf8')])
  }
  const templateSources = []
  for (const file of TEMPLATE_FILES) {
    templateSources.push([file, await readFile(join(templatesDir, file), 'utf8')])
  }
  const catalog = createDomainSkillCatalog(skillSources, templateSources)
  const content = `${JSON.stringify(catalog)}\n`
  await mkdir(dirname(outputPath), { recursive: true })
  const temporary = `${outputPath}.tmp-${process.pid}`
  await writeFile(temporary, content, 'utf8')
  await rename(temporary, outputPath)
  return {
    outputPath,
    bytes: Buffer.byteLength(content),
    skills: Object.keys(catalog.skills).length,
    templates: catalog.templates.length,
  }
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : ''
if (import.meta.url === invokedPath) {
  const result = await buildDomainSkills()
  console.log(`built ${result.outputPath} (${result.skills} skills, ${result.templates} templates, ${result.bytes} bytes)`)
}
