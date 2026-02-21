import type { PenDocument, PenNode } from '@/types/pen'

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

const btnPrimary: PenNode = {
  id: 'uikit-btn-primary',
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
  fill: [{ type: 'solid', color: '#2563EB' }],
  children: [
    {
      id: 'uikit-btn-primary-label',
      type: 'text',
      name: 'Label',
      content: 'Button',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    },
  ],
}

const btnSecondary: PenNode = {
  id: 'uikit-btn-secondary',
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
  fill: [{ type: 'solid', color: '#FFFFFF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#D1D5DB' }] },
  children: [
    {
      id: 'uikit-btn-secondary-label',
      type: 'text',
      name: 'Label',
      content: 'Button',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#374151' }],
    },
  ],
}

const btnGhost: PenNode = {
  id: 'uikit-btn-ghost',
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
      id: 'uikit-btn-ghost-label',
      type: 'text',
      name: 'Label',
      content: 'Button',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#2563EB' }],
    },
  ],
}

const btnDestructive: PenNode = {
  id: 'uikit-btn-destructive',
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
  fill: [{ type: 'solid', color: '#DC2626' }],
  children: [
    {
      id: 'uikit-btn-destructive-label',
      type: 'text',
      name: 'Label',
      content: 'Delete',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    },
  ],
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

const inputText: PenNode = {
  id: 'uikit-input-text',
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
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#D1D5DB' }] },
  children: [
    {
      id: 'uikit-input-text-placeholder',
      type: 'text',
      name: 'Placeholder',
      content: 'Enter text...',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#9CA3AF' }],
    },
  ],
}

const inputTextarea: PenNode = {
  id: 'uikit-input-textarea',
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
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#D1D5DB' }] },
  children: [
    {
      id: 'uikit-input-textarea-placeholder',
      type: 'text',
      name: 'Placeholder',
      content: 'Enter description...',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#9CA3AF' }],
      width: 'fill_container',
    },
  ],
}

const inputCheckbox: PenNode = {
  id: 'uikit-input-checkbox',
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
      id: 'uikit-input-checkbox-box',
      type: 'rectangle',
      name: 'Box',
      width: 18,
      height: 18,
      cornerRadius: 4,
      fill: [{ type: 'solid', color: '#2563EB' }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#2563EB' }] },
    },
    {
      id: 'uikit-input-checkbox-label',
      type: 'text',
      name: 'Label',
      content: 'Checkbox label',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#374151' }],
    },
  ],
}

const inputToggle: PenNode = {
  id: 'uikit-input-toggle',
  type: 'frame',
  name: 'Toggle Switch',
  reusable: true,
  x: 180,
  y: 176,
  width: 44,
  height: 24,
  cornerRadius: 12,
  fill: [{ type: 'solid', color: '#2563EB' }],
  children: [
    {
      id: 'uikit-input-toggle-thumb',
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
  id: 'uikit-input-radio',
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
      id: 'uikit-input-radio-circle',
      type: 'ellipse',
      name: 'Circle',
      width: 18,
      height: 18,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      stroke: { thickness: 2, fill: [{ type: 'solid', color: '#2563EB' }] },
    },
    {
      id: 'uikit-input-radio-label',
      type: 'text',
      name: 'Label',
      content: 'Radio label',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#374151' }],
    },
  ],
}

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

const cardBasic: PenNode = {
  id: 'uikit-card-basic',
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
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E5E7EB' }] },
  effects: [{ type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: 'rgba(0,0,0,0.1)' }],
  children: [
    {
      id: 'uikit-card-basic-title',
      type: 'text',
      name: 'Title',
      content: 'Card Title',
      fontSize: 18,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#111827' }],
    },
    {
      id: 'uikit-card-basic-desc',
      type: 'text',
      name: 'Description',
      content: 'Card description goes here with some supporting text.',
      fontSize: 14,
      lineHeight: 1.5,
      fill: [{ type: 'solid', color: '#6B7280' }],
      width: 'fill_container',
    },
  ],
}

