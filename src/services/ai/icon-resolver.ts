import type { PenNode, PathNode } from '@/types/pen'
import { useDocumentStore } from '@/stores/document-store'
import featherData from '@iconify-json/feather/icons.json'
import {
  clamp,
  toSizeNumber,
  toStrokeThicknessNumber,
  extractPrimaryColor,
} from './generation-utils'

// ---------------------------------------------------------------------------
// Core UI icon paths (Lucide-style, 24×24 viewBox)
// Hand-picked high-frequency icons for guaranteed instant sync resolution.
// Feather icons are added at module init from the bundled @iconify-json/feather.
// ---------------------------------------------------------------------------

type IconEntry = { d: string; style: 'stroke' | 'fill'; iconId: string }
// Helpers keep definitions concise
const s = (d: string, id: string): IconEntry => ({ d, style: 'stroke', iconId: id })
const f = (d: string, id: string): IconEntry => ({ d, style: 'fill',   iconId: id })

// Shared path objects — aliases reference the same entry, avoiding duplication.
// Resolution still works because every alias key stays in the map.
const _X        = s('M18 6L6 18M6 6l12 12',                                                'lucide:x')
const _PLUS     = s('M12 5v14M5 12h14',                                                     'lucide:plus')
const _THUMBSUP = s('M14 9V5a3 3 0 00-3-3l-4 9v11h11.28a2 2 0 002-1.7l1.38-9a2 2 0 00-2-2.3H14zM7 22H4a2 2 0 01-2-2v-7a2 2 0 012-2h3', 'lucide:thumbs-up')
const _USER     = s('M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2M16 7a4 4 0 11-8 0 4 4 0 018 0z', 'lucide:user')
const _SETTINGS = s('M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2zM15 12a3 3 0 11-6 0 3 3 0 016 0z', 'lucide:settings')
const _MAIL     = s('M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2zm16 2l-10 7L2 6', 'lucide:mail')
const _BELL     = s('M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9M13.73 21a2 2 0 01-3.46 0', 'lucide:bell')
const _ZAP      = f('M13 2L3 14h9l-1 8 10-12h-9l1-8z',                                     'lucide:zap')
const _IMAGE    = s('M21 3H3a2 2 0 00-2 2v14a2 2 0 002 2h18a2 2 0 002-2V5a2 2 0 00-2-2zM8.5 10a1.5 1.5 0 100-3 1.5 1.5 0 000 3zM21 15l-5-5L5 21', 'lucide:image')
const _MESSAGE  = s('M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2v10z',        'lucide:message-square')
const _MAPPIN   = s('M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0118 0zM12 13a3 3 0 100-6 3 3 0 000 6z', 'lucide:map-pin')
const _BARCHART = s('M18 20V10M12 20V4M6 20v-4',                                            'lucide:bar-chart-2')
const _ALERT    = s('M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01', 'lucide:alert-triangle')
const _HELP     = s('M12 22a10 10 0 100-20 10 10 0 000 20zM9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01', 'lucide:help-circle')
const _REFRESH  = s('M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15', 'lucide:refresh-cw')
const _CART     = s('M9 22a1 1 0 100-2 1 1 0 000 2zM20 22a1 1 0 100-2 1 1 0 000 2zM1 1h4l2.68 13.39a2 2 0 002 1.61h9.72a2 2 0 002-1.61L23 6H6', 'lucide:shopping-cart')
const _TRASH    = s('M3 6h18M8 6V4h8v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6M10 11v6M14 11v6', 'lucide:trash-2')
const _DOT      = f('M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0',                            'lucide:circle')

