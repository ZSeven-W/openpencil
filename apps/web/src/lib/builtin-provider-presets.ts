import type { BuiltinProviderConfig, BuiltinProviderPreset } from '@/stores/agent-settings-store';

export interface PresetRegion {
  baseURL: string;
}

export interface BuiltinPresetConfig {
  label: string;
  type: 'anthropic' | 'openai-compat';
  baseURL?: string;
  /** Alternative baseURL for the other API format (if provider supports both) */
  altBaseURL?: string;
  /** Region-specific alternative baseURLs (overrides altBaseURL when region is selected) */
  altRegions?: { cn: string; global: string };
  /** The API format that altBaseURL corresponds to */
  altType?: 'anthropic' | 'openai-compat';
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
    altBaseURL: 'https://openrouter.ai/api',
    altType: 'anthropic',
    placeholder: 'sk-or-...',
    modelPlaceholder: 'anthropic/claude-sonnet-4.6',
  },
  deepseek: {
    label: 'DeepSeek',
    type: 'openai-compat',
    baseURL: 'https://api.deepseek.com/v1',
    altBaseURL: 'https://api.deepseek.com/anthropic',
    altType: 'anthropic',
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
    altBaseURL: 'https://api.minimaxi.com/v1',
    altRegions: { cn: 'https://api.minimaxi.com/v1', global: 'https://api.minimax.io/v1' },
    altType: 'openai-compat',
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
    altBaseURL: 'https://open.bigmodel.cn/api/anthropic',
    altRegions: {
      cn: 'https://open.bigmodel.cn/api/anthropic',
      global: 'https://api.z.ai/api/anthropic',
    },
    altType: 'anthropic',
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
    altBaseURL: 'https://open.bigmodel.cn/api/anthropic',
    altRegions: {
      cn: 'https://open.bigmodel.cn/api/anthropic',
      global: 'https://api.z.ai/api/anthropic',
    },
    altType: 'anthropic',
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
    altBaseURL: 'https://api.moonshot.cn/anthropic',
    altRegions: {
      cn: 'https://api.moonshot.cn/anthropic',
      global: 'https://api.moonshot.ai/anthropic',
    },
    altType: 'anthropic',
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
    altBaseURL: 'https://dashscope.aliyuncs.com/apps/anthropic',
    altRegions: {
      cn: 'https://dashscope.aliyuncs.com/apps/anthropic',
      global: 'https://dashscope-intl.aliyuncs.com/apps/anthropic',
    },
    altType: 'anthropic',
    placeholder: 'sk-...',
    modelPlaceholder: 'qwen-plus',
    regions: {
      cn: { baseURL: 'https://dashscope.aliyuncs.com/compatible-mode/v1' },
      global: { baseURL: 'https://dashscope-intl.aliyuncs.com/compatible-mode/v1' },
    },
  },
  'bailian-coding': {
    label: 'Bailian Coding Plan',
    type: 'openai-compat',
    baseURL: 'https://coding.dashscope.aliyuncs.com/v1',
    altBaseURL: 'https://coding.dashscope.aliyuncs.com/apps/anthropic',
    altRegions: {
      cn: 'https://coding.dashscope.aliyuncs.com/apps/anthropic',
      global: 'https://coding-intl.dashscope.aliyuncs.com/apps/anthropic',
    },
    altType: 'anthropic',
    placeholder: 'sk-sp-...',
    modelPlaceholder: 'qwen3-coder-plus',
    regions: {
      cn: { baseURL: 'https://coding.dashscope.aliyuncs.com/v1' },
      global: { baseURL: 'https://coding-intl.dashscope.aliyuncs.com/v1' },
    },
  },
  doubao: {
    label: 'DouBao Seed',
    type: 'openai-compat',
    baseURL: 'https://ark.cn-beijing.volces.com/api/v3',
    altBaseURL: 'https://ark.cn-beijing.volces.com/api/coding',
    altType: 'anthropic',
    placeholder: 'ARK API Key',
    modelPlaceholder: 'doubao-seed-2.0-pro',
  },
  'ark-coding': {
    label: 'Ark Coding Plan',
    type: 'openai-compat',
    baseURL: 'https://ark.cn-beijing.volces.com/api/coding/v3',
    altBaseURL: 'https://ark.cn-beijing.volces.com/api/coding',
    altType: 'anthropic',
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
    altBaseURL: 'https://api-inference.modelscope.cn',
    altType: 'anthropic',
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
  'stepfun-coding': {
    label: 'StepFun Coding Plan',
    type: 'openai-compat',
    baseURL: 'https://api.stepfun.com/step_plan/v1',
    placeholder: 'API Key',
    modelPlaceholder: 'step-3-coding',
    regions: {
      cn: { baseURL: 'https://api.stepfun.com/step_plan/v1' },
      global: { baseURL: 'https://api.stepfun.ai/step_plan/v1' },
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
    const k = key as BuiltinProviderPreset;
    if (cfg.baseURL) acc[cfg.baseURL] = k;
    if (cfg.regions) {
      acc[cfg.regions.cn.baseURL] = k;
      acc[cfg.regions.global.baseURL] = k;
    }
    // Include alternative-format URLs so a saved Anthropic-format config
    // for an OpenAI-default preset (or vice versa) still maps back to the
    // correct preset on reload. Without this the canonicalize pass falls
    // through to inferBuiltinProviderPreset and may collapse to 'custom'.
    if (cfg.altBaseURL) acc[cfg.altBaseURL] = k;
    if (cfg.altRegions) {
      acc[cfg.altRegions.cn] = k;
      acc[cfg.altRegions.global] = k;
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

function lookupPresetByURL(url?: string): BuiltinProviderPreset | undefined {
  const normalizedURL = normalizeURL(url);
  if (!normalizedURL) return undefined;
  return PRESET_URL_LOOKUP[normalizedURL] ?? LEGACY_URL_LOOKUP[normalizedURL];
}

/** Whether `url` equals `base`, or `base` followed by a `/v<digits>` segment.
 *  Catches legacy entries where an extra version suffix was appended manually
 *  (`/v1`, `/v3`, etc.) on top of a base that already has its own version. */
function urlMatchesIgnoringVersionSuffix(url: string, base: string): boolean {
  if (url === base) return true;
  if (!url.startsWith(base + '/')) return false;
  const tail = url.slice(base.length + 1);
  return /^v\d+$/.test(tail);
}

function inferRegionFromURL(preset: BuiltinProviderPreset, normalizedURL: string): 'cn' | 'global' {
  const cfg = BUILTIN_PROVIDER_PRESETS[preset];
  const regions = cfg.regions;
  const altRegions = cfg.altRegions;
  if (!regions && !altRegions) return 'cn';
  const legacyGlobalURLs = LEGACY_GLOBAL_URL_LOOKUP[preset];
  const isGlobal =
    (regions && urlMatchesIgnoringVersionSuffix(normalizedURL, regions.global.baseURL)) ||
    (altRegions && urlMatchesIgnoringVersionSuffix(normalizedURL, altRegions.global)) ||
    legacyGlobalURLs?.has(normalizedURL);
  return isGlobal ? 'global' : 'cn';
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

/** Get baseURL for a specific API format. Returns altBaseURL if format matches altType. */
export function getBaseURLForFormat(
  preset: BuiltinProviderPreset,
  format: 'anthropic' | 'openai-compat',
  region: 'cn' | 'global' = 'cn',
): string | undefined {
  const cfg = BUILTIN_PROVIDER_PRESETS[preset];
  if (format === cfg.altType) {
    if (cfg.altRegions) return cfg.altRegions[region];
    if (cfg.altBaseURL) return cfg.altBaseURL;
  }
  if (cfg.regions) return cfg.regions[region].baseURL;
  return cfg.baseURL;
}

/** Check if a preset supports a given API format (has altBaseURL for it). */
export function presetSupportsFormat(
  preset: BuiltinProviderPreset,
  format: 'anthropic' | 'openai-compat',
): boolean {
  const cfg = BUILTIN_PROVIDER_PRESETS[preset];
  return cfg.type === format || cfg.altType === format;
}

export function getCanonicalBuiltinBaseURL(
  preset: BuiltinProviderPreset,
  region: 'cn' | 'global' = 'cn',
): string | undefined {
  const cfg = BUILTIN_PROVIDER_PRESETS[preset];
  if (cfg.regions) return cfg.regions[region].baseURL;
  return cfg.baseURL;
}

/** Whether the given preset's URL family covers `normalizedURL`. */
function presetMatchesURL(preset: BuiltinProviderPreset, normalizedURL: string): boolean {
  const cfg = BUILTIN_PROVIDER_PRESETS[preset];
  if (cfg.baseURL && urlMatchesIgnoringVersionSuffix(normalizedURL, cfg.baseURL)) return true;
  if (cfg.regions) {
    if (urlMatchesIgnoringVersionSuffix(normalizedURL, cfg.regions.cn.baseURL)) return true;
    if (urlMatchesIgnoringVersionSuffix(normalizedURL, cfg.regions.global.baseURL)) return true;
  }
  if (cfg.altBaseURL && urlMatchesIgnoringVersionSuffix(normalizedURL, cfg.altBaseURL)) return true;
  if (cfg.altRegions) {
    if (urlMatchesIgnoringVersionSuffix(normalizedURL, cfg.altRegions.cn)) return true;
    if (urlMatchesIgnoringVersionSuffix(normalizedURL, cfg.altRegions.global)) return true;
  }
  return false;
}

export function canonicalizeBuiltinProviderConfig(
  config: BuiltinProviderConfig,
): BuiltinProviderConfig {
  if (config.preset === 'custom') return config;

  const normalizedURL = normalizeURL(config.baseURL);
  // Respect an explicit preset when its URL family covers the configured
  // baseURL — this is the path that disambiguates presets sharing the same
  // alt URL (e.g. zhipu vs glm-coding both point at /api/anthropic). When
  // the explicit preset is genuinely stale (URL no longer fits the family),
  // fall back to URL-based lookup so legacy entries can self-heal.
  // Note: config.preset === 'custom' is already handled by the early return above,
  // so config.preset here is non-custom (or undefined).
  const preset =
    config.preset &&
    BUILTIN_PROVIDER_PRESETS[config.preset] &&
    presetMatchesURL(config.preset, normalizedURL)
      ? config.preset
      : (lookupPresetByURL(config.baseURL) ?? inferBuiltinProviderPreset(config));
  if (preset === 'custom') return config;

  const region = inferRegionFromURL(preset, normalizedURL);
  // Pick the canonical URL for the user's chosen API format. Without this
  // the alternative-format selection (e.g. Anthropic on a preset whose
  // default is OpenAI-compat) would be silently overwritten on save.
  const canonicalBaseURL =
    getBaseURLForFormat(preset, config.type, region) ?? getCanonicalBuiltinBaseURL(preset, region);

  return {
    ...config,
    preset,
    ...(canonicalBaseURL ? { baseURL: canonicalBaseURL } : {}),
  };
}
