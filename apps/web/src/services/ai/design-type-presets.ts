export type DesignType = 'mobile-screen' | 'app-screen' | 'landing-page'

export interface DesignTypePreset {
  type: DesignType
  width: number
  /** Section total height (0 = auto based on section count) */
  height: number
  /** Explicit rootFrame height (0 = auto) */
  rootHeight: number
  defaultSections: string[]
  /** First preset with any matching pattern wins */
  patterns: RegExp[]
}

const PRESETS: DesignTypePreset[] = [
  {
    type: 'mobile-screen',
    width: 375,
    height: 812,
    rootHeight: 812,
    defaultSections: ['Header', 'Main Content', 'Actions'],
    patterns: [
      /mobile|手机|phone|移动端/i,
      /app\s*screen/i,
      /(登录|注册|login|sign\s*up)\b/i,
    ],
  },
  {
    type: 'app-screen',
    width: 1200,
    height: 800,
    rootHeight: 800,
    defaultSections: ['Header', 'Main Content', 'Actions'],
    patterns: [
      /(settings|设置|preference|偏好)/i,
      /(profile|个人|account|账户)/i,
      /(dashboard|admin|管理|后台|控制台)/i,
      /(form|表单|modal|dialog|弹窗)/i,
    ],
  },
  {
    type: 'landing-page',
    width: 1200,
    height: 0,
    rootHeight: 0,
    defaultSections: ['Navigation', 'Hero', 'Core Highlights', 'Feature Showcase', 'CTA', 'Footer'],
    patterns: [], // default fallback
  },
]

/** Detect design type from prompt text. Returns the first preset with a matching pattern, or the landing-page fallback. */
export function detectDesignType(prompt: string): DesignTypePreset {
  for (const preset of PRESETS) {
    if (preset.patterns.length === 0) continue
    if (preset.patterns.some((p) => p.test(prompt))) return preset
  }
  // Last preset (landing-page) is the default fallback
  return PRESETS[PRESETS.length - 1]
}
