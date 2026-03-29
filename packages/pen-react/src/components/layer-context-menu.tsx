import { useEffect, useRef } from 'react';
import {
  Trash2,
  Copy,
  Group,
  Lock,
  EyeOff,
  Component,
  Unlink,
  SquaresUnite,
  SquaresSubtract,
  SquaresIntersect,
} from 'lucide-react';

export interface LayerContextMenuProps {
  x: number;
  y: number;
  nodeId: string;
  canGroup: boolean;
  canBoolean: boolean;
  canCreateComponent: boolean;
  isReusable: boolean;
  isInstance: boolean;
  onAction: (action: string) => void;
  onClose: () => void;
}

interface MenuItem {
  action: string;
  label: string;
  icon: React.ComponentType<{ size?: number }>;
  requireGroup?: boolean;
  requireBoolean?: boolean;
  requireCreateComponent?: boolean;
  requireReusable?: boolean;
  requireInstance?: boolean;
}

const MENU_ITEMS: MenuItem[] = [
  { action: 'duplicate', label: 'Duplicate', icon: Copy },
  { action: 'delete', label: 'Delete', icon: Trash2 },
  { action: 'group', label: 'Group Selection', icon: Group, requireGroup: true },
  { action: 'boolean-union', label: 'Boolean Union', icon: SquaresUnite, requireBoolean: true },
  {
    action: 'boolean-subtract',
    label: 'Boolean Subtract',
    icon: SquaresSubtract,
    requireBoolean: true,
  },
  {
    action: 'boolean-intersect',
    label: 'Boolean Intersect',
    icon: SquaresIntersect,
    requireBoolean: true,
  },
  {
    action: 'make-component',
    label: 'Create Component',
    icon: Component,
    requireCreateComponent: true,
  },
  { action: 'detach-component', label: 'Detach Component', icon: Unlink, requireReusable: true },
  { action: 'detach-component', label: 'Detach Instance', icon: Unlink, requireInstance: true },
  { action: 'lock', label: 'Toggle Lock', icon: Lock },
  { action: 'hide', label: 'Toggle Visibility', icon: EyeOff },
];

export function LayerContextMenu({
  x,
  y,
  canGroup,
  canBoolean,
  canCreateComponent,
  isReusable,
  isInstance,
  onAction,
  onClose,
}: LayerContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('mousedown', handleClick);
    document.addEventListener('keydown', handleKey);
    return () => {
      document.removeEventListener('mousedown', handleClick);
      document.removeEventListener('keydown', handleKey);
    };
  }, [onClose]);

  const visibleItems = MENU_ITEMS.filter(
    (item) =>
      (!item.requireGroup || canGroup) &&
      (!item.requireBoolean || canBoolean) &&
      (!item.requireCreateComponent || canCreateComponent) &&
      (!item.requireReusable || isReusable) &&
      (!item.requireInstance || isInstance),
  );

  return (
    <div
      ref={menuRef}
      className="fixed z-50 bg-card border border-border rounded-md shadow-lg py-1 min-w-[160px]"
      style={{ left: x, top: y }}
    >
      {visibleItems.map((item, idx) => {
        const Icon = item.icon;
        return (
          <button
            key={`${item.action}-${idx}`}
            type="button"
            className="w-full flex items-center gap-2 px-3 py-1.5 text-xs text-foreground hover:bg-muted hover:text-foreground text-left transition-colors"
            onClick={() => onAction(item.action)}
          >
            <Icon size={12} />
            {item.label}
          </button>
        );
      })}
    </div>
  );
}
