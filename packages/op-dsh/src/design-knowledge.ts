/**
 * Deterministic App-knowledge injection for `openpencil_pipeline_begin`.
 *
 * OpenPencil's built-in design agent resolves a full style guide, domain
 * skills, and scene templates before it draws; the DSH pipeline is a blind
 * two-batch transaction. This module closes part of that gap without any
 * extra model or tool round trip: build-time digests of the App's style-guide
 * and domain-skill corpora (`scripts/build-style-guides.mjs`,
 * `scripts/build-domain-skills.mjs`) are matched to the brief with plain
 * keyword rules, and the winners ride the begin result.
 *
 * Everything here is pure and deterministic — same brief, same injection.
 * @module dsh-openpencil/design-knowledge
 */

import { readFileSync } from 'node:fs'

export interface StyleGuideDigest {
  name: string
  platform: string
  tags: string[]
  summary: string
  aesthetics: string[]
  palette: Record<string, string>
  fonts: { heading?: string; body?: string; mono?: string }
  type: [string, number, number][]
}

interface DomainSkillDigest {
  keywords: string[]
  digest: string
}

interface TemplateSkeleton {
  name: string
  width: unknown
  sections: string[]
}

interface DomainSkillCatalog {
  skills: Record<string, DomainSkillDigest>
  templates: TemplateSkeleton[]
}

/** The palette key shape `continuationStyle.palette` already exposes. */
export interface ContinuationPalette {
  page: string
  panel: string
  surface: string
  onSurface: string
  mutedOnSurface: string
  accent: string
  accentHighlight: string
  onAccent: string
  ink: string
  muted: string
  line: string
  surfaceLine: string
}

export interface SelectedStyleGuide {
  name: string
  tags: string[]
  palette: ContinuationPalette
  /** `[size, weight]` rows for display/heading/body/label when parseable. */
  typeScale: Partial<Record<'display' | 'heading' | 'body' | 'label', [number, number]>>
  direction: string
  surfaces: string
}

let styleGuides: StyleGuideDigest[] | undefined
let domainCatalog: DomainSkillCatalog | undefined

function loadStyleGuides(): StyleGuideDigest[] {
  if (styleGuides !== undefined) return styleGuides
  const raw: unknown = JSON.parse(readFileSync(
    new URL('./assets/openpencil-design/style-guides.json', import.meta.url),
    'utf8',
  ))
  const guides = (raw as { guides?: StyleGuideDigest[] }).guides
  if (!Array.isArray(guides) || guides.length < 40) {
    throw new Error('OpenPencil style-guide asset is missing or malformed')
  }
  styleGuides = guides
  return guides
}

function loadDomainCatalog(): DomainSkillCatalog {
  if (domainCatalog !== undefined) return domainCatalog
  const raw: unknown = JSON.parse(readFileSync(
    new URL('./assets/openpencil-design/domain-skills.json', import.meta.url),
    'utf8',
  ))
  const catalog = raw as DomainSkillCatalog
  if (typeof catalog !== 'object' || catalog === null || typeof catalog.skills !== 'object') {
    throw new Error('OpenPencil domain-skill asset is missing or malformed')
  }
  domainCatalog = catalog
  return catalog
}

/**
 * Ordered brief-category rules. The first matching row wins; a row picks the
 * guide by platform. `ecommerce-modern-light` deliberately maps commerce-web
 * briefs onto the pipeline's existing builtin so that path stays untouched.
 */
