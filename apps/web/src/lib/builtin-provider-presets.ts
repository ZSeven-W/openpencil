import type { BuiltinProviderConfig, BuiltinProviderPreset } from '@/stores/agent-settings-store';

export interface PresetRegion {
  baseURL: string;
}

export interface BuiltinPresetConfig {
  label: string;
  type: 'anthropic' | 'openai-compat';
  baseURL?: string;
  /** Alternative baseURL 用于其他 API 格式（如果提供商支持两者） */
  altBaseURL?: string;
  /** Region-specific alternative baseURLs (overrides altBaseURL when region is selected) */
  altRegions?: { cn: string; global: string };
  /** The API 对应的格式 */
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
    modelPlaceholder: 'deepseek-v4-pro',
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
    // Include 替代格式 URLs 所以保存了 Anthropic 格式配置
// for an OpenAI-default preset (or vice versa) still maps back to the
    // 重新加载时正确预设。 Without 这个规范化通行证落下
    // 到 inferBuiltinProviderPreset 并可能折叠为“自定义”。
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

/** Whether `url` 等于 `base`，或 `base` 后跟 `/v<digits>` 段。 Catches
 * 旧条目，在已有版本的基础之上手动附加额外版本后缀（`/v1`、`/v3` 等）。
 *  */
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

/** Get baseURL 用于特定的 API 格式。 Returns altBaseURL 如果格式匹配 altType。 */
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

/** Check 如果预设支持给定的 API 格式（有 altBaseURL）。 */
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

/** Whether 给定预设的 URL 系列涵盖 `normalizedURL`。 */
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
  // 当 URL 系列覆盖已配置的 baseURL 时，Respect 是一个显式预设 — 这是消除共享相同 alt URL
  // 的预设歧义的路径（例如 zhipu 与 glm-coding 都指向 /api/anthropic）。 When
  // 显式预设确实已经过时（URL 不再适合该系列），回退到基于 URL 的查找，以便遗留条目可以自我修复。 Note:
  // config.preset === 'custom' 已经被上面的早期返回处理了，所以这里的 config.preset
  // 是非自定义的（或未定义的）。
  const preset =
    config.preset &&
    BUILTIN_PROVIDER_PRESETS[config.preset] &&
    presetMatchesURL(config.preset, normalizedURL)
      ? config.preset
      : (lookupPresetByURL(config.baseURL) ?? inferBuiltinProviderPreset(config));
  if (preset === 'custom') return config;

  const region = inferRegionFromURL(preset, normalizedURL);
  // Pick 用户选择的 API 格式的规范 URL。 Without 这个替代格式选择（例如默认为 OpenAI-compat 的预设上的
  // Anthropic）将在保存时被静默覆盖。
  const canonicalBaseURL =
    getBaseURLForFormat(preset, config.type, region) ?? getCanonicalBuiltinBaseURL(preset, region);

  return {
    ...config,
    preset,
    ...(canonicalBaseURL ? { baseURL: canonicalBaseURL } : {}),
  };
}
