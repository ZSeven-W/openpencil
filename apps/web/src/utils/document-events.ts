// apps/web/src/utils/document-events.ts
//
// Tiny typed event emitter for document lifecycle signals. Used by features
// that need a reliable "the document just hit disk" signal — e.g. the Git
// integration's autosave subscriber and the withCleanWorkingTree retry path.
//
// We don't subscribe to Zustand's isDirty transitions because that fires for
// many reasons besides "the user just saved" (load file, MCP sync, undo to
// clean, etc.). The single `useDocumentStore.save()` action is the only place
// 'saved' is ever emitted, after a successful disk write.

import type { PenDocument } from '@/types/pen';

export interface DocumentEventMap {
  saved: {
    filePath: string | null; // null only in browser-download fallback
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
    // Snapshot to avoid re-entry mutation issues if a handler unsubscribes.
    for (const handler of Array.from(set)) {
      try {
        handler(payload);
      } catch (err) {
        console.error(`[documentEvents] handler for "${event}" threw:`, err);
      }
    }
  }

  // Test-only: clear all handlers between tests.
  _clear(): void {
    this.handlers = {};
  }
}

export const documentEvents = new DocumentEventEmitter();
