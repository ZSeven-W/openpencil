import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, Loader2, AlertCircle, Play, Square, Copy } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import type { MCPTransportMode } from '@/types/agent-settings';

async function callMcpInstall(
  tool: string,
  action: 'install' | 'uninstall',
  transportMode?: MCPTransportMode,
  httpPort?: number,
): Promise<{ success: boolean; error?: string; fallbackHttp?: boolean }> {
  const res = await fetch('/api/ai/mcp-install', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ tool, action, transportMode, httpPort }),
  });
  return res.json();
}

export function McpTab() {
  const { t } = useTranslation();
  const mcpIntegrations = useAgentSettingsStore((s) => s.mcpIntegrations);
  const mcpHttpPort = useAgentSettingsStore((s) => s.mcpHttpPort);
  const toggleMCP = useAgentSettingsStore((s) => s.toggleMCPIntegration);
  const setMCPTransport = useAgentSettingsStore((s) => s.setMCPTransport);
  const persist = useAgentSettingsStore((s) => s.persist);
  const mcpServerRunning = useAgentSettingsStore((s) => s.mcpServerRunning);
  const mcpServerLocalIp = useAgentSettingsStore((s) => s.mcpServerLocalIp);
  const setMcpServerStatus = useAgentSettingsStore((s) => s.setMcpServerStatus);

  const [mcpInstalling, setMcpInstalling] = useState<string | null>(null);
  const [mcpError, setMcpError] = useState<string | null>(null);
  const [mcpServerLoading, setMcpServerLoading] = useState(false);
  const [mcpServerError, setMcpServerError] = useState<string | null>(null);
  const [configCopied, setConfigCopied] = useState(false);

  useEffect(() => {
    fetch('/api/mcp/server')
      .then((r) => r.json())
      .then((data: { running: boolean; port: number | null; localIp: string | null }) => {
        setMcpServerStatus(data.running, data.localIp);
      })
      .catch(() => {});
  }, [setMcpServerStatus]);

  const handleMcpServerToggle = useCallback(async () => {
    setMcpServerLoading(true);
    setMcpServerError(null);
    try {
      const action = mcpServerRunning ? 'stop' : 'start';
      const res = await fetch('/api/mcp/server', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action, port: mcpHttpPort }),
      });
      const data = await res.json();
      if (data.error) {
        setMcpServerError(data.error);
      } else {
        setMcpServerStatus(data.running ?? false, data.localIp);
      }
    } catch {
      setMcpServerError(t('agents.failedToMcp', { action: mcpServerRunning ? 'stop' : 'start' }));
    } finally {
      setMcpServerLoading(false);
    }
  }, [mcpServerRunning, mcpHttpPort, setMcpServerStatus, t]);

  const handleCopyConfig = useCallback(() => {
    if (!mcpServerLocalIp) return;
    const config = JSON.stringify(
      { type: 'http', url: `http://${mcpServerLocalIp}:${mcpHttpPort}/mcp` },
      null,
      2,
    );
    navigator.clipboard.writeText(config);
    setConfigCopied(true);
    setTimeout(() => setConfigCopied(false), 2000);
  }, [mcpServerLocalIp, mcpHttpPort]);

  const handleToggleMCP = useCallback(
    async (tool: string) => {
      const current = mcpIntegrations.find((m) => m.tool === tool);
      if (!current) return;
      const action = current.enabled ? 'uninstall' : 'install';

      setMcpInstalling(tool);
      setMcpError(null);
      try {
        const result = await callMcpInstall(tool, action);
        if (result.success) {
          toggleMCP(tool);
          persist();
          if (result.fallbackHttp) {
            setMcpServerStatus(true, null);
            setTimeout(() => {
              fetch('/api/mcp/server')
                .then((r) => r.json())
                .then((data: { running: boolean; localIp: string | null }) => {
                  setMcpServerStatus(data.running, data.localIp);
                })
                .catch(() => {});
            }, 500);
          }
        } else {
          setMcpError(result.error ?? t('agents.failedTo', { action }));
        }
      } catch {
        setMcpError(t('agents.failedToMcp', { action }));
      } finally {
        setMcpInstalling(null);
      }
    },
    [mcpIntegrations, toggleMCP, persist, setMcpServerStatus, t],
  );

  const handlePortBlur = useCallback(
    async (value: string) => {
      const port = parseInt(value, 10);
      if (isNaN(port) || port < 1 || port > 65535 || port === mcpHttpPort) return;
      setMCPTransport('stdio', port);
      persist();
    },
    [mcpHttpPort, setMCPTransport, persist],
  );

  const isBusy = mcpInstalling !== null;

  return (
    <div>
      {/* MCP Server */}
      <div className="mb-6">
        <h3 className="text-[15px] font-semibold text-foreground mb-3">{t('agents.mcpServer')}</h3>
        <div className="flex items-center gap-2.5 px-3.5 py-2.5 rounded-lg border border-border bg-secondary/20">
          <div
            className={cn(
              'w-2 h-2 rounded-full shrink-0',
              mcpServerRunning ? 'bg-green-500' : 'bg-muted-foreground/30',
            )}
          />
          <span className="text-[13px] text-foreground flex-1">
            {mcpServerRunning ? t('agents.mcpServerRunning') : t('agents.mcpServerStopped')}
          </span>
          <span className="text-[11px] text-muted-foreground shrink-0">{t('agents.port')}</span>
          <input
            type="text"
            inputMode="numeric"
            defaultValue={mcpHttpPort}
            key={mcpHttpPort}
            onBlur={(e) => handlePortBlur(e.target.value)}
            disabled={mcpServerRunning || mcpServerLoading}
            className="h-6 w-[52px] text-[11px] text-center tabular-nums bg-secondary text-foreground rounded border border-input focus:border-ring outline-none transition-colors disabled:opacity-50"
          />
          <Button
            size="sm"
            variant={mcpServerRunning ? 'outline' : 'default'}
            onClick={handleMcpServerToggle}
            disabled={mcpServerLoading}
            className="h-7 px-3 text-[11px] shrink-0"
          >
            {mcpServerLoading ? (
              <Loader2 size={11} className="animate-spin" />
            ) : mcpServerRunning ? (
              <>
                <Square size={10} className="mr-1" />
                {t('agents.mcpServerStop')}
              </>
            ) : (
              <>
                <Play size={10} className="mr-1" />
                {t('agents.mcpServerStart')}
              </>
            )}
          </Button>
        </div>
        {mcpServerRunning && mcpServerLocalIp && (
          <div className="mt-2 px-3.5 py-2 rounded-lg bg-secondary/15 border border-border/50">
            <div className="flex items-center justify-between mb-1">
              <span className="text-[11px] text-muted-foreground">
                {t('agents.mcpClientConfig')}
              </span>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={handleCopyConfig}
                className="shrink-0 h-5 w-5"
              >
                {configCopied ? <Check size={9} className="text-green-500" /> : <Copy size={9} />}
              </Button>
            </div>
            <code className="text-[10px] text-muted-foreground font-mono select-all leading-none">
              {`{ "type": "http", "url": "http://${mcpServerLocalIp}:${mcpHttpPort}/mcp" }`}
            </code>
          </div>
        )}
        {mcpServerError && (
          <div className="flex items-center gap-1.5 mt-2 px-1">
            <AlertCircle size={11} className="text-destructive shrink-0" />
            <p className="text-[11px] text-destructive">{mcpServerError}</p>
          </div>
        )}
      </div>

      {/* MCP Integrations */}
      <div>
        <h3 className="text-[15px] font-semibold text-foreground mb-1">
          {t('agents.mcpIntegrations')}
        </h3>
        <p className="text-[11px] text-muted-foreground mb-1">{t('agents.mcpRestart')}</p>
        <p className="text-[11px] text-muted-foreground mb-3">{t('agents.mcpReinstallHint')}</p>
        <div className="grid grid-cols-2 gap-1.5">
          {mcpIntegrations.map((m) => (
            <div
              key={m.tool}
              className={cn(
                'flex items-center justify-between py-2 px-3.5 rounded-lg border transition-colors',
                m.enabled
                  ? 'bg-secondary/30 border-border'
                  : 'border-transparent hover:bg-secondary/20',
              )}
            >
              <div className="flex items-center gap-1.5 min-w-0">
                <span
                  className={cn(
                    'text-[12px] truncate',
                    m.enabled ? 'text-foreground' : 'text-muted-foreground',
                  )}
                >
                  {m.displayName}
                </span>
                {mcpInstalling === m.tool && (
                  <Loader2 size={10} className="animate-spin text-muted-foreground shrink-0" />
                )}
              </div>
              <Switch
                checked={m.enabled}
                disabled={isBusy}
                onCheckedChange={() => handleToggleMCP(m.tool)}
                className="shrink-0 ml-2"
              />
            </div>
          ))}
        </div>
        {mcpError && (
          <div className="flex items-center gap-1.5 mt-2 px-1">
            <AlertCircle size={11} className="text-destructive shrink-0" />
            <p className="text-[11px] text-destructive">{mcpError}</p>
          </div>
        )}
      </div>
    </div>
  );
}
