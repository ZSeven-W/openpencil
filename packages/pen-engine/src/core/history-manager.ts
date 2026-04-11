import type { PenDocument } from '@zseven-w/pen-types';
import { DEFAULT_MAX_HISTORY, HISTORY_DEBOUNCE_MS } from './constants.js';

export interface HistoryManagerOptions {
  maxStates?: number;
  onChange?: (state: { canUndo: boolean; canRedo: boolean }) => void;
}

/**
 * Framework-agnostic undo/redo manager.
 * Extracted from apps/web/src/stores/history-store.ts.
 * Owns its state internally -- no Zustand dependency.
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
   * Push the current document state before a mutation.
   * In batch mode, pushes are suppressed (batch captures the base state).
   * Debounces rapid pushes within HISTORY_DEBOUNCE_MS.
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
   * Undo: restore the previous document state.
   * Returns the restored document, or null if nothing to undo.
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
   * Redo: restore the next document state.
   * Returns the restored document, or null if nothing to redo.
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
   * Start a batch: all push() calls are suppressed until endBatch().
   * Supports nesting. Only the outermost endBatch() commits.
   */
  startBatch(doc: PenDocument): void {
    if (this.batchDepth === 0) {
      this.batchBaseState = structuredClone(doc);
    }
    this.batchDepth++;
  }

  /**
   * End a batch. On the outermost call, pushes the base state to undo stack
   * (unless the document is unchanged).
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

  /** Clear all history. */
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