const ICON_PATH_MAP: Record<string, IconEntry> = {
  // ── Navigation & actions ────────────────────────────────────────────────
  menu:         s('M4 6h16M4 12h16M4 18h16',                                           'lucide:menu'),
  check:        s('M20 6L9 17l-5-5',                                                    'lucide:check'),
  minus:        s('M5 12h14',                                                            'lucide:minus'),
  search:       s('M11 19a8 8 0 100-16 8 8 0 000 16zM21 21l-4.35-4.35',               'lucide:search'),
  arrowright:   s('M5 12h14M12 5l7 7-7 7',                                              'lucide:arrow-right'),
  arrowleft:    s('M19 12H5M12 19l-7-7 7-7',                                            'lucide:arrow-left'),
  arrowup:      s('M12 19V5M5 12l7-7 7 7',                                              'lucide:arrow-up'),
  arrowdown:    s('M12 5v14M19 12l-7 7-7-7',                                            'lucide:arrow-down'),
  chevronright: s('M9 18l6-6-6-6',                                                      'lucide:chevron-right'),
  chevronleft:  s('M15 18l-6-6 6-6',                                                    'lucide:chevron-left'),
  chevrondown:  s('M6 9l6 6 6-6',                                                       'lucide:chevron-down'),
  chevronup:    s('M18 15l-6-6-6 6',                                                    'lucide:chevron-up'),
  // aliases
  x: _X, close: _X,
  plus: _PLUS, add: _PLUS,
  // ── People & account ────────────────────────────────────────────────────
  star:         f('M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z', 'lucide:star'),
  heart:        s('M20.84 4.61a5.5 5.5 0 00-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 00-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 000-7.78z', 'lucide:heart'),
  thumbsup:     _THUMBSUP,
  home:         s('M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2V9zM9 22V12h6v10',     'lucide:home'),
  user:         _USER,
  users:        s('M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2M9 11a4 4 0 100-8 4 4 0 000 8zM23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75', 'lucide:users'),
  // aliases
  like: _THUMBSUP, profile: _USER, avatar: _USER,
  // ── System & settings ───────────────────────────────────────────────────
  settings:     _SETTINGS,
  mail:         _MAIL,
  eye:          s('M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8zM15 12a3 3 0 11-6 0 3 3 0 016 0z', 'lucide:eye'),
  lock:         s('M19 11H5a2 2 0 00-2 2v7a2 2 0 002 2h14a2 2 0 002-2v-7a2 2 0 00-2-2zM7 11V7a5 5 0 0110 0v4', 'lucide:lock'),
  bell:         _BELL,
  shield:       s('M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z',                     'lucide:shield'),
  zap:          _ZAP,
  // aliases
  gear: _SETTINGS, email: _MAIL, notification: _BELL, bolt: _ZAP,
  // ── Media & content ─────────────────────────────────────────────────────
  play:         f('M5 3l14 9-14 9V3z',                                                  'lucide:play'),
  pause:        f('M6 4h4v16H6zM14 4h4v16h-4z',                                         'lucide:pause'),
  download:     s('M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3',    'lucide:download'),
  upload:       s('M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12',   'lucide:upload'),
  image:        _IMAGE,
  camera:       s('M23 19a2 2 0 01-2 2H3a2 2 0 01-2-2V8a2 2 0 012-2h4l2-3h6l2 3h4a2 2 0 012 2v11zM12 17a4 4 0 100-8 4 4 0 000 8z', 'lucide:camera'),
  video:        s('M23 7l-7 5 7 5V7zM1 5h15a2 2 0 012 2v10a2 2 0 01-2 2H1a2 2 0 01-2-2V7a2 2 0 012-2z', 'lucide:video'),
  // alias
  photo: _IMAGE,
  // ── Communication ───────────────────────────────────────────────────────
  message:      _MESSAGE,
  phone:        s('M22 16.92v3a2 2 0 01-2.18 2 19.79 19.79 0 01-8.63-3.07A19.5 19.5 0 013.07 9.81a19.79 19.79 0 01-3.07-8.63A2 2 0 012 1h3a2 2 0 012 1.72c.127.96.361 1.903.7 2.81a2 2 0 01-.45 2.11L6.09 8.91a16 16 0 006 6l1.27-1.27a2 2 0 012.11-.45c.907.339 1.85.573 2.81.7A2 2 0 0122 16.92z', 'lucide:phone'),
  send:         s('M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z',                              'lucide:send'),
  share:        s('M4 12v8a2 2 0 002 2h12a2 2 0 002-2v-8M16 6l-4-4-4 4M12 2v13',     'lucide:share'),
  globe:        s('M12 22a10 10 0 100-20 10 10 0 000 20zM2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z', 'lucide:globe'),
  // alias
  chat: _MESSAGE,
  // ── Content & data ──────────────────────────────────────────────────────
  code:         s('M16 18l6-6-6-6M8 6l-6 6 6 6',                                       'lucide:code-2'),
  bookmark:     s('M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2v16z',                 'lucide:bookmark'),
  tag:          s('M20.59 13.41l-7.17 7.17a2 2 0 01-2.83 0L2 12V2h10l8.59 8.59a2 2 0 010 2.82zM7 7h.01', 'lucide:tag'),
  link:         s('M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71', 'lucide:link'),
  externallink: s('M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3', 'lucide:external-link'),
  copy:         s('M20 9h-9a2 2 0 00-2 2v9a2 2 0 002 2h9a2 2 0 002-2V11a2 2 0 00-2-2zM5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1', 'lucide:copy'),
  clipboard:    s('M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2M9 2h6a1 1 0 011 1v2a1 1 0 01-1 1H9a1 1 0 01-1-1V3a1 1 0 011-1z', 'lucide:clipboard'),
  edit:         s('M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z', 'lucide:edit'),
  // pencil is a distinct icon (no bounding square)
  pencil:       s('M17 3a2.828 2.828 0 114 4L7.5 20.5 2 22l1.5-5.5L17 3z',            'lucide:pencil'),
  trash:        _TRASH,
  // alias
  delete: _TRASH,
  // ── Time & location ─────────────────────────────────────────────────────
  calendar:     s('M8 2v4M16 2v4M3 10h18M5 4h14a2 2 0 012 2v14a2 2 0 01-2 2H5a2 2 0 01-2-2V6a2 2 0 012-2z', 'lucide:calendar'),
  clock:        s('M12 22a10 10 0 100-20 10 10 0 000 20zM12 6v6l4 2',                  'lucide:clock'),
  // timer is distinct: stopwatch with a top indicator
  timer:        s('M10 2h4M12 6v4l2 2M21 12a9 9 0 11-18 0 9 9 0 0118 0',              'lucide:timer'),
  mappin:       _MAPPIN,
  map:          s('M1 6v16l7-4 8 4 7-4V2l-7 4-8-4-7 4zM8 2v16M16 6v16',              'lucide:map'),
  // alias
  location: _MAPPIN,
  // ── Analytics & status ──────────────────────────────────────────────────
  barchart:     _BARCHART,
  trendingup:   s('M23 6l-9.5 9.5-5-5L1 18M17 6h6v6',                                 'lucide:trending-up'),
  activity:     s('M22 12h-4l-3 9L9 3l-3 9H2',                                         'lucide:activity'),
  info:         s('M12 22a10 10 0 100-20 10 10 0 000 20zM12 8v4M12 16h.01',           'lucide:info'),
  alert:        _ALERT,
  help:         _HELP,
  checkcircle:  s('M22 11.08V12a10 10 0 11-5.93-9.14M22 4L12 14.01l-3-3',             'lucide:check-circle'),
  refresh:      _REFRESH,
  filter:       s('M22 3H2l8 9.46V19l4 2V12.46L22 3z',                                 'lucide:filter'),
  // aliases
  chart: _BARCHART, analytics: _BARCHART, warning: _ALERT, question: _HELP, reload: _REFRESH,
  // ── Layout & UI ─────────────────────────────────────────────────────────
  grid:         s('M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z',              'lucide:grid'),
  list:         s('M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',               'lucide:list'),
  layers:       s('M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5',         'lucide:layers'),
  // ── Commerce ────────────────────────────────────────────────────────────
  creditcard:   s('M21 4H3a2 2 0 00-2 2v12a2 2 0 002 2h18a2 2 0 002-2V6a2 2 0 00-2-2zM1 10h22', 'lucide:credit-card'),
  cart:         _CART,
  award:        s('M12 15a7 7 0 100-14 7 7 0 000 14zM8.21 13.89L7 23l5-3 5 3-1.21-9.12', 'lucide:award'),
  // alias
  shoppingcart: _CART,
  // ── Misc ────────────────────────────────────────────────────────────────
  dot:          _DOT,
  circlefill:   f('M12 12m-4 0a4 4 0 1 0 8 0a4 4 0 1 0 -8 0',                         'lucide:circle'),
  // aliases
  bullet: _DOT, point: _DOT,
}

