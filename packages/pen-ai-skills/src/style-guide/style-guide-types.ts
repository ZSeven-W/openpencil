export interface StyleGuideMeta {
  name: string;
  tags: string[];
  platform: 'webapp' | 'mobile' | 'landing-page' | 'slides';
}

export interface ParsedStyleGuide extends StyleGuideMeta {
  /** Full markdown content of the style guide (injected into prompts) */
  content: string;
}

export const STYLE_GUIDE_TAGS = [
  // visual
  'brutalist',
  'classical',
  'clean',
  'colorful',
  'editorial',
  'elegant',
  'geometric',
  'minimal',
  'organic',
  'playful',
  'scandinavian',
  'swiss',
  'bauhaus',
  'zen',
  // tone
  'calm',
  'dark-mode',
  'high-contrast',
  'light-mode',
  'monochrome',
  'neon',
  'pastel',
  'vibrant',
  'warm-tones',
  // industry
  'corporate',
  'creative',
  'data-focused',
  'developer',
  'education',
  'enterprise',
  'fintech',
  'luxury',
  'tech',
  'terminal',
  'wellness',
  // typography
  'bold-typography',
  'condensed',
  'display',
  'dual-font',
  'magazine',
  'monospace',
  'serif',
  'uppercase',
  // layout
  'bento-grid',
  'floating-nav',
  'flush-layout',
  'horizontal-nav',
  'icon-sidebar',
  'sidebar',
  // mood
  'austere',
  'confident',
  'cozy',
  'crisp',
  'electric',
  'friendly',
  'quiet',
  'refined',
  'sophisticated',
  'urban',
  // accent
  'blue-accent',
  'cyan-accent',
  'gold-accent',
  'lime-accent',
  'orange-accent',
  'red-accent',
  'sage-green',
  // technique
  'flat',
  'gradient',
  'mesh-gradient',
  'rounded',
  'sharp-corners',
  'soft-corners',
  'soft-shadows',
  'stroke-based',
] as const;

export type StyleGuideTag = (typeof STYLE_GUIDE_TAGS)[number];
