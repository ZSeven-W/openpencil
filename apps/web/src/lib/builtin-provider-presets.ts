import type {
  BuiltinProviderConfig,
  BuiltinProviderPreset,
} from '@/stores/agent-settings-store';

export interface PresetRegion {
  baseURL: string;
}

export interface BuiltinPresetConfig {
  label: string;
  type: 'anthropic' | 'openai-compat';
  baseURL?: string;
  placeholder: string;
  modelPlaceholder: string;
  regions?: { cn: PresetRegion; global: PresetRegion };
}

export const BUILTIN_PROVIDER_PRESETS: Record<BuiltinProviderPreset, BuiltinPresetConfig> = {
  anthropic: {
    label: 'Anthropic',
    type: 'anthropic',
    baseURL: 'https://api.anthropic.com',
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
    type: 'anthropic',
    baseURL: 'https://api.minimaxi.com/anthropic',
    placeholder: 'eyJ...',
    modelPlaceholder: 'MiniMax-M2.7',
    regions: {
      cn: { baseURL: 'https://api.minimaxi.com/anthropic' },
      global: { baseURL: 'https://api.minimax.io/anthropic' },
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
      global: { baseURL: 'https://api.z.ai/api/paas/v4' },
    },
  },
  'glm-coding': {
    label: 'GLM Coding Plan',
    type: 'openai-compat',
    placeholder: 'xxx.yyy',
    modelPlaceholder: 'glm-4.7',
    regions: {
      cn: { baseURL: 'https://open.bigmodel.cn/api/coding/paas/v4' },
      global: { baseURL: 'https://api.z.ai/api/coding/paas/v4' },
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
  'ark-coding': {
    label: 'Ark Coding Plan',
    type: 'openai-compat',
    baseURL: 'https://ark.cn-beijing.volces.com/api/coding/v3',
    placeholder: 'ARK API Key',
    modelPlaceholder: 'ark-code-latest',
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

const PRESET_URL_LOOKUP = Object.entries(BUILTIN_PROVIDER_PRESETS).reduce(
  (acc, [key, cfg]) => {
    if (cfg.baseURL) acc[cfg.baseURL] = key as BuiltinProviderPreset;
    if (cfg.regions) {
      acc[cfg.regions.cn.baseURL] = key as BuiltinProviderPreset;
      acc[cfg.regions.global.baseURL] = key as BuiltinProviderPreset;
    }
    return acc;
  },
  {} as Record<string, BuiltinProviderPreset>,
);

const LEGACY_URL_LOOKUP: Record<string, BuiltinProviderPreset> = {
  'https://api.anthropic.com/v1': 'anthropic',
  'https://api.openai.com': 'openai',
  'https://api.minimaxi.com/anthropic/v1': 'minimax',
  'https://api.minimax.io/anthropic/v1': 'minimax',
  'https://ark.cn-beijing.volces.com/api/v3/v1': 'doubao',
  'https://ark.cn-beijing.volces.com/api/coding/v3/v1': 'ark-coding',
  'https://open.z.ai/api/paas/v4': 'zhipu',
  'https://open.z.ai/api/coding/paas/v4': 'glm-coding',
};

const LEGACY_GLOBAL_URL_LOOKUP: Partial<Record<BuiltinProviderPreset, Set<string>>> = {
  zhipu: new Set(['https://open.z.ai/api/paas/v4']),
  'glm-coding': new Set(['https://open.z.ai/api/coding/paas/v4']),
};

function normalizeURL(url?: string): string {
  return url?.trim().replace(/\/+$/, '') ?? '';
}

function normalizeKnownMalformedOpenAICompatURL(url?: string): string {
  const normalizedURL = normalizeURL(url);
  if (normalizedURL === 'https://ark.cn-beijing.volces.com/api/v3/v1') {
    return 'https://ark.cn-beijing.volces.com/api/v3';
  }
  if (normalizedURL === 'https://ark.cn-beijing.volces.com/api/coding/v3/v1') {
    return 'https://ark.cn-beijing.volces.com/api/coding/v3';
  }
  if (normalizedURL === 'https://open.bigmodel.cn/api/paas/v4/v1') {
    return 'https://open.bigmodel.cn/api/paas/v4';
  }
  if (
    normalizedURL === 'https://api.z.ai/api/paas/v4/v1' ||
    normalizedURL === 'https://open.z.ai/api/paas/v4/v1'
  ) {
    return 'https://api.z.ai/api/paas/v4';
  }
  if (normalizedURL === 'https://open.bigmodel.cn/api/coding/paas/v4/v1') {
    return 'https://open.bigmodel.cn/api/coding/paas/v4';
  }
  if (
    normalizedURL === 'https://api.z.ai/api/coding/paas/v4/v1' ||
    normalizedURL === 'https://open.z.ai/api/coding/paas/v4/v1'
  ) {
    return 'https://api.z.ai/api/coding/paas/v4';
  }
  if (normalizedURL === 'https://generativelanguage.googleapis.com/v1beta/openai/v1') {
    return 'https://generativelanguage.googleapis.com/v1beta/openai';
  }
  return normalizedURL;
}

function lookupPresetByURL(url?: string): BuiltinProviderPreset | undefined {
  const normalizedURL = normalizeKnownMalformedOpenAICompatURL(url);
  if (!normalizedURL) return undefined;
  return PRESET_URL_LOOKUP[normalizedURL] ?? LEGACY_URL_LOOKUP[normalizedURL];
}

function inferRegionFromURL(
  preset: BuiltinProviderPreset,
  normalizedURL: string,
): 'cn' | 'global' {
  const regions = BUILTIN_PROVIDER_PRESETS[preset].regions;
  if (!regions) return 'cn';
  const legacyGlobalURLs = LEGACY_GLOBAL_URL_LOOKUP[preset];
  return normalizedURL === regions.global.baseURL ||
    normalizedURL === `${regions.global.baseURL}/v1` ||
    legacyGlobalURLs?.has(normalizedURL)
    ? 'global'
    : 'cn';
}

export function inferBuiltinProviderPreset(
  config: Pick<BuiltinProviderConfig, 'preset' | 'type' | 'baseURL'>,
): BuiltinProviderPreset {
  if (config.preset) return config.preset;

  const presetFromURL = lookupPresetByURL(config.baseURL);
  if (presetFromURL) {
    return presetFromURL;
  }

  return config.type === 'anthropic' ? 'anthropic' : 'custom';
}

export function inferBuiltinProviderRegion(
  config: Pick<BuiltinProviderConfig, 'preset' | 'type' | 'baseURL'>,
): 'cn' | 'global' {
  return inferRegionFromURL(inferBuiltinProviderPreset(config), normalizeURL(config.baseURL));
}

export function getCanonicalBuiltinBaseURL(
  preset: BuiltinProviderPreset,
  region: 'cn' | 'global' = 'cn',
): string | undefined {
  const cfg = BUILTIN_PROVIDER_PRESETS[preset];
  if (cfg.regions) return cfg.regions[region].baseURL;
  return cfg.baseURL;
}

export function canonicalizeBuiltinProviderConfig(
  config: BuiltinProviderConfig,
): BuiltinProviderConfig {
  if (config.preset === 'custom') {
    const normalizedBaseURL = normalizeKnownMalformedOpenAICompatURL(config.baseURL);
    return normalizedBaseURL && normalizedBaseURL !== config.baseURL
      ? { ...config, baseURL: normalizedBaseURL }
      : config;
  }

  const preset = lookupPresetByURL(config.baseURL) ?? inferBuiltinProviderPreset(config);
  if (preset === 'custom') return config;

  const region = inferRegionFromURL(preset, normalizeKnownMalformedOpenAICompatURL(config.baseURL));
  const canonicalBaseURL = getCanonicalBuiltinBaseURL(preset, region);

  return {
    ...config,
    preset,
    ...(canonicalBaseURL ? { baseURL: canonicalBaseURL } : {}),
  };
}