// Snapshot the hand-picked keys BEFORE Feather expansion so BUILTIN_ICONS
// only contains the curated Lucide/hand-written icons, not the full Feather set.
const _handPickedKeys = new Set(Object.keys(ICON_PATH_MAP))

// ---------------------------------------------------------------------------
// Feather icon set — bundled from @iconify-json/feather (286 icons, all stroke)
// Populated at module init so AI-generated designs never need async network fetches
// for standard Feather icons.
// ---------------------------------------------------------------------------

/**
 * Parse a Feather SVG body string into a compound SVG path `d` string.
 * Feather uses <path>, <circle>, <rect>, <ellipse> (all inside optional <g>).
 * All elements are converted to equivalent path commands and joined.
 */
function featherBodyToPathD(body: string): string | null {
  const parts: string[] = []

  // <path d="...">
  const pathRe = /\bd="([^"]+)"/g
  let m: RegExpExecArray | null
  while ((m = pathRe.exec(body)) !== null) parts.push(m[1])

  // <circle cx="x" cy="y" r="r"> → two half-arcs forming a closed circle
  const circleRe = /<circle[^>]+>/g
  while ((m = circleRe.exec(body)) !== null) {
    const tag = m[0]
    const cx = parseFloat(tag.match(/\bcx="([^"]+)"/)?.[1] ?? 'NaN')
    const cy = parseFloat(tag.match(/\bcy="([^"]+)"/)?.[1] ?? 'NaN')
    const r  = parseFloat(tag.match(/\br="([^"]+)"/)?.[1] ?? 'NaN')
    if (!isNaN(cx) && !isNaN(cy) && !isNaN(r)) {
      parts.push(`M ${cx - r} ${cy} a ${r} ${r} 0 1 0 ${r * 2} 0 a ${r} ${r} 0 1 0 ${-r * 2} 0 Z`)
    }
  }

  // <ellipse cx="x" cy="y" rx="rx" ry="ry">
  const ellipseRe = /<ellipse[^>]+>/g
  while ((m = ellipseRe.exec(body)) !== null) {
    const tag = m[0]
    const cx = parseFloat(tag.match(/\bcx="([^"]+)"/)?.[1] ?? 'NaN')
    const cy = parseFloat(tag.match(/\bcy="([^"]+)"/)?.[1] ?? 'NaN')
    const rx = parseFloat(tag.match(/\brx="([^"]+)"/)?.[1] ?? 'NaN')
    const ry = parseFloat(tag.match(/\bry="([^"]+)"/)?.[1] ?? 'NaN')
    if (!isNaN(cx) && !isNaN(cy) && !isNaN(rx) && !isNaN(ry)) {
      parts.push(`M ${cx - rx} ${cy} a ${rx} ${ry} 0 1 0 ${rx * 2} 0 a ${rx} ${ry} 0 1 0 ${-rx * 2} 0 Z`)
    }
  }

  // <rect x="x" y="y" width="w" height="h" rx="r">
  const rectRe = /<rect[^>]+>/g
  while ((m = rectRe.exec(body)) !== null) {
    const tag = m[0]
    const x  = parseFloat(tag.match(/\bx="([^"]+)"/)?.[1] ?? '0') || 0
    const y  = parseFloat(tag.match(/\by="([^"]+)"/)?.[1] ?? '0') || 0
    const w  = parseFloat(tag.match(/\bwidth="([^"]+)"/)?.[1] ?? 'NaN')
    const h  = parseFloat(tag.match(/\bheight="([^"]+)"/)?.[1] ?? 'NaN')
    if (!isNaN(w) && !isNaN(h)) {
      const rx = parseFloat(tag.match(/\brx="([^"]+)"/)?.[1] ?? '0') || 0
      if (rx > 0) {
        parts.push(
          `M ${x + rx} ${y} L ${x + w - rx} ${y} Q ${x + w} ${y} ${x + w} ${y + rx}` +
          ` L ${x + w} ${y + h - rx} Q ${x + w} ${y + h} ${x + w - rx} ${y + h}` +
          ` L ${x + rx} ${y + h} Q ${x} ${y + h} ${x} ${y + h - rx}` +
          ` L ${x} ${y + rx} Q ${x} ${y} ${x + rx} ${y} Z`,
        )
      } else {
        parts.push(`M ${x} ${y} L ${x + w} ${y} L ${x + w} ${y + h} L ${x} ${y + h} Z`)
      }
    }
  }

  // <line x1="x1" y1="y1" x2="x2" y2="y2"> → M x1 y1 L x2 y2
  const lineRe = /<line[^>]+>/g
  while ((m = lineRe.exec(body)) !== null) {
    const tag = m[0]
    const x1 = parseFloat(tag.match(/\bx1="([^"]+)"/)?.[1] ?? 'NaN')
    const y1 = parseFloat(tag.match(/\by1="([^"]+)"/)?.[1] ?? 'NaN')
    const x2 = parseFloat(tag.match(/\bx2="([^"]+)"/)?.[1] ?? 'NaN')
    const y2 = parseFloat(tag.match(/\by2="([^"]+)"/)?.[1] ?? 'NaN')
    if (!isNaN(x1) && !isNaN(y1) && !isNaN(x2) && !isNaN(y2)) {
      parts.push(`M ${x1} ${y1} L ${x2} ${y2}`)
    }
  }

  // <polyline points="x1,y1 x2,y2 ..."> → M x1 y1 L x2 y2 ...
  // <polygon points="..."> → same but closed with Z
  const polyRe = /<(polyline|polygon)([^>]+)>/g
  while ((m = polyRe.exec(body)) !== null) {
    const tag = m[0]
    const closed = m[1] === 'polygon'
    const pointsAttr = tag.match(/\bpoints="([^"]+)"/)?.[1]
    if (!pointsAttr) continue
    const coords = pointsAttr.trim().split(/[\s,]+/).map(Number)
    if (coords.length < 4 || coords.some(isNaN)) continue
    const cmds: string[] = [`M ${coords[0]} ${coords[1]}`]
    for (let i = 2; i + 1 < coords.length; i += 2) {
      cmds.push(`L ${coords[i]} ${coords[i + 1]}`)
    }
    if (closed) cmds.push('Z')
    parts.push(cmds.join(' '))
  }

  return parts.length > 0 ? parts.join(' ') : null
}

