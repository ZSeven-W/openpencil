import type { AIProviderType } from '@/types/agent-settings';
import type { DesignMdSpec } from '@/types/design-md';

export interface ChatAttachment {
  id: string;
  name: string;
  mediaType: string; // “image/png”、“image/jpeg”等
  data: string; // Base64 编码（无数据 URL 前缀）
  size: number; // 字节
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
  isStreaming?: boolean;
  attachments?: ChatAttachment[];
  source?: string;
}

/**
 * 当用户要求“扩展”现有的
 * 非空页面而不是生成新屏幕时，Emitted 由 `detectAppendIntent()` 执行。 When
 * 目前，编排器会跳过创建新的根框架并将生成的部分插入到现有的内容根框架中。
 *
 */
export interface AppendContext {
  /** 新的子代理部分应插入到的框架的 ID 中。 */
  targetParentId: string;
  /** 目标父级的 Width （用于调整子代理区域的大小）。 */
  targetWidth: number;
  /** Labels 现有顶级部分 - 子代理被告知不要重复这些。 */
  existingSectionLabels: string[];
  /** True 当 `targetParentId` 属于移动页面（宽度 <= 480）时。 */
  isMobile: boolean;
}

export interface AIDesignRequest {
  prompt: string;
  model?: string;
  provider?: AIProviderType;
  concurrency?: number;
  context?: {
    selectedNodes?: string[];
    documentSummary?: string;
    canvasSize?: { width: number; height: number };
    variables?: Record<string, import('@/types/variables').VariableDefinition>;
    themes?: Record<string, string[]>;
    designMd?: DesignMdSpec;
    appendContext?: AppendContext;
  };
}

// ---------------------------------------------------------------------------
// Design System 类型 — 生成的设计令牌以确保一致性
// ---------------------------------------------------------------------------

export interface DesignSystem {
  palette: {
    background: string;
    surface: string;
    text: string;
    textSecondary: string;
    primary: string;
    primaryLight: string;
    accent: string;
    border: string;
  };
  typography: {
    headingFont: string;
    bodyFont: string;
    scale: number[];
  };
  spacing: {
    unit: number;
    scale: number[];
  };
  radius: number[];
  aesthetic: string;
}

// ---------------------------------------------------------------------------
// Visual Reference 类型 — 用于视觉参考管道
// ---------------------------------------------------------------------------

export interface VisualReference {
  /** Generated HTML/CSS 代码 */
  html: string;
  /** 渲染的 HTML 的 Screenshot （base64 PNG，无数据：前缀） */
  screenshot: string;
  /** Design 使用的系统令牌 */
  designSystem: DesignSystem;
  /** Structural 摘要从 HTML 中提取 */
  structureSummary: string;
}

export interface AICodeRequest {
  prompt?: string;
  format: 'react-tailwind' | 'html-css' | 'react-inline';
  nodeIds?: string[];
}

export interface AIStreamChunk {
  type: 'text' | 'thinking' | 'done' | 'error' | 'ping';
  content: string;
}

// ---------------------------------------------------------------------------
// Orchestrator 类型 — 用于并行子代理设计生成
// ---------------------------------------------------------------------------

/** 由协调器生成的子任务（轻量级 - 仅空间信息） */
export interface SubTask {
  /** Unique ID 用于此子任务，例如“侧边栏”、“标题” */
  id: string;
  /** Human-进度的可读标签 UI */
  label: string;
  /** Specific UI 此子任务必须生成的元素（来自 Orchestrator） */
  elements?: string;
  /** Spatial 画布上分配的区域 */
  region: { width: number; height: number };
  /** ID 该子代理生成的所有节点的前缀（在运行时分配） */
  idPrefix: string;
  /** 要插入的 Parent 帧 ID（在运行时分配） */
  parentFrameId: string | null;
  /** Screen/page 分组 — 同屏的子任务共享一个根框架 */
  screen?: string;
  /** 本节的 HTML 参考片段（来自视觉参考管道） */
  htmlReference?: string;
  /** Actual 为此子任务生成根节点 ID（在运行时捕获） */
  generatedRootId?: string;
  /** Propagated 来自 AppendContext，因此子代理提示可以避免重复。 */
  existingSectionLabels?: string[];
}

/** Style 由协调器制作的指南，以实现视觉一致性 */
export interface StyleGuide {
  palette: {
    background: string;
    surface: string;
    text: string;
    secondary: string;
    accent: string;
    accent2: string;
    border: string;
  };
  fonts: {
    heading: string;
    body: string;
  };
  aesthetic: string;
}

/** Plan 由协调器生成（轻量级 - 仅结构） */
export interface OrchestratorPlan {
  rootFrame: {
    id: string;
    name: string;
    width: number;
    height: number;
    layout?: 'none' | 'vertical' | 'horizontal';
    gap?: number;
    padding?: number | [number, number] | [number, number, number, number];
    fill?: Array<{ type: string; color: string }>;
  };
  styleGuide?: StyleGuide;
  styleGuideName?: string;
  selectedStyleGuideContent?: string;
  subtasks: SubTask[];
}

/** Progress 协调生成的状态 */
export interface OrchestrationProgress {
  phase: 'planning' | 'generating' | 'merging' | 'done' | 'error';
  subtasks: Array<{
    id: string;
    label: string;
    status: 'pending' | 'streaming' | 'done' | 'error';
    nodeCount: number;
    /** Accumulated 子代理的思考内容 */
    thinking?: string;
    /** Agent 视觉指示器的标识（仅限并发模式） */
    agentColor?: string;
    agentName?: string;
  }>;
  totalNodes: number;
}

/** 来自单个子代理的 Result */
export interface SubAgentResult {
  subtaskId: string;
  nodes: import('@/types/pen').PenNode[];
  rawResponse: string;
  error?: string;
}
