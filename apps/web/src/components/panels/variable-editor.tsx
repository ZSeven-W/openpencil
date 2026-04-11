import { useState, useRef, useEffect, useCallback } from 'react';
import { Plus, ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import type { VariableDefinition, ThemedValue } from '@/types/variables';

const DEFAULT_THEME_VALUES = ['Default'];

export interface VariableEditorProps {
  variables: Record<string, VariableDefinition> | undefined;
  themes: Record<string, string[]> | undefined;
  themeAxis: string;
  setVariable: (name: string, def: VariableDefinition) => void;
  ensureThemes: () => void;
}

/** Footer bar with the "Add variable" button and type-selection dropdown. */
export default function VariableEditor({
  variables,
  themes,
  themeAxis,
  setVariable,
  ensureThemes,
}: VariableEditorProps) {
  const { t } = useTranslation();
  const [showAddMenu, setShowAddMenu] = useState(false);
  const addMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (showAddMenu && addMenuRef.current && !addMenuRef.current.contains(e.target as Node))
        setShowAddMenu(false);
    };
    if (showAddMenu) {
      document.addEventListener('mousedown', handler);
      return () => document.removeEventListener('mousedown', handler);
    }
  }, [showAddMenu]);

  const handleAdd = useCallback(
    (type: VariableDefinition['type']) => {
      ensureThemes();
      const existing = variables ? Object.keys(variables) : [];
      let counter = 1;
      const baseName = type === 'color' ? 'color' : type === 'number' ? 'number' : 'string';
      let varName = `${baseName}-${counter}`;
      while (existing.includes(varName)) {
        counter++;
        varName = `${baseName}-${counter}`;
      }
      const currentTV = themes?.[themeAxis] ?? DEFAULT_THEME_VALUES;
      let defaultValue: VariableDefinition['value'];
      if (currentTV.length > 1) {
        defaultValue = currentTV.map((tv) => ({
          value: type === 'color' ? '#000000' : type === 'number' ? 0 : '',
          theme: { [themeAxis]: tv },
        })) as ThemedValue[];
      } else {
        defaultValue = type === 'color' ? '#000000' : type === 'number' ? 0 : '';
      }
      setVariable(varName, { type, value: defaultValue });
      setShowAddMenu(false);
    },
    [ensureThemes, variables, themes, themeAxis, setVariable],
  );

  return (
    <div
      className="relative h-10 flex items-center px-4 shrink-0 border-t border-border/30 z-10"
      ref={addMenuRef}
    >
      <button
        type="button"
        onClick={() => setShowAddMenu(!showAddMenu)}
        className={cn(
          'flex items-center gap-2 text-[13px] transition-colors',
          showAddMenu ? 'text-foreground' : 'text-muted-foreground hover:text-foreground',
        )}
      >
        <Plus size={14} />
        {t('variables.addVariable')}
        <ChevronDown
          size={11}
          className={cn('transition-transform', showAddMenu && 'rotate-180')}
        />
      </button>
      {showAddMenu && (
        <div className="absolute left-4 bottom-full z-50 mb-1.5 w-44 bg-popover border border-border rounded-xl shadow-xl py-1 animate-in fade-in slide-in-from-bottom-1 duration-150">
          {(['color', 'number', 'string'] as const).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => handleAdd(t)}
              className="w-full text-left px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg capitalize transition-colors"
            >
              {t}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
