import { useState, useCallback } from 'react';
import type { ComponentType, SVGProps } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, Loader2, Unplug, Download, ExternalLink } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import { BuiltinProvidersSection } from './builtin-provider-settings';
import type { AIProviderType, GroupedModel } from '@/types/agent-settings';
import ClaudeLogo from '@/components/icons/claude-logo';
import OpenAILogo from '@/components/icons/openai-logo';
import OpenCodeLogo from '@/components/icons/opencode-logo';
import CopilotLogo from '@/components/icons/copilot-logo';
import GeminiLogo from '@/components/icons/gemini-logo';

/** Provider display metadata -- labels/descriptions are i18n keys resolved at render time */
const PROVIDER_META: Record<
  AIProviderType,
  {
    labelKey: string;
    descriptionKey: string;
    agent: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot' | 'gemini-cli';
    Icon: ComponentType<SVGProps<SVGSVGElement>>;
  }
> = {
  anthropic: {
    labelKey: 'agents.claudeCode',
    descriptionKey: 'agents.claudeModels',
    agent: 'claude-code',
    Icon: ClaudeLogo,
  },
  openai: {
    labelKey: 'agents.codexCli',
    descriptionKey: 'agents.openaiModels',
    agent: 'codex-cli',
    Icon: OpenAILogo,
  },
  opencode: {
    labelKey: 'agents.opencode',
    descriptionKey: 'agents.opencodeDesc',
    agent: 'opencode',
    Icon: OpenCodeLogo,
  },
  copilot: {
    labelKey: 'agents.copilot',
    descriptionKey: 'agents.copilotDesc',
    agent: 'copilot',
    Icon: CopilotLogo,
  },
  gemini: {
    labelKey: 'agents.geminiCli',
    descriptionKey: 'agents.geminiDesc',
    agent: 'gemini-cli',
    Icon: GeminiLogo,
  },
};

async function connectAgent(
  agent: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot' | 'gemini-cli',
): Promise<{
  connected: boolean;
  models: GroupedModel[];
  error?: string;
  warning?: string;
  notInstalled?: boolean;
  connectionInfo?: string;
  hintPath?: string;
}> {
  try {
    const res = await fetch('/api/ai/connect-agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ agent }),
    });
    if (!res.ok) return { connected: false, models: [], error: `server_error_${res.status}` };
    return await res.json();
  } catch {
    return { connected: false, models: [], error: 'connection_failed' };
  }
}

async function installAgent(
  agent: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot' | 'gemini-cli',
): Promise<{ success: boolean; error?: string; command?: string; docsUrl?: string }> {
  try {
    const res = await fetch('/api/ai/install-agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ agent }),
    });
    if (!res.ok) return { success: false, error: `Server error ${res.status}` };
    return await res.json();
  } catch {
    return { success: false, error: 'Request failed' };
  }
}

