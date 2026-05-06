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
 * Reusable
 * 工具按钮，通过笔引擎使 reads/writes 成为活动工具。 Uses `isActive` 条件 className （不是
 Radix 数据状态）每个代码风格指南。
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
