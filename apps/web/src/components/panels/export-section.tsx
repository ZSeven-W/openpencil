import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { PenNode } from '@zseven-w/pen-types';
import { findNodeInTree } from '@zseven-w/pen-core';
import { resolveRefs } from '@zseven-w/pen-renderer';
import SectionHeader from '@/components/shared/section-header';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { getSkiaEngineRef } from '@/canvas/skia-engine-ref';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore, getActivePageChildren, getAllChildren } from '@/stores/document-store';

const SCALE_OPTIONS = [
  { value: '1', label: '1x' },
  { value: '2', label: '2x' },
  { value: '3', label: '3x' },
];

const FORMAT_OPTIONS = [
  { value: 'png', label: 'PNG' },
  { value: 'jpeg', label: 'JPEG' },
  { value: 'webp', label: 'WEBP' },
] as const;

type ExportFormat = (typeof FORMAT_OPTIONS)[number]['value'];

interface ExportSectionProps {
  nodeId: string;
  nodeName: string;
}

export default function ExportSection({ nodeId, nodeName }: ExportSectionProps) {
  const { t } = useTranslation();
  const [scale, setScale] = useState('1');
  const [format, setFormat] = useState<ExportFormat>('png');
  const [exporting, setExporting] = useState(false);

  const handleExport = () => {
    const engine = getSkiaEngineRef();
    if (!engine) {
      console.error('[ExportSection] SkiaEngine not available');
      return;
    }

    // 1. Resolve target subtree in the document tree.
    //    Use post-resolveRefs tree so descendant IDs match what's in
    //    engine.renderNodes (ref instances get remapped descendant IDs).
    const docState = useDocumentStore.getState();
    const activePageId = useCanvasStore.getState().activePageId;
    const pageChildren = getActivePageChildren(docState.document, activePageId);
    const allNodes = getAllChildren(docState.document);
    const resolvedTree = resolveRefs(pageChildren, allNodes);
    const targetNode = findNodeInTree(resolvedTree, nodeId);
    if (!targetNode) {
      console.error('[ExportSection] Target node not found in document tree');
      return;
    }

    const subtreeIds = new Set<string>();
    const collectIds = (n: PenNode) => {
      subtreeIds.add(n.id);
      if ('children' in n && n.children) {
        for (const child of n.children) collectIds(child);
      }
    };
    collectIds(targetNode);

    // 2. Find target render node to get absolute bounds.
    const rootRn = engine.renderNodes.find((rn) => rn.node.id === nodeId);
    if (!rootRn) {
      console.error('[ExportSection] Target render node not found — node may be hidden');
      return;
    }
    const { absX: originX, absY: originY, absW: width, absH: height } = rootRn;
    if (width <= 0 || height <= 0) {
      console.error('[ExportSection] Target has zero dimensions');
      return;
    }

    // 3. Filter to subtree, preserving render order from the flattener.
    const subtreeRNs = engine.renderNodes.filter((rn) => subtreeIds.has(rn.node.id));

    // 4. Create offscreen surface sized to bounds × scale multiplier.
    const multiplier = Math.max(1, parseInt(scale, 10) || 1);
    const outW = Math.max(1, Math.ceil(width * multiplier));
    const outH = Math.max(1, Math.ceil(height * multiplier));

    const ck = engine.ck;
    const surface = ck.MakeSurface(outW, outH);
    if (!surface) {
      console.error('[ExportSection] Failed to create offscreen surface');
      return;
    }

    setExporting(true);
    try {
      const canvas = surface.getCanvas();
      // JPEG has no alpha — fill with white; PNG/WEBP preserve transparency.
      canvas.clear(format === 'jpeg' ? ck.WHITE : ck.TRANSPARENT);

      canvas.save();
      canvas.scale(multiplier, multiplier);
      canvas.translate(-originX, -originY);
      for (const rn of subtreeRNs) {
        engine.renderer.drawNode(canvas, rn);
      }
      canvas.restore();
      surface.flush();

      const img = surface.makeImageSnapshot();
      try {
        const fmtEnum =
          format === 'jpeg'
            ? ck.ImageFormat.JPEG
            : format === 'webp'
              ? ck.ImageFormat.WEBP
              : ck.ImageFormat.PNG;
        const quality = format === 'png' ? 100 : 92;
        const bytes = img.encodeToBytes(fmtEnum, quality);
        if (!bytes) {
          console.error('[ExportSection] Failed to encode image');
          return;
        }

        const mime =
          format === 'jpeg' ? 'image/jpeg' : format === 'webp' ? 'image/webp' : 'image/png';
        const ext = format === 'jpeg' ? 'jpg' : format;
        // Preserve CJK and word chars in filename; replace other punctuation with _
        const safeName = (nodeName || 'layer').replace(/[^\p{L}\p{N}_-]+/gu, '_') || 'layer';
        const filename = `${safeName}.${ext}`;

        // Copy into a plain ArrayBuffer so the Blob doesn't retain WASM memory
        const copy = new Uint8Array(bytes.length);
        copy.set(bytes);
        const blob = new Blob([copy], { type: mime });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(url);
      } finally {
        img.delete();
      }
    } finally {
      surface.delete();
      setExporting(false);
    }
  };

  return (
    <div className="space-y-1.5">
      <SectionHeader title={t('export.title')} />
      <div className="flex gap-1.5">
        <Select value={scale} onValueChange={setScale}>
          <SelectTrigger className="flex-1 h-6 text-[11px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {SCALE_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={format} onValueChange={(v) => setFormat(v as ExportFormat)}>
          <SelectTrigger className="flex-1 h-6 text-[11px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {FORMAT_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <Button
        variant="outline"
        size="sm"
        className="w-full text-xs"
        onClick={handleExport}
        disabled={exporting}
      >
        {t('export.exportLayer')}
      </Button>
    </div>
  );
}