// Populate ICON_PATH_MAP with all 286 Feather icons at module load time.
// Keys are stored both in original kebab-case and normalized (no separator)
// form to match the icon resolver's name normalization.
;(function initFeatherIcons() {
  const icons = (featherData as { icons: Record<string, { body: string }> }).icons
  for (const [name, icon] of Object.entries(icons)) {
    const d = featherBodyToPathD(icon.body)
    if (!d) continue
    const iconId = `feather:${name}`
    const entry = { d, style: 'stroke' as const, iconId }
    // kebab-case key (e.g. "arrow-right") — for direct lookup in icon picker
    if (!ICON_PATH_MAP[name]) ICON_PATH_MAP[name] = entry
    // normalized key (e.g. "arrowright") — matches applyIconPathResolution normalization
    const normalized = name.replace(/-/g, '')
    if (!ICON_PATH_MAP[normalized]) ICON_PATH_MAP[normalized] = entry
  }
})()

// ---------------------------------------------------------------------------
// Pending async icon resolution tracking
// ---------------------------------------------------------------------------

/** Maps nodeId → normalized icon name for icons that need async resolution */
const pendingIconResolutions = new Map<string, string>()

/**
 * Fire an immediate icon fetch during streaming with a short timeout.
 * If the server responds in time, update the node right away and remove it
 * from pendingIconResolutions so post-streaming resolution can skip it.
 * On timeout or failure, the node stays in pendingIconResolutions as a fallback.
 */
