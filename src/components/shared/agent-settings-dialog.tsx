import { useState, useEffect, useCallback, useRef } from 'react'
import type { ComponentType, SVGProps } from 'react'
import { useTranslation } from 'react-i18next'
import {
  X, Check, Loader2, Unplug, AlertCircle, Terminal, Play, Square,
  Copy, Download, ExternalLink, Pen, Settings,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import type { AIProviderType, MCPTransportMode, GroupedModel } from '@/types/agent-settings'
import ClaudeLogo from '@/components/icons/claude-logo'
import OpenAILogo from '@/components/icons/openai-logo'
import OpenCodeLogo from '@/components/icons/opencode-logo'
import CopilotLogo from '@/components/icons/copilot-logo'

/** Provider display metadata — labels/descriptions are i18n keys resolved at render time */
const PROVIDER_META: Record<
  AIProviderType,
  { labelKey: string; descriptionKey: string; agent: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot'; Icon: ComponentType<SVGProps<SVGSVGElement>> }
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
}

type SettingsTab = 'agents' | 'mcp' | 'system'

async function connectAgent(
  agent: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot',
): Promise<{ connected: boolean; models: GroupedModel[]; error?: string; notInstalled?: boolean; connectionInfo?: string; hintPath?: string }> {
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

async function installAgent(
  agent: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot',
): Promise<{ success: boolean; error?: string; command?: string; docsUrl?: string }> {
  try {
    const res = await fetch('/api/ai/install-agent', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ agent }),
    })
    if (!res.ok) return { success: false, error: `Server error ${res.status}` }
    return await res.json()
  } catch {
    return { success: false, error: 'Request failed' }
  }
}

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
  })
  return res.json()
}

/* ---------- ProviderCard ---------- */
function ProviderCard({ type }: { type: AIProviderType }) {
  const { t } = useTranslation()
  const provider = useAgentSettingsStore((s) => s.providers[type])
  const connect = useAgentSettingsStore((s) => s.connectProvider)
  const disconnect = useAgentSettingsStore((s) => s.disconnectProvider)
  const persist = useAgentSettingsStore((s) => s.persist)

  const [isConnecting, setIsConnecting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notInstalled, setNotInstalled] = useState(false)
  const [isInstalling, setIsInstalling] = useState(false)
  const [installInfo, setInstallInfo] = useState<{ command: string; docsUrl: string } | null>(null)

  const meta = PROVIDER_META[type]

  const handleConnect = useCallback(async () => {
    setIsConnecting(true)
    setError(null)
    setNotInstalled(false)
    setInstallInfo(null)
    const result = await connectAgent(meta.agent)
    if (result.connected) {
      connect(type, meta.agent, result.models, result.connectionInfo, result.hintPath)
      persist()
    } else if (result.notInstalled) {
      setNotInstalled(true)
    } else {
      if (result.error?.startsWith('server_error_')) {
        const status = result.error.replace('server_error_', '')
        setError(t('agents.serverError', { status }))
      } else if (result.error && result.error !== 'connection_failed') {
        setError(result.error)
      } else {
        setError(t('agents.connectionFailed'))
      }
    }
    setIsConnecting(false)
  }, [type, meta.agent, connect, persist, t])

  const handleInstall = useCallback(async () => {
    setIsInstalling(true)
    setError(null)
    setInstallInfo(null)
    const result = await installAgent(meta.agent)
    if (result.success) {
      setIsInstalling(false)
      setNotInstalled(false)
      handleConnect()
    } else {
      setIsInstalling(false)
      setError(result.error || t('agents.installFailed'))
      if (result.command || result.docsUrl) {
        setInstallInfo({
          command: result.command || '',
          docsUrl: result.docsUrl || '',
        })
      }
    }
  }, [meta.agent, handleConnect, t])

  const handleDisconnect = useCallback(() => {
    disconnect(type)
    setError(null)
    setNotInstalled(false)
    setInstallInfo(null)
    persist()
  }, [type, disconnect, persist])

  const { Icon } = meta

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
      )
    }
    if (isInstalling) {
      return (
        <Button size="sm" disabled className="h-7 px-3 text-[11px] shrink-0">
          <Loader2 size={11} className="animate-spin mr-1" />
          {t('agents.installing')}
        </Button>
      )
    }
    if (notInstalled && !installInfo) {
      return (
        <Button size="sm" onClick={handleInstall} className="h-7 px-3 text-[11px] shrink-0">
          <Download size={11} className="mr-1" />
          {t('agents.install')}
        </Button>
      )
    }
    return (
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
    )
  }

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
            provider.isConnected ? 'bg-foreground/8 text-foreground' : 'bg-secondary text-muted-foreground',
          )}
        >
          <Icon className="w-5 h-5" />
        </div>

        {/* Name + status */}
        <div className="flex-1 min-w-0">
          <span className="text-[13px] font-medium text-foreground leading-tight block">{t(meta.labelKey)}</span>
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
            <span className="text-[11px] text-muted-foreground leading-tight mt-0.5 block">{t(meta.descriptionKey)}</span>
          )}
          {notInstalled && !isInstalling && !error && (
            <span className="text-[11px] text-amber-500 leading-tight mt-0.5 block">
              {t('agents.notInstalled')}
            </span>
          )}
          {error && (
            <span className="text-[11px] text-destructive leading-tight mt-0.5 block">{error}</span>
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
  )
}

