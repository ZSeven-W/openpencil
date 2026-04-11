import { Minus, Plus } from 'lucide-react';
import { useDesignEngine } from '../hooks/use-design-engine.js';
import { useViewport } from '../hooks/use-viewport.js';

export interface StatusBarProps {
  className?: string;
}

export function StatusBar({ className }: StatusBarProps) {
  const engine = useDesignEngine();
  const { zoom } = useViewport();

  const zoomPercent = Math.round(zoom * 100);

  const applyZoom = (newZoom: number) => {
    (engine as any).setZoom?.(newZoom);
  };

  const handleZoomOut = () => applyZoom(zoom / 1.2);
  const handleZoomIn = () => applyZoom(zoom * 1.2);
  const handleZoomReset = () => applyZoom(1);

  return (
    <div
      className={`h-7 bg-card border border-border rounded-lg flex items-center px-1 gap-0.5 shadow-lg ${className ?? ''}`}
    >
      <button
        type="button"
        onClick={handleZoomOut}
        aria-label="Zoom out"
        className="inline-flex items-center justify-center h-5 w-5 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
      >
        <Minus size={14} />
      </button>
      <button
        type="button"
        onClick={handleZoomReset}
        className="min-w-[48px] h-5 text-[11px] text-muted-foreground hover:text-foreground tabular-nums text-center cursor-pointer bg-transparent border-none"
        aria-label="Reset zoom"
      >
        {zoomPercent}%
      </button>
      <button
        type="button"
        onClick={handleZoomIn}
        aria-label="Zoom in"
        className="inline-flex items-center justify-center h-5 w-5 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
      >
        <Plus size={14} />
      </button>
    </div>
  );
}
