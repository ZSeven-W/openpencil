/**
 * 5 essential content templates for LinkedIn V1.
 *
 * Each template is a PenNode tree with ALL styling bound to $variable refs.
 * Templates are format-agnostic — they use fill_container sizing so they
 * reflow to any FormatPreset dimensions.
 */

import { nanoid } from 'nanoid'
import type { PenNode, FrameNode, TextNode, RectangleNode } from '@/types/pen'

type ContentType = 'slide' | 'post' | 'video-frame'

export interface TemplateDefinition {
  id: string
  name: string
  description: string
  contentType: ContentType
  supportedFormats: string[]
  /** Factory that produces a fresh PenNode tree with unique IDs */
  create: () => FrameNode
}

function text(overrides: Partial<TextNode> & { content: string }): TextNode {
  return {
    id: nanoid(),
    type: 'text',
    textGrowth: 'fixed-width',
    width: 'fill_container',
    ...overrides,
  }
}

function frame(overrides: Partial<FrameNode> & { children: PenNode[] }): FrameNode {
  return {
    id: nanoid(),
    type: 'frame',
    ...overrides,
  }
}

function rect(overrides: Partial<RectangleNode>): RectangleNode {
  return {
    id: nanoid(),
    type: 'rectangle',
    ...overrides,
  }
}

// ---------------------------------------------------------------------------
// Template 1: Title / Intro
// ---------------------------------------------------------------------------

const titleIntro: TemplateDefinition = {
  id: 'tpl-title-intro',
  name: 'Title / Intro',
  description: 'Large heading, subtitle, author name with background fill',
  contentType: 'slide',
  supportedFormats: ['linkedin-carousel', 'linkedin-video', 'linkedin-post'],
  create: () =>
    frame({
      name: 'Title Slide',
      reusable: true,
      width: 'fill_container',
      height: 'fill_container',
      layout: 'vertical',
      justifyContent: 'center',
      alignItems: 'center',
      padding: '$space-xl',
      gap: '$space-md',
      fill: [{ type: 'solid', color: '$color-bg' }],
      children: [
        text({
          name: 'Eyebrow',
          content: 'YOUR TOPIC',
          fontFamily: '$font-body',
          fontSize: '$size-eyebrow' as unknown as number,
          letterSpacing: 2 as unknown as number,
          fill: [{ type: 'solid', color: '$color-text-muted' }],
        }),
        text({
          name: 'Title',
          content: 'Your Main Headline Goes Here',
          fontFamily: '$font-heading',
          fontSize: '$size-heading-xl' as unknown as number,
          fontWeight: 700,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-text' }],
        }),
        text({
          name: 'Subtitle',
          content: 'A brief description of what this carousel covers',
          fontFamily: '$font-body',
          fontSize: '$size-body' as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-text-muted' }],
        }),
        rect({
          name: 'Divider',
          width: 60,
          height: '$stroke-decorative' as unknown as number,
          fill: [{ type: 'solid', color: '$color-primary' }],
        }),
        text({
          name: 'Author',
          content: 'Your Name',
          fontFamily: '$font-body',
          fontSize: '$size-caption' as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-text-muted' }],
        }),
      ],
    }),
}

// ---------------------------------------------------------------------------
// Template 2: Content
// ---------------------------------------------------------------------------

const content: TemplateDefinition = {
  id: 'tpl-content',
  name: 'Content',
  description: 'Heading plus body text with optional emphasis',
  contentType: 'slide',
  supportedFormats: ['linkedin-carousel', 'linkedin-video', 'linkedin-post'],
  create: () =>
    frame({
      name: 'Content Slide',
      reusable: true,
      width: 'fill_container',
      height: 'fill_container',
      layout: 'vertical',
      justifyContent: 'center',
      padding: '$space-xl',
      gap: '$space-lg',
      fill: [{ type: 'solid', color: '$color-bg' }],
      children: [
        text({
          name: 'Slide Number',
          content: '01',
          fontFamily: '$font-mono',
          fontSize: '$size-caption' as unknown as number,
          fill: [{ type: 'solid', color: '$color-primary' }],
        }),
        text({
          name: 'Heading',
          content: 'Key Point Title',
          fontFamily: '$font-heading',
          fontSize: '$size-heading-lg' as unknown as number,
          fontWeight: 700,
          fill: [{ type: 'solid', color: '$color-text' }],
        }),
        text({
          name: 'Body',
          content: 'Explain your key point here. Keep it concise and actionable. Use short paragraphs for readability on LinkedIn.',
          fontFamily: '$font-body',
          fontSize: '$size-body' as unknown as number,
          lineHeight: 1.6 as unknown as number,
          fill: [{ type: 'solid', color: '$color-text' }],
        }),
        frame({
          name: 'Highlight Box',
          width: 'fill_container',
          layout: 'horizontal',
          padding: '$space-md',
          gap: '$space-sm',
          cornerRadius: '$size-radius-md' as unknown as number,
          fill: [{ type: 'solid', color: '$color-surface' }],
          children: [
            rect({
              name: 'Accent Bar',
              width: 4,
              height: 'fill_container',
              fill: [{ type: 'solid', color: '$color-accent' }],
            }),
            text({
              name: 'Highlight Text',
              content: 'A key takeaway or important note to remember.',
              fontFamily: '$font-body',
              fontSize: '$size-body' as unknown as number,
              fontStyle: 'italic',
              fill: [{ type: 'solid', color: '$color-text' }],
            }),
          ],
        }),
      ],
    }),
}

// ---------------------------------------------------------------------------
// Template 3: Quote
// ---------------------------------------------------------------------------

