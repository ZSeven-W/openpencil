/**
 * Catalog adapter for the opt-in style picker (spec 1599ef92 §3A).
 *
 * One reader over the installed style-guide catalog: the catalog `name` is the
 * canonical guide ID, platform semantics map the pipeline's web/mobile canvas
 * onto the catalog's `webapp`/`mobile` values, malformed entries are
 * quarantined with visible diagnostics instead of being reinterpreted, and a
 * content-addressed revision hash freezes the snapshot each selection session
 * validates against. Browse and search are deterministic and local; no second
 * hand-maintained list of guide names exists here.
 *
 * @module dsh-openpencil/style-catalog
 */

import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import {
  guideTypeScaleOf,
  mapGuidePalette,
  type ContinuationPalette,
  type SelectedStyleGuide,
  type StyleGuideDigest,
} from './design-knowledge.js'

/** Bumped whenever the adapter's validation or mapping semantics change. */
export const STYLE_CATALOG_ADAPTER_VERSION = 'style-catalog-v1'

/** Host-supported generated-text font stack (buildContract.node.text). */
export const SUPPORTED_FONT_STACK = 'Inter, system-ui, sans-serif'

/** Catalog platform values a selection handoff can target in v1. */
type HandoffPlatform = 'webapp' | 'mobile'
const CATALOG_PLATFORMS: Record<string, true> = { webapp: true, mobile: true, slides: true, card: true }
const HANDOFF_PLATFORMS: Record<string, true> = { webapp: true, mobile: true }
const HEX_COLOR = /^#[0-9A-Fa-f]{6}$/u

/** One valid catalog guide, keyed by its canonical catalog `name`. */
export interface StyleCatalogGuide {
  id: string
  platform: HandoffPlatform | 'slides' | 'card'
  tags: string[]
  summary: string
  aesthetics: string[]
  palette: Record<string, string>
  fonts: { heading?: string; body?: string; mono?: string }
  type: [string, number, number][]
}

/** A quarantined catalog entry with its visible local diagnostic. */
export interface StyleCatalogQuarantine {
  name?: string
  reason: string
}