const cardStats: PenNode = {
  id: 'uikit-card-stats',
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
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E5E7EB' }] },
  effects: [{ type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: 'rgba(0,0,0,0.1)' }],
  children: [
    {
      id: 'uikit-card-stats-label',
      type: 'text',
      name: 'Label',
      content: 'Total Revenue',
      fontSize: 12,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#6B7280' }],
    },
    {
      id: 'uikit-card-stats-value',
      type: 'text',
      name: 'Value',
      content: '$45,231',
      fontSize: 28,
      fontWeight: 700,
      fill: [{ type: 'solid', color: '#111827' }],
    },
    {
      id: 'uikit-card-stats-change',
      type: 'text',
      name: 'Change',
      content: '+20.1% from last month',
      fontSize: 12,
      fill: [{ type: 'solid', color: '#16A34A' }],
    },
  ],
}

const cardImage: PenNode = {
  id: 'uikit-card-image',
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
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E5E7EB' }] },
  effects: [{ type: 'shadow', offsetX: 0, offsetY: 1, blur: 3, spread: 0, color: 'rgba(0,0,0,0.1)' }],
  children: [
    {
      id: 'uikit-card-image-placeholder',
      type: 'rectangle',
      name: 'Image',
      width: 'fill_container',
      height: 140,
      fill: [{ type: 'solid', color: '#F3F4F6' }],
      cornerRadius: [12, 12, 0, 0],
    },
    {
      id: 'uikit-card-image-body',
      type: 'frame',
      name: 'Body',
      width: 'fill_container',
      layout: 'vertical',
      gap: 4,
      padding: [12, 16, 16, 16],
      children: [
        {
          id: 'uikit-card-image-title',
          type: 'text',
          name: 'Title',
          content: 'Card Title',
          fontSize: 16,
          fontWeight: 600,
          fill: [{ type: 'solid', color: '#111827' }],
        },
        {
          id: 'uikit-card-image-desc',
          type: 'text',
          name: 'Description',
          content: 'Brief description text.',
          fontSize: 13,
          fill: [{ type: 'solid', color: '#6B7280' }],
        },
      ],
    },
  ],
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

const navbar: PenNode = {
  id: 'uikit-nav-bar',
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
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E5E7EB' }] },
  children: [
    {
      id: 'uikit-nav-bar-brand',
      type: 'text',
      name: 'Brand',
      content: 'Brand',
      fontSize: 18,
      fontWeight: 700,
      fill: [{ type: 'solid', color: '#111827' }],
    },
    {
      id: 'uikit-nav-bar-links',
      type: 'frame',
      name: 'Links',
      layout: 'horizontal',
      gap: 24,
      alignItems: 'center',
      children: [
        {
          id: 'uikit-nav-bar-link-1',
          type: 'text',
          name: 'Link',
          content: 'Home',
          fontSize: 14,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#374151' }],
        },
        {
          id: 'uikit-nav-bar-link-2',
          type: 'text',
          name: 'Link',
          content: 'Products',
          fontSize: 14,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#6B7280' }],
        },
        {
          id: 'uikit-nav-bar-link-3',
          type: 'text',
          name: 'Link',
          content: 'About',
          fontSize: 14,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#6B7280' }],
        },
      ],
    },
  ],
}

const tabBar: PenNode = {
  id: 'uikit-tab-bar',
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
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E5E7EB' }] },
  cornerRadius: 8,
  children: [
    {
      id: 'uikit-tab-bar-tab-1',
      type: 'frame',
      name: 'Tab Active',
      height: 'fill_container',
      width: 'fill_container',
      layout: 'horizontal',
      justifyContent: 'center',
      alignItems: 'center',
      fill: [{ type: 'solid', color: '#EFF6FF' }],
      cornerRadius: [8, 0, 0, 8],
      children: [
        {
          id: 'uikit-tab-bar-tab-1-label',
          type: 'text',
          name: 'Label',
          content: 'Tab 1',
          fontSize: 13,
          fontWeight: 600,
          fill: [{ type: 'solid', color: '#2563EB' }],
        },
      ],
    },
    {
      id: 'uikit-tab-bar-tab-2',
      type: 'frame',
      name: 'Tab',
      height: 'fill_container',
      width: 'fill_container',
      layout: 'horizontal',
      justifyContent: 'center',
      alignItems: 'center',
      children: [
        {
          id: 'uikit-tab-bar-tab-2-label',
          type: 'text',
          name: 'Label',
          content: 'Tab 2',
          fontSize: 13,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#6B7280' }],
        },
      ],
    },
    {
      id: 'uikit-tab-bar-tab-3',
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
          id: 'uikit-tab-bar-tab-3-label',
          type: 'text',
          name: 'Label',
          content: 'Tab 3',
          fontSize: 13,
          fontWeight: 500,
          fill: [{ type: 'solid', color: '#6B7280' }],
        },
      ],
    },
  ],
}

