export interface StyleGuideMeta {
  name: string;
  tags: string[];
  platform: 'webapp' | 'mobile' | 'landing-page' | 'slides';
}

export interface ParsedStyleGuide extends StyleGuideMeta {
  /** Full 风格指南的 Markdown 内容（注入提示中） */
  content: string;
}

export const STYLE_GUIDE_TAGS = [
  // 视觉的
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
  // 语气
  'calm',
  'dark-mode',
  'high-contrast',
  'light-mode',
  'monochrome',
  'neon',
  'pastel',
  'vibrant',
  'warm-tones',
  // 工业
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
  // 版式
  'bold-typography',
  'condensed',
  'display',
  'dual-font',
  'magazine',
  'monospace',
  'serif',
  'uppercase',
  // 布局
  'bento-grid',
  'floating-nav',
  'flush-layout',
  'horizontal-nav',
  'icon-sidebar',
  'sidebar',
  // 心情
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
  // 口音
  'blue-accent',
  'cyan-accent',
  'gold-accent',
  'lime-accent',
  'orange-accent',
  'red-accent',
  'sage-green',
  // 技术
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
