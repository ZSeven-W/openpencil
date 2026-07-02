import { useState, useRef, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useDesignMdAIGenerator } from '@/components/panels/design-md-ai-generator';
import {
  useDesignMdActions,
  DesignMdHeaderActions,
  DesignMdEmptyState,
  DesignMdGeneratingOverlay,
} from '@/components/panels/design-md-actions';
import { DesignMdContent } from '@/components/panels/design-md-editor';

const MIN_WIDTH = 420;
const MIN_HEIGHT = 300;
const DEFAULT_WIDTH = 560;
const DEFAULT_HEIGHT = 520;

// ---------------------------------------------------------------------------
// Panel component
// ---------------------------------------------------------------------------

export default function DesignMdPanel() {
  const { t } = useTranslation();
  const { isGenerating, hasAI, handleAutoGenerate } = useDesignMdAIGenerator();
  const {
    designMd,
    handleImport,
    handleExport,
    handleSyncColor,
    handleSyncAllColors,
    handleClear,
  } = useDesignMdActions();

  const [panelWidth, setPanelWidth] = useState(DEFAULT_WIDTH);
  const [panelHeight, setPanelHeight] = useState(DEFAULT_HEIGHT);
  const [panelX, setPanelX] = useState(() => Math.round((window.innerWidth - DEFAULT_WIDTH) / 2));
  const [panelY, setPanelY] = useState(() => Math.round((window.innerHeight - DEFAULT_HEIGHT) / 2));

  const panelRef = useRef<HTMLDivElement>(null);
  const resizeRef = useRef<{
    edge: 'right' | 'bottom' | 'corner';
    startX: number;
    startY: number;
    startW: number;
    startH: number;
  } | null>(null);
  const dragRef = useRef<{
    startX: number;
    startY: number;
    startPanelX: number;
    startPanelY: number;
  } | null>(null);

  // Drag + resize handlers
  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const d = dragRef.current;
      if (d) {
        setPanelX(
          Math.max(0, Math.min(window.innerWidth - 100, d.startPanelX + (e.clientX - d.startX))),
        );
        setPanelY(
          Math.max(0, Math.min(window.innerHeight - 40, d.startPanelY + (e.clientY - d.startY))),
        );
        return;
      }
      const r = resizeRef.current;
      if (!r) return;
      const maxW = window.innerWidth - 72;
      const maxH = window.innerHeight - 72;
      if (r.edge === 'right' || r.edge === 'corner')
        setPanelWidth(Math.min(maxW, Math.max(MIN_WIDTH, r.startW + (e.clientX - r.startX))));
      if (r.edge === 'bottom' || r.edge === 'corner')
        setPanelHeight(Math.min(maxH, Math.max(MIN_HEIGHT, r.startH + (e.clientY - r.startY))));
    };
    const onUp = () => {
      dragRef.current = null;
      resizeRef.current = null;
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
  }, []);

  const startDrag = useCallback(
    (e: React.PointerEvent) => {
      if ((e.target as HTMLElement).closest('button')) return;
      e.preventDefault();
      dragRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        startPanelX: panelX,
        startPanelY: panelY,
      };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [panelX, panelY],
  );

  const startResize = useCallback(
    (edge: 'right' | 'bottom' | 'corner', e: React.PointerEvent) => {
      e.preventDefault();
      resizeRef.current = {
        edge,
        startX: e.clientX,
        startY: e.clientY,
        startW: panelWidth,
        startH: panelHeight,
      };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
    },
    [panelWidth, panelHeight],
  );

  // Check if designMd has any meaningful content beyond just raw text
  const hasContent =
    designMd &&
    (designMd.visualTheme ||
      (designMd.colorPalette && designMd.colorPalette.length > 0) ||
      designMd.typography ||
      designMd.componentStyles ||
      designMd.layoutPrinciples ||
      designMd.generationNotes);

  return (
    <div
      ref={panelRef}
      className="fixed z-50 bg-card border border-border rounded-xl shadow-2xl flex flex-col overflow-hidden"
      style={{ width: panelWidth, height: panelHeight, left: panelX, top: panelY }}
    >
      {/* Header — draggable */}
      <div
        className="flex items-center justify-between px-3 py-2 border-b border-border bg-muted/40 shrink-0 cursor-grab active:cursor-grabbing select-none"
        onPointerDown={startDrag}
      >
        <span className="text-xs font-medium text-foreground">{t('designMd.title')}</span>
        <DesignMdHeaderActions
          onImport={handleImport}
          onExport={handleExport}
          onAutoGenerate={handleAutoGenerate}
          isGenerating={isGenerating}
          hasAI={hasAI}
        />
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto min-h-0">
        {!hasContent ? (
          <DesignMdEmptyState
            designMd={designMd}
            onImport={handleImport}
            onAutoGenerate={handleAutoGenerate}
            onClear={handleClear}
            isGenerating={isGenerating}
            hasAI={hasAI}
          />
        ) : (
          <DesignMdContent
            designMd={designMd}
            onClear={handleClear}
            onSyncColor={handleSyncColor}
            onSyncAllColors={handleSyncAllColors}
          />
        )}
      </div>

      {/* Generating overlay */}
      {isGenerating && <DesignMdGeneratingOverlay onStop={handleAutoGenerate} />}

      {/* Resize handles */}
      <div
        className="absolute right-0 top-0 w-1.5 h-full cursor-ew-resize"
        onPointerDown={(e) => startResize('right', e)}
      />
      <div
        className="absolute bottom-0 left-0 w-full h-1.5 cursor-ns-resize"
        onPointerDown={(e) => startResize('bottom', e)}
      />
      <div
        className="absolute right-0 bottom-0 w-3 h-3 cursor-nwse-resize"
        onPointerDown={(e) => startResize('corner', e)}
      />
    </div>
  );
}
