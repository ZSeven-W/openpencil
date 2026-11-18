import { readdir, readFile, mkdir, rename, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')

export const DEFAULT_SOURCE_DIR = resolve(
  root,
  '../../crates/op-ai-skills/skills/style-guides',
)
export const DEFAULT_OUTPUT_PATH = resolve(root, 'lib/assets/openpencil-design/style-guides.json')

/** Upper bound for one serialized guide digest (JSON bytes). */
const MAX_GUIDE_BYTES = 1200
/** The corpus must stay a real catalogue, not a couple of survivors. */
const MIN_GUIDES = 40
const HEX = /#[0-9A-Fa-f]{6}\b/

function fail(message) {
  throw new Error(`build-style-guides: ${message}`)
}

function parseFrontmatter(markdown) {
  const match = /^---\n([\s\S]*?)\n---\n/.exec(markdown)
  if (!match) return undefined
  const name = /name:\s*'?([A-Za-z0-9-]+)'?/.exec(match[1])?.[1]
  const platform = /platform:\s*([A-Za-z]+)/.exec(match[1])?.[1]
  const tagsRaw = /tags:\s*\[([^\]]*)\]/.exec(match[1])?.[1] ?? ''
  const tags = tagsRaw.split(',').map(tag => tag.trim()).filter(tag => tag.length > 0)
  if (!name) return undefined
  return { name, platform: platform ?? 'webapp', tags, body: markdown.slice(match[0].length) }
}

/** First paragraph under `## Style Summary`, trimmed to a budget. */
function summaryOf(body) {
  const section = /## Style Summary\n+([\s\S]*?)(?:\n##|\n### )/.exec(body)?.[1] ?? ''
  const paragraph = section.split(/\n\s*\n/).map(part => part.trim()).find(part => part.length > 0) ?? ''
  return paragraph.replace(/\s+/g, ' ').slice(0, 220)
}

/** `- **Label**: text` bullets from the Key aesthetics list. */
function aestheticsOf(body) {
  const bullets = []
  for (const match of body.matchAll(/^- \*\*([^*]+)\*\*:\s*([^\n]+)$/gm)) {
    bullets.push(`${match[1].trim()}: ${match[2].trim()}`)
    if (bullets.length >= 5) break
  }
  return bullets
}