function tryImmediateIconResolution(nodeId: string, iconName: string): void {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), 600)

  fetch(`/api/ai/icon?name=${encodeURIComponent(iconName)}`, { signal: controller.signal })
    .then((res) => (res.ok ? res.json() : null))
    .then((data) => {
      clearTimeout(timer)
      const icon = data?.icon as {
        d: string
        style: 'stroke' | 'fill'
        width: number
        height: number
        iconId?: string
      } | null
      if (!icon) return

      // Still pending (post-streaming resolution hasn't claimed it yet)?
      if (!pendingIconResolutions.has(nodeId)) return
      pendingIconResolutions.delete(nodeId)

      const { getNodeById, updateNode } = useDocumentStore.getState()
      const node = getNodeById(nodeId)
      if (!node || node.type !== 'path') return

      const update: Partial<PenNode> = { d: icon.d }
      if (icon.iconId) (update as Partial<PathNode>).iconId = icon.iconId

      const existingColor =
        extractPrimaryColor('fill' in node ? node.fill : undefined) ??
        extractPrimaryColor(node.stroke?.fill) ??
        '#64748B'

      if (icon.style === 'stroke') {
        const sw = toStrokeThicknessNumber(node.stroke, 0)
        update.stroke = { thickness: sw > 0 ? sw : 2, fill: [{ type: 'solid', color: existingColor }] }
        update.fill = []
      } else {
        update.fill = [{ type: 'solid', color: existingColor }]
        ;(update as Partial<PathNode>).stroke = undefined
      }

      updateNode(nodeId, update)
    })
    .catch(() => clearTimeout(timer))
}

/**
 * Resolve icon path nodes by their name. When the AI generates a path node
 * with a name like "SearchIcon" or "MenuIcon", look up the verified SVG path
 * from ICON_PATH_MAP and replace the d attribute.
 *
 * On local map miss for icon-like names, sets a generic placeholder and
 * records the node for async resolution via the Iconify API.
 */
