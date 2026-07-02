import { useState, useMemo, useCallback, useRef } from 'react';
import { X, Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import VariableRow from './variable-row';
import { ThemeTabsHeader, VariantColumnsHeader } from './variable-theme-manager';
import VariableEditor from './variable-editor';

const DEFAULT_THEME_AXIS = 'Theme-1';
const DEFAULT_THEME_VALUES = ['Default'];
const MIN_WIDTH = 480;
const MIN_HEIGHT = 240;
const DEFAULT_WIDTH = 820;
const DEFAULT_HEIGHT = 480;

export default function VariablesPanel() {
  const { t } = useTranslation();
  const variables = useDocumentStore((s) => s.document.variables);
  const themes = useDocumentStore((s) => s.document.themes);
  const setVariable = useDocumentStore((s) => s.setVariable);
  const removeVariable = useDocumentStore((s) => s.removeVariable);
  const renameVariable = useDocumentStore((s) => s.renameVariable);
  const setThemes = useDocumentStore((s) => s.setThemes);
  const toggleVariablesPanel = useCanvasStore((s) => s.toggleVariablesPanel);

  const [search, setSearch] = useState('');
  const [activeAxis, setActiveAxis] = useState<string | null>(null);
  const [panelWidth, setPanelWidth] = useState(DEFAULT_WIDTH);
  const [panelHeight, setPanelHeight] = useState(DEFAULT_HEIGHT);

  const panelRef = useRef<HTMLDivElement>(null);
  const resizeRef = useRef<{
    edge: 'right' | 'bottom' | 'corner';
    startX: number;
    startY: number;
    startW: number;
    startH: number;
  } | null>(null);

  /* --- Resize --- */
  const handleResizeStart = useCallback(
    (edge: 'right' | 'bottom' | 'corner', e: React.PointerEvent) => {
      e.preventDefault();
      e.stopPropagation();
      resizeRef.current = {
        edge,
        startX: e.clientX,
        startY: e.clientY,
        startW: panelWidth,
        startH: panelHeight,
      };
      e.currentTarget.setPointerCapture(e.pointerId);
    },
    [panelWidth, panelHeight],
  );

  const handleResizeMove = useCallback((e: React.PointerEvent) => {
    if (!resizeRef.current) return;
    e.preventDefault();
    const { edge, startX, startY, startW, startH } = resizeRef.current;
    const container = panelRef.current?.parentElement;
    const maxW = container ? container.clientWidth - 72 : 1400;
    const maxH = container ? container.clientHeight - 16 : 900;
    if (edge === 'right' || edge === 'corner')
      setPanelWidth(Math.max(MIN_WIDTH, Math.min(maxW, startW + e.clientX - startX)));
    if (edge === 'bottom' || edge === 'corner')
      setPanelHeight(Math.max(MIN_HEIGHT, Math.min(maxH, startH + e.clientY - startY)));
  }, []);

  const handleResizeEnd = useCallback((e: React.PointerEvent) => {
    if (!resizeRef.current) return;
    resizeRef.current = null;
    e.currentTarget.releasePointerCapture(e.pointerId);
  }, []);

  /* --- Theme axes & variants --- */
  const themeAxes = useMemo(() => {
    if (!themes) return [];
    return Object.keys(themes);
  }, [themes]);

  const currentAxis = useMemo(() => {
    if (activeAxis && themes?.[activeAxis]) return activeAxis;
    if (themeAxes.length > 0) return themeAxes[0];
    return null;
  }, [activeAxis, themes, themeAxes]);

  const themeValues = useMemo(() => {
    if (!currentAxis || !themes?.[currentAxis]) return DEFAULT_THEME_VALUES;
    return themes[currentAxis].length > 0 ? themes[currentAxis] : DEFAULT_THEME_VALUES;
  }, [themes, currentAxis]);

  const themeAxis = currentAxis ?? DEFAULT_THEME_AXIS;

  const ensureThemes = useCallback(() => {
    if (!themes || Object.keys(themes).length === 0) {
      setThemes({ [DEFAULT_THEME_AXIS]: DEFAULT_THEME_VALUES });
    }
  }, [themes, setThemes]);

  const entries = useMemo(() => {
    if (!variables) return [];
    return Object.entries(variables)
      .filter(([n]) => !search || n.toLowerCase().includes(search.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b));
  }, [variables, search]);

  return (
    <div
      ref={panelRef}
      className="absolute left-14 top-2 z-20 flex flex-col select-none"
      style={{ width: panelWidth, height: panelHeight }}
    >
      {/* Background layer with rounded corners */}
      <div className="absolute inset-0 bg-card/95 backdrop-blur-sm border border-border/80 rounded-2xl shadow-2xl pointer-events-none" />

      {/* Header: Theme tabs | Presets | Close */}
      <div className="relative h-11 flex items-center px-4 shrink-0 gap-1 z-20">
        <ThemeTabsHeader
          themes={themes}
          variables={variables}
          setThemes={setThemes}
          setVariable={setVariable}
          currentAxis={currentAxis}
          themeAxes={themeAxes}
          onActiveAxisChange={setActiveAxis}
        />

        <div className="flex-1" />

        <button
          type="button"
          onClick={toggleVariablesPanel}
          className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary/60 transition-colors shrink-0"
          title={t('variables.closeShortcut')}
        >
          <X size={16} />
        </button>
      </div>

      {/* Column headers: Name | Default | Variant-1 | ... | + */}
      <VariantColumnsHeader
        themeValues={themeValues}
        themeAxis={themeAxis}
        themes={themes}
        currentAxis={currentAxis}
        setThemes={setThemes}
        ensureThemes={ensureThemes}
      />

      {/* Search */}
      {entries.length > 6 && (
        <div className="relative px-4 py-2 shrink-0 border-b border-border/30">
          <div className="flex items-center gap-2 bg-secondary/40 rounded-lg px-2.5 h-7 border border-transparent focus-within:border-ring transition-colors">
            <Search size={13} className="text-muted-foreground/60 shrink-0" />
            <input
              type="text"
              placeholder={t('variables.searchVariables')}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="flex-1 bg-transparent text-foreground text-[12px] focus:outline-none placeholder:text-muted-foreground/40"
            />
          </div>
        </div>
      )}

      {/* Variable rows */}
      <div className="relative flex-1 overflow-y-auto overflow-x-auto min-h-0 px-2 py-0.5">
        {entries.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full gap-1.5">
            <span className="text-[13px] text-muted-foreground/50">
              {search ? t('variables.noMatch') : t('variables.noDefined')}
            </span>
          </div>
        )}
        {entries.map(([varName, def]) => (
          <VariableRow
            key={varName}
            name={varName}
            definition={def}
            themeValues={themeValues}
            themeAxis={themeAxis}
            onUpdateValue={(n, d) => setVariable(n, d)}
            onRename={(o, n) => renameVariable(o, n)}
            onDelete={(n) => removeVariable(n)}
          />
        ))}
      </div>

      {/* Footer: Add variable */}
      <VariableEditor
        variables={variables}
        themes={themes}
        themeAxis={themeAxis}
        setVariable={setVariable}
        ensureThemes={ensureThemes}
      />

      {/* Resize handles */}
      <div
        className="absolute top-0 right-0 w-1.5 h-full cursor-ew-resize hover:bg-primary/10 transition-colors z-10"
        onPointerDown={(e) => handleResizeStart('right', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
      <div
        className="absolute bottom-0 left-0 w-full h-1.5 cursor-ns-resize hover:bg-primary/10 transition-colors z-10"
        onPointerDown={(e) => handleResizeStart('bottom', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
      <div
        className="absolute bottom-0 right-0 w-3 h-3 cursor-nwse-resize hover:bg-primary/15 transition-colors rounded-br-2xl z-10"
        onPointerDown={(e) => handleResizeStart('corner', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
    </div>
  );
}