/** Immutable snapshot of the installed catalog for one selection session. */
export interface StyleCatalogSnapshot {
  adapterVersion: string
  revision: string
  guides: StyleCatalogGuide[]
  quarantined: StyleCatalogQuarantine[]
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function quarantineOf(entry: unknown, reason: string): StyleCatalogQuarantine {
  const name = isRecord(entry) && typeof entry.name === 'string' ? entry.name : undefined
  return { ...(name === undefined ? {} : { name }), reason }
}

/** Validate one raw catalog entry; malformed entries quarantine, never throw. */
function validateGuide(entry: unknown): { guide?: StyleCatalogGuide; quarantine?: StyleCatalogQuarantine } {
  if (!isRecord(entry)) return { quarantine: quarantineOf(entry, 'entry is not an object') }
  const { name, platform, tags, summary, aesthetics, palette, fonts, type } = entry
  if (typeof name !== 'string' || name.trim().length === 0) return { quarantine: quarantineOf(entry, 'missing canonical name') }
  if (typeof platform !== 'string' || CATALOG_PLATFORMS[platform] !== true) {
    return { quarantine: quarantineOf(entry, `unsupported catalog platform: ${String(platform)}`) }
  }
  if (typeof summary !== 'string') return { quarantine: quarantineOf(entry, 'missing summary') }
  if (!Array.isArray(tags) || tags.some(tag => typeof tag !== 'string')) {
    return { quarantine: quarantineOf(entry, 'malformed tags') }
  }
  if (!Array.isArray(aesthetics) || aesthetics.some(item => typeof item !== 'string')) {
    return { quarantine: quarantineOf(entry, 'malformed aesthetics') }
  }
  if (!isRecord(palette)) return { quarantine: quarantineOf(entry, 'malformed palette') }
  for (const [token, value] of Object.entries(palette)) {
    if (typeof value !== 'string' || !HEX_COLOR.test(value)) {
      return { quarantine: quarantineOf(entry, `palette token "${token}" is not a #RRGGBB color`) }
    }
  }
  if (!isRecord(fonts)) return { quarantine: quarantineOf(entry, 'malformed fonts') }
  for (const value of Object.values(fonts)) {
    if (typeof value !== 'string') return { quarantine: quarantineOf(entry, 'malformed font family') }
  }
  if (!Array.isArray(type)) return { quarantine: quarantineOf(entry, 'malformed type scale') }
  for (const row of type) {
    if (!Array.isArray(row) || row.length !== 3
      || typeof row[0] !== 'string'
      || !Number.isFinite(row[1]) || !Number.isFinite(row[2])) {
      return { quarantine: quarantineOf(entry, 'malformed type scale row') }
    }
  }
  return {
    guide: {
      id: name,
      platform: platform as StyleCatalogGuide['platform'],
      tags: [...tags as string[]],
      summary,
      aesthetics: [...aesthetics as string[]],
      palette: { ...(palette as Record<string, string>) },
      fonts: { ...(fonts as StyleCatalogGuide['fonts']) },
      type: [...(type as [string, number, number][])],
    },
  }
}

/** Canonical serialization the revision hash is computed over. */
function canonicalGuideJson(guides: StyleCatalogGuide[]): string {
  const ordered = [...guides].sort((first, second) => (first.id < second.id ? -1 : first.id > second.id ? 1 : 0))
  return JSON.stringify(ordered.map(guide => ({
    name: guide.id,
    platform: guide.platform,
    tags: guide.tags,
    summary: guide.summary,
    aesthetics: guide.aesthetics,
    palette: guide.palette,
    fonts: guide.fonts,
    type: guide.type,
  })))
}

/**
 * Load and validate a style-guide catalog asset. Pure: identical input always
 * yields the identical snapshot (including the revision hash), and malformed
 * entries are quarantined with visible diagnostics instead of crashing.
 */
export function loadStyleCatalog(asset: unknown): StyleCatalogSnapshot {
  const raw = isRecord(asset) ? asset.guides : undefined
  const quarantined: StyleCatalogQuarantine[] = []
  const guides: StyleCatalogGuide[] = []
  const seen = new Set<string>()
  if (!Array.isArray(raw)) {
    throw new Error('style-catalog: asset must carry a guides array')
  }
  for (const entry of raw) {
    const { guide, quarantine } = validateGuide(entry)
    if (quarantine !== undefined) {
      quarantined.push(quarantine)
      continue
    }
    if (seen.has(guide!.id)) {
      quarantined.push(quarantineOf(entry, `duplicate canonical id: ${guide!.id}`))
      continue
    }
    seen.add(guide!.id)
    guides.push(guide!)
  }
  const digest = createHash('sha256').update(canonicalGuideJson(guides)).digest('hex')
  return {
    adapterVersion: STYLE_CATALOG_ADAPTER_VERSION,
    revision: `${STYLE_CATALOG_ADAPTER_VERSION}:sha256-${digest}`,
    guides,
    quarantined,
  }
}

let installedSnapshot: StyleCatalogSnapshot | undefined

/** Snapshot of the installed catalog asset shipped with the plugin build. */
export function loadInstalledStyleCatalog(): StyleCatalogSnapshot {
  if (installedSnapshot !== undefined) return installedSnapshot
  const raw: unknown = JSON.parse(readFileSync(
    new URL('./assets/openpencil-design/style-guides.json', import.meta.url),
    'utf8',
  ))
  installedSnapshot = loadStyleCatalog(raw)
  return installedSnapshot
}

/** Hard platform filter using the pipeline's canonical mapping (web→webapp). */
export function eligibleStyleGuides(snapshot: StyleCatalogSnapshot, platform: 'web' | 'mobile'): StyleCatalogGuide[] {
  const catalogPlatform = platform === 'web' ? 'webapp' : 'mobile'
  return snapshot.guides
    .filter(guide => guide.platform === catalogPlatform)
    .sort((first, second) => (first.id < second.id ? -1 : first.id > second.id ? 1 : 0))
}

/** Deterministic local search over ids, tags and summaries. */
export function searchStyleGuides(guides: StyleCatalogGuide[], query: string): StyleCatalogGuide[] {
  const needle = query.trim().toLowerCase()
  const pool = needle === ''
    ? guides
    : guides.filter(guide => (
      guide.id.toLowerCase().includes(needle)
      || guide.summary.toLowerCase().includes(needle)
      || guide.tags.some(tag => tag.toLowerCase().includes(needle))
    ))
  return [...pool].sort((first, second) => (first.id < second.id ? -1 : first.id > second.id ? 1 : 0))
}

/**
 * Verified theme metadata. Only catalog tags count; a guide without a theme
 * tag has no theme metadata and none may be guessed from its name.
 */
export function styleGuideTheme(guide: StyleCatalogGuide): 'dark' | 'light' | undefined {
  const dark = guide.tags.includes('dark-mode')
  const light = guide.tags.includes('light-mode')
  if (dark === light) return undefined
  return dark ? 'dark' : 'light'
}

/** Whether the guide's platform can receive a confirmed selection handle. */
export function guideSupportsHandoff(guide: StyleCatalogGuide): boolean {
  return HANDOFF_PLATFORMS[guide.platform] === true
}

/** Font disclosure: requested families mapped onto the supported contract. */
export interface SupportedFontDisclosure {
  supported: string
  mapping: { role: string; requested: string; supported: string; substitution: 'material' | 'none' }[]
  note: string
}

export function supportedFontMappingOf(guide: StyleCatalogGuide): SupportedFontDisclosure {
  const roles = ['heading', 'body', 'mono'] as const
  const mapping = roles
    .filter(role => typeof guide.fonts[role] === 'string' && guide.fonts[role]!.trim().length > 0)
    .map(role => {
      const requested = guide.fonts[role]!
      return {
        role,
        requested,
        supported: SUPPORTED_FONT_STACK,
        substitution: requested.toLowerCase() === 'inter' ? ('none' as const) : ('material' as const),
      }
    })
  const material = mapping.filter(entry => entry.substitution === 'material')
  const note = material.length === 0
    ? `All catalog font families map onto the supported ${SUPPORTED_FONT_STACK} stack; no substitution needed.`
    : `The catalog families ${material.map(entry => entry.requested).join(', ')} are not installed or generated: the host maps them onto the supported ${SUPPORTED_FONT_STACK} stack. The selection discloses this substitution before confirmation; no font is installed and generation keeps the portable font rules.`
  return { supported: SUPPORTED_FONT_STACK, mapping, note }
}

/** The immutable `selectedStyle` section carried by the begin contract (§4). */
export interface SelectedStyleSection {
  guideId: string
  catalogRevision: string
  palette: ContinuationPalette
  typography: {
    fontFamily: string
    scale: SelectedStyleGuide['typeScale']
  }
  fonts: SupportedFontDisclosure
  guidance: { direction: string; surfaces: string }
}

function asDigest(guide: StyleCatalogGuide): StyleGuideDigest {
  return {
    name: guide.id,
    platform: guide.platform,
    tags: guide.tags,
    summary: guide.summary,
    aesthetics: guide.aesthetics,
    palette: guide.palette,
    fonts: guide.fonts,
    type: guide.type,
  }
}

/**
 * Project a catalog guide onto the generation-facing selected-style section:
 * effective palette roles (not just the page color), typography scale mapped
 * onto the supported font contract, and the catalog's layout/aesthetic
 * guidance. Pure: the same guide and snapshot always project identically.
 */
export function selectedStyleOf(
  guide: StyleCatalogGuide,
  snapshot: StyleCatalogSnapshot,
  fallbackPalette: ContinuationPalette,
): SelectedStyleSection {
  const digest = asDigest(guide)
  const direction = [guide.summary, ...guide.aesthetics.slice(0, 3)].filter(part => part.length > 0).join(' ')
  return {
    guideId: guide.id,
    catalogRevision: snapshot.revision,
    palette: mapGuidePalette(digest, fallbackPalette),
    typography: {
      fontFamily: SUPPORTED_FONT_STACK,
      scale: guideTypeScaleOf(digest),
    },
    fonts: supportedFontMappingOf(guide),
    guidance: {
      direction: direction.slice(0, 460),
      surfaces: guide.aesthetics.slice(3, 5).join(' ').slice(0, 220),
    },
  }
}

/**
 * The same guide as the knowledge module's `SelectedStyleGuide` so the begin
 * path reuses its continuation-style and seed-color plumbing unchanged.
 */
export function asSelectedStyleGuide(
  guide: StyleCatalogGuide,
  fallbackPalette: ContinuationPalette,
): SelectedStyleGuide {
  const digest = asDigest(guide)
  const direction = [guide.summary, ...guide.aesthetics.slice(0, 3)].filter(part => part.length > 0).join(' ')
  return {
    name: guide.id,
    tags: guide.tags,
    palette: mapGuidePalette(digest, fallbackPalette),
    typeScale: guideTypeScaleOf(digest),
    direction: direction.slice(0, 460),
    surfaces: guide.aesthetics.slice(3, 5).join(' ').slice(0, 220),
  }
}