export function applyIconPathResolution(node: PenNode): void {
  if (node.type !== 'path') return
  const rawName = (node.name ?? node.id ?? '').toLowerCase()
    .replace(/[-_\s]+/g, '')       // normalize separators
    .replace(/(icon|logo)$/, '')   // strip trailing "icon" or "logo"

  let match = ICON_PATH_MAP[rawName]

  if (!match) {
    // 1. Try prefix fallback: "arrowdowncircle" → "arrowdown", "shieldcheck" → "shield"
    const prefixKey = findPrefixFallback(rawName)
    if (prefixKey) match = ICON_PATH_MAP[prefixKey]
  }

  if (!match) {
    // 2. Try substring fallback: "badgecheck" → "check", "uploadcloud" → "upload"
    const substringKey = findSubstringFallback(rawName)
    if (substringKey) match = ICON_PATH_MAP[substringKey]
  }

  const originalNormalized = (node.name ?? node.id ?? '').toLowerCase().replace(/[-_\s]+/g, '')
  const queueName = rawName || originalNormalized

  if (!match) {
    // 3. Last resort: circle from Feather, queued for async.
    if (isIconLikeName(node.name ?? '', queueName)) {
      const fallback = ICON_PATH_MAP['circle'] ?? ICON_PATH_MAP['feather:circle']
      if (fallback) {
        node.d = fallback.d
        node.iconId = fallback.iconId
        applyIconStyle(node as import('@/types/pen').PathNode, fallback.style)
      }
      pendingIconResolutions.set(node.id, queueName)
      tryImmediateIconResolution(node.id, queueName)
    }
    return
  }

  // Replace with verified path data and mark as resolved icon
  node.d = match.d
  node.iconId = match.iconId ?? `feather:${rawName}`
  applyIconStyle(node, match.style)
}

const EMOJI_REGEX = /[\p{Extended_Pictographic}\p{Emoji_Presentation}\uFE0F]/gu

export function applyNoEmojiIconHeuristic(node: PenNode): void {
  if (node.type !== 'text') return
  if (typeof node.content !== 'string' || !node.content) return

  EMOJI_REGEX.lastIndex = 0
  if (!EMOJI_REGEX.test(node.content)) return
  EMOJI_REGEX.lastIndex = 0
  const cleaned = node.content.replace(EMOJI_REGEX, '').replace(/\s{2,}/g, ' ').trim()
  if (cleaned.length > 0) {
    node.content = cleaned
    return
  }

  const iconSize = clamp(toSizeNumber(node.height, toSizeNumber(node.width, node.fontSize ?? 20)), 14, 24)
  const iconFill = extractPrimaryColor('fill' in node ? node.fill : undefined) ?? '#64748B'
  const fallbackCircle = ICON_PATH_MAP['circle'] ?? ICON_PATH_MAP['feather:circle']
  const replacement: PenNode = {
    id: node.id,
    type: 'path',
    name: `${node.name ?? 'Icon'} Path`,
    d: fallbackCircle?.d ?? 'M 2 12 a 10 10 0 1 0 20 0 a 10 10 0 1 0 -20 0 Z',
    width: iconSize,
    height: iconSize,
    stroke: fallbackCircle?.style === 'stroke' ? { thickness: 2, fill: [{ type: 'solid', color: iconFill }] } : undefined,
    fill: fallbackCircle?.style === 'stroke' ? [] : [{ type: 'solid', color: iconFill }],
  } as PenNode

  if (typeof node.x === 'number') replacement.x = node.x
  if (typeof node.y === 'number') replacement.y = node.y
  if (typeof node.opacity === 'number') replacement.opacity = node.opacity
  if (typeof node.rotation === 'number') replacement.rotation = node.rotation
  replaceNode(node, replacement)
}

// ---------------------------------------------------------------------------
// Async icon resolution via Iconify API proxy
// ---------------------------------------------------------------------------

/**
 * Resolve pending icons asynchronously after streaming completes.
 * Walks the subtree rooted at `rootNodeId`, collects pending entries,
 * fetches from `/api/ai/icon` in parallel, and updates nodes in store.
 */
export async function resolveAsyncIcons(rootNodeId: string): Promise<void> {
  if (pendingIconResolutions.size === 0) return

  const { getNodeById, updateNode } = useDocumentStore.getState()

  // Collect pending entries that belong to this subtree
  const entries: Array<{ nodeId: string; iconName: string }> = []
  collectPendingInSubtree(rootNodeId, getNodeById, entries)
  if (entries.length === 0) return

  await fetchAndApplyIconResults(entries, getNodeById, updateNode)
}

