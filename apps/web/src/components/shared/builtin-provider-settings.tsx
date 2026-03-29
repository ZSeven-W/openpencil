import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Loader2, Eye, EyeOff, Search, ChevronDown, Plus } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import type { BuiltinProviderConfig, BuiltinProviderPreset } from '@/stores/agent-settings-store';
import ModelSearchDropdown, { BUILTIN_MODEL_LISTS, fetchProviderModels } from './model-selector';
import { BuiltinProviderCard } from './provider-card';

/* ---------- Provider Preset Config ---------- */
interface PresetRegion {
  baseURL: string;
}

interface PresetConfig {
  label: string;
  type: 'anthropic' | 'openai-compat';
  baseURL?: string;
  placeholder: string;
  modelPlaceholder: string;
  regions?: { cn: PresetRegion; global: PresetRegion };
}

const PROVIDER_PRESETS: Record<BuiltinProviderPreset, PresetConfig> = {
  anthropic: {
    label: 'Anthropic',
    type: 'anthropic',
    placeholder: 'sk-ant-...',
    modelPlaceholder: 'claude-sonnet-4-6-20250916',
  },
  openai: {
    label: 'OpenAI',
    type: 'openai-compat',
    baseURL: 'https://api.openai.com/v1',
    placeholder: 'sk-...',
    modelPlaceholder: 'gpt-5.4',
  },
  openrouter: {
    label: 'OpenRouter',
    type: 'openai-compat',
    baseURL: 'https://openrouter.ai/api/v1',
    placeholder: 'sk-or-...',
    modelPlaceholder: 'anthropic/claude-sonnet-4.6',
  },
  deepseek: {
    label: 'DeepSeek',
    type: 'openai-compat',
    baseURL: 'https://api.deepseek.com/v1',
    placeholder: 'sk-...',
    modelPlaceholder: 'deepseek-chat',
  },
  gemini: {
    label: 'Google Gemini',
    type: 'openai-compat',
    baseURL: 'https://generativelanguage.googleapis.com/v1beta/openai',
    placeholder: 'AIza...',
    modelPlaceholder: 'gemini-3-flash-preview',
  },
  minimax: {
    label: 'MiniMax',
    type: 'openai-compat',
    baseURL: 'https://api.minimaxi.com/v1',
    placeholder: 'eyJ...',
    modelPlaceholder: 'MiniMax-M2.7',
    regions: {
      cn: { baseURL: 'https://api.minimaxi.com/v1' },
      global: { baseURL: 'https://api.minimax.io/v1' },
    },
  },
  zhipu: {
    label: '智谱 (Zhipu)',
    type: 'openai-compat',
    baseURL: 'https://open.bigmodel.cn/api/paas/v4',
    placeholder: 'xxx.yyy',
    modelPlaceholder: 'glm-5',
    regions: {
      cn: { baseURL: 'https://open.bigmodel.cn/api/paas/v4' },
      global: { baseURL: 'https://open.z.ai/api/paas/v4' },
    },
  },
  kimi: {
    label: 'Kimi (Moonshot)',
    type: 'openai-compat',
    baseURL: 'https://api.moonshot.cn/v1',
    placeholder: 'sk-...',
    modelPlaceholder: 'kimi-k2.5',
    regions: {
      cn: { baseURL: 'https://api.moonshot.cn/v1' },
      global: { baseURL: 'https://api.moonshot.ai/v1' },
    },
  },
  bailian: {
    label: 'Bailian (DashScope)',
    type: 'openai-compat',
    baseURL: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    placeholder: 'sk-...',
    modelPlaceholder: 'qwen-plus',
    regions: {
      cn: { baseURL: 'https://dashscope.aliyuncs.com/compatible-mode/v1' },
      global: { baseURL: 'https://dashscope-intl.aliyuncs.com/compatible-mode/v1' },
    },
  },
  doubao: {
    label: 'DouBao Seed',
    type: 'openai-compat',
    baseURL: 'https://ark.cn-beijing.volces.com/api/v3',
    placeholder: 'ARK API Key',
    modelPlaceholder: 'doubao-seed-2.0-pro',
  },
  xiaomi: {
    label: 'Xiaomi MiMo',
    type: 'openai-compat',
    baseURL: 'https://api.xiaomimimo.com/v1',
    placeholder: 'API Key',
    modelPlaceholder: 'mimo-v2-pro',
  },
  modelscope: {
    label: 'ModelScope',
    type: 'openai-compat',
    baseURL: 'https://api-inference.modelscope.cn/v1',
    placeholder: 'API Key',
    modelPlaceholder: 'qwen-plus',
  },
  stepfun: {
    label: 'StepFun',
    type: 'openai-compat',
    baseURL: 'https://api.stepfun.com/v1',
    placeholder: 'API Key',
    modelPlaceholder: 'step-3.5-flash',
    regions: {
      cn: { baseURL: 'https://api.stepfun.com/v1' },
      global: { baseURL: 'https://api.stepfun.ai/v1' },
    },
  },
  nvidia: {
    label: 'NVIDIA NIM',
    type: 'openai-compat',
    baseURL: 'https://integrate.api.nvidia.com/v1',
    placeholder: 'nvapi-...',
    modelPlaceholder: 'nvidia/llama-3.1-nemotron-70b-instruct',
  },
  custom: {
    label: 'Custom',
    type: 'openai-compat',
    placeholder: 'sk-...',
    modelPlaceholder: 'model-name',
  },
};

