import { useState, useRef, useEffect, memo } from 'react';
import { Plus, ChevronDown, Pencil, Trash2, BookMarked, Upload, Download } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import { useThemePresetStore } from '@/stores/theme-preset-store';
import { exportThemePreset, importThemePreset } from '@/utils/theme-preset-io';
import type { VariableDefinition } from '@/types/variables';

const DEFAULT_THEME_VALUES = ['Default'];

export interface ThemeManagerProps {
  themes: Record<string, string[]> | undefined;
  variables: Record<string, VariableDefinition> | undefined;
  setThemes: (themes: Record<string, string[]>) => void;
  setVariable: (name: string, def: VariableDefinition) => void;
  /** Currently displayed theme axis */
  currentAxis: string | null;
  themeAxis: string;
  themeValues: string[];
  themeAxes: string[];
  onActiveAxisChange: (axis: string | null) => void;
}

/** Theme tabs header row — manages axes, presets, and import/export. */
function ThemeTabsHeaderInner({
  themes,
  variables,
  setThemes,
  setVariable,
  currentAxis,
  themeAxes,
  onActiveAxisChange,
}: Omit<ThemeManagerProps, 'themeAxis' | 'themeValues'>) {
  const { t } = useTranslation();

  const presets = useThemePresetStore((s) => s.presets);
  const savePreset = useThemePresetStore((s) => s.savePreset);
  const deletePreset = useThemePresetStore((s) => s.deletePreset);

  const [activeThemeMenu, setActiveThemeMenu] = useState<string | null>(null);
  const [renamingTheme, setRenamingTheme] = useState<string | null>(null);
  const [renameThemeValue, setRenameThemeValue] = useState('');
  const [showPresetMenu, setShowPresetMenu] = useState(false);
  const [showPresetNameInput, setShowPresetNameInput] = useState(false);
  const [presetNameValue, setPresetNameValue] = useState('');

  const themeMenuRef = useRef<HTMLDivElement>(null);
  const presetMenuRef = useRef<HTMLDivElement>(null);
  const presetNameInputRef = useRef<HTMLInputElement>(null);
  const themeRenameInputRef = useRef<HTMLInputElement>(null);

  // Close menus on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (
        activeThemeMenu &&
        themeMenuRef.current &&
        !themeMenuRef.current.contains(e.target as Node)
      ) {
        setActiveThemeMenu(null);
        setRenamingTheme(null);
      }
      if (
        showPresetMenu &&
        presetMenuRef.current &&
        !presetMenuRef.current.contains(e.target as Node)
      ) {
        setShowPresetMenu(false);
        setShowPresetNameInput(false);
      }
    };
    if (activeThemeMenu || showPresetMenu) {
      document.addEventListener('mousedown', handler);
      return () => document.removeEventListener('mousedown', handler);
    }
  }, [activeThemeMenu, showPresetMenu]);

  useEffect(() => {
    if (showPresetNameInput && presetNameInputRef.current) {
      presetNameInputRef.current.focus();
      presetNameInputRef.current.select();
    }
  }, [showPresetNameInput]);

  useEffect(() => {
    if (renamingTheme && themeRenameInputRef.current) {
      themeRenameInputRef.current.focus();
      themeRenameInputRef.current.select();
    }
  }, [renamingTheme]);

  /* --- Theme actions --- */
  const handleAddTheme = () => {
    const current = themes ?? {};
    let counter = 1;
    let name = `Theme-${counter}`;
    while (current[name]) {
      counter++;
      name = `Theme-${counter}`;
    }
    setThemes({ ...current, [name]: ['Default'] });
    onActiveAxisChange(name);
  };

  const handleDeleteTheme = (axis: string) => {
    if (!themes) return;
    const updated = { ...themes };
    delete updated[axis];
    setThemes(updated);
    if (currentAxis === axis) onActiveAxisChange(null);
    setActiveThemeMenu(null);
  };

  const handleRenameTheme = (oldName: string, newName: string) => {
    setRenamingTheme(null);
    setActiveThemeMenu(null);
    if (!newName.trim() || newName === oldName) return;
    if (themes?.[newName]) return;
    const current = themes ?? {};
    const values = current[oldName] ?? DEFAULT_THEME_VALUES;
    const updated: Record<string, string[]> = {};
    for (const key of Object.keys(current)) {
      if (key === oldName) updated[newName] = values;
      else updated[key] = current[key];
    }
    setThemes(updated);
    if (currentAxis === oldName) onActiveAxisChange(newName);
  };

  /* --- Preset actions --- */
  const handleSavePreset = (name: string) => {
    if (!name.trim()) return;
    savePreset(name.trim(), themes ?? {}, variables ?? {});
    setShowPresetNameInput(false);
    setPresetNameValue('');
  };

  const handleLoadPreset = (preset: {
    themes: Record<string, string[]>;
    variables: Record<string, VariableDefinition>;
  }) => {
    const mergedThemes = { ...themes, ...preset.themes };
    setThemes(mergedThemes);
    const currentVars = variables ?? {};
    for (const [name, def] of Object.entries(preset.variables)) {
      if (!currentVars[name] || JSON.stringify(currentVars[name]) !== JSON.stringify(def)) {
        setVariable(name, def);
      }
    }
    setShowPresetMenu(false);
  };

  const handleImportFromFile = async () => {
    setShowPresetMenu(false);
    const result = await importThemePreset();
    if (!result) return;
    handleLoadPreset({ themes: result.themes, variables: result.variables });
  };

  const handleExportToFile = async () => {
    setShowPresetMenu(false);
    const name = 'theme-preset';
    await exportThemePreset(name, themes ?? {}, variables ?? {});
  };

  return (
    <>
      {/* Theme tabs */}
      {themeAxes.map((axis) => (
        <div
          key={axis}
          className="relative shrink-0"
          ref={activeThemeMenu === axis ? themeMenuRef : undefined}
        >
          {renamingTheme === axis ? (
            <input
              ref={themeRenameInputRef}
              type="text"
              value={renameThemeValue}
              onChange={(e) => setRenameThemeValue(e.target.value)}
              onBlur={() => handleRenameTheme(axis, renameThemeValue)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleRenameTheme(axis, renameThemeValue);
                if (e.key === 'Escape') {
                  setRenamingTheme(null);
                  setActiveThemeMenu(null);
                }
              }}
              className="text-[13px] text-foreground bg-secondary px-2 py-0.5 rounded-lg border border-ring focus:outline-none w-24"
            />
          ) : (
            <button
              type="button"
              onClick={() => {
                if (axis === currentAxis) {
                  setActiveThemeMenu(activeThemeMenu === axis ? null : axis);
                } else {
                  onActiveAxisChange(axis);
                  setActiveThemeMenu(null);
                }
              }}
              className={cn(
                'flex items-center gap-1 text-[13px] px-2 py-1 rounded-lg transition-colors whitespace-nowrap',
                axis === currentAxis
                  ? 'text-foreground font-medium'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {axis}
              {axis === currentAxis && (
                <ChevronDown
                  size={11}
                  className={cn(
                    'text-muted-foreground/60 transition-transform',
                    activeThemeMenu === axis && 'rotate-180',
                  )}
                />
              )}
            </button>
          )}
          {/* Theme dropdown: Rename / Delete */}
          {activeThemeMenu === axis && !renamingTheme && (
            <div className="absolute left-0 top-full z-50 mt-1 w-44 bg-popover border border-border rounded-xl shadow-xl py-1 animate-in fade-in slide-in-from-top-1 duration-150">
              <button
                type="button"
                onClick={() => {
                  setRenameThemeValue(axis);
                  setRenamingTheme(axis);
                  setActiveThemeMenu(null);
                }}
                className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
              >
                <Pencil size={14} className="text-muted-foreground" />
                {t('common.rename')}
              </button>
              {themeAxes.length > 1 && (
                <button
                  type="button"
                  onClick={() => handleDeleteTheme(axis)}
                  className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
                >
                  <Trash2 size={14} className="text-muted-foreground" />
                  {t('common.delete')}
                </button>
              )}
            </div>
          )}
        </div>
      ))}

      {/* + add theme */}
      <button
        type="button"
        onClick={handleAddTheme}
        className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary/60 transition-colors shrink-0"
        title={t('variables.addTheme')}
      >
        <Plus size={15} />
      </button>

      {/* Presets dropdown */}
      <div className="relative shrink-0" ref={presetMenuRef}>
        <button
          type="button"
          onClick={() => {
            setShowPresetMenu(!showPresetMenu);
            setShowPresetNameInput(false);
          }}
          className={cn(
            'flex items-center gap-1 text-[13px] px-2 py-1 rounded-lg transition-colors whitespace-nowrap',
            showPresetMenu
              ? 'text-foreground bg-secondary/60'
              : 'text-muted-foreground hover:text-foreground hover:bg-secondary/60',
          )}
        >
          <BookMarked size={13} />
          {t('variables.presets')}
          <ChevronDown
            size={11}
            className={cn(
              'text-muted-foreground/60 transition-transform',
              showPresetMenu && 'rotate-180',
            )}
          />
        </button>

        {showPresetMenu && (
          <div className="absolute right-0 top-full z-50 mt-1 w-56 bg-popover border border-border rounded-xl shadow-xl py-1 animate-in fade-in slide-in-from-top-1 duration-150">
            {/* Save current as preset */}
            {showPresetNameInput ? (
              <div className="px-3 py-2">
                <input
                  ref={presetNameInputRef}
                  type="text"
                  value={presetNameValue}
                  onChange={(e) => setPresetNameValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleSavePreset(presetNameValue);
                    if (e.key === 'Escape') setShowPresetNameInput(false);
                  }}
                  placeholder={t('variables.presetName')}
                  className="w-full text-[13px] text-foreground bg-secondary px-2 py-1 rounded-lg border border-ring focus:outline-none"
                />
              </div>
            ) : (
              <button
                type="button"
                onClick={() => {
                  setPresetNameValue('');
                  setShowPresetNameInput(true);
                }}
                className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
              >
                <BookMarked size={14} className="text-muted-foreground" />
                {t('variables.savePreset')}
              </button>
            )}

            {/* Separator */}
            <div className="h-px bg-border/50 my-1" />

            {/* Saved presets list */}
            {presets.length === 0 ? (
              <div className="px-3 py-2 text-[12px] text-muted-foreground/50">
                {t('variables.noPresets')}
              </div>
            ) : (
              presets.map((p) => (
                <div
                  key={p.id}
                  className="flex items-center gap-1 px-3 py-1.5 hover:bg-secondary/60 rounded-lg transition-colors group"
                >
                  <button
                    type="button"
                    onClick={() => handleLoadPreset(p)}
                    className="flex-1 text-left text-[13px] text-foreground truncate"
                  >
                    {p.name}
                  </button>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      deletePreset(p.id);
                    }}
                    className="opacity-0 group-hover:opacity-100 p-0.5 rounded text-muted-foreground hover:text-foreground transition-opacity"
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              ))
            )}

            {/* Separator */}
            <div className="h-px bg-border/50 my-1" />

            {/* Import from file */}
            <button
              type="button"
              onClick={handleImportFromFile}
              className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
            >
              <Upload size={14} className="text-muted-foreground" />
              {t('variables.importPreset')}
            </button>

            {/* Export to file */}
            <button
              type="button"
              onClick={handleExportToFile}
              className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
            >
              <Download size={14} className="text-muted-foreground" />
              {t('variables.exportPreset')}
            </button>
          </div>
        )}
      </div>
    </>
  );
}

