// apps/web/src/utils/load-op-file.ts
//
// Standalone 帮助程序通过以下方式从绝对路径加载 .op 文件
// Electron readFile IPC。 Lives 在它自己的模块中（NOT 里面
// file-operations.ts) 避免循环： document-store.ts 导入
// writeToFileHandle/etc。从 file-operations.ts，所以 file-operations.ts
// 如果不创建循环，则无法导入 useDocumentStore。 This 文件
// 导入文档存储，但 NOT 由文档存储导入或
// 文件操作，因此依赖图保持非循环。
//
// Used 作者：
//   - apps/web/src/hooks/use-electron-menu.ts（文件关联打开事件）
//   - apps/web/src/components/panels/git-panel/git-panel-tracked-picker.tsx
// （[跟踪并打开] 按钮 — Phase 4b）
//   - apps/web/src/stores/git-store.ts acknowledgeAutoBindAndOpen 行动
// （自动绑定横幅 [打开] 按钮 — Phase 4b）

import { useDocumentStore } from '@/stores/document-store';
import { normalizePenDocument } from '@/utils/normalize-pen-file';
import { zoomToFitContent } from '@/canvas/skia-engine-ref';

/**
 * Load 通过
 * Electron readFile IPC 从绝对路径获取 .op 文件，解析 + 规范化它，然后分派到
 * useDocumentStore。 Returns 成功时为 true，任何失败时为 false（不抛出 -
 * 失败是无声的，因为调用者通常是不应崩溃的 UI 按钮）。
 */
export async function loadOpFileFromPath(filePath: string): Promise<boolean> {
  const api = typeof window !== 'undefined' ? window.electronAPI : undefined;
  if (!api?.readFile) return false;
  try {
    const result = await api.readFile(filePath);
    if (!result) return false;
    const raw = JSON.parse(result.content);
    if (!raw.version || (!Array.isArray(raw.children) && !Array.isArray(raw.pages))) {
      return false;
    }
    const doc = normalizePenDocument(raw);
    const name = filePath.split(/[/\\]/).pop() || 'untitled.op';
    useDocumentStore.getState().loadDocument(doc, name, null, filePath);
    // zoomToFitContent 在下一帧上调度，因此 React 有时间在画布读取文档更新之前提交文档更新。
    requestAnimationFrame(() => zoomToFitContent());
    return true;
  } catch {
    return false;
  }
}
