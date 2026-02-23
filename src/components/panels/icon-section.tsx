import { useState, useRef, useEffect } from 'react'
import { ChevronDown } from 'lucide-react'
import type { PenNode, PathNode } from '@/types/pen'
import type { PenFill, PenStroke } from '@/types/styles'
import SectionHeader from '@/components/shared/section-header'
import IconPickerDialog, { type IconPickerPosition } from '@/components/shared/icon-picker-dialog'
import { parseSvgToNodes } from '@/utils/svg-parser'

const POPULAR_COLLECTIONS = [
  { id: 'feather', name: 'Feather' },
  { id: 'lucide', name: 'Lucide' },
  { id: 'mdi', name: 'Material' },
  { id: 'ph', name: 'Phosphor' },
  { id: 'ri', name: 'Remix' },
  { id: 'tabler', name: 'Tabler' },
  { id: 'heroicons', name: 'Heroicons' },
  { id: 'fa6-solid', name: 'Font Awesome' },
  { id: 'simple-icons', name: 'Brand Icons' },
]

interface IconSectionProps {
  node: PathNode
  onUpdate: (updates: Partial<PathNode>) => void
}

/**
 * Convert any parsed SVG node type into an SVG path `d` string.
 * SVGs can contain <circle>, <ellipse>, <rect>, <line>, <path>, etc.
 * We convert them all to path commands so they can be merged into one compound path.
 */
function nodeToPathD(node: PenNode): string | null {
  if (node.type === 'path') {
    return typeof (node as PathNode).d === 'string' ? (node as PathNode).d : null
  }
  if (node.type === 'ellipse') {
    // Center + radii from the node's bounding box position
    const rx = typeof node.width === 'number' ? node.width / 2 : 12
    const ry = typeof node.height === 'number' ? node.height / 2 : 12
    const cx = (node.x ?? 0) + rx
    const cy = (node.y ?? 0) + ry
    // Two half-arc commands form a closed ellipse/circle
    return `M ${cx - rx} ${cy} a ${rx} ${ry} 0 1 0 ${rx * 2} 0 a ${rx} ${ry} 0 1 0 ${-rx * 2} 0 Z`
  }
  if (node.type === 'rectangle') {
    const x = node.x ?? 0
    const y = node.y ?? 0
    const w = typeof node.width === 'number' ? node.width : 24
    const h = typeof node.height === 'number' ? node.height : 24
    return `M ${x} ${y} L ${x + w} ${y} L ${x + w} ${y + h} L ${x} ${y + h} Z`
  }
  if (node.type === 'line') {
    return `M ${node.x ?? 0} ${node.y ?? 0} L ${node.x2 ?? 0} ${node.y2 ?? 0}`
  }
  return null
}

