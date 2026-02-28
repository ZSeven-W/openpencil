import { useState, useEffect, useCallback, useRef } from 'react'
import type { ComponentType, SVGProps } from 'react'
import { X, Check, Loader2, Unplug, AlertCircle, Zap, Terminal } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import type { AIProviderType, GroupedModel } from '@/types/agent-settings'
import ClaudeLogo from '@/components/icons/claude-logo'
import OpenAILogo from '@/components/icons/openai-logo'
import OpenCodeLogo from '@/components/icons/opencode-logo'

/** Provider display metadata */
const PROVIDER_META: Record<
  AIProviderType,
  { label: string; description: string; agent: 'claude-code' | 'codex-cli' | 'opencode'; Icon: ComponentType<SVGProps<SVGSVGElement>> }
> = {
  anthropic: {
    label: 'Claude Code',
    description: 'Claude models',
    agent: 'claude-code',
    Icon: ClaudeLogo,
  },
  openai: {
    label: 'Codex CLI',
    description: 'OpenAI models',
    agent: 'codex-cli',
    Icon: OpenAILogo,
  },
  opencode: {
    label: 'OpenCode',
    description: '75+ LLM providers',
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
    if (!res.ok) return { connected: false, models: [], error: `Server error ${res.status}` }
    return await res.json()
  } catch {
    return { connected: false, models: [], error: 'Connection failed' }
  }
}

function ProviderRow({ type }: { type: AIProviderType }) {
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
      setError(result.error ?? 'Connection failed')
    }
    setIsConnecting(false)
  }, [type, meta.agent, connect, persist])

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
            <span className="text-[13px] font-medium text-foreground leading-tight">{meta.label}</span>
            <span className="text-[10px] text-muted-foreground leading-tight hidden sm:inline">{meta.description}</span>
          </div>
          {provider.isConnected && (
            <span className="text-[11px] text-green-500 leading-tight flex items-center gap-1 mt-0.5">
              <Check size={10} strokeWidth={2.5} />
              {provider.models.length} model{provider.models.length !== 1 ? 's' : ''}
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
            Disconnect
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
              'Connect'
            )}
          </Button>
        )}
      </div>
    </div>
  )
}

export default function AgentSettingsDialog() {
  const open = useAgentSettingsStore((s) => s.dialogOpen)
  const setDialogOpen = useAgentSettingsStore((s) => s.setDialogOpen)
  const mcpIntegrations = useAgentSettingsStore((s) => s.mcpIntegrations)
  const toggleMCP = useAgentSettingsStore((s) => s.toggleMCPIntegration)
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

  const handleToggleMCP = useCallback(
    async (tool: string) => {
      const current = mcpIntegrations.find((m) => m.tool === tool)
      if (!current) return
      const action = current.enabled ? 'uninstall' : 'install'

      setMcpInstalling(tool)
      setMcpError(null)
      try {
        const res = await fetch('/api/ai/mcp-install', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ tool, action }),
        })
        const result = await res.json()
        if (result.success) {
          toggleMCP(tool)
          persist()
        } else {
          setMcpError(result.error ?? `Failed to ${action}`)
        }
      } catch {
        setMcpError(`Failed to ${action} MCP server`)
      } finally {
        setMcpInstalling(null)
      }
    },
    [mcpIntegrations, toggleMCP, persist],
  )

  if (!open) return null

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
            Setup Agents & MCP
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
                Agents on Canvas
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
                MCP Integrations in Terminal
              </h4>
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
                    disabled={mcpInstalling !== null}
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
              MCP integrations will take effect after restarting the terminal.
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}