/**
 * Resolve ALL pending icons regardless of which subtree they belong to.
 * Use this after non-streaming apply paths (animateNodesToCanvas, applyNodesToCanvas).
 */
export async function resolveAllPendingIcons(): Promise<void> {
  if (pendingIconResolutions.size === 0) return

  const { getNodeById, updateNode } = useDocumentStore.getState()
  const entries = Array.from(pendingIconResolutions.entries()).map(
    ([nodeId, iconName]) => ({ nodeId, iconName }),
  )

  await fetchAndApplyIconResults(entries, getNodeById, updateNode)
}

async function fetchAndApplyIconResults(
  entries: Array<{ nodeId: string; iconName: string }>,
  getNodeById: ReturnType<typeof useDocumentStore.getState>['getNodeById'],
  updateNode: ReturnType<typeof useDocumentStore.getState>['updateNode'],
): Promise<void> {
  // Fetch all in parallel
  const results = await Promise.allSettled(
    entries.map(async ({ nodeId, iconName }) => {
      const res = await fetch(`/api/ai/icon?name=${encodeURIComponent(iconName)}`)
      if (!res.ok) return { nodeId, icon: null }
      const data = (await res.json()) as {
        icon: { d: string; style: 'stroke' | 'fill'; width: number; height: number; iconId?: string } | null
      }
      return { nodeId, icon: data.icon }
    }),
  )

  // Apply resolved icons to the store
  for (const result of results) {
    if (result.status !== 'fulfilled') continue
    const { nodeId, icon } = result.value
    pendingIconResolutions.delete(nodeId)

    if (!icon) continue
    const node = getNodeById(nodeId)
    if (!node || node.type !== 'path') continue

    // Build update payload with resolved path + correct styling
    const update: Partial<PenNode> = { d: icon.d }
    if (icon.iconId) (update as Partial<PathNode>).iconId = icon.iconId
    const existingColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'

    if (icon.style === 'stroke') {
      const strokeWidth = toStrokeThicknessNumber(node.stroke, 0)
      update.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      }
      update.fill = []
    } else {
      update.fill = [{ type: 'solid', color: existingColor }]
      // Clear any stroke left over from the placeholder (brand icons are fill-only)
      ;(update as Partial<PathNode>).stroke = undefined
    }

    updateNode(nodeId, update)
  }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Check if a name looks like an icon reference (not just any path node). */
function isIconLikeName(originalName: string, normalized: string): boolean {
  // Explicit icon/logo suffix in original name
  if (/icon|logo/i.test(originalName)) return true
  // Short normalized name (likely an icon name, not a complex path description)
  if (normalized.length > 0 && normalized.length <= 30) return true
  return false
}

/** Apply stroke/fill styling to a resolved icon node (caller must ensure path type). */
function applyIconStyle(
  node: PathNode,
  style: 'stroke' | 'fill',
): void {
  if (style === 'stroke') {
    const existingColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'
    const strokeWidth = toStrokeThicknessNumber(node.stroke, 0)
    const strokeColor = extractPrimaryColor(node.stroke?.fill)
    // Ensure stroke is renderable for line icons
    if (!node.stroke || strokeWidth <= 0 || !strokeColor) {
      node.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      }
    }
    // Line icons should NOT have opaque fill (transparent to show stroke only)
    if (node.fill && node.fill.length > 0) {
      // Move fill color to stroke if stroke has no color
      const fillColor = extractPrimaryColor(node.fill)
      if (fillColor && node.stroke) {
        node.stroke.fill = [{ type: 'solid', color: fillColor }]
      }
      node.fill = []
    }
  } else {
    // Fill icons must always keep a visible fill.
    const fillColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'
    node.fill = [{ type: 'solid', color: fillColor }]
    // Remove non-renderable stroke definitions to avoid transparent-only paths.
    if (node.stroke && toStrokeThicknessNumber(node.stroke, 0) <= 0) {
      node.stroke = undefined
    }
  }
}

/** Walk subtree and collect entries from pendingIconResolutions. */
function collectPendingInSubtree(
  nodeId: string,
  getNodeById: (id: string) => PenNode | undefined,
  out: Array<{ nodeId: string; iconName: string }>,
): void {
  const iconName = pendingIconResolutions.get(nodeId)
  if (iconName) {
    out.push({ nodeId, iconName })
  }

  const node = getNodeById(nodeId)
  if (!node || !('children' in node) || !Array.isArray(node.children)) return
  for (const child of node.children) {
    collectPendingInSubtree(child.id, getNodeById, out)
  }
}

