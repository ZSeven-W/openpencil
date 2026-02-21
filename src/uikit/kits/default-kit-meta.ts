import type { ComponentCategory } from '@/types/uikit'

export const DEFAULT_KIT_META: Record<
  string,
  { category: ComponentCategory; tags: string[] }
> = {
  'uikit-btn-primary': { category: 'buttons', tags: ['button', 'primary', 'cta', 'action'] },
  'uikit-btn-secondary': { category: 'buttons', tags: ['button', 'secondary', 'outline'] },
  'uikit-btn-ghost': { category: 'buttons', tags: ['button', 'ghost', 'text', 'link'] },
  'uikit-btn-destructive': { category: 'buttons', tags: ['button', 'destructive', 'danger', 'delete'] },
  'uikit-input-text': { category: 'inputs', tags: ['input', 'text', 'field', 'form'] },
  'uikit-input-textarea': { category: 'inputs', tags: ['textarea', 'multiline', 'text', 'form'] },
  'uikit-input-checkbox': { category: 'inputs', tags: ['checkbox', 'check', 'toggle', 'form'] },
  'uikit-input-toggle': { category: 'inputs', tags: ['toggle', 'switch', 'on', 'off'] },
  'uikit-input-radio': { category: 'inputs', tags: ['radio', 'option', 'select', 'form'] },
  'uikit-card-basic': { category: 'cards', tags: ['card', 'container', 'surface', 'basic'] },
  'uikit-card-stats': { category: 'cards', tags: ['card', 'stats', 'metric', 'dashboard', 'kpi'] },
  'uikit-card-image': { category: 'cards', tags: ['card', 'image', 'media', 'thumbnail'] },
  'uikit-nav-bar': { category: 'navigation', tags: ['navbar', 'header', 'navigation', 'top'] },
  'uikit-tab-bar': { category: 'navigation', tags: ['tab', 'tabs', 'navigation', 'segment'] },
  'uikit-breadcrumb': { category: 'navigation', tags: ['breadcrumb', 'navigation', 'path'] },
  'uikit-alert-banner': { category: 'feedback', tags: ['alert', 'banner', 'notification', 'info'] },
  'uikit-badge': { category: 'feedback', tags: ['badge', 'tag', 'chip', 'label', 'status'] },
  'uikit-avatar': { category: 'feedback', tags: ['avatar', 'user', 'profile', 'icon'] },
  'uikit-divider': { category: 'layout', tags: ['divider', 'separator', 'line', 'hr'] },
}
