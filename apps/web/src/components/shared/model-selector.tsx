import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { BuiltinProviderPreset } from '@/stores/agent-settings-store';

/** Hardcoded model lists for providers that don't expose /models endpoint */
export const BUILTIN_MODEL_LISTS: Partial<
  Record<BuiltinProviderPreset, Array<{ id: string; name: string }>>
> = {
  anthropic: [
    { id: 'claude-opus-4-6-20250916', name: 'Claude Opus 4.6' },
    { id: 'claude-sonnet-4-6-20250916', name: 'Claude Sonnet 4.6' },
    { id: 'claude-sonnet-4-5-20250929', name: 'Claude Sonnet 4.5' },
    { id: 'claude-haiku-4-5-20251001', name: 'Claude Haiku 4.5' },
    { id: 'claude-opus-4-20250514', name: 'Claude Opus 4' },
    { id: 'claude-sonnet-4-20250514', name: 'Claude Sonnet 4' },
  ],
  gemini: [
    { id: 'gemini-3.1-pro-preview', name: 'Gemini 3.1 Pro' },
    { id: 'gemini-3-flash-preview', name: 'Gemini 3 Flash' },
    { id: 'gemini-3.1-flash-lite-preview', name: 'Gemini 3.1 Flash-Lite' },
    { id: 'gemini-2.5-pro', name: 'Gemini 2.5 Pro' },
    { id: 'gemini-2.5-flash', name: 'Gemini 2.5 Flash' },
  ],
  minimax: [
    { id: 'MiniMax-M2.7', name: 'MiniMax M2.7' },
    { id: 'MiniMax-M2.7-highspeed', name: 'MiniMax M2.7 Highspeed' },
    { id: 'MiniMax-M2.5', name: 'MiniMax M2.5' },
    { id: 'MiniMax-M2.5-highspeed', name: 'MiniMax M2.5 Highspeed' },
    { id: 'MiniMax-M2.1', name: 'MiniMax M2.1' },
    { id: 'MiniMax-M1', name: 'MiniMax M1' },
  ],
  'glm-coding': [
    { id: 'glm-5', name: 'GLM-5' },
    { id: 'glm-4.7', name: 'GLM-4.7' },
    { id: 'glm-4.6', name: 'GLM-4.6' },
    { id: 'glm-4.5-air', name: 'GLM-4.5 Air' },
  ],
  doubao: [
    { id: 'doubao-seed-2.0-pro', name: 'Doubao Seed 2.0 Pro' },
    { id: 'doubao-seed-2.0-lite', name: 'Doubao Seed 2.0 Lite' },
    { id: 'doubao-seed-2.0-code', name: 'Doubao Seed 2.0 Code' },
    { id: 'doubao-seed-code', name: 'Doubao Seed Code' },
  ],
  'ark-coding': [
    { id: 'ark-code-latest', name: 'Ark Code Latest' },
    { id: 'doubao-seed-2.0-code', name: 'Doubao Seed 2.0 Code' },
    { id: 'doubao-seed-code', name: 'Doubao Seed Code' },
    { id: 'doubao-seed-2.0-pro', name: 'Doubao Seed 2.0 Pro' },
    { id: 'doubao-seed-2.0-lite', name: 'Doubao Seed 2.0 Lite' },
    { id: 'glm-4.7', name: 'GLM-4.7' },
    { id: 'deepseek-v3.2', name: 'DeepSeek V3.2' },
    { id: 'kimi-k2.5', name: 'Kimi K2.5' },
    { id: 'minimax-m2.5', name: 'MiniMax M2.5' },
  ],
};

/** Fetch model list from a provider via our server-side proxy */
export async function fetchProviderModels(
  baseURL: string,
  apiKey?: string,
): Promise<{ models: Array<{ id: string; name: string }>; error?: string }> {
  try {
    const res = await fetch('/api/ai/provider-models', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ baseURL, apiKey }),
    });
    if (!res.ok) return { models: [], error: `Server error ${res.status}` };
    return await res.json();
  } catch {
    return { models: [], error: 'Request failed' };
  }
}

/* ---------- Model Search Dropdown ---------- */
export default function ModelSearchDropdown({
  models,
  onSelect,
  onClose,
}: {
  models: Array<{ id: string; name: string }>;
  onSelect: (model: { id: string; name: string }) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState('');
  const listRef = useRef<HTMLDivElement>(null);

  const filtered = models.filter((m) => {
    const q = filter.toLowerCase();
    return m.id.toLowerCase().includes(q) || m.name.toLowerCase().includes(q);
  });

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (listRef.current && !listRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [onClose]);

  return (
    <div
      ref={listRef}
      className="absolute bottom-full left-0 right-0 mb-1 rounded-md border border-border bg-popover shadow-md z-10 overflow-hidden"
    >
      <div className="p-1.5 border-b border-border">
        <input
          autoFocus
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder={t('builtin.filterModels')}
          className="w-full h-7 px-2 text-[12px] bg-card text-foreground rounded border border-input focus:border-ring outline-none transition-colors"
        />
      </div>
      <div className="max-h-48 overflow-y-auto">
        {filtered.length === 0 && (
          <div className="px-3 py-4 text-center text-[11px] text-muted-foreground">
            {t('builtin.noModels')}
          </div>
        )}
        {filtered.map((m) => (
          <button
            key={m.id}
            onClick={() => {
              onSelect(m);
              onClose();
            }}
            className="w-full text-left px-3 py-1.5 text-[12px] text-foreground hover:bg-secondary/50 transition-colors flex flex-col"
          >
            <span className="font-medium truncate">{m.name !== m.id ? m.name : m.id}</span>
            {m.name !== m.id && (
              <span className="text-[10px] text-muted-foreground font-mono truncate">{m.id}</span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
