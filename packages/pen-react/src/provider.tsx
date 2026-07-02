import { useRef, useEffect } from 'react';
import type { ReactNode } from 'react';
import { DesignEngine } from '@zseven-w/pen-engine';
import type { DesignEngineOptions } from '@zseven-w/pen-types';
import type { PenDocument } from '@zseven-w/pen-types';
import { DesignEngineContext } from './context.js';

export interface DesignProviderProps {
  children: ReactNode;
  options?: DesignEngineOptions;
  /** Uncontrolled mode: initial document, loaded once. */
  initialDocument?: PenDocument;
  /** Controlled mode: external source of truth. Must pair with onDocumentChange. */
  document?: PenDocument;
  /** Controlled mode: called when engine mutates document internally. */
  onDocumentChange?: (doc: PenDocument) => void;
}

/**
 * Provides a DesignEngine instance to the React tree.
 *
 * Two modes (mutually exclusive):
 * - Uncontrolled: pass `initialDocument` — engine owns the document.
 * - Controlled: pass `document` + `onDocumentChange` — external state is source of truth.
 *
 * Echo-loop prevention (controlled mode):
 * - Maintains `lastEmittedDocRef` to track the last document reference emitted outbound.
 * - When `document` prop changes, compares by reference to `lastEmittedDocRef.current`.
 * - If same reference: skip (it's an echo from our own emission).
 * - If different reference: call `engine.loadDocument(controlledDoc)` (external replacement).
 */
export function DesignProvider({
  children,
  options,
  initialDocument,
  document: controlledDoc,
  onDocumentChange,
}: DesignProviderProps) {
  const engineRef = useRef<DesignEngine | null>(null);
  // Track the last doc reference we emitted outbound to detect echo-loops.
  // Uses reference comparison (not version) for precise detection.
  const lastEmittedDocRef = useRef<PenDocument | null>(null);

  if (!engineRef.current) {
    engineRef.current = new DesignEngine(options);
    const initial = controlledDoc ?? initialDocument;
    if (initial) engineRef.current.loadDocument(initial);
  }

  const engine = engineRef.current;

  // Controlled mode — outbound: engine changes -> notify parent
  useEffect(() => {
    if (!onDocumentChange) return;
    return engine.on('document:change', (doc: PenDocument) => {
      lastEmittedDocRef.current = doc; // remember what we sent
      onDocumentChange(doc);
    });
  }, [engine, onDocumentChange]);

  // Controlled mode — inbound: parent changes -> sync to engine (skip echo)
  useEffect(() => {
    if (!controlledDoc) return;
    // Reference equality: if parent's doc IS the one we last emitted, it's echo
    if (controlledDoc === lastEmittedDocRef.current) return;
    // Different reference -> external replacement (file open, MCP sync, etc.)
    engine.loadDocument(controlledDoc);
  }, [engine, controlledDoc]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      engineRef.current?.dispose();
      engineRef.current = null;
    };
  }, []);

  return <DesignEngineContext.Provider value={engine}>{children}</DesignEngineContext.Provider>;
}