const GUIDE_RULES: { match: RegExp; web?: string; mobile?: string }[] = [
  { match: /咖啡|奶茶|外卖|餐饮|餐厅|美食|食品|甜品|烘焙|生鲜|超市|coffee|cafe|food|restaurant|bakery|grocery/iu, mobile: 'warm-food-mobile-light', web: 'retro-warm-light' },
  { match: /旅行|旅游|酒店|机票|民宿|travel|trip|hotel|booking/iu, mobile: 'travel-warm-mobile-light', web: 'retro-warm-light' },
  { match: /教育|课程|学习|培训|school|education|course|learning/iu, mobile: 'minimal-playful-light', web: 'education-friendly-light' },
  { match: /健身|冥想|瑜伽|睡眠|wellness|fitness|meditation|yoga/iu, mobile: 'wellness-green-mobile-light', web: 'wellness-organic-light' },
  { match: /医疗|医院|诊所|健康管理|healthcare|medical|clinic/iu, mobile: 'wellness-green-mobile-light', web: 'healthcare-trust-light' },
  { match: /金融|理财|银行|支付|钱包|证券|fintech|finance|banking|payment/iu, mobile: 'finance-clean-mobile-light', web: 'fintech-dark-blue-light' },
  { match: /加密|crypto|web3|区块链/iu, web: 'crypto-dark-bold', mobile: 'dark-bold-mobile' },
  { match: /仪表盘|仪表板|数据看板|管理后台|后台管理|控制台|dashboard|admin panel|analytics dashboard/iu, web: 'dashboard-analytics-dark', mobile: 'finance-clean-mobile-light' },
  { match: /开发者|开发工具|终端|命令行|developer|terminal|devtool|\bapi\b/iu, web: 'tech-developer-dark', mobile: 'terminal-minimal-dark' },
  { match: /游戏|电竞|game|gaming|esports/iu, web: 'gaming-electric-dark', mobile: 'neon-purple-mobile-dark' },
  { match: /音乐|播放器|music|podcast|电台/iu, mobile: 'music-dark-mobile' },
  { match: /奢侈|奢华|高端|珠宝|腕表|luxury|jewelry|premium fashion/iu, web: 'luxury-brand-dark', mobile: 'luxury-fashion-mobile-dark' },
  { match: /社交|社区|动态|朋友圈|social|community|feed/iu, mobile: 'social-vibrant-mobile-light', web: 'creative-bold-light' },
  { match: /作品集|个人主页|portfolio/iu, web: 'portfolio-minimal-light' },
  { match: /公益|慈善|非营利|nonprofit|charity/iu, web: 'nonprofit-warm-light' },
  { match: /人工智能|大模型|\bai\b|llm|智能体/iu, web: 'ai-product-dark' },
  { match: /创业|startup/iu, web: 'startup-gradient-dark' },
  { match: /电商|商城|购物|商品|商店|e-?commerce|shop|storefront|retail/iu, web: 'ecommerce-modern-light' },
  { match: /saas|软件服务|b2b|企业服务|产品官网/iu, web: 'saas-modern-light', mobile: 'clean-blue-mobile-light' },
  { match: /企业|corporate|enterprise/iu, web: 'corporate-blue-light' },
  { match: /落地页|官网|首页|landing|homepage|marketing site/iu, web: 'saas-clean-light' },
]

const DARK_BRIEF = /暗色|深色|夜间|dark\s*(?:mode|theme)|暗黑/iu
const CJK_BRIEF = /[一-鿿぀-ヿ가-힯]/u

function findToken(palette: Record<string, string>, patterns: RegExp[]): string | undefined {
  for (const pattern of patterns) {
    for (const [token, value] of Object.entries(palette)) {
      if (pattern.test(token)) return value
    }
  }
  return undefined
}

