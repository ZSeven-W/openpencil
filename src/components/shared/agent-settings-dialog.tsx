import { useState, useEffect, useCallback, useRef } from 'react'
import type { ComponentType, SVGProps } from 'react'
import { X, Check, Loader2, Unplug, AlertCircle } from 'lucide-react'
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
    description: 'Connect to local Claude Code CLI to use Claude models',
    agent: 'claude-code',
    Icon: ClaudeLogo,
  },
  openai: {
    label: 'Codex CLI',
    description: 'Connect to local Codex CLI to use OpenAI models',
    agent: 'codex-cli',
    Icon: OpenAILogo,
  },
  opencode: {
    label: 'OpenCode',
    description: 'Connect to local OpenCode server to use 75+ LLM providers',
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

function ProviderCard({ type }: { type: AIProviderType }) {
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
    <div className="rounded-lg border border-border bg-background/50 p-3">
      <div className="flex items-center gap-2 mb-1">
        <div className="w-8 h-8 rounded-md bg-secondary flex items-center justify-center text-foreground">
          <Icon className="w-5 h-5" />
        </div>
        <span className="text-sm font-medium text-foreground">{meta.label}</span>
      </div>
      <p className="text-[11px] text-muted-foreground mb-2.5">{meta.description}</p>

      {provider.isConnected ? (
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5">
            <Check size={13} className="text-green-500" />
            <span className="text-xs text-green-500">
              Connected — {provider.models.length} model{provider.models.length !== 1 ? 's' : ''}
            </span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleDisconnect}
            className="h-7 text-xs text-muted-foreground hover:text-destructive"
          >
            <Unplug size={12} className="mr-1" />
            Disconnect
          </Button>
        </div>
      ) : (
        <>
          <Button
            size="sm"
            onClick={handleConnect}
            disabled={isConnecting}
            className="h-7 text-xs"
          >
            {isConnecting ? (
              <>
                <Loader2 size={12} className="animate-spin mr-1" />
                Connecting...
              </>
            ) : (
              'Connect'
            )}
          </Button>
          {error && (
            <p className="text-xs text-destructive mt-1.5">{error}</p>
          )}
        </>
      )}
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
        className="relative bg-card rounded-lg border border-border p-5 w-[420px] max-h-[80vh] overflow-y-auto shadow-xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
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

        {/* Agents section */}
        <div className="mb-5">
          <div className="flex items-center mb-2">
            <h4 className="text-xs font-medium text-foreground">
              Agents on Canvas
            </h4>
            <span className="text-[10px] text-muted-foreground bg-secondary px-1.5 py-0.5 ml-2 rounded">
              Recommended
            </span>
          </div>
          <div className="space-y-2">
            <ProviderCard type="anthropic" />
            <ProviderCard type="openai" />
            <ProviderCard type="opencode" />
          </div>
        </div>

        {/* MCP integrations section */}
        <div>
          <h4 className="text-xs font-medium text-foreground mb-2">
            MCP Integrations in Terminal
          </h4>
          <div className="space-y-1">
            {mcpIntegrations.map((m) => (
              <div
                key={m.tool}
                className="flex items-center justify-between py-2 px-1"
              >
                <div className="flex items-center gap-1.5">
                  <span
                    className={cn(
                      'text-xs',
                      m.enabled ? 'text-foreground' : 'text-muted-foreground',
                    )}
                  >
                    {m.displayName}
                  </span>
                  {mcpInstalling === m.tool && (
                    <Loader2 size={11} className="animate-spin text-muted-foreground" />
                  )}
                </div>
                <Switch
                  checked={m.enabled}
                  disabled={mcpInstalling !== null}
                  onCheckedChange={() => handleToggleMCP(m.tool)}
                />
              </div>
            ))}
          </div>
          {mcpError && (
            <div className="flex items-center gap-1.5 mt-1.5 px-1">
              <AlertCircle size={11} className="text-destructive shrink-0" />
              <p className="text-[10px] text-destructive">{mcpError}</p>
            </div>
          )}
          <p className="text-[10px] text-muted-foreground mt-2">
            MCP integrations will take effect after restarting the terminal.
          </p>
        </div>
      </div>
    </div>
  )
}