/* ---------- ProviderCard ---------- */
function ProviderCard({ type }: { type: AIProviderType }) {
  const { t } = useTranslation();
  const provider = useAgentSettingsStore((s) => s.providers[type]);
  const connect = useAgentSettingsStore((s) => s.connectProvider);
  const disconnect = useAgentSettingsStore((s) => s.disconnectProvider);
  const persist = useAgentSettingsStore((s) => s.persist);

  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [notInstalled, setNotInstalled] = useState(false);
  const [isInstalling, setIsInstalling] = useState(false);
  const [installInfo, setInstallInfo] = useState<{ command: string; docsUrl: string } | null>(null);

  const meta = PROVIDER_META[type];

  const handleConnect = useCallback(async () => {
    setIsConnecting(true);
    setError(null);
    setWarning(null);
    setNotInstalled(false);
    setInstallInfo(null);
    const result = await connectAgent(meta.agent);
    if (result.connected) {
      connect(type, meta.agent, result.models, result.connectionInfo, result.hintPath);
      persist();
      if (result.warning) setWarning(result.warning);
    } else if (result.notInstalled) {
      setNotInstalled(true);
    } else {
      if (result.error?.startsWith('server_error_')) {
        const status = result.error.replace('server_error_', '');
        setError(t('agents.serverError', { status }));
      } else if (result.error && result.error !== 'connection_failed') {
        setError(result.error);
      } else {
        setError(t('agents.connectionFailed'));
      }
    }
    setIsConnecting(false);
  }, [type, meta.agent, connect, persist, t]);

  const handleInstall = useCallback(async () => {
    setIsInstalling(true);
    setError(null);
    setInstallInfo(null);
    const result = await installAgent(meta.agent);
    if (result.success) {
      setIsInstalling(false);
      setNotInstalled(false);
      handleConnect();
    } else {
      setIsInstalling(false);
      setError(result.error || t('agents.installFailed'));
      if (result.command || result.docsUrl) {
        setInstallInfo({
          command: result.command || '',
          docsUrl: result.docsUrl || '',
        });
      }
    }
  }, [meta.agent, handleConnect, t]);

  const handleDisconnect = useCallback(() => {
    disconnect(type);
    setError(null);
    setNotInstalled(false);
    setInstallInfo(null);
    persist();
  }, [type, disconnect, persist]);

  const { Icon } = meta;

  const renderAction = () => {
    if (provider.isConnected) {
      return (
        <Button
          variant="ghost"
          size="sm"
          onClick={handleDisconnect}
          className="h-7 px-2.5 text-[11px] text-muted-foreground hover:text-destructive shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
        >
          <Unplug size={11} className="mr-1" />
          {t('common.disconnect')}
        </Button>
      );
    }
    if (isInstalling) {
      return (
        <Button size="sm" disabled className="h-7 px-3 text-[11px] shrink-0">
          <Loader2 size={11} className="animate-spin mr-1" />
          {t('agents.installing')}
        </Button>
      );
    }
    if (notInstalled && !installInfo) {
      return (
        <Button size="sm" onClick={handleInstall} className="h-7 px-3 text-[11px] shrink-0">
          <Download size={11} className="mr-1" />
          {t('agents.install')}
        </Button>
      );
    }
    return (
      <Button
        size="sm"
        onClick={handleConnect}
        disabled={isConnecting}
        className="h-7 px-3 text-[11px] shrink-0"
      >
        {isConnecting ? <Loader2 size={11} className="animate-spin" /> : t('common.connect')}
      </Button>
    );
  };

  return (
    <div className="group">
      <div
        className={cn(
          'flex items-center gap-3 px-3.5 py-2.5 rounded-lg border transition-colors',
          provider.isConnected
            ? 'bg-secondary/30 border-border'
            : 'border-transparent hover:bg-secondary/20',
        )}
      >
        {/* Icon */}
        <div
          className={cn(
            'w-9 h-9 rounded-lg flex items-center justify-center shrink-0 transition-colors',
            provider.isConnected
              ? 'bg-foreground/8 text-foreground'
              : 'bg-secondary text-muted-foreground',
          )}
        >
          <Icon className="w-5 h-5" />
        </div>

        {/* Name + status */}
        <div className="flex-1 min-w-0">
          <span className="text-[13px] font-medium text-foreground leading-tight block">
            {t(meta.labelKey)}
          </span>
          {provider.isConnected && provider.connectionInfo && (
            <span className="text-[11px] text-green-500 leading-tight flex items-center gap-1 mt-0.5">
              <Check size={10} strokeWidth={2.5} />
              {provider.connectionInfo}
            </span>
          )}
          {provider.isConnected && !provider.connectionInfo && (
            <span className="text-[11px] text-green-500 leading-tight flex items-center gap-1 mt-0.5">
              <Check size={10} strokeWidth={2.5} />
              {t('agents.modelCount', { count: provider.models.length })}
            </span>
          )}
          {!provider.isConnected && !notInstalled && !error && (
            <span className="text-[11px] text-muted-foreground leading-tight mt-0.5 block">
              {t(meta.descriptionKey)}
            </span>
          )}
          {notInstalled && !isInstalling && !error && (
            <span className="text-[11px] text-amber-500 leading-tight mt-0.5 block">
              {t('agents.notInstalled')}
            </span>
          )}
          {error && (
            <span className="text-[11px] text-destructive leading-tight mt-0.5 block">{error}</span>
          )}
          {warning && !error && (
            <span className="text-[11px] text-amber-500 leading-tight mt-0.5 block">{warning}</span>
          )}
        </div>

        {/* Action */}
        {renderAction()}
      </div>

      {/* Install instructions (shown after install failure) */}
      {installInfo && (
        <div className="mx-3 mt-1 mb-1 px-2.5 py-2 rounded-md bg-secondary/30 flex items-center gap-2">
          {installInfo.command && (
            <code className="text-[10px] text-foreground font-mono flex-1 truncate select-all">
              {installInfo.command}
            </code>
          )}
          {installInfo.docsUrl && (
            <a
              href={installInfo.docsUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[10px] text-blue-500 hover:underline inline-flex items-center gap-0.5 shrink-0"
            >
              {t('agents.viewDocs')}
              <ExternalLink size={9} />
            </a>
          )}
        </div>
      )}

      {/* Provider-specific hint */}
      {provider.isConnected && provider.hintPath && (
        <p className="text-[10px] text-muted-foreground/60 px-3.5 mt-1">
          {t('settings.envHint', { path: provider.hintPath })}
        </p>
      )}
    </div>
  );
}

/* ---------- ProvidersTab ---------- */
export function ProvidersTab() {
  const { t } = useTranslation();
  return (
    <div>
      <div className="mb-6">
        <BuiltinProvidersSection />
      </div>
      <h3 className="text-[15px] font-semibold text-foreground mb-4">{t('settings.agents')}</h3>
      <div className="space-y-1">
        <ProviderCard type="anthropic" />
        <ProviderCard type="openai" />
        <ProviderCard type="opencode" />
        <ProviderCard type="copilot" />
        <ProviderCard type="gemini" />
      </div>
    </div>
  );
}