/** All known region URLs for reverse-lookup in inferPreset */
const REGION_URLS: Record<string, BuiltinProviderPreset> = Object.entries(PROVIDER_PRESETS).reduce(
  (acc, [key, cfg]) => {
    if (cfg.regions) {
      acc[cfg.regions.cn.baseURL] = key as BuiltinProviderPreset;
      acc[cfg.regions.global.baseURL] = key as BuiltinProviderPreset;
    }
    return acc;
  },
  {} as Record<string, BuiltinProviderPreset>,
);

/** Infer preset from an existing provider config (for editing) */
function inferPreset(config: BuiltinProviderConfig): BuiltinProviderPreset {
  if (config.preset) return config.preset;
  if (config.type === 'anthropic') return 'anthropic';
  const url = config.baseURL?.replace(/\/+$/, '') ?? '';
  if (url === 'https://api.openai.com/v1') return 'openai';
  if (url === 'https://openrouter.ai/api/v1') return 'openrouter';
  if (url === 'https://api.deepseek.com/v1') return 'deepseek';
  if (REGION_URLS[url]) return REGION_URLS[url];
  return 'custom';
}

/** Infer region from a provider's baseURL */
function inferRegion(config: BuiltinProviderConfig): 'cn' | 'global' {
  const preset = inferPreset(config);
  const regions = PROVIDER_PRESETS[preset].regions;
  if (!regions) return 'cn';
  const url = config.baseURL?.replace(/\/+$/, '') ?? '';
  return url === regions.global.baseURL ? 'global' : 'cn';
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
    initial ? inferPreset(initial) : 'anthropic',
  );
  const presetConfig = PROVIDER_PRESETS[preset];
  const [region, setRegion] = useState<'cn' | 'global'>(initial ? inferRegion(initial) : 'cn');
  const [displayName, setDisplayName] = useState(initial?.displayName ?? '');
  const [apiKey, setApiKey] = useState(initial?.apiKey ?? '');
  const [modelName, setModelName] = useState(initial?.model ?? '');
  const [baseURL, setBaseURL] = useState(initial?.baseURL ?? presetConfig.baseURL ?? '');
  const [showApiKey, setShowApiKey] = useState(false);
  const [customApiFormat, setCustomApiFormat] = useState<'openai-compat' | 'anthropic'>(
    initial?.type ?? 'openai-compat',
  );

  const [modelList, setModelList] = useState<Array<{ id: string; name: string }>>([]);
  const [showModelDropdown, setShowModelDropdown] = useState(false);
  const [modelLoading, setModelLoading] = useState(false);
  const [modelError, setModelError] = useState<string | null>(null);

  const handlePresetChange = useCallback(
    (newPreset: BuiltinProviderPreset) => {
      setPreset(newPreset);
      const cfg = PROVIDER_PRESETS[newPreset];
      if (!displayName.trim() || displayName === PROVIDER_PRESETS[preset].label) {
        setDisplayName(cfg.label);
      }
      setRegion('cn');
      setBaseURL(cfg.regions?.cn.baseURL ?? cfg.baseURL ?? '');
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
    const cfg = PROVIDER_PRESETS[preset];
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
  }, [preset, baseURL, apiKey]);

  const handleModelSelect = useCallback((model: { id: string; name: string }) => {
    setModelName(model.id);
    setShowModelDropdown(false);
  }, []);

  const isBaseURLLocked = preset !== 'custom';
  const effectiveType = preset === 'custom' ? customApiFormat : presetConfig.type;
  const showBaseURL = effectiveType === 'openai-compat' || preset === 'custom';
  const canSave =
    displayName.trim().length > 0 && apiKey.trim().length > 0 && modelName.trim().length > 0;

  return (
    <div className="space-y-3 rounded-lg border border-border bg-secondary/20 p-3.5">
      <div>
        <label className="text-[11px] text-muted-foreground mb-1 block">
          {t('builtin.displayName')}
        </label>
        <input
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          placeholder={t('builtin.displayNamePlaceholder')}
          className="w-full h-8 px-2.5 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors"
        />
      </div>
      <div>
        <label className="text-[11px] text-muted-foreground mb-1 block">
          {t('builtin.provider')}
        </label>
        <select
          value={preset}
          onChange={(e) => handlePresetChange(e.target.value as BuiltinProviderPreset)}
          className="w-full h-8 px-2 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors"
        >
          <option value="anthropic">Anthropic</option>
          <option value="openai">OpenAI</option>
          <option value="openrouter">OpenRouter</option>
          <option value="deepseek">DeepSeek</option>
          <option value="gemini">Google Gemini</option>
          <option value="minimax">MiniMax</option>
          <option value="zhipu">智谱 (Zhipu)</option>
          <option value="kimi">Kimi (Moonshot)</option>
          <option value="bailian">Bailian (DashScope)</option>
          <option value="doubao">DouBao Seed</option>
          <option value="xiaomi">Xiaomi MiMo</option>
          <option value="modelscope">ModelScope</option>
          <option value="stepfun">StepFun</option>
          <option value="nvidia">NVIDIA NIM</option>
          <option value="custom">{t('builtin.custom')}</option>
        </select>
      </div>
      {presetConfig.regions && (
        <div>
          <label className="text-[11px] text-muted-foreground mb-1 block">
            {t('builtin.region')}
          </label>
          <div className="flex gap-1">
            {(['cn', 'global'] as const).map((r) => (
              <button
                key={r}
                type="button"
                onClick={() => handleRegionChange(r)}
                className={cn(
                  'flex-1 h-7 text-[11px] rounded-md border transition-colors',
                  region === r
                    ? 'bg-primary text-primary-foreground border-primary'
                    : 'bg-card text-muted-foreground border-input hover:bg-accent',
                )}
              >
                {t(`builtin.region${r === 'cn' ? 'China' : 'Global'}`)}
              </button>
            ))}
          </div>
        </div>
      )}
      <div>
        <label className="text-[11px] text-muted-foreground mb-1 block">
          {t('builtin.apiKey')}
        </label>
        <div className="relative">
          <input
            type={showApiKey ? 'text' : 'password'}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={presetConfig.placeholder}
            className="w-full h-8 px-2.5 pr-8 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors font-mono"
          />
          <button
            type="button"
            onClick={() => setShowApiKey((v) => !v)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          >
            {showApiKey ? <EyeOff size={13} /> : <Eye size={13} />}
          </button>
        </div>
      </div>
      <div>
        <label className="text-[11px] text-muted-foreground mb-1 block">{t('builtin.model')}</label>
        <div className="relative">
          <input
            value={modelName}
            onChange={(e) => setModelName(e.target.value)}
            placeholder={presetConfig.modelPlaceholder}
            className="w-full h-8 px-2.5 pr-16 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors font-mono"
          />
          <button
            type="button"
            onClick={handleFetchModels}
            disabled={modelLoading}
            title={t('builtin.searchModels')}
            className="absolute right-1.5 top-1/2 -translate-y-1/2 h-6 px-1.5 rounded flex items-center gap-1 text-muted-foreground hover:text-foreground hover:bg-secondary/50 transition-colors disabled:opacity-50"
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
      {preset === 'custom' && (
        <div>
          <label className="text-[11px] text-muted-foreground mb-1 block">
            {t('builtin.apiFormat')}
          </label>
          <div className="flex gap-1">
            {(['openai-compat', 'anthropic'] as const).map((fmt) => (
              <button
                key={fmt}
                type="button"
                onClick={() => setCustomApiFormat(fmt)}
                className={cn(
                  'flex-1 h-7 text-[11px] rounded-md border transition-colors',
                  customApiFormat === fmt
                    ? 'bg-primary text-primary-foreground border-primary'
                    : 'bg-card text-muted-foreground border-input hover:bg-accent',
                )}
              >
                {fmt === 'openai-compat' ? t('builtin.openaiCompat') : 'Anthropic'}
              </button>
            ))}
          </div>
        </div>
      )}
      {showBaseURL && (
        <div>
          <label className="text-[11px] text-muted-foreground mb-1 block">
            {isBaseURLLocked ? t('builtin.baseUrl') : t('builtin.baseUrlRequired')}
          </label>
          <input
            value={baseURL}
            onChange={(e) => setBaseURL(e.target.value)}
            placeholder={t('builtin.baseUrlPlaceholder')}
            readOnly={isBaseURLLocked}
            className={cn(
              'w-full h-8 px-2.5 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors font-mono',
              isBaseURLLocked && 'opacity-60 cursor-default',
            )}
          />
        </div>
      )}
      <div className="flex items-center justify-end gap-2 pt-1">
        <Button variant="ghost" size="sm" onClick={onCancel} className="h-7 px-3 text-[11px]">
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
              ...(showBaseURL && baseURL.trim() ? { baseURL: baseURL.trim() } : {}),
              enabled: initial?.enabled ?? true,
            })
          }
          disabled={!canSave}
          className="h-7 px-3 text-[11px]"
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