function luminance(hex: string): number {
  const channel = (value: number): number => {
    const scaled = value / 255
    return scaled <= 0.04045 ? scaled / 12.92 : ((scaled + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel(parseInt(hex.slice(1, 3), 16))
    + 0.7152 * channel(parseInt(hex.slice(3, 5), 16))
    + 0.0722 * channel(parseInt(hex.slice(5, 7), 16))
}

function contrast(first: string, second: string): number {
  const high = Math.max(luminance(first), luminance(second))
  const low = Math.min(luminance(first), luminance(second))
  return (high + 0.05) / (low + 0.05)
}

/**
 * Darken a color (linear interpolation toward black) until `text` reaches
 * WCAG AA on it. Guide accents are tuned for large fills, not for the
 * pipeline's white-label CTA contract, so an accent that cannot carry white
 * 4.5:1 is deepened deterministically instead of failing the finish gate.
 */
function darkenUntilAa(color: string, text: string): string {
  let current = color
  for (let step = 0; step < 20 && contrast(current, text) < 4.5; step += 1) {
    const mix = (offset: number): number =>
      Math.max(0, Math.round(parseInt(current.slice(offset, offset + 2), 16) * 0.92))
    current = `#${[1, 3, 5]
      .map(offset => mix(offset).toString(16).padStart(2, '0'))
      .join('')}`.toUpperCase()
  }
  return contrast(current, text) >= 4.5 ? current : '#1C1917'
}

/**
 * Map a guide's named palette tokens onto the fixed continuation-palette
 * shape. Unmapped keys inherit the caller's builtin fallback so a sparse
 * guide can never produce an incomplete palette.
 */
export function mapGuidePalette(guide: StyleGuideDigest, fallback: ContinuationPalette): ContinuationPalette {
  const tokens = guide.palette
  const dark = guide.tags.includes('dark-mode')
  const page = findToken(tokens, [/^page background$/i, /^app background$/i, /background$/i]) ?? fallback.page
  const ink = findToken(tokens, [/^primary text$/i, /^text primary$/i, /heading/i]) ?? fallback.ink
  const rawAccent = findToken(tokens, [/^primary accent$/i, /^accent$/i, /^primary$/i, /accent/i]) ?? fallback.accent
  // The CTA contract paints white labels on the accent; hold it to AA.
  const accent = luminance(rawAccent) > 0.45 ? rawAccent : darkenUntilAa(rawAccent, '#FFFFFF')
  const surface = findToken(tokens, [/inverse surface/i, /footer/i])
    ?? (dark ? findToken(tokens, [/elevated/i, /card surface/i, /^section alt$/i]) : undefined)
    ?? fallback.surface
  const onSurface = contrast(surface, '#FFFFFF') >= contrast(surface, ink) ? '#FFFFFF' : ink
  const mapped: ContinuationPalette = {
    page,
    panel: findToken(tokens, [/^card surface$/i, /^panel$/i, /^surface$/i, /elevated/i]) ?? fallback.panel,
    surface,
    onSurface,
    mutedOnSurface: onSurface === '#FFFFFF'
      ? fallback.mutedOnSurface
      : findToken(tokens, [/tertiary text/i]) ?? fallback.mutedOnSurface,
    accent,
    accentHighlight: findToken(tokens, [/accent light/i, /accent soft/i, /^section alt$/i, /tinted/i]) ?? fallback.accentHighlight,
    onAccent: (() => {
      // Best-contrast label color on the accent: white, the guide's ink,
      // or plain dark — bright lime accents on dark guides need the last.
      const candidates: string[] = ['#FFFFFF', ink, '#1C1917']
      return candidates.reduce((best, next) => (
        contrast(next, accent) > contrast(best, accent) ? next : best
      ))
    })(),
    ink,
    muted: findToken(tokens, [/^secondary text$/i, /^text secondary$/i]) ?? fallback.muted,
    line: findToken(tokens, [/^default border$/i, /^border$/i, /divider/i]) ?? fallback.line,
    surfaceLine: findToken(tokens, [/^subtle border$/i, /^light border$/i]) ?? fallback.surfaceLine,
  }
  // AA guard: a digest that parsed into an unreadable base is worse than the
  // builtin. Body text and footer text must both stay legible.
  if (contrast(mapped.ink, mapped.page) < 4.5 || contrast(mapped.onSurface, mapped.surface) < 4.5) {
    return fallback
  }
  return mapped
}

export function guideTypeScaleOf(guide: StyleGuideDigest): SelectedStyleGuide['typeScale'] {
  const rows = new Map(guide.type.map(([level, size, weight]) => [level.toLowerCase(), [size, weight] as [number, number]]))
  const pick = (...levels: string[]): [number, number] | undefined => {
    for (const level of levels) {
      const row = rows.get(level)
      if (row) return row
    }
    return undefined
  }
  const scale: SelectedStyleGuide['typeScale'] = {}
  const display = pick('display', 'hero')
  const heading = pick('title 1', 'heading 1', 'h1', 'title')
  const body = pick('body', 'body large')
  const label = pick('label', 'caption')
  if (display) scale.display = display
  if (heading) scale.heading = heading
  if (body) scale.body = body
  if (label) scale.label = label
  return scale
}

/**
 * Deterministically match the brief to one App style guide. Returns
 * `undefined` when no category rule fires or when the winner is the builtin
 * commerce guide the pipeline already hardcodes — both keep today's behavior.
 */
export function selectStyleGuide(
  brief: string,
  platform: 'web' | 'mobile',
  fallback: ContinuationPalette,
): SelectedStyleGuide | undefined {
  let name: string | undefined
  for (const rule of GUIDE_RULES) {
    if (!rule.match.test(brief)) continue
    name = platform === 'mobile' ? rule.mobile ?? rule.web : rule.web ?? rule.mobile
    if (name !== undefined) break
  }
  if (name === undefined) return undefined
  const guides = loadStyleGuides()
  let guide = guides.find(candidate => candidate.name === name)
  if (guide !== undefined && DARK_BRIEF.test(brief) && !guide.tags.includes('dark-mode')) {
    const darkName = platform === 'mobile' ? 'dark-bold-mobile' : 'midnight-minimal-dark'
    guide = guides.find(candidate => candidate.name === darkName) ?? guide
  }
  if (guide === undefined || guide.name === 'ecommerce-modern-light') return undefined
  const direction = [guide.summary, ...guide.aesthetics.slice(0, 3)].filter(part => part.length > 0).join(' ')
  return {
    name: guide.name,
    tags: guide.tags,
    palette: mapGuidePalette(guide, fallback),
    typeScale: guideTypeScaleOf(guide),
    direction: direction.slice(0, 460),
    surfaces: guide.aesthetics.slice(3, 5).join(' ').slice(0, 220),
  }
}

/** Total serialized budget for the injected domain guidance. */
const MAX_DOMAIN_GUIDANCE_BYTES = 2560

/**
 * Pick 1-2 domain-skill digests for the brief. Mobile always carries the
 * mobile-app architecture; a CJK brief always carries the CJK typography
 * rules; the landing/web entries additionally carry one shipped scene
 * template's section skeleton as a proven layout scaffold.
 */
export function selectDomainGuidance(
  brief: string,
  platform: 'web' | 'mobile',
): Record<string, string> | undefined {
  const catalog = loadDomainCatalog()
  const picked: string[] = []
  if (platform === 'mobile') picked.push('mobile-app')
  if (CJK_BRIEF.test(brief)) picked.push('cjk-typography')
  if (picked.length < 2 && /仪表盘|仪表板|数据看板|后台|控制台|dashboard|admin/iu.test(brief)) {
    picked.push('dashboard')
  }
  if (picked.length < 2 && /表单|登录|注册|form|sign\s?up|log\s?in|register/iu.test(brief)) {
    picked.push('form-ui')
  }
  if (picked.length < 2 && platform === 'web'
    && /落地页|官网|首页|landing|homepage|saas|marketing/iu.test(brief)) {
    picked.push('landing-page')
  }
  if (picked.length < 2 && platform === 'web') picked.push('web-app')
  const guidance: Record<string, string> = {}
  for (const name of picked.slice(0, 2)) {
    const skill = catalog.skills[name]
    if (skill !== undefined && skill.digest.length > 0) guidance[name] = skill.digest
  }
  // The landing/web scaffold: one proven template outline, trimmed hard.
  const scaffoldFor = guidance['landing-page'] !== undefined
    ? 'product-landing-light'
    : guidance['web-app'] !== undefined && /saas/iu.test(brief)
      ? 'saas-landing-orange'
      : undefined
  if (scaffoldFor !== undefined) {
    const template = catalog.templates.find(candidate => candidate.name === scaffoldFor)
    if (template !== undefined) {
      const key = guidance['landing-page'] !== undefined ? 'landing-page' : 'web-app'
      const outline = `Proven section skeleton (${template.name}): ${template.sections.join(' -> ')}`.slice(0, 400)
      guidance[key] = `${guidance[key]} • ${outline}`
    }
  }
  if (Object.keys(guidance).length === 0) return undefined
  while (Buffer.byteLength(JSON.stringify(guidance)) > MAX_DOMAIN_GUIDANCE_BYTES) {
    const keys = Object.keys(guidance)
    const last = keys[keys.length - 1]
    if (keys.length === 1) {
      guidance[last] = guidance[last].slice(0, 800)
      break
    }
    delete guidance[last]
  }
  return guidance
}
