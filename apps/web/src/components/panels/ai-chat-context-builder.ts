import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';

/**
 * Build a context string from the current canvas state — selected nodes,
 * document node summary, and variable definitions.
 */
export function buildContextString(): string {
  const selectedIds = useCanvasStore.getState().selection.selectedIds;
  const { getFlatNodes, document: doc } = useDocumentStore.getState();
  const flatNodes = getFlatNodes();

  const parts: string[] = [];

  if (flatNodes.length > 0) {
    const summary = flatNodes
      .slice(0, 20)
      .map((n) => `${n.type}:${n.name ?? n.id}`)
      .join(', ');
    parts.push(`Document has ${flatNodes.length} nodes: ${summary}`);
  }

  if (selectedIds.length > 0) {
    const selectedNodes = selectedIds
      .map((id) => useDocumentStore.getState().getNodeById(id))
      .filter(Boolean);
    const selectedSummary = selectedNodes
      .map((n) => {
        const dims = 'width' in n! && 'height' in n! ? ` (${n!.width}x${n!.height})` : '';
        return `${n!.type}:${n!.name ?? n!.id}${dims}`;
      })
      .join(', ');
    parts.push(`Selected: ${selectedSummary}`);
  }

  // Include variable summary so chat mode also knows about design tokens
  if (doc.variables && Object.keys(doc.variables).length > 0) {
    const varNames = Object.entries(doc.variables)
      .map(([n, d]) => `$${n}(${d.type})`)
      .join(', ');
    parts.push(`Variables: ${varNames}`);
  }

  return parts.length > 0 ? `\n\n[Canvas context: ${parts.join('. ')}]` : '';
}
