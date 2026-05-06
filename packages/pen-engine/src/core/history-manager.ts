import type { PenDocument } from '@zseven-w/pen-types';
import { DEFAULT_MAX_HISTORY, HISTORY_DEBOUNCE_MS } from './constants.js';

export interface HistoryManagerOptions {
  maxStates?: number;
  onChange?: (state: { canUndo: boolean; canRedo: boolean }) => void;
}

/**
 * Framework 与
 * undo/redo 无关的管理器。 Extracted 来自
 * apps/web/src/stores/history-store.ts。 Owns 其内部状态——没有
 Zustand 依赖性。
 */
export class HistoryManager {
  private undoStack: PenDocument[] = [];
  private redoStack: PenDocument[] = [];
  private batchDepth = 0;
  private batchBaseState: PenDocument | null = null;
  private maxStates: number;
  private lastPushTime = 0;
  private onChangeCb?: (state: { canUndo: boolean; canRedo: boolean }) => void;

  constructor(options?: HistoryManagerOptions) {
    this.maxStates = options?.maxStates ?? DEFAULT_MAX_HISTORY;
    this.onChangeCb = options?.onChange;
  }

  get canUndo(): boolean {
    return this.undoStack.length > 0;
  }

  get canRedo(): boolean {
    return this.redoStack.length > 0;
  }

  /**
   * Push 突变前的当前文
   * 档状态。 In 批处理模式，推送被抑制（批处理捕获基本状态）。 Debounces 在
   * HISTORY_DEBOUNCE_MS 内快速推进。
   */
  push(doc: PenDocument): void {
    if (this.batchDepth > 0) return;

    const now = Date.now();
    if (now - this.lastPushTime < HISTORY_DEBOUNCE_MS) {
      this.lastPushTime = now;
      if (this.redoStack.length > 0) {
        this.redoStack = [];
        this.notify();
      }
      return;
    }
    this.lastPushTime = now;

    const last = this.undoStack[this.undoStack.length - 1];
    if (last && this.areEqual(last, doc)) {
      this.redoStack = [];
      this.notify();
      return;
    }

    this.undoStack = [...this.undoStack.slice(-(this.maxStates - 1)), structuredClone(doc)];
    this.redoStack = [];
    this.notify();
  }

  /**
   * Undo：恢复之前的文档
   * 状态。 Returns 恢复的文档，如果没有要撤消的内容则为 null。
   */
  undo(currentDoc: PenDocument): PenDocument | null {
    if (this.undoStack.length === 0) return null;
    const previous = this.undoStack[this.undoStack.length - 1];
    this.undoStack = this.undoStack.slice(0, -1);
    this.redoStack = [...this.redoStack, structuredClone(currentDoc)];
    this.notify();
    return structuredClone(previous);
  }

  /**
   * Redo：恢复下一个文档
   * 状态。 Returns 已恢复的文档，如果无需重做，则为 null。
   */
  redo(currentDoc: PenDocument): PenDocument | null {
    if (this.redoStack.length === 0) return null;
    const next = this.redoStack[this.redoStack.length - 1];
    this.redoStack = this.redoStack.slice(0, -1);
    this.undoStack = [...this.undoStack, structuredClone(currentDoc)];
    this.notify();
    return structuredClone(next);
  }

  /**
   * Start 一批：所有
   * push() 调用都被抑制，直到 endBatch()。 Supports 嵌套。 Only 最外面的 endBatch()
   提交。
   */
  startBatch(doc: PenDocument): void {
    if (this.batchDepth === 0) {
      this.batchBaseState = structuredClone(doc);
    }
    this.batchDepth++;
  }

  /**
   * End 一批。 On
   * 最外面的调用，将基本状态推送到撤消堆栈（除非文档未更改）。
   */
  endBatch(currentDoc?: PenDocument): void {
    if (this.batchDepth <= 0) return;
    this.batchDepth--;

    if (this.batchDepth === 0 && this.batchBaseState) {
      const unchanged = currentDoc ? this.areEqual(this.batchBaseState, currentDoc) : false;

      if (!unchanged) {
        this.undoStack = [...this.undoStack.slice(-(this.maxStates - 1)), this.batchBaseState];
        this.redoStack = [];
        this.notify();
      }
      this.batchBaseState = null;
    }
  }

  /** Clear 所有历史记录。 */
  clear(): void {
    this.undoStack = [];
    this.redoStack = [];
    this.batchDepth = 0;
    this.batchBaseState = null;
    this.notify();
  }

  private areEqual(a: PenDocument, b: PenDocument): boolean {
    return JSON.stringify(a) === JSON.stringify(b);
  }

  private notify(): void {
    this.onChangeCb?.({ canUndo: this.canUndo, canRedo: this.canRedo });
  }
}
