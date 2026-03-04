import { useState, useEffect, useCallback, useRef } from 'react'
import type { ComponentType, SVGProps } from 'react'
import { useTranslation } from 'react-i18next'
import { X, Check, Loader2, Unplug, AlertCircle, Zap, Terminal } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import type { AIProviderType, MCPTransportMode, GroupedModel } from '@/types/agent-settings'
import ClaudeLogo from '@/components/icons/claude-logo'
import OpenAILogo from '@/components/icons/openai-logo'
import OpenCodeLogo from '@/components/icons/opencode-logo'

/** Provider display metadata — labels/descriptions are i18n keys resolved at render time */
const PROVIDER_META: Record<
  AIProviderType,
  { labelKey: string; descriptionKey: string; agent: 'claude-code' | 'codex-cli' | 'opencode'; Icon: ComponentType<SVGProps<SVGSVGElement>> }
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
}

async function connectAgent(
  agent: 'claude-code' | 'codex-cli' | 'opencode',
): Promise<{ connected: boolean; models: GroupedModel[]; error?: string }> {
  try {
    const res = await fetch('/api/ai/connect-agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ agent }),
    })
    if (!res.ok) return { connected: false, models: [], error: `server_error_${res.status}` }
    return await res.json()
  } catch {
    return { connected: false, models: [], error: 'connection_failed' }
  }
}

async function callMcpInstall(
  tool: string,
  action: 'install' | 'uninstall',
  transportMode?: MCPTransportMode,
  httpPort?: number,
): Promise<{ success: boolean; error?: string }> {
  const res = await fetch('/api/ai/mcp-install', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ tool, action, transportMode, httpPort }),
  })
  return res.json()
}

