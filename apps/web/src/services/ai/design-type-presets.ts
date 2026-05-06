export type DesignType = 'mobile-screen' | 'desktop-screen' | 'landing-page';

export interface DesignTypePreset {
  type: DesignType;
  width: number;
  /** Section 总高度（0 = 自动基于部分计数） */
  height: number;
  /** Explicit rootFrame 高度（0 = 自动） */
  rootHeight: number;
  defaultSections: string[];
}

/**
 * Minimal
 *
 * 后备设计类型检测。当协调器无法解析 AI 的 JSON 计划时使用 ONLY。
 * In 正常运行，AI
 *
 * 通过 decomposition.md 分类。 Keeps 最小分类 — AI 的工作是推理意图。 This
 * 后备只需要选择一个合理的 width/height/section 集。
 */
export function detectDesignType(prompt: string): DesignTypePreset {
  // Explicit 移动指示器（NOT “应用程序”单独 - 太模糊）
  if (/mobile|手机|phone|移动端|ios|android/i.test(prompt)) {
    return {
      type: 'mobile-screen',
      width: 375,
      height: 812,
      rootHeight: 812,
      // Two 安全桶保留了一个可读的清单，同时仍然保持弱模型后备分解足够广泛，以避免大量的横截面重复。
      defaultSections: ['Top Summary', 'Main Content'],
    };
  }

  // Fixed-桌面屏幕高度
  if (/dashboard|admin|管理|后台|控制台/i.test(prompt)) {
    return {
      type: 'desktop-screen',
      width: 1200,
      height: 800,
      rootHeight: 800,
      defaultSections: ['Header', 'Main Content', 'Actions'],
    };
  }

  // Default：可滚动桌面页面（对于登陆、投资组合、定价等最安全）
  return {
    type: 'landing-page',
    width: 1200,
    height: 0,
    rootHeight: 0,
    defaultSections: ['Header', 'Main Content', 'Supporting Content', 'Footer'],
  };
}
