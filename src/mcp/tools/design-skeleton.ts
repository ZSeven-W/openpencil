import { openDocument, saveDocument, resolveDocPath } from '../document-manager'
import {
  getDocChildren,
  setDocChildren,
  insertNodeInTree,
  removeNodeFromTree,
} from '../utils/node-operations'
import { generateId } from '../utils/id'
import type { PenNode, ContainerProps } from '../../types/pen'
import type { PenFill } from '../../types/styles'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface DesignSkeletonParams {
  filePath?: string
  rootFrame: {
    name?: string
    width: number
    height: number
    layout?: 'vertical' | 'horizontal'
    gap?: number
    fill?: PenFill[]
    padding?: number | [number, number] | [number, number, number, number]
  }
  sections: Array<{
    name: string
    height?: number
    layout?: 'vertical' | 'horizontal'
    padding?: number | [number, number] | [number, number, number, number]
    gap?: number
    fill?: PenFill[]
    role?: string
    justifyContent?: string
    alignItems?: string
  }>
  styleGuide?: {
    palette?: Record<string, string>
    fonts?: { heading?: string; body?: string }
    aesthetic?: string
  }
  canvasWidth?: number
  pageId?: string
}

interface SectionResult {
  id: string
  name: string
  contentWidth: number
  guidelines: string
  suggestedRoles: string[]
}