const quote: TemplateDefinition = {
  id: 'tpl-quote',
  name: 'Quote',
  description: 'Large pull quote with attribution',
  contentType: 'slide',
  supportedFormats: ['linkedin-carousel', 'linkedin-video', 'linkedin-post'],
  create: () =>
    frame({
      name: 'Quote Slide',
      reusable: true,
      width: 'fill_container',
      height: 'fill_container',
      layout: 'vertical',
      justifyContent: 'center',
      alignItems: 'center',
      padding: '$space-xl',
      gap: '$space-lg',
      fill: [{ type: 'solid', color: '$color-surface' }],
      children: [
        text({
          name: 'Quote Mark',
          content: '\u201C',
          fontFamily: '$font-editorial',
          fontSize: 120 as unknown as number,
          lineHeight: 0.8 as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-primary' }],
        }),
        text({
          name: 'Quote Text',
          content: 'Insert a powerful, thought-provoking quote that resonates with your audience.',
          fontFamily: '$font-editorial',
          fontSize: '$size-heading-md' as unknown as number,
          textAlign: 'center',
          lineHeight: 1.4 as unknown as number,
          fill: [{ type: 'solid', color: '$color-text' }],
        }),
        rect({
          name: 'Divider',
          width: 40,
          height: 2,
          fill: [{ type: 'solid', color: '$color-border' }],
        }),
        text({
          name: 'Attribution',
          content: '— Author Name',
          fontFamily: '$font-body',
          fontSize: '$size-caption' as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-text-muted' }],
        }),
      ],
    }),
}

// ---------------------------------------------------------------------------
// Template 4: Stat / Metric
// ---------------------------------------------------------------------------

const statMetric: TemplateDefinition = {
  id: 'tpl-stat-metric',
  name: 'Stat / Metric',
  description: 'Big number with label and supporting text',
  contentType: 'slide',
  supportedFormats: ['linkedin-carousel', 'linkedin-video', 'linkedin-post'],
  create: () =>
    frame({
      name: 'Stat Slide',
      reusable: true,
      width: 'fill_container',
      height: 'fill_container',
      layout: 'vertical',
      justifyContent: 'center',
      alignItems: 'center',
      padding: '$space-xl',
      gap: '$space-md',
      fill: [{ type: 'solid', color: '$color-primary' }],
      children: [
        text({
          name: 'Label',
          content: 'KEY METRIC',
          fontFamily: '$font-body',
          fontSize: '$size-eyebrow' as unknown as number,
          letterSpacing: 2 as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-bg' }],
          opacity: 0.8,
        }),
        text({
          name: 'Number',
          content: '73%',
          fontFamily: '$font-heading',
          fontSize: 96 as unknown as number,
          fontWeight: 800,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-bg' }],
        }),
        text({
          name: 'Description',
          content: 'of professionals agree this metric matters',
          fontFamily: '$font-body',
          fontSize: '$size-heading-sm' as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-bg' }],
          opacity: 0.9,
        }),
        text({
          name: 'Source',
          content: 'Source: Industry Report 2026',
          fontFamily: '$font-body',
          fontSize: '$size-caption' as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-bg' }],
          opacity: 0.6,
        }),
      ],
    }),
}

// ---------------------------------------------------------------------------
// Template 5: CTA / Closing
// ---------------------------------------------------------------------------

const ctaClosing: TemplateDefinition = {
  id: 'tpl-cta-closing',
  name: 'CTA / Closing',
  description: 'Call-to-action with author bio and social handles',
  contentType: 'slide',
  supportedFormats: ['linkedin-carousel', 'linkedin-video', 'linkedin-post'],
  create: () =>
    frame({
      name: 'CTA Slide',
      reusable: true,
      width: 'fill_container',
      height: 'fill_container',
      layout: 'vertical',
      justifyContent: 'center',
      alignItems: 'center',
      padding: '$space-xl',
      gap: '$space-lg',
      fill: [{ type: 'solid', color: '$color-bg' }],
      children: [
        text({
          name: 'CTA Heading',
          content: 'Found this useful?',
          fontFamily: '$font-heading',
          fontSize: '$size-heading-lg' as unknown as number,
          fontWeight: 700,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-text' }],
        }),
        text({
          name: 'CTA Body',
          content: 'Follow for more insights on this topic. Like & share to help others discover this.',
          fontFamily: '$font-body',
          fontSize: '$size-body' as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-text-muted' }],
        }),
        frame({
          name: 'CTA Button',
          layout: 'horizontal',
          justifyContent: 'center',
          alignItems: 'center',
          padding: [12, 32, 12, 32],
          cornerRadius: '$size-radius-md' as unknown as number,
          fill: [{ type: 'solid', color: '$color-primary' }],
          children: [
            text({
              name: 'Button Text',
              content: 'Follow Me',
              fontFamily: '$font-body',
              fontSize: '$size-body' as unknown as number,
              fontWeight: 600,
              width: 'fit_content',
              fill: [{ type: 'solid', color: '$color-bg' }],
            }),
          ],
        }),
        rect({
          name: 'Divider',
          width: 'fill_container' as unknown as number,
          height: 1,
          fill: [{ type: 'solid', color: '$color-border' }],
        }),
        text({
          name: 'Author Bio',
          content: 'Your Name | Your Title\nyourname.com',
          fontFamily: '$font-body',
          fontSize: '$size-caption' as unknown as number,
          textAlign: 'center',
          fill: [{ type: 'solid', color: '$color-text-muted' }],
        }),
      ],
    }),
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

export const CONTENT_TEMPLATES: TemplateDefinition[] = [
  titleIntro,
  content,
  quote,
  statMetric,
  ctaClosing,
]

/** Look up a template by ID. */
export function getTemplateById(id: string): TemplateDefinition | undefined {
  return CONTENT_TEMPLATES.find((t) => t.id === id)
}
