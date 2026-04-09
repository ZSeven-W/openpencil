import type { ReactNode } from 'react';
import type { ToolType } from '@zseven-w/pen-types';
import { useActiveTool } from '../hooks/use-active-tool.js';

interface ToolButtonProps {
  tool: ToolType;
  icon: ReactNode;
  label: string;
  shortcut?: string;
}

/**
 * Reusable tool button that reads/writes the active tool via pen-engine.
 * Uses `isActive` conditional className (not Radix data-state) per code style guide.
 */
export function ToolButton({ tool, icon, label, shortcut }: ToolButtonProps) {
  const [activeTool, setActiveTool] = useActiveTool();
  const isActive = activeTool === tool;

  return (
    <button
      type="button"
      onClick={() => setActiveTool(tool)}
      aria-label={label}
      aria-pressed={isActive}
      title={shortcut ? `${label} (${shortcut})` : label}
      className={`inline-flex items-center justify-center h-8 min-w-8 px-1.5 rounded-lg transition-colors [&_svg]:size-5 [&_svg]:shrink-0 ${
        isActive
          ? 'bg-primary text-primary-foreground'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground'
      }`}
    >
      {icon}
    </button>
  );
}
