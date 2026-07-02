import { useState, useRef, useEffect } from 'react';
import { ChevronDown } from 'lucide-react';
import { SectionHeader } from '../section-header.js';
import { IconPickerDialog } from '../icon-picker-dialog.js';
import type { PenNode } from '@zseven-w/pen-types';
import type { PenFill, PenStroke } from '@zseven-w/pen-types';
import type { IconPickerPosition } from '../icon-picker-dialog.js';

const POPULAR_COLLECTIONS = [
  { id: 'feather', name: 'Feather' },
  { id: 'lucide', name: 'Lucide' },
  { id: 'mdi', name: 'Material' },
  { id: 'ph', name: 'Phosphor' },
  { id: 'ri', name: 'Remix' },
  { id: 'tabler', name: 'Tabler' },
  { id: 'heroicons', name: 'Heroicons' },
  { id: 'fa6-solid', name: 'Font Awesome' },
  { id: 'simple-icons', name: 'Brand Icons' },
];

type PathNode = PenNode & { d?: string; iconId?: string; fill?: PenFill[]; stroke?: PenStroke };
type IconFontNode = PenNode & { iconFontFamily?: string; iconFontName?: string };

interface IconSectionProps {
  node: PathNode | IconFontNode;
  onUpdate: (updates: Partial<PathNode> | Partial<IconFontNode>) => void;
}

export function IconSection({ node, onUpdate }: IconSectionProps) {
  const [pickerOpen, setPickerOpen] = useState(false);
  const [pickerInitialQuery, setPickerInitialQuery] = useState('');
  const [pickerCollection, setPickerCollection] = useState<string | undefined>(undefined);
  const [pickerPosition, setPickerPosition] = useState<IconPickerPosition | undefined>(undefined);
  const [displayCollection, setDisplayCollection] = useState<string>('');

  const triggerRef = useRef<HTMLButtonElement>(null);
  const isIconFont = node.type === 'icon_font';

  const iconId = isIconFont
    ? `${(node as IconFontNode).iconFontFamily || 'lucide'}:${(node as IconFontNode).iconFontName}`
    : ((node as PathNode).iconId ?? '');
  const colonIdx = iconId.indexOf(':');
  const collection = colonIdx >= 0 ? iconId.slice(0, colonIdx) : '';
  const iconName = colonIdx >= 0 ? iconId.slice(colonIdx + 1) : iconId;

  useEffect(() => {
    setDisplayCollection(collection);
  }, [collection]);

  const openPicker = (query: string, collectionFilter?: string) => {
    if (triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect();
      setPickerPosition({ top: rect.top, right: 0 });
    }
    setPickerInitialQuery(query);
    setPickerCollection(collectionFilter);
    setPickerOpen(true);
  };

  const handleIconSelect = (svgText: string, newIconId: string) => {
    if (isIconFont) {
      const colonIdx2 = newIconId.indexOf(':');
      const newFamily = colonIdx2 >= 0 ? newIconId.slice(0, colonIdx2) : 'lucide';
      const newName = colonIdx2 >= 0 ? newIconId.slice(colonIdx2 + 1) : newIconId;
      onUpdate({
        iconFontName: newName,
        iconFontFamily: newFamily,
        name: newIconId,
      } as Partial<IconFontNode>);
      return;
    }

    // For path icons: update svgText reference (caller parses SVG externally)
    // Simplified: just store the iconId and let downstream handle SVG parsing
    onUpdate({ iconId: newIconId, name: newIconId } as Partial<PathNode>);
    void svgText; // caller can extend this
  };

  const handleLibraryChange = (newCollection: string) => {
    setDisplayCollection(newCollection);
    openPicker('', newCollection);
  };

  const handlePickerClose = () => {
    setDisplayCollection(collection);
    setPickerOpen(false);
  };

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Icon" />

      <button
        ref={triggerRef}
        type="button"
        onClick={() => openPicker('')}
        className="w-full h-7 bg-secondary text-foreground text-[11px] px-2 rounded border border-transparent hover:border-input flex items-center justify-between cursor-pointer transition-colors"
      >
        <span className="truncate">{iconName}</span>
        <ChevronDown size={11} className="text-muted-foreground shrink-0 ml-1" />
      </button>

      <div className="relative">
        <select
          value={displayCollection}
          onChange={(e) => handleLibraryChange(e.target.value)}
          className="w-full h-7 bg-secondary text-foreground text-[11px] px-2 rounded border border-transparent hover:border-input focus:border-ring focus:outline-none cursor-pointer transition-colors appearance-none"
        >
          {!POPULAR_COLLECTIONS.some((c) => c.id === displayCollection) && displayCollection && (
            <option value={displayCollection}>{displayCollection}</option>
          )}
          {POPULAR_COLLECTIONS.map((col) => (
            <option key={col.id} value={col.id}>
              {col.name}
            </option>
          ))}
        </select>
        <ChevronDown
          size={11}
          className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground"
        />
      </div>

      <IconPickerDialog
        open={pickerOpen}
        onClose={handlePickerClose}
        onSelect={handleIconSelect}
        initialQuery={pickerInitialQuery}
        collectionFilter={pickerCollection}
        currentIconId={iconId}
        position={pickerPosition}
      />
    </div>
  );
}