function ProviderRow({ type }: { type: AIProviderType }) {
  const { t } = useTranslation()
  const provider = useAgentSettingsStore((s) => s.providers[type])
  const connect = useAgentSettingsStore((s) => s.connectProvider)
  const disconnect = useAgentSettingsStore((s) => s.disconnectProvider)
  const persist = useAgentSettingsStore((s) => s.persist)

  const [isConnecting, setIsConnecting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const meta = PROVIDER_META[type]

  const handleConnect = useCallback(async () => {
    setIsConnecting(true)
    setError(null)
    const result = await connectAgent(meta.agent)
    if (result.connected) {
      connect(type, meta.agent, result.models)
      persist()
    } else {
      if (result.error?.startsWith('server_error_')) {
        const status = result.error.replace('server_error_', '')
        setError(t('agents.serverError', { status }))
      } else {
        setError(t('agents.connectionFailed'))
      }
    }
    setIsConnecting(false)
  }, [type, meta.agent, connect, persist, t])

  const handleDisconnect = useCallback(() => {
    disconnect(type)
    setError(null)
    persist()
  }, [type, disconnect, persist])

  const { Icon } = meta

  return (
    <div className="group">
      <div
        className={cn(
          'flex items-center gap-3 px-3 py-2.5 rounded-lg transition-colors',
          provider.isConnected
            ? 'bg-secondary/40'
            : 'hover:bg-secondary/30',
        )}
      >
        {/* Icon */}
        <div
          className={cn(
            'w-7 h-7 rounded-md flex items-center justify-center shrink-0 transition-colors',
            provider.isConnected ? 'bg-foreground/10 text-foreground' : 'bg-secondary text-muted-foreground',
          )}
        >
          <Icon className="w-4 h-4" />
        </div>

        {/* Name + description */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-[13px] font-medium text-foreground leading-tight">{t(meta.labelKey)}</span>
            <span className="text-[10px] text-muted-foreground leading-tight hidden sm:inline">{t(meta.descriptionKey)}</span>
          </div>
          {provider.isConnected && (
            <span className="text-[11px] text-green-500 leading-tight flex items-center gap-1 mt-0.5">
              <Check size={10} strokeWidth={2.5} />
              {t('agents.modelCount', { count: provider.models.length })}
            </span>
          )}
          {error && (
            <span className="text-[10px] text-destructive leading-tight mt-0.5 block">{error}</span>
          )}
        </div>

        {/* Action */}
        {provider.isConnected ? (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleDisconnect}
            className="h-7 px-2.5 text-[11px] text-muted-foreground hover:text-destructive shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <Unplug size={11} className="mr-1" />
            {t('common.disconnect')}
          </Button>
        ) : (
          <Button
            size="sm"
            onClick={handleConnect}
            disabled={isConnecting}
            className="h-7 px-3 text-[11px] shrink-0"
          >
            {isConnecting ? (
              <Loader2 size={11} className="animate-spin" />
            ) : (
              t('common.connect')
            )}
          </Button>
        )}
      </div>
    </div>
  )
}

const TRANSPORT_OPTIONS: { value: MCPTransportMode; labelKey: string }[] = [
  { value: 'stdio', labelKey: 'agents.stdio' },
  { value: 'http', labelKey: 'agents.http' },
  { value: 'both', labelKey: 'agents.stdioHttp' },
]

export default function AgentSettingsDialog() {
  const { t } = useTranslation()
  const open = useAgentSettingsStore((s) => s.dialogOpen)
  const setDialogOpen = useAgentSettingsStore((s) => s.setDialogOpen)
  const mcpIntegrations = useAgentSettingsStore((s) => s.mcpIntegrations)
  const mcpTransportMode = useAgentSettingsStore((s) => s.mcpTransportMode)
  const mcpHttpPort = useAgentSettingsStore((s) => s.mcpHttpPort)
  const toggleMCP = useAgentSettingsStore((s) => s.toggleMCPIntegration)
  const setMCPTransport = useAgentSettingsStore((s) => s.setMCPTransport)
  const persist = useAgentSettingsStore((s) => s.persist)
  const dialogRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setDialogOpen(false)
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [open, setDialogOpen])

  const [mcpInstalling, setMcpInstalling] = useState<string | null>(null)
  const [mcpError, setMcpError] = useState<string | null>(null)

  /** Re-install all enabled CLIs with the current global transport settings */
  const reinstallEnabled = useCallback(
    async (mode: MCPTransportMode, port: number) => {
      const enabled = mcpIntegrations.filter((m) => m.enabled)
      if (enabled.length === 0) return
      setMcpError(null)
      setMcpInstalling('__transport__')
      try {
        for (const m of enabled) {
          const result = await callMcpInstall(
            m.tool,
            'install',
            mode,
            mode !== 'stdio' ? port : undefined,
          )
          if (!result.success) {
            setMcpError(result.error ?? t('agents.failedTransport'))
            return
          }
        }
      } catch {
        setMcpError(t('agents.failedMcpTransport'))
      } finally {
        setMcpInstalling(null)
      }
    },
    [mcpIntegrations],
  )

  const handleToggleMCP = useCallback(
    async (tool: string) => {
      const current = mcpIntegrations.find((m) => m.tool === tool)
      if (!current) return
      const action = current.enabled ? 'uninstall' : 'install'

      setMcpInstalling(tool)
      setMcpError(null)
      try {
        const result = await callMcpInstall(
          tool,
          action,
          action === 'install' ? mcpTransportMode : undefined,
          action === 'install' && mcpTransportMode !== 'stdio' ? mcpHttpPort : undefined,
        )
        if (result.success) {
          toggleMCP(tool)
          persist()
        } else {
          setMcpError(result.error ?? t('agents.failedTo', { action }))
        }
      } catch {
        setMcpError(t('agents.failedToMcp', { action }))
      } finally {
        setMcpInstalling(null)
      }
    },
    [mcpIntegrations, mcpTransportMode, mcpHttpPort, toggleMCP, persist],
  )

  const handleTransportChange = useCallback(
    async (mode: MCPTransportMode) => {
      if (mode === mcpTransportMode) return
      setMCPTransport(mode)
      persist()
      await reinstallEnabled(mode, mcpHttpPort)
    },
    [mcpTransportMode, mcpHttpPort, setMCPTransport, persist, reinstallEnabled],
  )

  const handlePortBlur = useCallback(
    async (value: string) => {
      const port = parseInt(value, 10)
      if (isNaN(port) || port < 1 || port > 65535 || port === mcpHttpPort) return
      setMCPTransport(mcpTransportMode, port)
      persist()
      await reinstallEnabled(mcpTransportMode, port)
    },
    [mcpTransportMode, mcpHttpPort, setMCPTransport, persist, reinstallEnabled],
  )

  if (!open) return null

  const isBusy = mcpInstalling !== null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div
        className="absolute inset-0 bg-background/80"
        onClick={() => setDialogOpen(false)}
      />
      <div
        ref={dialogRef}
        className="relative bg-card rounded-xl border border-border w-[480px] max-h-[80vh] overflow-hidden shadow-xl flex flex-col"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 pt-4 pb-3">
          <h3 className="text-sm font-semibold text-foreground">
            {t('agents.title')}
          </h3>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => setDialogOpen(false)}
          >
            <X size={14} />
          </Button>
        </div>

        {/* Scrollable content */}
        <div className="flex-1 overflow-y-auto px-5 pb-5">
          {/* Agents section */}
          <div className="mb-5">
            <div className="flex items-center gap-2 mb-2 px-1">
              <Zap size={12} className="text-muted-foreground" />
              <h4 className="text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
                {t('agents.agentsOnCanvas')}
              </h4>
            </div>
            <div className="space-y-0.5">
              <ProviderRow type="anthropic" />
              <ProviderRow type="openai" />
              <ProviderRow type="opencode" />
            </div>
          </div>

          {/* Divider */}
          <div className="h-px bg-border mb-5" />

          {/* MCP integrations section */}
          <div>
            <div className="flex items-center gap-2 mb-3 px-1">
              <Terminal size={12} className="text-muted-foreground" />
              <h4 className="text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
                {t('agents.mcpIntegrations')}
              </h4>
            </div>

            {/* Global transport selector */}
            <div className="flex items-center gap-2 mb-3 px-1">
              <span className="text-[11px] text-muted-foreground shrink-0">{t('agents.transport')}</span>
              <Select
                value={mcpTransportMode}
                onValueChange={(v) => handleTransportChange(v as MCPTransportMode)}
                disabled={isBusy}
              >
                <SelectTrigger className="h-6 w-[100px] text-[11px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {TRANSPORT_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>
                      {t(opt.labelKey)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              {mcpTransportMode !== 'stdio' && (
                <>
                  <span className="text-[11px] text-muted-foreground shrink-0">{t('agents.port')}</span>
                  <input
                    type="text"
                    inputMode="numeric"
                    defaultValue={mcpHttpPort}
                    key={mcpHttpPort}
                    onBlur={(e) => handlePortBlur(e.target.value)}
                    disabled={isBusy}
                    className="h-6 w-[52px] text-[11px] text-center tabular-nums bg-secondary text-foreground rounded border border-input focus:border-ring outline-none transition-colors"
                  />
                </>
              )}
              {mcpInstalling === '__transport__' && (
                <Loader2 size={10} className="animate-spin text-muted-foreground shrink-0" />
              )}
            </div>

            <div className="grid grid-cols-2 gap-x-2 gap-y-0.5">
              {mcpIntegrations.map((m) => (
                <div
                  key={m.tool}
                  className={cn(
                    'flex items-center justify-between py-2 px-3 rounded-lg transition-colors',
                    m.enabled ? 'bg-secondary/40' : 'hover:bg-secondary/20',
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
                <p className="text-[10px] text-destructive">{mcpError}</p>
              </div>
            )}
            <p className="text-[10px] text-muted-foreground/60 mt-3 px-1">
              {t('agents.mcpRestart')}
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}