function replaceNode(target: PenNode, replacement: PenNode): void {
  const targetRecord = target as unknown as Record<string, unknown>
  for (const key of Object.keys(target)) {
    delete targetRecord[key]
  }
  Object.assign(targetRecord, replacement as unknown as Record<string, unknown>)
}

// ---------------------------------------------------------------------------
// Available icon names export — used by AI prompts to constrain icon selection
// ---------------------------------------------------------------------------

/**
 * Sorted list of all available Feather icon names (kebab-case).
 * These are guaranteed to resolve instantly without any network request.
 */
export const AVAILABLE_FEATHER_ICONS: readonly string[] = Object.keys(
  (featherData as { icons: Record<string, unknown> }).icons,
).sort()

// ---------------------------------------------------------------------------
// Built-in icon collection export — powers the "OpenPencil" picker collection
// ---------------------------------------------------------------------------

/** A single entry in the locally bundled icon collection */
export interface BuiltinIconEntry {
  /** Iconify-compatible ID, e.g. "feather:arrow-right" or "lucide:x" */
  iconId: string
  /** Icon name without the collection prefix, e.g. "arrow-right" */
  name: string
  /** Combined SVG path data (24×24 viewBox) */
  d: string
  style: 'stroke' | 'fill'
}

/**
 * All locally bundled icons, deduplicated by iconId and sorted alphabetically.
 * Covers the full Feather set (286 icons) plus hand-picked Lucide aliases.
 * These icons resolve instantly without any network request.
 */
export const BUILTIN_ICONS: readonly BuiltinIconEntry[] = (() => {
  const seen = new Set<string>()
  const entries: BuiltinIconEntry[] = []
  // Only iterate the hand-picked keys (captured before Feather expansion).
  // This excludes the full Feather set which is already a separate picker collection.
  for (const key of _handPickedKeys) {
    const entry = ICON_PATH_MAP[key]
    if (!entry) continue
    const id = entry.iconId ?? `lucide:${key}`
    if (seen.has(id)) continue
    seen.add(id)
    const name = id.includes(':') ? id.split(':')[1] : key
    entries.push({ iconId: id, name, d: entry.d, style: entry.style })
  }
  return entries.sort((a, b) => a.iconId.localeCompare(b.iconId))
})()

/**
 * Try to resolve an unknown normalized icon name by finding the longest
 * known icon key that the name starts with (prefix match, min 4 chars).
 * e.g. "arrowdowncircle" → "arrowdown", "shieldcheck" → "shield"
 */
function findPrefixFallback(normalizedName: string): string | null {
  let best: string | null = null
  let bestLen = 3 // require at least 4-char match
  for (const key of Object.keys(ICON_PATH_MAP)) {
    if (key.length > bestLen && normalizedName.startsWith(key)) {
      best = key
      bestLen = key.length
    }
  }
  return best
}

/**
 * Find the longest ICON_PATH_MAP key that appears anywhere as a substring
 * of the normalized name. E.g. "badgecheck" → "check", "uploadcloud" → "upload".
 * Only keys of at least 4 characters are considered.
 */
function findSubstringFallback(normalizedName: string): string | null {
  let best: string | null = null
  let bestLen = 3
  for (const key of Object.keys(ICON_PATH_MAP)) {
    if (key.length > bestLen && normalizedName.includes(key)) {
      best = key
      bestLen = key.length
    }
  }
  return best
}

/**
 * Look up an icon by Figma node name. Returns { d, iconId, style } if matched.
 * Uses exact match, prefix fallback, and substring fallback (same strategy as AI icon resolution).
 */
export function lookupIconByName(
  name: string,
): { d: string; iconId: string; style: 'stroke' | 'fill' } | null {
  const normalized = name
    .replace(/[-_\s/]+/g, '')
    .replace(/icon$/i, '')
    .toLowerCase()
  if (!normalized) return null

  const entry =
    ICON_PATH_MAP[normalized] ??
    ICON_PATH_MAP[findPrefixFallback(normalized) ?? ''] ??
    ICON_PATH_MAP[findSubstringFallback(normalized) ?? '']

  if (!entry) return null
  return { d: entry.d, iconId: entry.iconId, style: entry.style }
}
