import { useRef, useEffect } from 'react';
import type { ReactNode } from 'react';
import { DesignEngine } from '@zseven-w/pen-engine';
import type { DesignEngineOptions } from '@zseven-w/pen-types';
import type { PenDocument } from '@zseven-w/pen-types';
import { DesignEngineContext } from './context.js';

export interface DesignProviderProps {
  children: ReactNode;
  options?: DesignEngineOptions;
  /** Uncontrolled 模式：初始文档，加载一次。 */
  initialDocument?: PenDocument;
  /** Controlled 模式：外部事实来源。 Must 与 onDocumentChange 配对。 */
  document?: PenDocument;
  /** Controlled 模式：当引擎内部改变文档时调用。 */
  onDocumentChange?: (doc: PenDocument) => void;
}

/**
 * Provides 一个
 *
 * DesignEngine 实例到 React 树。 Two 模式（互斥）： -
 * Uncontrolled
 * ：传递 `initialDocument` —
 *
 * 引擎拥有该文档。 - Controlled：传递 `document` + `onDocumentChange` —
 * 外部状态是事实来源。 Echo 环路预防（受控模式）： - Maintains `lastEmittedDocRef` 跟踪最后发出的出站文档引用。 - When
 * `document`
 * 属性更改，参考 `lastEmittedDocRef.current`
 * 进行比较。 - If 相同的参考：跳过（这是我们自己发射的回声）。 - If 不同参考：调用 `engine.loadDocument(controlledDoc)`（外部
 替换）。
 */
export function DesignProvider({
  children,
  options,
  initialDocument,
  document: controlledDoc,
  onDocumentChange,
}: DesignProviderProps) {
  const engineRef = useRef<DesignEngine | null>(null);
  // Track 我们发出的最后一个文档参考以检测回声循环。 Uses 参考比较（不是版本）以进行精确检测。
  const lastEmittedDocRef = useRef<PenDocument | null>(null);

  if (!engineRef.current) {
    engineRef.current = new DesignEngine(options);
    const initial = controlledDoc ?? initialDocument;
    if (initial) engineRef.current.loadDocument(initial);
  }

  const engine = engineRef.current;

  // Controlled 模式 — 出站：引擎更改 -> 通知父级
  useEffect(() => {
    if (!onDocumentChange) return;
    return engine.on('document:change', (doc: PenDocument) => {
      lastEmittedDocRef.current = doc; // 记住我们发送的内容
      onDocumentChange(doc);
    });
  }, [engine, onDocumentChange]);

  // Controlled 模式 — 入站：父级更改 -> 同步到引擎（跳过回显）
  useEffect(() => {
    if (!controlledDoc) return;
    // Reference 相等：如果父文档 IS 是我们上次发出的文档，则它是 echo
    if (controlledDoc === lastEmittedDocRef.current) return;
    // Different 参考 -> 外部替换（文件打开、MCP 同步等）
    engine.loadDocument(controlledDoc);
  }, [engine, controlledDoc]);

  // 卸载时 Cleanup
  useEffect(() => {
    return () => {
      engineRef.current?.dispose();
      engineRef.current = null;
    };
  }, []);

  return <DesignEngineContext.Provider value={engine}>{children}</DesignEngineContext.Provider>;
}
