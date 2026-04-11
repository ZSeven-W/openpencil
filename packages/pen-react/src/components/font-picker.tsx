import { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import { cn } from '../ui-primitives.js';
import { ChevronDown, Search, Loader2 } from 'lucide-react';

export interface FontInfo {
  family: string;
  source: 'bundled' | 'system';
}

/** Bundled font families (always available) */
const BUNDLED_FAMILIES = [
  'Inter',
  'Poppins',
  'Roboto',
  'Montserrat',
  'Open Sans',
  'Lato',
  'Raleway',
  'DM Sans',
  'Playfair Display',
  'Nunito',
  'Source Sans 3',
];

const FALLBACK_SYSTEM_FONTS = [
  'Arial',
  'Helvetica',
  'Helvetica Neue',
  'Georgia',
  'Times New Roman',
  'Courier New',
  'Verdana',
  'Trebuchet MS',
  'Tahoma',
  'Impact',
];

let cachedSystemFonts: string[] | null = null;
let fetchPromise: Promise<string[]> | null = null;

async function querySystemFonts(): Promise<string[]> {
  if (cachedSystemFonts) return cachedSystemFonts;
  if (fetchPromise) return fetchPromise;

  fetchPromise = (async () => {
    try {
      if ('queryLocalFonts' in window) {
        const fonts = await (
          window as unknown as { queryLocalFonts: () => Promise<Array<{ family: string }>> }
        ).queryLocalFonts();
        const families = new Set<string>();
        for (const font of fonts) {
          families.add(font.family);
        }
        const bundledSet = new Set(BUNDLED_FAMILIES.map((f) => f.toLowerCase()));
        const systemFonts = [...families]
          .filter((f) => !bundledSet.has(f.toLowerCase()))
          .sort((a, b) => a.localeCompare(b));
        cachedSystemFonts = systemFonts;
        return systemFonts;
      }
    } catch {
      // Permission denied or API not available
    }
    cachedSystemFonts = FALLBACK_SYSTEM_FONTS;
    return FALLBACK_SYSTEM_FONTS;
  })();

  return fetchPromise;
}

function useSystemFonts() {
  const [systemFonts, setSystemFonts] = useState<string[]>(cachedSystemFonts ?? []);
  const [loading, setLoading] = useState(!cachedSystemFonts);

  useEffect(() => {
    if (cachedSystemFonts) {
      setSystemFonts(cachedSystemFonts);
      setLoading(false);
      return;
    }
    let cancelled = false;
    querySystemFonts().then((fonts) => {
      if (!cancelled) {
        setSystemFonts(fonts);
        setLoading(false);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const allFonts: FontInfo[] = [
    ...BUNDLED_FAMILIES.map((f) => ({ family: f, source: 'bundled' as const })),
    ...systemFonts.map((f) => ({ family: f, source: 'system' as const })),
  ];

  return { allFonts, loading };
}

function displayName(value: string): string {
  return value.split(',')[0].trim().replace(/['"]/g, '');
}

export interface FontPickerProps {
  value: string;
  onChange: (fontFamily: string) => void;
  className?: string;
}

export function FontPicker({ value, onChange, className }: FontPickerProps) {
  const { allFonts, loading } = useSystemFonts();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const [highlightIndex, setHighlightIndex] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const filtered = useMemo(() => {
    if (!search) return allFonts;
    const q = search.toLowerCase();
    return allFonts.filter((f) => f.family.toLowerCase().includes(q));
  }, [allFonts, search]);

  const bundled = useMemo(() => filtered.filter((f) => f.source === 'bundled'), [filtered]);
  const system = useMemo(() => filtered.filter((f) => f.source === 'system'), [filtered]);

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

  useEffect(() => {
    if (open) {
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  useEffect(() => {
    if (highlightIndex < 0 || !listRef.current) return;
    const items = listRef.current.querySelectorAll('[data-font-item]');
    items[highlightIndex]?.scrollIntoView({ block: 'nearest' });
  }, [highlightIndex]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!open) {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        setOpen(true);
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

  useEffect(() => {
    setHighlightIndex(search ? 0 : -1);
  }, [search]);

  const currentDisplay = displayName(value);

  return (
    <div ref={containerRef} className={cn('relative', className)} onKeyDown={handleKeyDown}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className={cn(
          'flex items-center justify-between w-full h-6 px-2 text-[11px] rounded-md',
          'border border-border bg-card text-foreground',
          'hover:bg-secondary/50 transition-colors',
          'focus:outline-none focus:ring-1 focus:ring-ring',
        )}
        style={{ fontFamily: value }}
      >
        <span className="truncate">{currentDisplay}</span>
        <ChevronDown className="w-3 h-3 shrink-0 text-muted-foreground ml-1" />
      </button>

      {open && (
        <div
          className={cn(
            'absolute z-50 top-full left-0 mt-1 w-56',
            'rounded-md border border-border bg-card shadow-lg',
            'flex flex-col max-h-72',
          )}
        >
          <div className="flex items-center gap-1.5 px-2 py-1.5 border-b border-border">
            <Search className="w-3 h-3 text-muted-foreground shrink-0" />
            <input
              ref={inputRef}
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search fonts..."
              className={cn(
                'flex-1 bg-transparent text-[11px] text-foreground',
                'placeholder:text-muted-foreground',
                'outline-none border-none p-0',
              )}
            />
          </div>

          <div ref={listRef} className="overflow-y-auto flex-1 py-1">
            {loading && (
              <div className="flex items-center justify-center py-3 gap-1.5 text-[11px] text-muted-foreground">
                <Loader2 className="w-3 h-3 animate-spin" />
                Loading...
              </div>
            )}

            {bundled.length > 0 && (
              <>
                <div className="px-2 py-0.5 text-[9px] font-medium text-muted-foreground uppercase tracking-wider">
                  Bundled
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

            {system.length > 0 && (
              <>
                <div className="px-2 py-0.5 mt-1 text-[9px] font-medium text-muted-foreground uppercase tracking-wider">
                  System
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
                No fonts found
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
