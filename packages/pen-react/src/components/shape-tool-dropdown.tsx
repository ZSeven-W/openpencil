import { useState, useRef, useEffect } from 'react';
import type { ReactNode } from 'react';
import { Square, Circle, Minus, Triangle, Spline } from 'lucide-react';
import type { ToolType } from '@zseven-w/pen-types';
import { useActiveTool } from '../hooks/use-active-tool.js';

const SHAPE_TOOLS: Array<{ tool: ToolType; icon: typeof Square; label: string }> = [
  { tool: 'rectangle', icon: Square, label: 'Rectangle' },
  { tool: 'ellipse', icon: Circle, label: 'Ellipse' },
  { tool: 'line', icon: Minus, label: 'Line' },
  { tool: 'polygon', icon: Triangle, label: 'Polygon' },
  { tool: 'path', icon: Spline, label: 'Pen' },
];

const DRAWING_TOOLS = new Set<ToolType>(['rectangle', 'ellipse', 'line', 'polygon', 'path']);

export interface ShapeToolDropdownProps {
  /** Extra items to show at the bottom (e.g. icon picker, image import). */
  trailing?: ReactNode;
}

/**
 * Shape tool dropdown with submenu for shape types.
 * Shows the last-used shape tool as the main button.
 * The `trailing` slot allows apps to inject additional items (icon picker, image import).
 */
export function ShapeToolDropdown({ trailing }: ShapeToolDropdownProps) {
  const [activeTool, setActiveTool] = useActiveTool();
  const [lastShapeTool, setLastShapeTool] = useState<ToolType>('rectangle');
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const isActive = DRAWING_TOOLS.has(activeTool);
  const displayTool =
    SHAPE_TOOLS.find((s) => s.tool === (isActive ? activeTool : lastShapeTool)) ?? SHAPE_TOOLS[0];

  useEffect(() => {
    if (DRAWING_TOOLS.has(activeTool)) {
      setLastShapeTool(activeTool);
    }
  }, [activeTool]);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  const handleSelect = (tool: ToolType) => {
    setActiveTool(tool);
    setLastShapeTool(tool);
    setOpen(false);
  };

  const DisplayIcon = displayTool.icon;

  return (
    <div ref={containerRef} className="relative">
      <div className="flex items-center">
        <button
          type="button"
          onClick={() => setActiveTool(displayTool.tool)}
          aria-label={displayTool.label}
          aria-pressed={isActive}
          className={`inline-flex items-center justify-center h-8 w-7 rounded-l-lg transition-colors [&_svg]:size-5 [&_svg]:shrink-0 ${
            isActive
              ? 'bg-primary text-primary-foreground'
              : 'text-muted-foreground hover:bg-muted hover:text-foreground'
          }`}
        >
          <DisplayIcon size={20} strokeWidth={1.5} />
        </button>
        <button
          type="button"
          onClick={() => setOpen(!open)}
          aria-label="Shape options"
          className={`inline-flex items-center justify-center h-8 w-3 rounded-r-lg transition-colors ${
            isActive
              ? 'bg-primary text-primary-foreground'
              : 'text-muted-foreground hover:bg-muted hover:text-foreground'
          }`}
        >
          <svg viewBox="0 0 6 4" className="w-1.5 h-1 fill-current">
            <path d="M0 0l3 4 3-4z" />
          </svg>
        </button>
      </div>

      {open && (
        <div className="absolute left-full top-0 ml-1 bg-card border border-border rounded-lg shadow-lg py-1 z-20 min-w-36">
          {SHAPE_TOOLS.map((s) => {
            const Icon = s.icon;
            return (
              <button
                key={s.tool}
                type="button"
                onClick={() => handleSelect(s.tool)}
                className="flex items-center gap-2 w-full px-3 py-1.5 text-sm text-foreground hover:bg-muted transition-colors"
              >
                <Icon size={16} strokeWidth={1.5} />
                {s.label}
              </button>
            );
          })}
          {trailing && (
            <>
              <div className="h-px bg-border my-1" />
              {trailing}
            </>
          )}
        </div>
      )}
    </div>
  );
}
