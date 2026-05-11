import type { ModelGroup } from '@/types/agent-settings';
import type { BuiltinProviderConfig } from '@/stores/agent-settings-store';

export interface CodegenModelConfig {
  model: string;
  provider: string;
  builtinProvider?: BuiltinProviderConfig;
}

export function resolveCodegenModelConfig(
  model: string,
  modelGroups: ModelGroup[],
  builtinProviders: BuiltinProviderConfig[],
): CodegenModelConfig | null {
  if (model.startsWith('builtin:')) {
    const parts = model.split(':');
    const builtinProviderId = parts[1];
    const builtinProvider = builtinProviders.find((item) => item.id === builtinProviderId);
    if (!builtinProvider?.enabled || !builtinProvider.apiKey) return null;
    return { model, provider: 'builtin', builtinProvider };
  }

  const provider = modelGroups.find((group) =>
    group.models.some((item) => item.value === model),
  )?.provider;
  if (!provider) return null;
  return { model, provider };
}
