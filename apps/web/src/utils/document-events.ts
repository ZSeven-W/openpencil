// apps/web/src/utils/document-events.ts
//
// Tiny 用于文档生命周期信号的类型化事件发射器。 Used 按功能
// 需要可靠的“文档刚刚击中磁盘”信号 - 例如 Git
// 集成的自动保存订阅者和 withCleanWorkingTree 重试路径。
//
// We 不订阅 Zustand 的 isDirty 转换，因为它会触发
// 除了“用户刚刚保存”之外还有很多原因（加载文件、MCP 同步、撤消到
// 清洁等）。 The 单 `useDocumentStore.save()` 动作是唯一的地方
// 成功写入磁盘后，会发出“saved”。

import type { PenDocument } from '@/types/pen';

export interface DocumentEventMap {
  saved: {
    filePath: string | null; // 仅在浏览器下载回退中为 null
    fileName: string;
    document: PenDocument;
  };
}

type EventName = keyof DocumentEventMap;
type Handler<E extends EventName> = (payload: DocumentEventMap[E]) => void;

class DocumentEventEmitter {
  private handlers: Partial<{ [E in EventName]: Set<Handler<E>> }> = {};

  on<E extends EventName>(event: E, handler: Handler<E>): () => void {
    let set = this.handlers[event] as Set<Handler<E>> | undefined;
    if (!set) {
      set = new Set();
      this.handlers[event] = set as never;
    }
    set.add(handler);
    return () => {
      set!.delete(handler);
    };
  }

  emit<E extends EventName>(event: E, payload: DocumentEventMap[E]): void {
    const set = this.handlers[event] as Set<Handler<E>> | undefined;
    if (!set) return;
    // Snapshot 以避免处理程序取消订阅时出现重新进入突变问题。
    for (const handler of Array.from(set)) {
      try {
        handler(payload);
      } catch (err) {
        console.error(`[documentEvents] handler for "${event}" threw:`, err);
      }
    }
  }

  // Test-only：清除测试之间的所有处理程序。
  _clear(): void {
    this.handlers = {};
  }
}

export const documentEvents = new DocumentEventEmitter();
