import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Loader2,
  Eye,
  EyeOff,
  Search,
  ChevronDown,
  Plus,
  Key,
  Globe,
  Sparkles,
} from 'lucide-react';
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
  getBaseURLForFormat,
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
  'bailian-coding',
  'doubao',
  'ark-coding',
  'xiaomi',
  'modelscope',
  'stepfun',
  'stepfun-coding',
  'nvidia',
  'custom',
];

/* ---------- Shared field wrapper ---------- */
function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <label className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground/70 pl-0.5">
        {label}
      </label>
      {children}
    </div>
  );
}

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

  const effectiveType = apiFormat;

  const canSave =
    displayName.trim().length > 0 &&
    apiKey.trim().length > 0 &&
    modelName.trim().length > 0 &&
    (preset !== 'custom' || baseURL.trim().length > 0);

  const inputClass =
    'w-full h-8 px-2.5 text-[12px] bg-background text-foreground rounded-md border border-input focus:border-ring focus:ring-1 focus:ring-ring/20 outline-none transition-all';

  return (
    <div className="rounded-xl border border-border overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-2.5 bg-secondary/30 border-b border-border">
        <div className="h-5 w-5 rounded-md bg-primary/10 flex items-center justify-center">
          <Sparkles size={11} className="text-primary" />
        </div>
        <span className="text-[12px] font-medium text-foreground">
          {initial ? t('common.save') : t('builtin.addProvider')}
        </span>
      </div>

      <div className="p-4 space-y-3.5">
        {/* Row 1: Provider + Display Name */}
        <div className="grid grid-cols-2 gap-3">
          <Field label={t('builtin.provider')}>
            <Select
              value={preset}
              onValueChange={(v) => handlePresetChange(v as BuiltinProviderPreset)}
            >
              <SelectTrigger className="h-8 rounded-md text-[12px] bg-background">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="max-h-64">
                {PRESET_ORDER.map((key) => (
                  <SelectItem key={key} value={key} className="text-[12px]">
                    {key === 'custom' ? t('builtin.custom') : BUILTIN_PROVIDER_PRESETS[key].label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <Field label={t('builtin.displayName')}>
            <input
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder={presetConfig.label}
              className={inputClass}
            />
          </Field>
        </div>

        {/* Region toggle (inline) */}
        {presetConfig.regions && (
          <Field label={t('builtin.region')}>
            <div className="flex gap-1">
              {(['cn', 'global'] as const).map((r) => (
                <button
                  key={r}
                  type="button"
                  onClick={() => handleRegionChange(r)}
                  className={cn(
                    'flex-1 h-7 text-[11px] rounded-md border transition-all font-medium',
                    region === r
                      ? 'bg-primary text-primary-foreground border-primary'
                      : 'bg-background text-muted-foreground border-input hover:bg-accent',
                  )}
                >
                  {r === 'cn' ? '🇨🇳' : '🌍'} {t(`builtin.region${r === 'cn' ? 'China' : 'Global'}`)}
                </button>
              ))}
            </div>
          </Field>
        )}

        {/* API Key */}
        <Field label={t('builtin.apiKey')}>
          <div className="relative">
            <Key
              size={12}
              className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/40"
            />
            <input
              type={showApiKey ? 'text' : 'password'}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={presetConfig.placeholder}
              className={cn(inputClass, 'pl-7 pr-8 font-mono')}
            />
            <button
              type="button"
              onClick={() => setShowApiKey((v) => !v)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground/50 hover:text-foreground transition-colors"
            >
              {showApiKey ? <EyeOff size={12} /> : <Eye size={12} />}
            </button>
          </div>
        </Field>

        {/* Model */}
        <Field label={t('builtin.model')}>
          <div className="relative">
            <input
              value={modelName}
              onChange={(e) => setModelName(e.target.value)}
              placeholder={presetConfig.modelPlaceholder}
              className={cn(inputClass, 'pr-16 font-mono')}
            />
            <button
              type="button"
              onClick={handleFetchModels}
              disabled={modelLoading}
              title={t('builtin.searchModels')}
              className="absolute right-1 top-1/2 -translate-y-1/2 h-6 px-1.5 rounded flex items-center gap-0.5 text-muted-foreground/50 hover:text-foreground hover:bg-accent transition-all disabled:opacity-50"
            >
              {modelLoading ? (
                <Loader2 size={11} className="animate-spin" />
              ) : (
                <>
                  <Search size={11} />
                  <ChevronDown size={9} />
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
          {modelError && <p className="text-[10px] text-destructive mt-0.5">{modelError}</p>}
        </Field>

        {/* API Format + Base URL — compact row */}
        {(presetConfig.altType || preset === 'custom') && (
          <Field label={t('builtin.apiFormat')}>
            <div className="flex rounded-md border border-input overflow-hidden h-7">
              {(['openai-compat', 'anthropic'] as const).map((fmt) => (
                <button
                  key={fmt}
                  type="button"
                  onClick={() => {
                    setApiFormat(fmt);
                    if (preset !== 'custom') {
                      const url = getBaseURLForFormat(preset, fmt, region);
                      if (url) setBaseURL(url);
                    }
                  }}
                  className={cn(
                    'flex-1 text-[11px] font-medium transition-all',
                    apiFormat === fmt
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-background text-muted-foreground hover:bg-accent',
                  )}
                >
                  {fmt === 'openai-compat' ? 'OpenAI' : 'Anthropic'}
                </button>
              ))}
            </div>
          </Field>
        )}

        {/* Base URL — read-only for Anthropic/OpenAI, editable for others */}
        {preset === 'anthropic' || preset === 'openai' ? (
          <Field label={t('builtin.baseUrl')}>
            <div className="relative">
              <Globe
                size={12}
                className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/30"
              />
              <input
                value={baseURL}
                readOnly
                className={cn(inputClass, 'pl-7 font-mono text-[11px] opacity-50 cursor-default')}
              />
            </div>
          </Field>
        ) : (
          <Field label={t('builtin.baseUrl')}>
            <div className="relative">
              <Globe
                size={12}
                className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/40"
              />
              <input
                value={baseURL}
                onChange={(e) => setBaseURL(e.target.value)}
                placeholder={t('builtin.baseUrlPlaceholder')}
                className={cn(inputClass, 'pl-7 font-mono text-[11px]')}
              />
            </div>
          </Field>
        )}

        {/* Actions */}
        <div className="flex items-center justify-end gap-2 pt-1.5 border-t border-border/50">
          <Button
            variant="ghost"
            size="sm"
            onClick={onCancel}
            className="h-7 px-3 text-[11px] rounded-md"
          >
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
            className="h-7 px-4 text-[11px] rounded-md"
          >
            {initial ? t('common.save') : t('builtin.add')}
          </Button>
        </div>
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
            className="text-[11px] text-primary hover:text-primary/80 flex items-center gap-1 transition-colors font-medium"
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
