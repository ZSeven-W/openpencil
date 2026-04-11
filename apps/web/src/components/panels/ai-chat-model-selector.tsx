import { useState, useRef, useEffect } from 'react';
import { Check, Search, X, Zap, Key } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import { useAIStore } from '@/stores/ai-store';
import type { AIProviderType, ModelGroup } from '@/types/agent-settings';
import ClaudeLogo from '@/components/icons/claude-logo';
import OpenAILogo from '@/components/icons/openai-logo';
import OpenCodeLogo from '@/components/icons/opencode-logo';
import CopilotLogo from '@/components/icons/copilot-logo';
import GeminiLogo from '@/components/icons/gemini-logo';

const PROVIDER_ICON: Record<AIProviderType, typeof ClaudeLogo> = {
  anthropic: ClaudeLogo,
  openai: OpenAILogo,
  opencode: OpenCodeLogo,
  copilot: CopilotLogo,
  gemini: GeminiLogo,
};

export { PROVIDER_ICON };

/** Pick the best available model: keep current, fall back to preferred, then first. */
export function resolveNextModel(
  models: Array<{ value: string }>,
  currentModel: string,
  preferredModel: string,
): string | null {
  if (models.length === 0) return null;
  if (models.some((m) => m.value === currentModel)) return currentModel;
  if (models.some((m) => m.value === preferredModel)) return preferredModel;
  return models[0].value;
}

/**
 * Compact concurrency selector — cycles 1x through 6x on click.
 */
export function ConcurrencyButton() {
  const { t } = useTranslation();
  const concurrency = useAIStore((s) => s.concurrency);
  const setConcurrency = useAIStore((s) => s.setConcurrency);
  const isStreaming = useAIStore((s) => s.isStreaming);

  return (
    <button
      type="button"
      onClick={() => setConcurrency(concurrency >= 6 ? 1 : concurrency + 1)}
      disabled={isStreaming}
      title={t('builtin.parallelAgents', { count: concurrency })}
      className={cn(
        'flex items-center gap-0.5 text-[10px] px-1.5 py-0.5 rounded-md transition-colors shrink-0',
        concurrency > 1
          ? 'text-primary bg-primary/10 hover:bg-primary/20'
          : 'text-muted-foreground/50 hover:text-muted-foreground hover:bg-secondary',
      )}
    >
      <Zap size={10} />
      <span>{concurrency}x</span>
    </button>
  );
}

/**
 * Upward model dropdown with search, provider grouping, and builtin badges.
 */
export function ModelDropdown({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const availableModels = useAIStore((s) => s.availableModels);
  const modelGroups = useAIStore((s) => s.modelGroups) as ModelGroup[];
  const model = useAIStore((s) => s.model);
  const selectModel = useAIStore((s) => s.selectModel);
  const [modelSearch, setModelSearch] = useState('');
  const modelSearchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setModelSearch('');
      setTimeout(() => modelSearchRef.current?.focus(), 50);
    }
  }, [open]);

  // Close on outside click
  const dropdownRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open, onClose]);

  if (!open || availableModels.length === 0) return null;

  const q = modelSearch.toLowerCase().trim();

  const renderGrouped = () => {
    const filtered = modelGroups
      .map((group) => ({
        ...group,
        models: group.models.filter(
          (m) =>
            !q ||
            m.displayName.toLowerCase().includes(q) ||
            m.value.toLowerCase().includes(q) ||
            group.providerName.toLowerCase().includes(q),
        ),
      }))
      .filter((group) => group.models.length > 0);

    if (filtered.length === 0) {
      return (
        <div className="px-3 py-4 text-xs text-muted-foreground text-center">
          {t('ai.noModelsFound')}
        </div>
      );
    }

    return filtered.map((group, groupIdx) => {
      const GIcon = PROVIDER_ICON[group.provider];
      const groupKey = `${group.provider}-${group.providerName}-${groupIdx}`;
      const isBuiltinGroup = group.models.some((m) => m.value.startsWith('builtin:'));
      return (
        <div key={groupKey}>
          <div className="flex items-center gap-1.5 px-3 pt-2.5 pb-1">
            {isBuiltinGroup ? (
              <Key size={10} className="text-muted-foreground shrink-0" />
            ) : (
              <GIcon className="w-3 h-3 text-muted-foreground" />
            )}
            <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
              {group.providerName}
            </span>
          </div>
          {group.models.map((m, idx) => {
            const isSelected = m.value === model;
            const isBuiltin = m.value.startsWith('builtin:');
            return (
              <button
                key={m.value}
                type="button"
                onClick={() => {
                  selectModel(m.value);
                  onClose();
                }}
                className={cn(
                  'w-full flex items-center gap-2 px-3 py-1.5 text-xs transition-colors',
                  isSelected
                    ? 'bg-secondary text-foreground'
                    : 'text-muted-foreground hover:bg-accent hover:text-foreground',
                )}
              >
                <span className="w-3.5 shrink-0">{isSelected && <Check size={12} />}</span>
                <span className="font-medium">{m.displayName}</span>
                {isBuiltin && (
                  <span className="text-[9px] text-muted-foreground bg-secondary px-1 py-0.5 rounded ml-auto">
                    {t('builtin.apiKeyBadge')}
                  </span>
                )}
                {!isBuiltin && idx === 0 && !q && (
                  <span className="text-[9px] text-muted-foreground bg-secondary px-1 py-0.5 rounded ml-auto">
                    {t('common.best')}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      );
    });
  };

  const renderFlat = () => {
    const filtered = availableModels.filter(
      (m) => !q || m.displayName.toLowerCase().includes(q) || m.value.toLowerCase().includes(q),
    );
    if (filtered.length === 0) {
      return (
        <div className="px-3 py-4 text-xs text-muted-foreground text-center">
          {t('ai.noModelsFound')}
        </div>
      );
    }
    return filtered.map((m) => {
      const isSelected = m.value === model;
      return (
        <button
          key={m.value}
          type="button"
          onClick={() => {
            selectModel(m.value);
            onClose();
          }}
          className={cn(
            'w-full flex items-center gap-2 px-3 py-1.5 text-xs transition-colors',
            isSelected
              ? 'bg-secondary text-foreground'
              : 'text-muted-foreground hover:bg-accent hover:text-foreground',
          )}
        >
          <span className="w-3.5 shrink-0">{isSelected && <Check size={12} />}</span>
          <span className="font-medium">{m.displayName}</span>
        </button>
      );
    });
  };

  return (
    <div
      ref={dropdownRef}
      className="absolute bottom-full left-0 right-0 mb-1 z-[60] rounded-lg border border-border bg-card shadow-xl py-1 max-h-72 flex flex-col"
    >
      <div className="px-2 pt-1 pb-1.5 border-b border-border shrink-0">
        <div className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary/50">
          <Search size={12} className="text-muted-foreground shrink-0" />
          <input
            ref={modelSearchRef}
            value={modelSearch}
            onChange={(e) => setModelSearch(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') onClose();
            }}
            placeholder={t('ai.searchModels')}
            className="w-full bg-transparent text-xs text-foreground placeholder-muted-foreground outline-none"
          />
          {modelSearch && (
            <button
              type="button"
              onClick={() => setModelSearch('')}
              className="text-muted-foreground hover:text-foreground shrink-0"
            >
              <X size={10} />
            </button>
          )}
        </div>
      </div>
      <div className="overflow-y-auto">
        {modelGroups.length > 0 ? renderGrouped() : renderFlat()}
      </div>
    </div>
  );
}