/* ---------- Sidebar nav item ---------- */
function NavItem({ icon: IconComp, label, active, onClick }: {
  icon: ComponentType<{ size?: number; className?: string }>; label: string; active?: boolean; onClick: () => void
}) {
  return (
    <button onClick={onClick} className={cn(
      'flex items-center gap-2.5 w-full px-3 py-1.5 rounded-lg text-[13px] transition-colors text-left',
      active ? 'bg-secondary text-foreground font-medium' : 'text-muted-foreground hover:text-foreground hover:bg-secondary/40',
    )}>
      <IconComp size={14} className="shrink-0" />
      {label}
    </button>
  )
}

/* ---------- Agents page ---------- */
function AgentsPage() {
  const { t } = useTranslation()
  return (
    <div>
      <h3 className="text-[15px] font-semibold text-foreground mb-4">{t('settings.agents')}</h3>
      <div className="space-y-1">
        <ProviderCard type="anthropic" />
        <ProviderCard type="openai" />
        <ProviderCard type="opencode" />
        <ProviderCard type="copilot" />
      </div>
    </div>
  )
}

/* ---------- MCP page ---------- */
interface McpPageProps {
  mcpServerRunning: boolean; mcpServerLocalIp: string | null; mcpServerLoading: boolean
  mcpServerError: string | null; mcpHttpPort: number; configCopied: boolean
  mcpIntegrations: { tool: string; displayName: string; enabled: boolean }[]
  mcpInstalling: string | null; mcpError: string | null; isBusy: boolean
  onServerToggle: () => void; onCopyConfig: () => void
  onPortBlur: (value: string) => void; onToggleMCP: (tool: string) => void
}
function McpPage(props: McpPageProps) {
  const {
    mcpServerRunning, mcpServerLocalIp, mcpServerLoading, mcpServerError,
    mcpHttpPort, mcpIntegrations, mcpInstalling, mcpError, isBusy, configCopied,
    onServerToggle, onCopyConfig, onPortBlur, onToggleMCP,
  } = props
  const { t } = useTranslation()
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
            onBlur={(e) => onPortBlur(e.target.value)}
            disabled={mcpServerRunning || mcpServerLoading}
            className="h-6 w-[52px] text-[11px] text-center tabular-nums bg-secondary text-foreground rounded border border-input focus:border-ring outline-none transition-colors disabled:opacity-50"
          />
          <Button
            size="sm"
            variant={mcpServerRunning ? 'outline' : 'default'}
            onClick={onServerToggle}
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
              <span className="text-[11px] text-muted-foreground">{t('agents.mcpClientConfig')}</span>
              <Button variant="ghost" size="icon-sm" onClick={onCopyConfig} className="shrink-0 h-5 w-5">
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
        <h3 className="text-[15px] font-semibold text-foreground mb-1">{t('agents.mcpIntegrations')}</h3>
        <p className="text-[11px] text-muted-foreground mb-3">{t('agents.mcpRestart')}</p>
        <div className="grid grid-cols-2 gap-1.5">
          {mcpIntegrations.map((m) => (
            <div
              key={m.tool}
              className={cn(
                'flex items-center justify-between py-2 px-3.5 rounded-lg border transition-colors',
                m.enabled ? 'bg-secondary/30 border-border' : 'border-transparent hover:bg-secondary/20',
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
                onCheckedChange={() => onToggleMCP(m.tool)}
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
  )
}

/* ---------- Main Dialog ---------- */
export default function AgentSettingsDialog() {
  const { t } = useTranslation()
  const open = useAgentSettingsStore((s) => s.dialogOpen)
  const setDialogOpen = useAgentSettingsStore((s) => s.setDialogOpen)
  const mcpIntegrations = useAgentSettingsStore((s) => s.mcpIntegrations)
  const mcpHttpPort = useAgentSettingsStore((s) => s.mcpHttpPort)
  const toggleMCP = useAgentSettingsStore((s) => s.toggleMCPIntegration)
  const setMCPTransport = useAgentSettingsStore((s) => s.setMCPTransport)
  const persist = useAgentSettingsStore((s) => s.persist)
  const mcpServerRunning = useAgentSettingsStore((s) => s.mcpServerRunning)
  const mcpServerLocalIp = useAgentSettingsStore((s) => s.mcpServerLocalIp)
  const setMcpServerStatus = useAgentSettingsStore((s) => s.setMcpServerStatus)
  const dialogRef = useRef<HTMLDivElement>(null)

  const [activeTab, setActiveTab] = useState<SettingsTab>('agents')

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
  const [mcpServerLoading, setMcpServerLoading] = useState(false)
  const [mcpServerError, setMcpServerError] = useState<string | null>(null)
  const [configCopied, setConfigCopied] = useState(false)
  const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(true)
  const [isElectron, setIsElectron] = useState(false)

  useEffect(() => {
    setIsElectron(!!window.electronAPI)
  }, [])

  useEffect(() => {
    if (!open) return
    fetch('/api/mcp/server')
      .then((r) => r.json())
      .then((data: { running: boolean; port: number | null; localIp: string | null }) => {
        setMcpServerStatus(data.running, data.localIp)
      })
      .catch(() => {})
  }, [open, setMcpServerStatus])

  useEffect(() => {
    if (!open || !window.electronAPI?.updater?.getAutoCheck) return
    window.electronAPI.updater.getAutoCheck()
      .then(setAutoUpdateEnabled)
      .catch((err) => console.error('[auto-update getAutoCheck]', err))
  }, [open])

  const handleAutoUpdateToggle = useCallback(async (enabled: boolean) => {
    setAutoUpdateEnabled(enabled)
    try {
      await window.electronAPI?.updater?.setAutoCheck?.(enabled)
    } catch (err) {
      console.error('[auto-update toggle]', err)
    }
  }, [])

  const handleMcpServerToggle = useCallback(async () => {
    setMcpServerLoading(true)
    setMcpServerError(null)
    try {
      const action = mcpServerRunning ? 'stop' : 'start'
      const res = await fetch('/api/mcp/server', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action, port: mcpHttpPort }),
      })
      const data = await res.json()
      if (data.error) {
        setMcpServerError(data.error)
      } else {
        setMcpServerStatus(data.running ?? false, data.localIp)
      }
    } catch {
      setMcpServerError(t('agents.failedToMcp', { action: mcpServerRunning ? 'stop' : 'start' }))
    } finally {
      setMcpServerLoading(false)
    }
  }, [mcpServerRunning, mcpHttpPort, setMcpServerStatus, t])

  const handleCopyConfig = useCallback(() => {
    if (!mcpServerLocalIp) return
    const config = JSON.stringify(
      { type: 'http', url: `http://${mcpServerLocalIp}:${mcpHttpPort}/mcp` },
      null,
      2,
    )
    navigator.clipboard.writeText(config)
    setConfigCopied(true)
    setTimeout(() => setConfigCopied(false), 2000)
  }, [mcpServerLocalIp, mcpHttpPort])

  const handleToggleMCP = useCallback(
    async (tool: string) => {
      const current = mcpIntegrations.find((m) => m.tool === tool)
      if (!current) return
      const action = current.enabled ? 'uninstall' : 'install'

      setMcpInstalling(tool)
      setMcpError(null)
      try {
        const result = await callMcpInstall(tool, action)
        if (result.success) {
          toggleMCP(tool)
          persist()
          if (result.fallbackHttp) {
            setMcpServerStatus(true, null)
            setTimeout(() => {
              fetch('/api/mcp/server')
                .then((r) => r.json())
                .then((data: { running: boolean; localIp: string | null }) => {
                  setMcpServerStatus(data.running, data.localIp)
                })
                .catch(() => {})
            }, 500)
          }
        } else {
          setMcpError(result.error ?? t('agents.failedTo', { action }))
        }
      } catch {
        setMcpError(t('agents.failedToMcp', { action }))
      } finally {
        setMcpInstalling(null)
      }
    },
    [mcpIntegrations, toggleMCP, persist, setMcpServerStatus],
  )

  const handlePortBlur = useCallback(
    async (value: string) => {
      const port = parseInt(value, 10)
      if (isNaN(port) || port < 1 || port > 65535 || port === mcpHttpPort) return
      setMCPTransport('stdio', port)
      persist()
    },
    [mcpHttpPort, setMCPTransport, persist],
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
        className="relative bg-card rounded-xl border border-border w-[720px] h-[520px] overflow-hidden shadow-xl flex"
      >
        {/* Sidebar */}
        <div className="w-[200px] shrink-0 border-r border-border flex flex-col bg-card">
          <div className="px-4 pt-4 pb-3">
            <h2 className="text-[15px] font-semibold text-foreground">{t('settings.title')}</h2>
          </div>
          <nav className="flex-1 px-2 space-y-0.5">
            <NavItem
              icon={Pen}
              label={t('settings.agents')}
              active={activeTab === 'agents'}
              onClick={() => setActiveTab('agents')}
            />
            <NavItem
              icon={Terminal}
              label={t('settings.mcp')}
              active={activeTab === 'mcp'}
              onClick={() => setActiveTab('mcp')}
            />
            <NavItem
              icon={Settings}
              label={t('settings.system')}
              active={activeTab === 'system'}
              onClick={() => setActiveTab('system')}
            />
          </nav>
        </div>

        {/* Content */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Close button */}
          <div className="flex justify-end px-4 pt-3">
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => setDialogOpen(false)}
            >
              <X size={14} />
            </Button>
          </div>

          {/* Page content */}
          <div className="flex-1 overflow-y-auto px-5 pb-5">
            {activeTab === 'agents' && <AgentsPage />}

            {activeTab === 'mcp' && (
              <McpPage
                mcpServerRunning={mcpServerRunning}
                mcpServerLocalIp={mcpServerLocalIp}
                mcpServerLoading={mcpServerLoading}
                mcpServerError={mcpServerError}
                mcpHttpPort={mcpHttpPort}
                mcpIntegrations={mcpIntegrations}
                mcpInstalling={mcpInstalling}
                mcpError={mcpError}
                isBusy={isBusy}
                configCopied={configCopied}
                onServerToggle={handleMcpServerToggle}
                onCopyConfig={handleCopyConfig}
                onPortBlur={handlePortBlur}
                onToggleMCP={handleToggleMCP}
              />
            )}
            {activeTab === 'system' && (
              <div>
                <h3 className="text-[15px] font-semibold text-foreground mb-4">{t('settings.system')}</h3>
                {isElectron && (
                  <div className="flex items-center justify-between px-3.5 py-2.5 rounded-lg border border-border bg-secondary/20">
                    <div>
                      <span className="text-[13px] text-foreground block leading-tight">{t('agents.autoUpdate')}</span>
                      <span className="text-[11px] text-muted-foreground mt-0.5 block">{t('settings.autoUpdateDesc')}</span>
                    </div>
                    <Switch checked={autoUpdateEnabled} onCheckedChange={handleAutoUpdateToggle} />
                  </div>
                )}
                {!isElectron && (
                  <div className="rounded-lg border border-border bg-secondary/20 px-4 py-6 text-center">
                    <p className="text-[13px] text-muted-foreground">{t('settings.systemDesktopOnly')}</p>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
