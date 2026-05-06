/**
 * Text 编辑 DOM
 * 叠加支持。 Manages 用于内联文本编辑的
 <textarea> 覆盖层。
 */
export interface TextEditOverlayOptions {
  onEditStart?: (nodeId: string) => void;
  onEditEnd?: (nodeId: string, content: string) => void;
}

// Full 实现将管理位于节点屏幕坐标处的画布上的 <textarea>，处理 blur/Enter 上的提交。