export default function IconSection({ node, onUpdate }: IconSectionProps) {
  const [pickerOpen, setPickerOpen] = useState(false)
  const [pickerInitialQuery, setPickerInitialQuery] = useState('')
  const [pickerCollection, setPickerCollection] = useState<string | undefined>(undefined)
  const [pickerPosition, setPickerPosition] = useState<IconPickerPosition | undefined>(undefined)
  // Local display state for the library select — updates immediately on change
  const [displayCollection, setDisplayCollection] = useState<string>('')

  const triggerRef = useRef<HTMLButtonElement>(null)

  const iconId = node.iconId!
  const colonIdx = iconId.indexOf(':')
  const collection = colonIdx >= 0 ? iconId.slice(0, colonIdx) : ''
  const iconName = colonIdx >= 0 ? iconId.slice(colonIdx + 1) : iconId

  // Sync displayCollection when the node changes (e.g. after icon replacement)
  useEffect(() => {
    setDisplayCollection(collection)
  }, [collection])

  const openPicker = (query: string, collectionFilter?: string) => {
    if (triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect()
      setPickerPosition({ top: rect.top, right: 0 })
    }
    setPickerInitialQuery(query)
    setPickerCollection(collectionFilter)
    setPickerOpen(true)
  }

  /**
   * Parse the selected SVG and update the icon node.
   *
   * Multi-element SVGs (e.g. face icons with <circle> + <path> elements) are
   * merged into a single compound path `d` string. Non-path elements (circles,
   * rects, lines) are converted to equivalent SVG path commands so no shapes
   * are lost. Invisible bounding-box placeholders (stroke=none, fill=none) are
   * skipped. Width/height are preserved so the icon stays in its layout slot.
   */
  const handleIconSelect = (svgText: string, newIconId: string) => {
    const nodes = parseSvgToNodes(svgText)
    if (nodes.length === 0) return

    const first = nodes[0]
    let d: string
    let fill: PenFill[] | undefined = node.fill
    let stroke: PenStroke | undefined = node.stroke

    if (first.type === 'path') {
      d = first.d
      fill = first.fill
      stroke = first.stroke
    } else if (first.type === 'frame' && first.children && first.children.length > 0) {
      // Collect path `d` from ALL child element types (paths, circles, rects, lines).
      // This preserves every shape in the icon — e.g. the circle in a face emoji.
      interface Part { d: string; fill: PenFill[] | undefined; stroke: PenStroke | undefined }
      const parts: Part[] = []

      for (const child of first.children) {
        const childFill = ('fill' in child ? (child as PathNode).fill : undefined) as PenFill[] | undefined
        const childStroke = ('stroke' in child ? (child as PathNode).stroke : undefined) as PenStroke | undefined

        // Skip invisible bounding-box paths only (e.g. Tabler's <path stroke="none" d="M0 0h24v24H0z" fill="none"/>)
        // Non-path shapes (ellipse, rect, line) are never bounding boxes — always include them.
        if (child.type === 'path') {
          const isInvisible =
            childStroke === undefined &&
            (!childFill || childFill.every((f) => f.type === 'solid' && f.color === 'transparent'))
          if (isInvisible) continue
        }

        const pathD = nodeToPathD(child)
        if (pathD) parts.push({ d: pathD, fill: childFill, stroke: childStroke })
      }

      if (parts.length === 0) return
      d = parts.map((p) => p.d).join(' ')
      fill = parts[0].fill
      stroke = parts[0].stroke
    } else {
      return
    }

    // Carry over the old icon's display color to the new icon.
    // Check stroke first (stroke-based icons like Feather/Lucide), then fill.
    const oldColor = (() => {
      if (node.stroke?.fill?.[0]?.type === 'solid') {
        const c = node.stroke.fill[0].color
        if (c !== 'transparent') return c
      }
      if (node.fill?.[0]?.type === 'solid') {
        const c = node.fill[0].color
        if (c !== 'transparent') return c
      }
      return null
    })()

    if (oldColor) {
      if (stroke !== undefined) {
        stroke = { ...stroke, fill: [{ type: 'solid', color: oldColor }] }
      }
      if (fill?.some((f) => f.type === 'solid' && f.color !== 'transparent')) {
        fill = fill.map((f) =>
          f.type === 'solid' && f.color !== 'transparent' ? { ...f, color: oldColor } : f,
        )
      }
    }

    // Preserve original width/height — canvas factory scales path to fit existing bounds
    onUpdate({ d, fill, stroke, iconId: newIconId, name: newIconId })
    // Keep picker open so user can browse and try other icons
  }

  const handleLibraryChange = (newCollection: string) => {
    setDisplayCollection(newCollection)
    openPicker('', newCollection)
  }

  const handlePickerClose = () => {
    // Reset display collection to actual node collection if user cancelled
    setDisplayCollection(collection)
    setPickerOpen(false)
  }

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Icon" />

      {/* Icon name row — opens full picker */}
      <button
        ref={triggerRef}
        type="button"
        onClick={() => openPicker('')}
        className="w-full h-7 bg-secondary text-foreground text-[11px] px-2 rounded border border-transparent hover:border-input flex items-center justify-between cursor-pointer transition-colors"
      >
        <span className="truncate">{iconName}</span>
        <ChevronDown size={11} className="text-muted-foreground shrink-0 ml-1" />
      </button>

      {/* Library row — native select for quick switching */}
      <div className="relative">
        <select
          value={displayCollection}
          onChange={(e) => handleLibraryChange(e.target.value)}
          className="w-full h-7 bg-secondary text-foreground text-[11px] px-2 rounded border border-transparent hover:border-input focus:border-ring focus:outline-none cursor-pointer transition-colors appearance-none"
        >
          {!POPULAR_COLLECTIONS.some((c) => c.id === displayCollection) && displayCollection && (
            <option value={displayCollection}>{displayCollection}</option>
          )}
          {POPULAR_COLLECTIONS.map((col) => (
            <option key={col.id} value={col.id}>
              {col.name}
            </option>
          ))}
        </select>
        <ChevronDown
          size={11}
          className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground"
        />
      </div>

      <IconPickerDialog
        open={pickerOpen}
        onClose={handlePickerClose}
        onSelect={handleIconSelect}
        initialQuery={pickerInitialQuery}
        collectionFilter={pickerCollection}
        currentIconId={iconId}
        position={pickerPosition}
      />
    </div>
  )
}