export const ThemeTabsHeader = memo(ThemeTabsHeaderInner);

export interface VariantColumnsHeaderProps {
  themeValues: string[];
  themeAxis: string;
  themes: Record<string, string[]> | undefined;
  currentAxis: string | null;
  setThemes: (themes: Record<string, string[]>) => void;
  ensureThemes: () => void;
}

/** Column headers row — variant names with rename/delete dropdown + add variant button. */
function VariantColumnsHeaderInner({
  themeValues,
  themeAxis,
  themes,
  currentAxis,
  setThemes,
  ensureThemes,
}: VariantColumnsHeaderProps) {
  const { t } = useTranslation();

  const [activeColumnMenu, setActiveColumnMenu] = useState<string | null>(null);
  const [renamingColumn, setRenamingColumn] = useState<string | null>(null);
  const [renameColumnValue, setRenameColumnValue] = useState('');

  const columnMenuRef = useRef<HTMLDivElement>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (
        activeColumnMenu &&
        columnMenuRef.current &&
        !columnMenuRef.current.contains(e.target as Node)
      )
        setActiveColumnMenu(null);
    };
    if (activeColumnMenu) {
      document.addEventListener('mousedown', handler);
      return () => document.removeEventListener('mousedown', handler);
    }
  }, [activeColumnMenu]);

  useEffect(() => {
    if (renamingColumn && renameInputRef.current) {
      renameInputRef.current.focus();
      renameInputRef.current.select();
    }
  }, [renamingColumn]);

  const handleAddVariant = () => {
    ensureThemes();
    const axis = currentAxis ?? themeAxis;
    const currentValues = themes?.[axis] ?? ['Default'];
    let counter = 1;
    let n = `Variant-${counter}`;
    while (currentValues.includes(n)) {
      counter++;
      n = `Variant-${counter}`;
    }
    const updatedThemes = { ...(themes ?? { [themeAxis]: ['Default'] }) };
    updatedThemes[axis] = [...currentValues, n];
    setThemes(updatedThemes);
  };

  const handleRemoveVariant = (value: string) => {
    if (!currentAxis || !themes) return;
    const currentValues = themes[currentAxis] ?? [];
    if (currentValues.length <= 1) return;
    setThemes({ ...themes, [currentAxis]: currentValues.filter((v) => v !== value) });
    setActiveColumnMenu(null);
  };

  const handleRenameVariant = (oldName: string, newName: string) => {
    if (!newName.trim() || newName === oldName) {
      setRenamingColumn(null);
      return;
    }
    if (!currentAxis || !themes) {
      setRenamingColumn(null);
      return;
    }
    const currentValues = themes[currentAxis] ?? [];
    if (currentValues.includes(newName)) {
      setRenamingColumn(null);
      return;
    }
    setThemes({
      ...themes,
      [currentAxis]: currentValues.map((v) => (v === oldName ? newName : v)),
    });
    setRenamingColumn(null);
  };

  const startRenameVariant = (tv: string) => {
    setRenameColumnValue(tv);
    setRenamingColumn(tv);
    setActiveColumnMenu(null);
  };

  return (
    <div className="relative flex items-center px-4 h-9 shrink-0 border-t border-b border-border/40 z-10">
      <div className="w-[220px] shrink-0">
        <span className="text-[13px] font-medium text-muted-foreground">{t('common.name')}</span>
      </div>
      {themeValues.map((tv) => (
        <div
          key={tv}
          className="flex-1 min-w-0 pl-4 relative"
          ref={activeColumnMenu === tv ? columnMenuRef : undefined}
        >
          {renamingColumn === tv ? (
            <input
              ref={renameInputRef}
              type="text"
              value={renameColumnValue}
              onChange={(e) => setRenameColumnValue(e.target.value)}
              onBlur={() => handleRenameVariant(tv, renameColumnValue)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleRenameVariant(tv, renameColumnValue);
                if (e.key === 'Escape') setRenamingColumn(null);
              }}
              className="text-[13px] font-medium text-foreground bg-secondary px-1.5 py-0.5 rounded border border-ring focus:outline-none w-32"
            />
          ) : (
            <button
              type="button"
              onClick={() => setActiveColumnMenu(activeColumnMenu === tv ? null : tv)}
              className="flex items-center gap-1 text-[13px] font-medium text-muted-foreground hover:text-foreground transition-colors"
            >
              {tv}
              <ChevronDown
                size={11}
                className={cn(
                  'text-muted-foreground/60 transition-transform',
                  activeColumnMenu === tv && 'rotate-180',
                )}
              />
            </button>
          )}
          {activeColumnMenu === tv && (
            <div className="absolute left-4 top-full z-50 mt-1 w-44 bg-popover border border-border rounded-xl shadow-xl py-1 animate-in fade-in slide-in-from-top-1 duration-150">
              <button
                type="button"
                onClick={() => startRenameVariant(tv)}
                className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
              >
                <Pencil size={14} className="text-muted-foreground" />
                {t('common.rename')}
              </button>
              {themeValues.length > 1 && (
                <button
                  type="button"
                  onClick={() => handleRemoveVariant(tv)}
                  className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
                >
                  <Trash2 size={14} className="text-muted-foreground" />
                  {t('common.delete')}
                </button>
              )}
            </div>
          )}
        </div>
      ))}
      <div className="w-[44px] shrink-0 flex justify-center">
        <button
          type="button"
          onClick={handleAddVariant}
          className="p-1 rounded text-muted-foreground/50 hover:text-foreground transition-colors"
          title={t('variables.addVariant')}
        >
          <Plus size={14} />
        </button>
      </div>
    </div>
  );
}

export const VariantColumnsHeader = memo(VariantColumnsHeaderInner);
