import type { ReactNode } from 'react';
import { MousePointer2, Type, Frame, Hand, Undo2, Redo2 } from 'lucide-react';
import { ToolButton } from './tool-button.js';
import { ShapeToolDropdown } from './shape-tool-dropdown.js';
import { useHistory } from '../hooks/use-history.js';

export interface CoreToolbarProps {
  /** Extra buttons to render after the core tools (app-specific: variables, design.md, etc.). */
  trailing?: ReactNode;
  /** Extra items for the shape tool dropdown (icon picker, image import). */
  shapeTrailing?: ReactNode;
  className?: string;
}

/**
 * Core editor toolbar with standard design tool buttons.
 *
 * Includes: Select, Shape dropdown, Text, Frame, Hand, Undo, Redo.
 * App-specific buttons are injected via the `trailing` slot.
 */
export function CoreToolbar({ trailing, shapeTrailing, className }: CoreToolbarProps) {
  const { canUndo, canRedo, undo, redo } = useHistory();

  return (
    <div
      className={`bg-card border border-border rounded-xl flex flex-col items-center py-2 gap-1 shadow-lg ${className ?? ''}`}
    >
      <ToolButton
        tool="select"
        icon={<MousePointer2 size={20} strokeWidth={1.5} />}
        label="Select"
        shortcut="V"
      />
      <ShapeToolDropdown trailing={shapeTrailing} />
      <ToolButton
        tool="text"
        icon={<Type size={20} strokeWidth={1.5} />}
        label="Text"
        shortcut="T"
      />
      <ToolButton
        tool="frame"
        icon={<Frame size={20} strokeWidth={1.5} />}
        label="Frame"
        shortcut="F"
      />
      <ToolButton
        tool="hand"
        icon={<Hand size={20} strokeWidth={1.5} />}
        label="Hand"
        shortcut="H"
      />

      {/* Separator */}
      <div className="h-px w-8 bg-border my-1" />

      {/* Undo / Redo */}
      <button
        type="button"
        onClick={undo}
        disabled={!canUndo}
        aria-label="Undo"
        title="Undo (Cmd+Z)"
        className="inline-flex items-center justify-center h-8 min-w-8 px-1.5 rounded-lg transition-colors text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40 disabled:pointer-events-none"
      >
        <Undo2 size={18} strokeWidth={1.5} />
      </button>
      <button
        type="button"
        onClick={redo}
        disabled={!canRedo}
        aria-label="Redo"
        title="Redo (Cmd+Shift+Z)"
        className="inline-flex items-center justify-center h-8 min-w-8 px-1.5 rounded-lg transition-colors text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-40 disabled:pointer-events-none"
      >
        <Redo2 size={18} strokeWidth={1.5} />
      </button>

      {/* Trailing slot for app-specific buttons */}
      {trailing && (
        <>
          <div className="h-px w-8 bg-border my-1" />
          {trailing}
        </>
      )}
    </div>
  );
}
