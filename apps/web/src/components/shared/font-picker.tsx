import { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import { cn } from '@/lib/utils';
import { useSystemFonts, type FontInfo } from '@/hooks/use-system-fonts';
import { ChevronDown, Search, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface FontPickerProps {
  value: string;
  onChange: (fontFamily: string) => void;
  className?: string;
}

/** Extract display name from a font value like "Arial, sans-serif" → "Arial" */
function displayName(value: string): string {
  return value.split(',')[0].trim().replace(/['"]/g, '');
}

export default function FontPicker({ value, onChange, className }: FontPickerProps) {
  const { t } = useTranslation();
  const { allFonts, loading, permissionState, requestAccess } = useSystemFonts();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const [highlightIndex, setHighlightIndex] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleToggle = useCallback(() => {
    const opening = !open;
    setOpen(opening);
    if (opening && permissionState === 'prompt') {
      requestAccess(); // user gesture context — browser shows permission prompt
    }
  }, [open, permissionState, requestAccess]);

  // Filter fonts by search
  const filtered = useMemo(() => {
    if (!search) return allFonts;
    const q = search.toLowerCase();
    return allFonts.filter((f) => f.family.toLowerCase().includes(q));
  }, [allFonts, search]);

  // Group into bundled and system
  const bundled = useMemo(() => filtered.filter((f) => f.source === 'bundled'), [filtered]);
  const system = useMemo(() => filtered.filter((f) => f.source === 'system'), [filtered]);

  // Flat list for keyboard navigation
  const flatList = useMemo(() => {
    const items: FontInfo[] = [];
    items.push(...bundled);
    items.push(...system);
    return items;
  }, [bundled, system]);

  const handleSelect = useCallback(
    (font: FontInfo) => {
      onChange(font.family);
      setOpen(false);
      setSearch('');
      setHighlightIndex(-1);
    },
    [onChange],
  );

  // Close on click outside
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
        setSearch('');
        setHighlightIndex(-1);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  // Focus search input when opened
  useEffect(() => {
    if (open) {
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Scroll highlighted item into view
  useEffect(() => {
    if (highlightIndex < 0 || !listRef.current) return;
    const items = listRef.current.querySelectorAll('[data-font-item]');
    items[highlightIndex]?.scrollIntoView({ block: 'nearest' });
  }, [highlightIndex]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!open) {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        handleToggle();
      }
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setHighlightIndex((i) => Math.min(i + 1, flatList.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setHighlightIndex((i) => Math.max(i - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (highlightIndex >= 0 && highlightIndex < flatList.length) {
          handleSelect(flatList[highlightIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        setOpen(false);
        setSearch('');
        setHighlightIndex(-1);
        break;
    }
  };

  // Reset highlight when search changes
  useEffect(() => {
    setHighlightIndex(search ? 0 : -1);
  }, [search]);

  const currentDisplay = displayName(value);

  return (
    <div ref={containerRef} className={cn('relative', className)} onKeyDown={handleKeyDown}>
      {/* Trigger button */}
      <button
        type="button"
        onClick={handleToggle}
        className={cn(
          'flex items-center justify-between w-full h-6 px-2 text-[11px] rounded-md',
          'border border-border bg-card text-foreground',
          'hover:bg-secondary/50 transition-colors',
          'focus:outline-none focus:ring-1 focus:ring-ring',
        )}
        style={{ fontFamily: value }}
      >
        <span className="truncate">{currentDisplay}</span>
        {loading ? (
          <Loader2 className="w-3 h-3 shrink-0 text-muted-foreground ml-1 animate-spin" />
        ) : (
          <ChevronDown className="w-3 h-3 shrink-0 text-muted-foreground ml-1" />
        )}
      </button>

      {/* Dropdown */}
      {open && (
        <div
          className={cn(
            'absolute z-50 top-full left-0 mt-1 w-56',
            'rounded-md border border-border bg-card shadow-lg',
            'flex flex-col max-h-72',
          )}
        >
          {/* Search */}
          <div className="flex items-center gap-1.5 px-2 py-1.5 border-b border-border">
            <Search className="w-3 h-3 text-muted-foreground shrink-0" />
            <input
              ref={inputRef}
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={t('text.font.search')}
              className={cn(
                'flex-1 bg-transparent text-[11px] text-foreground',
                'placeholder:text-muted-foreground',
                'outline-none border-none p-0',
              )}
            />
          </div>

          {/* Font list */}
          <div ref={listRef} className="overflow-y-auto flex-1 py-1">
            {loading && (
              <div className="flex items-center justify-center py-3 gap-1.5 text-[11px] text-muted-foreground">
                <Loader2 className="w-3 h-3 animate-spin" />
                {t('text.font.loading')}
              </div>
            )}

            {/* Permission denied info — user must change browser settings */}
            {permissionState === 'denied' && (
              <div className="px-2 py-1.5 border-b border-border">
                <p className="text-[9px] text-muted-foreground leading-tight">
                  System fonts unavailable. Allow local fonts in browser settings to enable them.
                </p>
              </div>
            )}

            {/* Bundled fonts group */}
            {bundled.length > 0 && (
              <>
                <div className="px-2 py-0.5 text-[9px] font-medium text-muted-foreground uppercase tracking-wider">
                  {t('text.font.bundled')}
                </div>
                {bundled.map((font, i) => (
                  <FontItem
                    key={font.family}
                    font={font}
                    selected={displayName(value) === font.family}
                    highlighted={highlightIndex === i}
                    onSelect={handleSelect}
                  />
                ))}
              </>
            )}

            {/* System fonts group */}
            {system.length > 0 && (
              <>
                <div className="px-2 py-0.5 mt-1 text-[9px] font-medium text-muted-foreground uppercase tracking-wider">
                  {t('text.font.system')}
                </div>
                {system.map((font, i) => (
                  <FontItem
                    key={font.family}
                    font={font}
                    selected={displayName(value) === font.family}
                    highlighted={highlightIndex === bundled.length + i}
                    onSelect={handleSelect}
                  />
                ))}
              </>
            )}

            {!loading && filtered.length === 0 && (
              <div className="px-2 py-3 text-[11px] text-muted-foreground text-center">
                {t('text.font.noResults')}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function FontItem({
  font,
  selected,
  highlighted,
  onSelect,
}: {
  font: FontInfo;
  selected: boolean;
  highlighted: boolean;
  onSelect: (font: FontInfo) => void;
}) {
  return (
    <button
      type="button"
      data-font-item
      onClick={() => onSelect(font)}
      className={cn(
        'w-full text-left px-2 py-1 text-[11px] truncate',
        'transition-colors cursor-pointer',
        selected
          ? 'bg-primary/10 text-foreground'
          : highlighted
            ? 'bg-secondary text-foreground'
            : 'text-foreground hover:bg-secondary/50',
      )}
      style={{ fontFamily: font.family }}
    >
      {font.family}
    </button>
  );
}
