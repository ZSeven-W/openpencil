import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Loader2, Eye, EyeOff, Search, ChevronDown, Plus } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import type { BuiltinProviderConfig, BuiltinProviderPreset } from '@/stores/agent-settings-store';
import {
  BUILTIN_PROVIDER_PRESETS,
  inferBuiltinProviderPreset,
  inferBuiltinProviderRegion,
} from '@/lib/builtin-provider-presets';
import ModelSearchDropdown, { BUILTIN_MODEL_LISTS, fetchProviderModels } from './model-selector';
import { BuiltinProviderCard } from './provider-card';

const PRESET_ORDER: BuiltinProviderPreset[] = [
  'anthropic',
  'openai',
  'openrouter',
  'deepseek',
  'gemini',
  'minimax',
  'zhipu',
  'glm-coding',
  'kimi',
  'bailian',
  'doubao',
  'ark-coding',
  'xiaomi',
  'modelscope',
  'stepfun',
  'nvidia',
  'custom',
];

/* ---------- Builtin Provider Form ---------- */
export function BuiltinProviderForm({
  initial,
  onSave,
  onCancel,
}: {
  initial?: BuiltinProviderConfig;
  onSave: (data: Omit<BuiltinProviderConfig, 'id'>) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const [preset, setPreset] = useState<BuiltinProviderPreset>(
    initial ? inferBuiltinProviderPreset(initial) : 'anthropic',
  );
  const presetConfig = BUILTIN_PROVIDER_PRESETS[preset];
  const [region, setRegion] = useState<'cn' | 'global'>(
    initial ? inferBuiltinProviderRegion(initial) : 'cn',
  );
  const [displayName, setDisplayName] = useState(initial?.displayName ?? '');
  const [apiKey, setApiKey] = useState(initial?.apiKey ?? '');
  const [modelName, setModelName] = useState(initial?.model ?? '');
  const [baseURL, setBaseURL] = useState(initial?.baseURL ?? presetConfig.baseURL ?? '');
  const [showApiKey, setShowApiKey] = useState(false);
  const [apiFormat, setApiFormat] = useState<'openai-compat' | 'anthropic'>(
    initial?.type ?? presetConfig.type ?? 'openai-compat',
  );

  const [modelList, setModelList] = useState<Array<{ id: string; name: string }>>([]);
  const [showModelDropdown, setShowModelDropdown] = useState(false);
  const [modelLoading, setModelLoading] = useState(false);
  const [modelError, setModelError] = useState<string | null>(null);

  const handlePresetChange = useCallback(
    (newPreset: BuiltinProviderPreset) => {
      setPreset(newPreset);
      const cfg = BUILTIN_PROVIDER_PRESETS[newPreset];
      if (!displayName.trim() || displayName === BUILTIN_PROVIDER_PRESETS[preset].label) {
        setDisplayName(cfg.label);
      }
      setRegion('cn');
      setBaseURL(cfg.regions?.cn.baseURL ?? cfg.baseURL ?? '');
      setApiFormat(cfg.type);
      setModelList([]);
      setShowModelDropdown(false);
      setModelError(null);
    },
    [displayName, preset],
  );

  const handleRegionChange = useCallback(
    (newRegion: 'cn' | 'global') => {
      setRegion(newRegion);
      const regions = presetConfig.regions;
      if (regions) {
        setBaseURL(regions[newRegion].baseURL);
        setModelList([]);
        setShowModelDropdown(false);
        setModelError(null);
      }
    },
    [presetConfig],
  );

  const handleFetchModels = useCallback(async () => {
    const builtinList = BUILTIN_MODEL_LISTS[preset];
    if (builtinList) {
      setModelList(builtinList);
      setShowModelDropdown(true);
      return;
    }
    const cfg = BUILTIN_PROVIDER_PRESETS[preset];
    const url =
      preset === 'custom'
        ? baseURL.trim()
        : cfg.regions
          ? cfg.regions[region].baseURL
          : cfg.baseURL;
    if (!url) {
      setModelError(t('builtin.searchError'));
      return;
    }
    setModelLoading(true);
    setModelError(null);
    const result = await fetchProviderModels(url, apiKey.trim() || undefined);
    setModelLoading(false);
    if (result.error) {
      setModelError(result.error);
      if (result.models.length > 0) {
        setModelList(result.models);
        setShowModelDropdown(true);
      }
    } else {
      setModelList(result.models);
      setShowModelDropdown(true);
    }
  }, [preset, baseURL, apiKey, region, t]);

  const handleModelSelect = useCallback((model: { id: string; name: string }) => {
    setModelName(model.id);
    setShowModelDropdown(false);
  }, []);

  const isBaseURLLocked = preset !== 'custom';
  const effectiveType = apiFormat;

  const canSave =
    displayName.trim().length > 0 &&
    apiKey.trim().length > 0 &&
    modelName.trim().length > 0 &&
    (preset !== 'custom' || baseURL.trim().length > 0);

  return (
    <div className="space-y-4 rounded-xl border border-border bg-secondary/10 p-4">
      {/* Display Name */}
      <div className="space-y-1.5">
        <label className="text-[11px] font-medium text-muted-foreground">
          {t('builtin.displayName')}
        </label>
        <input
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          placeholder={t('builtin.displayNamePlaceholder')}
          className="w-full h-8 px-2.5 text-[13px] bg-card text-foreground rounded-lg border border-input focus:border-ring focus:ring-1 focus:ring-ring/30 outline-none transition-all"
        />
      </div>

      {/* Provider — shadcn Select */}
      <div className="space-y-1.5">
        <label className="text-[11px] font-medium text-muted-foreground">
          {t('builtin.provider')}
        </label>
        <Select
          value={preset}
          onValueChange={(v) => handlePresetChange(v as BuiltinProviderPreset)}
        >
          <SelectTrigger className="h-8 rounded-lg text-[13px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent className="max-h-64">
            {PRESET_ORDER.map((key) => (
              <SelectItem key={key} value={key} className="text-[13px]">
                {key === 'custom' ? t('builtin.custom') : BUILTIN_PROVIDER_PRESETS[key].label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Region toggle */}
      {presetConfig.regions && (
        <div className="space-y-1.5">
          <label className="text-[11px] font-medium text-muted-foreground">
            {t('builtin.region')}
          </label>
          <div className="flex gap-1">
            {(['cn', 'global'] as const).map((r) => (
              <button
                key={r}
                type="button"
                onClick={() => handleRegionChange(r)}
                className={cn(
                  'flex-1 h-7 text-[11px] rounded-lg border transition-all',
                  region === r
                    ? 'bg-primary text-primary-foreground border-primary shadow-sm'
                    : 'bg-card text-muted-foreground border-input hover:bg-accent',
                )}
              >
                {t(`builtin.region${r === 'cn' ? 'China' : 'Global'}`)}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* API Key */}
      <div className="space-y-1.5">
        <label className="text-[11px] font-medium text-muted-foreground">
          {t('builtin.apiKey')}
        </label>
        <div className="relative">
          <input
            type={showApiKey ? 'text' : 'password'}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={presetConfig.placeholder}
            className="w-full h-8 px-2.5 pr-8 text-[13px] bg-card text-foreground rounded-lg border border-input focus:border-ring focus:ring-1 focus:ring-ring/30 outline-none transition-all font-mono"
          />
          <button
            type="button"
            onClick={() => setShowApiKey((v) => !v)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
          >
            {showApiKey ? <EyeOff size={13} /> : <Eye size={13} />}
          </button>
        </div>
      </div>

      {/* Model */}
      <div className="space-y-1.5">
        <label className="text-[11px] font-medium text-muted-foreground">{t('builtin.model')}</label>
        <div className="relative">
          <input
            value={modelName}
            onChange={(e) => setModelName(e.target.value)}
            placeholder={presetConfig.modelPlaceholder}
            className="w-full h-8 px-2.5 pr-16 text-[13px] bg-card text-foreground rounded-lg border border-input focus:border-ring focus:ring-1 focus:ring-ring/30 outline-none transition-all font-mono"
          />
          <button
            type="button"
            onClick={handleFetchModels}
            disabled={modelLoading}
            title={t('builtin.searchModels')}
            className="absolute right-1.5 top-1/2 -translate-y-1/2 h-6 px-1.5 rounded-md flex items-center gap-1 text-muted-foreground hover:text-foreground hover:bg-secondary/50 transition-all disabled:opacity-50"
          >
            {modelLoading ? (
              <Loader2 size={12} className="animate-spin" />
            ) : (
              <>
                <Search size={12} />
                <ChevronDown size={10} />
              </>
            )}
          </button>
          {showModelDropdown && modelList.length > 0 && (
            <ModelSearchDropdown
              models={modelList}
              onSelect={handleModelSelect}
              onClose={() => setShowModelDropdown(false)}
            />
          )}
        </div>
        {modelError && <p className="text-[10px] text-destructive mt-1">{modelError}</p>}
      </div>

      {/* API Format — hidden for anthropic/openai (format is fixed) */}
      {preset !== 'anthropic' && preset !== 'openai' && (
        <div className="space-y-1.5">
          <label className="text-[11px] font-medium text-muted-foreground">
            {t('builtin.apiFormat')}
          </label>
          <div className="flex gap-1">
            {(['openai-compat', 'anthropic'] as const).map((fmt) => (
              <button
                key={fmt}
                type="button"
                onClick={() => setApiFormat(fmt)}
                className={cn(
                  'flex-1 h-7 text-[11px] rounded-lg border transition-all',
                  apiFormat === fmt
                    ? 'bg-primary text-primary-foreground border-primary shadow-sm'
                    : 'bg-card text-muted-foreground border-input hover:bg-accent',
                )}
              >
                {fmt === 'openai-compat' ? t('builtin.openaiCompat') : 'Anthropic'}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Base URL */}
      <div className="space-y-1.5">
        <label className="text-[11px] font-medium text-muted-foreground">
          {isBaseURLLocked ? t('builtin.baseUrl') : t('builtin.baseUrlRequired')}
        </label>
        <input
          value={baseURL}
          onChange={(e) => setBaseURL(e.target.value)}
          placeholder={t('builtin.baseUrlPlaceholder')}
          readOnly={isBaseURLLocked}
          className={cn(
            'w-full h-8 px-2.5 text-[13px] bg-card text-foreground rounded-lg border border-input focus:border-ring focus:ring-1 focus:ring-ring/30 outline-none transition-all font-mono',
            isBaseURLLocked && 'opacity-50 cursor-default',
          )}
        />
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-2 pt-1">
        <Button variant="ghost" size="sm" onClick={onCancel} className="h-8 px-4 text-[12px] rounded-lg">
          {t('common.cancel')}
        </Button>
        <Button
          size="sm"
          onClick={() =>
            onSave({
              displayName: displayName.trim(),
              type: effectiveType,
              apiKey: apiKey.trim(),
              model: modelName.trim(),
              preset,
              ...(baseURL.trim() ? { baseURL: baseURL.trim() } : {}),
              enabled: initial?.enabled ?? true,
            })
          }
          disabled={!canSave}
          className="h-8 px-4 text-[12px] rounded-lg"
        >
          {initial ? t('common.save') : t('builtin.add')}
        </Button>
      </div>
    </div>
  );
}

/* ---------- Builtin Providers Section (used in AgentsPage) ---------- */
export function BuiltinProvidersSection() {
  const { t } = useTranslation();
  const builtinProviders = useAgentSettingsStore((s) => s.builtinProviders);
  const addBuiltinProvider = useAgentSettingsStore((s) => s.addBuiltinProvider);
  const persist = useAgentSettingsStore((s) => s.persist);
  const [showForm, setShowForm] = useState(false);

  const handleAdd = useCallback(
    (data: Omit<BuiltinProviderConfig, 'id'>) => {
      addBuiltinProvider(data);
      persist();
      setShowForm(false);
    },
    [addBuiltinProvider, persist],
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">{t('builtin.title')}</h3>
        {!showForm && (
          <button
            onClick={() => setShowForm(true)}
            className="text-[11px] text-muted-foreground hover:text-foreground flex items-center gap-1 transition-colors"
          >
            <Plus size={12} /> {t('builtin.addProvider')}
          </button>
        )}
      </div>
      <p className="text-[11px] text-muted-foreground leading-relaxed">
        {t('builtin.description')}
      </p>
      {showForm && <BuiltinProviderForm onSave={handleAdd} onCancel={() => setShowForm(false)} />}
      {builtinProviders.map((bp) => (
        <BuiltinProviderCard key={bp.id} provider={bp} />
      ))}
      {!showForm && builtinProviders.length === 0 && (
        <div className="text-center py-6 text-[11px] text-muted-foreground">
          {t('builtin.empty')}
        </div>
      )}
    </div>
  );
}