const breadcrumb: PenNode = {
  id: 'uikit-breadcrumb',
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
      id: 'uikit-breadcrumb-home',
      type: 'text',
      name: 'Home',
      content: 'Home',
      fontSize: 13,
      fill: [{ type: 'solid', color: '#6B7280' }],
    },
    {
      id: 'uikit-breadcrumb-sep-1',
      type: 'text',
      name: 'Separator',
      content: '/',
      fontSize: 13,
      fill: [{ type: 'solid', color: '#D1D5DB' }],
    },
    {
      id: 'uikit-breadcrumb-section',
      type: 'text',
      name: 'Section',
      content: 'Section',
      fontSize: 13,
      fill: [{ type: 'solid', color: '#6B7280' }],
    },
    {
      id: 'uikit-breadcrumb-sep-2',
      type: 'text',
      name: 'Separator',
      content: '/',
      fontSize: 13,
      fill: [{ type: 'solid', color: '#D1D5DB' }],
    },
    {
      id: 'uikit-breadcrumb-current',
      type: 'text',
      name: 'Current',
      content: 'Current Page',
      fontSize: 13,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#111827' }],
    },
  ],
}

// ---------------------------------------------------------------------------
// Feedback
// ---------------------------------------------------------------------------

const alertBanner: PenNode = {
  id: 'uikit-alert-banner',
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
  fill: [{ type: 'solid', color: '#EFF6FF' }],
  stroke: { thickness: 1, fill: [{ type: 'solid', color: '#BFDBFE' }] },
  children: [
    {
      id: 'uikit-alert-banner-icon',
      type: 'ellipse',
      name: 'Icon',
      width: 8,
      height: 8,
      fill: [{ type: 'solid', color: '#2563EB' }],
    },
    {
      id: 'uikit-alert-banner-text',
      type: 'text',
      name: 'Message',
      content: 'This is an informational alert message.',
      fontSize: 14,
      fill: [{ type: 'solid', color: '#1E40AF' }],
    },
  ],
}

const badge: PenNode = {
  id: 'uikit-badge',
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
  fill: [{ type: 'solid', color: '#DBEAFE' }],
  children: [
    {
      id: 'uikit-badge-label',
      type: 'text',
      name: 'Label',
      content: 'Badge',
      fontSize: 12,
      fontWeight: 500,
      fill: [{ type: 'solid', color: '#1D4ED8' }],
    },
  ],
}

const avatar: PenNode = {
  id: 'uikit-avatar',
  type: 'frame',
  name: 'Avatar',
  reusable: true,
  x: 504,
  y: 616,
  width: 40,
  height: 40,
  cornerRadius: 20,
  fill: [{ type: 'solid', color: '#DBEAFE' }],
  layout: 'horizontal',
  justifyContent: 'center',
  alignItems: 'center',
  children: [
    {
      id: 'uikit-avatar-initials',
      type: 'text',
      name: 'Initials',
      content: 'JD',
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#2563EB' }],
    },
  ],
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

const divider: PenNode = {
  id: 'uikit-divider',
  type: 'frame',
  name: 'Divider',
  reusable: true,
  x: 0,
  y: 692,
  width: 400,
  height: 1,
  fill: [{ type: 'solid', color: '#E5E7EB' }],
}

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

export const DEFAULT_KIT_DOCUMENT: PenDocument = {
  version: '1.0.0',
  name: 'Default UIKit',
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
  ],
}
