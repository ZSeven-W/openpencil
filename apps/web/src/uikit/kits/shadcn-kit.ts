import type { PenDocument, PenNode } from '@/types/pen'
import { extraComponents } from './shadcn-kit-extra'

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

const btnPrimary: PenNode = {
  id: 'shadcn-btn-primary',
  type: 'frame',
  name: 'Primary Button',
  reusable: true,
  x: 0,
  y: 0,
  width: 120,
  height: 40,
  layout: 'horizontal',
  justifyContent: 'center',
  alignItems: 'center',
  padding: [0, 20, 0, 20],
  cornerRadius: 8,
  fill: [{ type: 'solid', color: '#18181B' }],
  children: [
    {
      id: 'shadcn-btn-primary-label',
      type: 'text',
      name: 'Label',
      content: 'Button',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#FAFAFA' }],
    },
  ],
}

const btnSecondary: PenNode = {
  id: 'shadcn-btn-secondary',
  type: 'frame',
  name: 'Secondary Button',
  reusable: true,
  x: 140,
  y: 0,
  width: 120,
  height: 40,
  layout: 'horizontal',
  justifyContent: 'center',
  alignItems: 'center',
  padding: [0, 20, 0, 20],
  cornerRadius: 8,
  fill: [{ type: 'solid', color: '#F4F4F5' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  children: [
    {
      id: 'shadcn-btn-secondary-label',
      type: 'text',
      name: 'Label',
      content: 'Button',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
  ],
}

const btnGhost: PenNode = {
  id: 'shadcn-btn-ghost',
  type: 'frame',
  name: 'Ghost Button',
  reusable: true,
  x: 280,
  y: 0,
  width: 120,
  height: 40,
  layout: 'horizontal',
  justifyContent: 'center',
  alignItems: 'center',
  padding: [0, 20, 0, 20],
  cornerRadius: 8,
  children: [
    {
      id: 'shadcn-btn-ghost-label',
      type: 'text',
      name: 'Label',
      content: 'Button',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
  ],
}

const btnDestructive: PenNode = {
  id: 'shadcn-btn-destructive',
  type: 'frame',
  name: 'Destructive Button',
  reusable: true,
  x: 420,
  y: 0,
  width: 120,
  height: 40,
  layout: 'horizontal',
  justifyContent: 'center',
  alignItems: 'center',
  padding: [0, 20, 0, 20],
  cornerRadius: 8,
  fill: [{ type: 'solid', color: '#EF4444' }],
  children: [
    {
      id: 'shadcn-btn-destructive-label',
      type: 'text',
      name: 'Label',
      content: 'Delete',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#FAFAFA' }],
    },
  ],
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

const inputText: PenNode = {
  id: 'shadcn-input-text',
  type: 'frame',
  name: 'Text Input',
  reusable: true,
  x: 0,
  y: 60,
  width: 240,
  height: 40,
  layout: 'horizontal',
  alignItems: 'center',
  padding: [0, 12, 0, 12],
  cornerRadius: 8,
  fill: [{ type: 'solid', color: '#FFFFFF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  children: [
    {
      id: 'shadcn-input-text-placeholder',
      type: 'text',
      name: 'Placeholder',
      content: 'Enter text...',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#A1A1AA' }],
    },
  ],
}

const inputTextarea: PenNode = {
  id: 'shadcn-input-textarea',
  type: 'frame',
  name: 'Textarea',
  reusable: true,
  x: 260,
  y: 60,
  width: 240,
  height: 96,
  layout: 'vertical',
  padding: 12,
  cornerRadius: 8,
  fill: [{ type: 'solid', color: '#FFFFFF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  children: [
    {
      id: 'shadcn-input-textarea-placeholder',
      type: 'text',
      name: 'Placeholder',
      content: 'Enter description...',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#A1A1AA' }],
      width: 'fill_container',
    },
  ],
}

const inputCheckbox: PenNode = {
  id: 'shadcn-input-checkbox',
  type: 'frame',
  name: 'Checkbox',
  reusable: true,
  x: 0,
  y: 176,
  width: 160,
  height: 24,
  layout: 'horizontal',
  gap: 8,
  alignItems: 'center',
  children: [
    {
      id: 'shadcn-input-checkbox-box',
      type: 'rectangle',
      name: 'Box',
      width: 18,
      height: 18,
      cornerRadius: 4,
      fill: [{ type: 'solid', color: '#18181B' }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#18181B' }] },
    },
    {
      id: 'shadcn-input-checkbox-label',
      type: 'text',
      name: 'Label',
      content: 'Checkbox label',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
  ],
}

const inputToggle: PenNode = {
  id: 'shadcn-input-toggle',
  type: 'frame',
  name: 'Toggle Switch',
  reusable: true,
  x: 180,
  y: 176,
  width: 44,
  height: 24,
  cornerRadius: 12,
  fill: [{ type: 'solid', color: '#18181B' }],
  children: [
    {
      id: 'shadcn-input-toggle-thumb',
      type: 'ellipse',
      name: 'Thumb',
      x: 22,
      y: 2,
      width: 20,
      height: 20,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    },
  ],
}

const inputRadio: PenNode = {
  id: 'shadcn-input-radio',
  type: 'frame',
  name: 'Radio Button',
  reusable: true,
  x: 240,
  y: 176,
  width: 160,
  height: 24,
  layout: 'horizontal',
  gap: 8,
  alignItems: 'center',
  children: [
    {
      id: 'shadcn-input-radio-circle',
      type: 'ellipse',
      name: 'Circle',
      width: 18,
      height: 18,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      stroke: { thickness: 2, fill: [{ type: 'solid', color: '#18181B' }] },
    },
    {
      id: 'shadcn-input-radio-label',
      type: 'text',
      name: 'Label',
      content: 'Radio label',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
  ],
}

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

const cardBasic: PenNode = {
  id: 'shadcn-card-basic',
  type: 'frame',
  name: 'Basic Card',
  reusable: true,
  x: 0,
  y: 220,
  width: 280,
  height: 160,
  layout: 'vertical',
  gap: 8,
  padding: 20,
  cornerRadius: 12,
  fill: [{ type: 'solid', color: '#FFFFFF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  effects: [{ type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: 'rgba(0,0,0,0.05)' }],
  children: [
    {
      id: 'shadcn-card-basic-title',
      type: 'text',
      name: 'Title',
      content: 'Card Title',
      fontSize: 18,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
    {
      id: 'shadcn-card-basic-desc',
      type: 'text',
      name: 'Description',
      content: 'Card description goes here with some supporting text.',
      fontSize: 14,
      lineHeight: 1.5,
      fill: [{ type: 'solid', color: '#71717A' }],
      width: 'fill_container',
    },
  ],
}

const cardStats: PenNode = {
  id: 'shadcn-card-stats',
  type: 'frame',
  name: 'Stats Card',
  reusable: true,
  x: 300,
  y: 220,
  width: 200,
  height: 120,
  layout: 'vertical',
  gap: 4,
  padding: 20,
  cornerRadius: 12,
  fill: [{ type: 'solid', color: '#FFFFFF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  effects: [{ type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: 'rgba(0,0,0,0.05)' }],
  children: [
    {
      id: 'shadcn-card-stats-label',
      type: 'text',
      name: 'Label',
      content: 'Total Revenue',
      fontSize: 12,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#71717A' }],
    },
    {
      id: 'shadcn-card-stats-value',
      type: 'text',
      name: 'Value',
      content: '$45,231',
      fontSize: 28,
      fontWeight: 700,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
    {
      id: 'shadcn-card-stats-change',
      type: 'text',
      name: 'Change',
      content: '+20.1% from last month',
      fontSize: 12,
      fill: [{ type: 'solid', color: '#16A34A' }],
    },
  ],
}

const cardImage: PenNode = {
  id: 'shadcn-card-image',
  type: 'frame',
  name: 'Image Card',
  reusable: true,
  x: 520,
  y: 220,
  width: 280,
  height: 240,
  layout: 'vertical',
  cornerRadius: 12,
  fill: [{ type: 'solid', color: '#FFFFFF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  effects: [{ type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: 'rgba(0,0,0,0.05)' }],
  children: [
    {
      id: 'shadcn-card-image-placeholder',
      type: 'rectangle',
      name: 'Image',
      width: 'fill_container',
      height: 140,
      fill: [{ type: 'solid', color: '#F4F4F5' }],
      cornerRadius: [12, 12, 0, 0],
    },
    {
      id: 'shadcn-card-image-body',
      type: 'frame',
      name: 'Body',
      width: 'fill_container',
      layout: 'vertical',
      gap: 4,
      padding: [12, 16, 16, 16],
      children: [
        {
          id: 'shadcn-card-image-title',
          type: 'text',
          name: 'Title',
          content: 'Card Title',
          fontSize: 16,
          fontWeight: 600,
          fill: [{ type: 'solid', color: '#18181B' }],
        },
        {
          id: 'shadcn-card-image-desc',
          type: 'text',
          name: 'Description',
          content: 'Brief description text.',
          fontSize: 13,
          fill: [{ type: 'solid', color: '#71717A' }],
        },
      ],
    },
  ],
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

const navbar: PenNode = {
  id: 'shadcn-nav-bar',
  type: 'frame',
  name: 'Navbar',
  reusable: true,
  x: 0,
  y: 480,
  width: 800,
  height: 56,
  layout: 'horizontal',
  alignItems: 'center',
  justifyContent: 'space_between',
  padding: [0, 24, 0, 24],
  fill: [{ type: 'solid', color: '#FFFFFF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  children: [
    {
      id: 'shadcn-nav-bar-brand',
      type: 'text',
      name: 'Brand',
      content: 'Brand',
      fontSize: 18,
      fontWeight: 700,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
    {
      id: 'shadcn-nav-bar-links',
      type: 'frame',
      name: 'Links',
      layout: 'horizontal',
      gap: 24,
      alignItems: 'center',
      children: [
        {
          id: 'shadcn-nav-bar-link-1',
          type: 'text',
          name: 'Link',
          content: 'Home',
          fontSize: 14,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#18181B' }],
        },
        {
          id: 'shadcn-nav-bar-link-2',
          type: 'text',
          name: 'Link',
          content: 'Products',
          fontSize: 14,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#71717A' }],
        },
        {
          id: 'shadcn-nav-bar-link-3',
          type: 'text',
          name: 'Link',
          content: 'About',
          fontSize: 14,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#71717A' }],
        },
      ],
    },
  ],
}

const tabBar: PenNode = {
  id: 'shadcn-tab-bar',
  type: 'frame',
  name: 'Tab Bar',
  reusable: true,
  x: 0,
  y: 556,
  width: 400,
  height: 40,
  layout: 'horizontal',
  gap: 0,
  alignItems: 'center',
  fill: [{ type: 'solid', color: '#FFFFFF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  cornerRadius: 8,
  children: [
    {
      id: 'shadcn-tab-bar-tab-1',
      type: 'frame',
      name: 'Tab Active',
      height: 'fill_container',
      width: 'fill_container',
      layout: 'horizontal',
      justifyContent: 'center',
      alignItems: 'center',
      fill: [{ type: 'solid', color: '#18181B' }],
      cornerRadius: [8, 0, 0, 8],
      children: [
        {
          id: 'shadcn-tab-bar-tab-1-label',
          type: 'text',
          name: 'Label',
          content: 'Tab 1',
          fontSize: 13,
          fontWeight: 600,
          fill: [{ type: 'solid', color: '#FAFAFA' }],
        },
      ],
    },
    {
      id: 'shadcn-tab-bar-tab-2',
      type: 'frame',
      name: 'Tab',
      height: 'fill_container',
      width: 'fill_container',
      layout: 'horizontal',
      justifyContent: 'center',
      alignItems: 'center',
      children: [
        {
          id: 'shadcn-tab-bar-tab-2-label',
          type: 'text',
          name: 'Label',
          content: 'Tab 2',
          fontSize: 13,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#71717A' }],
        },
      ],
    },
    {
      id: 'shadcn-tab-bar-tab-3',
      type: 'frame',
      name: 'Tab',
      height: 'fill_container',
      width: 'fill_container',
      layout: 'horizontal',
      justifyContent: 'center',
      alignItems: 'center',
      cornerRadius: [0, 8, 8, 0],
      children: [
        {
          id: 'shadcn-tab-bar-tab-3-label',
          type: 'text',
          name: 'Label',
          content: 'Tab 3',
          fontSize: 13,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#71717A' }],
        },
      ],
    },
  ],
}

const breadcrumb: PenNode = {
  id: 'shadcn-breadcrumb',
  type: 'frame',
  name: 'Breadcrumb',
  reusable: true,
  x: 420,
  y: 556,
  width: 280,
  height: 32,
  layout: 'horizontal',
  gap: 8,
  alignItems: 'center',
  children: [
    {
      id: 'shadcn-breadcrumb-home',
      type: 'text',
      name: 'Home',
      content: 'Home',
      fontSize: 13,
      fill: [{ type: 'solid', color: '#71717A' }],
    },
    {
      id: 'shadcn-breadcrumb-sep-1',
      type: 'text',
      name: 'Separator',
      content: '/',
      fontSize: 13,
      fill: [{ type: 'solid', color: '#E4E4E7' }],
    },
    {
      id: 'shadcn-breadcrumb-section',
      type: 'text',
      name: 'Section',
      content: 'Section',
      fontSize: 13,
      fill: [{ type: 'solid', color: '#71717A' }],
    },
    {
      id: 'shadcn-breadcrumb-sep-2',
      type: 'text',
      name: 'Separator',
      content: '/',
      fontSize: 13,
      fill: [{ type: 'solid', color: '#E4E4E7' }],
    },
    {
      id: 'shadcn-breadcrumb-current',
      type: 'text',
      name: 'Current',
      content: 'Current Page',
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
  ],
}

// ---------------------------------------------------------------------------
// Feedback
// ---------------------------------------------------------------------------

const alertBanner: PenNode = {
  id: 'shadcn-alert-banner',
  type: 'frame',
  name: 'Alert Banner',
  reusable: true,
  x: 0,
  y: 616,
  width: 400,
  height: 56,
  layout: 'horizontal',
  gap: 12,
  alignItems: 'center',
  padding: [0, 16, 0, 16],
  cornerRadius: 8,
  fill: [{ type: 'solid', color: '#F4F4F5' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E4E4E7' }] },
  children: [
    {
      id: 'shadcn-alert-banner-icon',
      type: 'ellipse',
      name: 'Icon',
      width: 8,
      height: 8,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
    {
      id: 'shadcn-alert-banner-text',
      type: 'text',
      name: 'Message',
      content: 'This is an informational alert message.',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
  ],
}

const badge: PenNode = {
  id: 'shadcn-badge',
  type: 'frame',
  name: 'Badge',
  reusable: true,
  x: 420,
  y: 616,
  width: 64,
  height: 24,
  layout: 'horizontal',
  justifyContent: 'center',
  alignItems: 'center',
  padding: [0, 10, 0, 10],
  cornerRadius: 12,
  fill: [{ type: 'solid', color: '#F4F4F5' }],
  children: [
    {
      id: 'shadcn-badge-label',
      type: 'text',
      name: 'Label',
      content: 'Badge',
      fontSize: 12,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
  ],
}

const avatar: PenNode = {
  id: 'shadcn-avatar',
  type: 'frame',
  name: 'Avatar',
  reusable: true,
  x: 504,
  y: 616,
  width: 40,
  height: 40,
  cornerRadius: 20,
  fill: [{ type: 'solid', color: '#F4F4F5' }],
  layout: 'horizontal',
  justifyContent: 'center',
  alignItems: 'center',
  children: [
    {
      id: 'shadcn-avatar-initials',
      type: 'text',
      name: 'Initials',
      content: 'JD',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#18181B' }],
    },
  ],
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

const divider: PenNode = {
  id: 'shadcn-divider',
  type: 'frame',
  name: 'Divider',
  reusable: true,
  x: 0,
  y: 692,
  width: 400,
  height: 1,
  fill: [{ type: 'solid', color: '#E4E4E7' }],
}

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

export const SHADCN_KIT_DOCUMENT: PenDocument = {
  version: '1.0.0',
  name: 'shadcn UI',
  children: [
    btnPrimary,
    btnSecondary,
    btnGhost,
    btnDestructive,
    inputText,
    inputTextarea,
    inputCheckbox,
    inputToggle,
    inputRadio,
    cardBasic,
    cardStats,
    cardImage,
    navbar,
    tabBar,
    breadcrumb,
    alertBanner,
    badge,
    avatar,
    divider,
    ...extraComponents,
  ],
}
