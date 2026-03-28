import { useCallback } from 'react';
import type { KitComponent, UIKit } from '@/types/uikit';
import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { getCanvasSize } from '@/canvas/skia-engine-ref';
import { cloneNodeWithNewIds } from '@/stores/document-tree-utils';
import { findReusableNode, deepCloneNode, collectVariableRefs } from '@/uikit/kit-utils';
import NodePreviewSvg from './node-preview-svg';

interface ComponentBrowserCardProps {
  component: KitComponent;
  kit: UIKit;
}

export default function ComponentBrowserCard({ component, kit }: ComponentBrowserCardProps) {
  const handleInsert = useCallback(() => {
    const { addNode, document } = useDocumentStore.getState();
    const { viewport } = useCanvasStore.getState();

    const kitNode = findReusableNode(kit.document, component.id);
    if (!kitNode) return;

    // Copy referenced variables from the kit document
    if (kit.document.variables) {
      const refs = collectVariableRefs(kitNode);
      const { setVariable } = useDocumentStore.getState();
      for (const ref of refs) {
        const name = ref.startsWith('$') ? ref.slice(1) : ref;
        const varDef = kit.document.variables[name];
        if (varDef && !document.variables?.[name]) {
          setVariable(name, varDef);
        }
      }
    }

    // Deep clone with new IDs, remove reusable flag so it's standalone
    const cloned = cloneNodeWithNewIds(deepCloneNode(kitNode));
    if ('reusable' in cloned) {
      delete (cloned as unknown as Record<string, unknown>).reusable;
    }

    // Place at viewport center
    const { width: canvasW, height: canvasH } = getCanvasSize();
    const centerX = (-viewport.panX + canvasW / 2) / viewport.zoom;
    const centerY = (-viewport.panY + canvasH / 2) / viewport.zoom;
    cloned.x = centerX - component.width / 2;
    cloned.y = centerY - component.height / 2;
    cloned.name = component.name;

    addNode(null, cloned);
    useCanvasStore.getState().setSelection([cloned.id], cloned.id);
  }, [component, kit]);

  const kitNode = findReusableNode(kit.document, component.id);

  return (
    <button
      type="button"
      onClick={handleInsert}
      className="flex flex-col items-center gap-2 p-3 rounded-lg border border-border bg-card hover:bg-muted transition-colors cursor-pointer group"
    >
      <div className="flex items-center justify-center w-full h-16">
        {kitNode ? (
          <NodePreviewSvg
            node={kitNode}
            maxWidth={120}
            maxHeight={64}
            variables={kit.document.variables}
          />
        ) : (
          <div className="w-16 h-8 rounded bg-muted" />
        )}
      </div>
      <span className="text-xs text-foreground truncate w-full text-center">{component.name}</span>
    </button>
  );
}