interface DesignSkeletonResult {
  rootId: string
  sections: SectionResult[]
  nextSteps: string
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

export async function handleDesignSkeleton(
  params: DesignSkeletonParams,
): Promise<DesignSkeletonResult> {
  const filePath = resolveDocPath(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)
  const pageId = params.pageId
  const canvasWidth = params.canvasWidth ?? params.rootFrame.width ?? 1200

  // Build root frame node
  const rootId = generateId()
  const rootNode: PenNode = {
    id: rootId,
    type: 'frame',
    name: params.rootFrame.name ?? 'Page',
    width: params.rootFrame.width,
    height: params.rootFrame.height,
    layout: params.rootFrame.layout ?? 'vertical',
    gap: params.rootFrame.gap ?? 0,
    fill: params.rootFrame.fill ?? [{ type: 'solid', color: '#F8FAFC' }],
    children: [],
  } as PenNode
  if (params.rootFrame.padding !== undefined) {
    ;(rootNode as PenNode & ContainerProps).padding = params.rootFrame.padding
  }

  // Build section frames as children
  const sectionResults: SectionResult[] = []
  const rootChildren: PenNode[] = []

  for (const section of params.sections) {
    const sectionId = generateId()
    const sectionNode: PenNode = {
      id: sectionId,
      type: 'frame',
      name: section.name,
      width: 'fill_container',
      height: section.height ?? 'fit_content',
      layout: section.layout ?? 'vertical',
      children: [],
    } as PenNode

    const sContainer = sectionNode as PenNode & ContainerProps
    if (section.gap !== undefined) sContainer.gap = section.gap
    if (section.padding !== undefined) sContainer.padding = section.padding
    if (section.fill) sContainer.fill = section.fill
    if (section.role) sectionNode.role = section.role
    if (section.justifyContent) sContainer.justifyContent = section.justifyContent as ContainerProps['justifyContent']
    if (section.alignItems) sContainer.alignItems = section.alignItems as ContainerProps['alignItems']

    rootChildren.push(sectionNode)

    // Compute content width for this section
    const contentWidth = computeContentWidth(sectionNode, canvasWidth)
    const { guidelines, suggestedRoles } = generateSectionGuidelines(
      section,
      contentWidth,
      canvasWidth,
      params.styleGuide,
    )

    sectionResults.push({
      id: sectionId,
      name: section.name,
      contentWidth,
      guidelines,
      suggestedRoles,
    })
  }

  ;(rootNode as PenNode & ContainerProps).children = rootChildren

  // Auto-replace empty root frame if exists
  const children = getDocChildren(doc, pageId)
  const emptyIdx = children.findIndex((n) => isEmptyFrame(n))
  if (emptyIdx !== -1) {
    const emptyFrame = children[emptyIdx]
    if (emptyFrame.x !== undefined) rootNode.x = emptyFrame.x
    if (emptyFrame.y !== undefined) rootNode.y = emptyFrame.y
    let updated = removeNodeFromTree(children, emptyFrame.id)
    updated = insertNodeInTree(updated, null, rootNode, emptyIdx)
    setDocChildren(doc, updated, pageId)
  } else {
    setDocChildren(doc, insertNodeInTree(children, null, rootNode), pageId)
  }

  await saveDocument(filePath, doc)

  return {
    rootId,
    sections: sectionResults,
    nextSteps:
      `Skeleton created with ${sectionResults.length} sections. ` +
      `For each section, call design_content with the sectionId and an array of child nodes. ` +
      `After all sections are populated, call design_refine with rootId="${rootId}" to run full-tree validation.`,
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isEmptyFrame(node: PenNode): boolean {
  return (
    node.type === 'frame' &&
    (!('children' in node) || !node.children || node.children.length === 0)
  )
}

function computeContentWidth(section: PenNode, canvasWidth: number): number {
  const pad = parsePadding(
    'padding' in section ? (section as PenNode & ContainerProps).padding as number | [number, number] | [number, number, number, number] | undefined : undefined,
  )
  return canvasWidth - pad.left - pad.right
}

function parsePadding(
  padding: number | [number, number] | [number, number, number, number] | undefined,
): { top: number; right: number; bottom: number; left: number } {
  if (padding === undefined) return { top: 0, right: 0, bottom: 0, left: 0 }
  if (typeof padding === 'number')
    return { top: padding, right: padding, bottom: padding, left: padding }
  if (padding.length === 2)
    return { top: padding[0], right: padding[1], bottom: padding[0], left: padding[1] }
  if (padding.length === 4)
    return { top: padding[0], right: padding[1], bottom: padding[2], left: padding[3] }
  return { top: 0, right: 0, bottom: 0, left: 0 }
}

// ---------------------------------------------------------------------------
// Per-section content guidelines generator
// ---------------------------------------------------------------------------

function generateSectionGuidelines(
  section: { name: string; layout?: string; role?: string },
  contentWidth: number,
  canvasWidth: number,
  styleGuide?: DesignSkeletonParams['styleGuide'],
): { guidelines: string; suggestedRoles: string[] } {
  const name = section.name.toLowerCase()
  const isMobile = canvasWidth <= 500
  const palette = styleGuide?.palette
  const accentColor = palette?.accent ?? '#2563EB'

  // Navigation
  if (name.includes('nav') || section.role === 'navbar') {
    return {
      guidelines:
        `Horizontal layout with 3 child groups: logo frame, nav-links frame, CTA button. ` +
        `Use justifyContent="space_between", alignItems="center". ` +
        `Logo as text (fontSize 18-20, fontWeight 700) or frame with icon. ` +
        `Nav links: horizontal frame with gap=${isMobile ? 16 : 32}, each link as text node. ` +
        `CTA: button with accent fill [{"type":"solid","color":"${accentColor}"}], white text. ` +
        `Content width: ${contentWidth}px.`,
      suggestedRoles: ['navbar', 'nav-links', 'nav-link', 'button', 'label', 'icon'],
    }
  }

  // Hero
  if (name.includes('hero') || section.role === 'hero') {
    return {
      guidelines:
        `Large headline (${isMobile ? '28-36' : '40-56'}px, fontWeight 700), subtitle (16-18px, secondary color), CTA button(s). ` +
        (isMobile
          ? `Stack vertically with gap 16-24. Center-align content.`
          : `For desktop with phone mockup: two-column horizontal layout (left text, right phone). ` +
            `Without mockup: center-aligned vertical stack.`) +
        ` Use gap=24 between elements. Headline text: textGrowth="fixed-width" if >15 chars. ` +
        `Content width: ${contentWidth}px.`,
      suggestedRoles: ['hero', 'heading', 'subheading', 'body-text', 'button', 'phone-mockup', 'row'],
    }
  }

  // Features / Feature Cards
  if (name.includes('feature') || name.includes('功能')) {
    return {
      guidelines:
        `Section title (heading, 28-36px) + subtitle, then ${isMobile ? '2-3' : '3-4'} feature cards in a ${isMobile ? 'vertical' : 'horizontal'} layout. ` +
        `Each card: frame with role="feature-card", containing icon (path 20-24px), title (text 18-20px), description (text 14-16px). ` +
        `Cards in horizontal row: ALL must use width="fill_container" + height="fill_container". ` +
        `Use gap=${isMobile ? 16 : 24} between cards. clipContent=true + cornerRadius=12 on cards. ` +
        `Content width: ${contentWidth}px.`,
      suggestedRoles: ['section', 'heading', 'subheading', 'feature-card', 'feature-grid', 'icon', 'body-text'],
    }
  }

  // Footer
  if (name.includes('footer') || section.role === 'footer') {
    return {
      guidelines:
        `${isMobile ? 'Vertical stack' : 'Horizontal layout with 3-4 column groups'}: logo+tagline, navigation links grouped by category, social icons. ` +
        `Use muted text colors for secondary content. Add a divider (height=1, fill border color) above footer if needed. ` +
        `Bottom row: copyright text, small links. Content width: ${contentWidth}px.`,
      suggestedRoles: ['footer', 'row', 'column', 'nav-links', 'label', 'caption', 'divider', 'icon'],
    }
  }

  // CTA / Call to Action
  if (name.includes('cta') || name.includes('call to action') || section.role === 'cta-section') {
    return {
      guidelines:
        `Centered content: bold headline (28-36px), short subtitle, prominent CTA button. ` +
        `Use accent background or gradient for visual distinction. ` +
        `Button: large (padding [16, 40]), contrasting color, cornerRadius 8-12. ` +
        `Content width: ${contentWidth}px.`,
      suggestedRoles: ['cta-section', 'heading', 'subheading', 'button', 'centered-content'],
    }
  }

  // Testimonials
  if (name.includes('testimonial') || name.includes('review') || name.includes('评价')) {
    return {
      guidelines:
        `Section title + ${isMobile ? '1-2' : '2-3'} testimonial cards in ${isMobile ? 'vertical' : 'horizontal'} layout. ` +
        `Each card: quote text (italic or normal, 14-16px), author name, author title/company. ` +
        `Optional: avatar (circle, 48px), star rating (5 star icons). ` +
        `Cards in horizontal: width="fill_container" + height="fill_container". Content width: ${contentWidth}px.`,
      suggestedRoles: ['section', 'card', 'heading', 'body-text', 'caption', 'avatar', 'row'],
    }
  }

  // Pricing
  if (name.includes('pricing') || name.includes('价格') || name.includes('plan')) {
    return {
      guidelines:
        `Section title + ${isMobile ? '1-2' : '2-3'} pricing cards in ${isMobile ? 'vertical' : 'horizontal'} layout. ` +
        `Each card: plan name, price (large text 36-48px), feature list (each item with check icon + text), CTA button. ` +
        `Highlight the recommended plan with accent border or fill. ` +
        `Cards in horizontal: width="fill_container" + height="fill_container". Content width: ${contentWidth}px.`,
      suggestedRoles: ['section', 'pricing-card', 'heading', 'label', 'body-text', 'button', 'icon', 'divider'],
    }
  }

  // Stats
  if (name.includes('stat') || name.includes('数据') || name.includes('metric')) {
    return {
      guidelines:
        `${isMobile ? '2x2 grid' : '3-4 stat cards in horizontal'} layout. ` +
        `Each stat: large number (fontSize 36-48px, fontWeight 700), label text (14px, secondary color). ` +
        `Optional: icon or trend indicator. Cards: width="fill_container" + height="fill_container". ` +
        `Content width: ${contentWidth}px.`,
      suggestedRoles: ['stats-section', 'stat-card', 'heading', 'caption', 'icon', 'row'],
    }
  }

  // Form / Login / Signup
  if (
    name.includes('form') || name.includes('login') || name.includes('signup') ||
    name.includes('register') || name.includes('表单') || name.includes('登录') || name.includes('注册')
  ) {
    return {
      guidelines:
        `Vertical layout with gap=16-20. ALL inputs MUST use width="fill_container". ` +
        `Input fields: frame with role="form-input", height=48, light bg, subtle border. ` +
        `Include placeholder text nodes inside inputs. ` +
        `Submit button: width="fill_container", height=48, accent fill, white text. ` +
        `Keep form elements (inputs + submit button) together — do NOT split. ` +
        `Optional: social login buttons (horizontal frame, each width="fit_content"). ` +
        `Content width: ${contentWidth}px.`,
      suggestedRoles: ['form-group', 'form-input', 'input', 'button', 'label', 'caption', 'divider', 'icon'],
    }
  }

  // Header (app screens)
  if (name.includes('header') || name.includes('顶部')) {
    return {
      guidelines:
        `Horizontal layout with justifyContent="space_between", alignItems="center". ` +
        `Left: back icon or menu icon. Center: title text. Right: action icon(s). ` +
        `Height: ${isMobile ? 56 : 64}px. Content width: ${contentWidth}px.`,
      suggestedRoles: ['row', 'heading', 'icon-button', 'icon', 'label'],
    }
  }

  // Sidebar
  if (name.includes('sidebar') || name.includes('侧边栏')) {
    return {
      guidelines:
        `Vertical layout with gap=4-8. Fixed width (240-280px). ` +
        `Items: horizontal frame with icon (20px) + text label, padding=[8,16], gap=12, alignItems="center". ` +
        `Active item: accent fill or left border indicator. ` +
        `Group labels: uppercase caption text, letterSpacing=1-2. Content width: ${contentWidth}px.`,
      suggestedRoles: ['column', 'nav-link', 'icon', 'label', 'caption', 'divider', 'heading'],
    }
  }

  // Default / generic section
  return {
    guidelines:
      `Vertical layout section. Content should be wrapped in a centered content frame if desktop. ` +
      `Use heading (28-36px) for section title, body-text (16px) for descriptions. ` +
      `All text >15 chars: textGrowth="fixed-width" + width="fill_container". ` +
      `Content width: ${contentWidth}px.`,
    suggestedRoles: ['section', 'heading', 'subheading', 'body-text', 'button', 'row', 'card'],
  }
}