/** Every `| Token | #HEX |`-shaped table row across the Color System. */
function paletteOf(body) {
  const palette = {}
  for (const match of body.matchAll(/^\|\s*([^|#\n]{2,40}?)\s*\|\s*(#[0-9A-Fa-f]{6})[0-9A-Fa-f]{0,2}\s*\|/gm)) {
    const token = match[1].trim()
    if (/^-+$/.test(token) || token.toLowerCase() === 'token') continue
    if (palette[token] === undefined) palette[token] = match[2].toUpperCase()
    if (Object.keys(palette).length >= 18) break
  }
  return palette
}

/** Font families table (`Role | Family | Usage`). */
function fontsOf(body) {
  const fonts = {}
  const section = /### Font Families\n([\s\S]*?)(?:\n###|\n##)/.exec(body)?.[1] ?? ''
  for (const match of section.matchAll(/^\|\s*([^|\n]+?)\s*\|\s*([^|\n]+?)\s*\|/gm)) {
    const role = match[1].trim().toLowerCase()
    const family = match[2].trim()
    if (role.startsWith('-') || role === 'role' || family.startsWith('-')) continue
    if (role.includes('display') || role.includes('heading')) fonts.heading ??= family
    else if (role.includes('body')) fonts.body ??= family
    else if (role.includes('mono') || role.includes('data')) fonts.mono ??= family
    else if (role.includes('everything') || role.includes('all')) {
      fonts.heading ??= family
      fonts.body ??= family
    }
  }
  return fonts
}

/** Type scale rows `Level | Size | Font | Weight` → `[level, px, weight]`. */
function typeScaleOf(body) {
  const rows = []
  const section = /### Type Scale\n([\s\S]*?)(?:\n###|\n##)/.exec(body)?.[1] ?? ''
  for (const match of section.matchAll(/^\|\s*([^|\n]+?)\s*\|\s*(\d+)px\s*\|\s*[^|\n]+\|\s*(\d{3})\s*\|/gm)) {
    const level = match[1].trim()
    if (level.startsWith('-')) continue
    rows.push([level, Number(match[2]), Number(match[3])])
    if (rows.length >= 8) break
  }
  return rows
}

function digestGuide(markdown) {
  const front = parseFrontmatter(markdown)
  if (!front) return undefined
  const digest = {
    name: front.name,
    platform: front.platform,
    tags: front.tags,
    summary: summaryOf(front.body),
    aesthetics: aestheticsOf(front.body),
    palette: paletteOf(front.body),
    fonts: fontsOf(front.body),
    type: typeScaleOf(front.body),
  }
  // Trim the free-text parts until the serialized digest fits its budget.
  while (Buffer.byteLength(JSON.stringify(digest)) > MAX_GUIDE_BYTES) {
    if (digest.aesthetics.length > 2) digest.aesthetics.pop()
    else if (digest.summary.length > 80) digest.summary = digest.summary.slice(0, digest.summary.length - 40)
    else if (digest.type.length > 4) digest.type.pop()
    else {
      const keys = Object.keys(digest.palette)
      if (keys.length <= 6) fail(`${digest.name} cannot fit ${MAX_GUIDE_BYTES} bytes`)
      delete digest.palette[keys[keys.length - 1]]
    }
  }
  return digest
}

export function createStyleGuideCatalog(sources) {
  const guides = []
  for (const [file, markdown] of sources) {
    const digest = digestGuide(markdown)
    if (!digest) fail(`${file} has no parseable frontmatter`)
    if (Object.keys(digest.palette).length < 4) fail(`${file} yields fewer than 4 palette tokens`)
    if (digest.fonts.heading === undefined && digest.fonts.body === undefined) {
      fail(`${file} yields no font families`)
    }
    guides.push(digest)
  }
  guides.sort((left, right) => left.name.localeCompare(right.name))
  if (guides.length < MIN_GUIDES) fail(`only ${guides.length} guides parsed, expected >= ${MIN_GUIDES}`)
  const names = new Set(guides.map(guide => guide.name))
  for (const sentinel of ['ecommerce-modern-light', 'warm-food-mobile-light', 'saas-modern-light']) {
    if (!names.has(sentinel)) fail(`catalogue is missing sentinel guide ${sentinel}`)
  }
  for (const guide of guides) {
    if (!Object.values(guide.palette).every(value => HEX.test(value))) {
      fail(`${guide.name} carries a non-hex palette value`)
    }
  }
  return { guides }
}

export async function buildStyleGuides(options = {}) {
  const sourceDir = resolve(options.sourceDir ?? DEFAULT_SOURCE_DIR)
  const outputPath = resolve(options.outputPath ?? DEFAULT_OUTPUT_PATH)
  const files = (await readdir(sourceDir)).filter(file => file.endsWith('.md')).sort()
  const sources = []
  for (const file of files) {
    sources.push([file, await readFile(join(sourceDir, file), 'utf8')])
  }
  const catalog = createStyleGuideCatalog(sources)
  const content = `${JSON.stringify(catalog)}\n`
  await mkdir(dirname(outputPath), { recursive: true })
  const temporary = `${outputPath}.tmp-${process.pid}`
  await writeFile(temporary, content, 'utf8')
  await rename(temporary, outputPath)
  return { outputPath, bytes: Buffer.byteLength(content), guides: catalog.guides.length }
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : ''
if (import.meta.url === invokedPath) {
  const result = await buildStyleGuides()
  console.log(`built ${result.outputPath} (${result.guides} guides, ${result.bytes} bytes)`)
}
