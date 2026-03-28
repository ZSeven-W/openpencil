import { useState, useEffect, useCallback, useRef } from 'react'
import {
  Loader2, Check, Key, Plus, Trash2, Pencil, Eye, EyeOff, Search, ChevronDown,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import type { BuiltinProviderConfig, BuiltinProviderPreset } from '@/stores/agent-settings-store'

/* ---------- Provider Preset Config ---------- */
const PROVIDER_PRESETS: Record<BuiltinProviderPreset, {
  label: string
  type: 'anthropic' | 'openai-compat'
  baseURL?: string
  placeholder: string
  modelPlaceholder: string
}> = {
  anthropic: {
    label: 'Anthropic',
    type: 'anthropic',
    placeholder: 'sk-ant-...',
    modelPlaceholder: 'claude-sonnet-4-20250514',
  },
  openai: {
    label: 'OpenAI',
    type: 'openai-compat',
    baseURL: 'https://api.openai.com/v1',
    placeholder: 'sk-...',
    modelPlaceholder: 'gpt-4o',
  },
  openrouter: {
    label: 'OpenRouter',
    type: 'openai-compat',
    baseURL: 'https://openrouter.ai/api/v1',
    placeholder: 'sk-or-...',
    modelPlaceholder: 'anthropic/claude-sonnet-4',
  },
  deepseek: {
    label: 'DeepSeek',
    type: 'openai-compat',
    baseURL: 'https://api.deepseek.com/v1',
    placeholder: 'sk-...',
    modelPlaceholder: 'deepseek-chat',
  },
  custom: {
    label: 'Custom',
    type: 'openai-compat',
    placeholder: 'sk-...',
    modelPlaceholder: 'model-name',
  },
}

const ANTHROPIC_MODELS = [
  { id: 'claude-opus-4-20250514', name: 'Claude Opus 4' },
  { id: 'claude-sonnet-4-20250514', name: 'Claude Sonnet 4' },
  { id: 'claude-sonnet-4-5-20250929', name: 'Claude Sonnet 4.5' },
  { id: 'claude-haiku-4-5-20251001', name: 'Claude Haiku 4.5' },
]

/** Infer preset from an existing provider config (for editing) */
function inferPreset(config: BuiltinProviderConfig): BuiltinProviderPreset {
  if (config.preset) return config.preset
  if (config.type === 'anthropic') return 'anthropic'
  const url = config.baseURL?.replace(/\/+$/, '') ?? ''
  if (url === 'https://api.openai.com/v1') return 'openai'
  if (url === 'https://openrouter.ai/api/v1') return 'openrouter'
  if (url === 'https://api.deepseek.com/v1') return 'deepseek'
  return 'custom'
}

/** Fetch model list from a provider via our server-side proxy */
async function fetchProviderModels(
  baseURL: string,
  apiKey?: string,
): Promise<{ models: Array<{ id: string; name: string }>; error?: string }> {
  try {
    const res = await fetch('/api/ai/provider-models', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ baseURL, apiKey }),
    })
    if (!res.ok) return { models: [], error: `Server error ${res.status}` }
    return await res.json()
  } catch {
    return { models: [], error: 'Request failed' }
  }
}

/* ---------- Model Search Dropdown ---------- */
function ModelSearchDropdown({
  models,
  onSelect,
  onClose,
}: {
  models: Array<{ id: string; name: string }>
  onSelect: (model: { id: string; name: string }) => void
  onClose: () => void
}) {
  const [filter, setFilter] = useState('')
  const listRef = useRef<HTMLDivElement>(null)

  const filtered = models.filter((m) => {
    const q = filter.toLowerCase()
    return m.id.toLowerCase().includes(q) || m.name.toLowerCase().includes(q)
  })

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (listRef.current && !listRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [onClose])

  return (
    <div
      ref={listRef}
      className="absolute bottom-full left-0 right-0 mb-1 rounded-md border border-border bg-popover shadow-md z-10 overflow-hidden"
    >
      <div className="p-1.5 border-b border-border">
        <input
          autoFocus
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder="Filter models..."
          className="w-full h-7 px-2 text-[12px] bg-card text-foreground rounded border border-input focus:border-ring outline-none transition-colors"
        />
      </div>
      <div className="max-h-48 overflow-y-auto">
        {filtered.length === 0 && (
          <div className="px-3 py-4 text-center text-[11px] text-muted-foreground">
            No models found
          </div>
        )}
        {filtered.map((m) => (
          <button
            key={m.id}
            onClick={() => {
              onSelect(m)
              onClose()
            }}
            className="w-full text-left px-3 py-1.5 text-[12px] text-foreground hover:bg-secondary/50 transition-colors flex flex-col"
          >
            <span className="font-medium truncate">{m.name !== m.id ? m.name : m.id}</span>
            {m.name !== m.id && (
              <span className="text-[10px] text-muted-foreground font-mono truncate">{m.id}</span>
            )}
          </button>
        ))}
      </div>
    </div>
  )
}

/* ---------- Builtin Provider Form ---------- */
export function BuiltinProviderForm({
  initial,
  onSave,
  onCancel,
}: {
  initial?: BuiltinProviderConfig
  onSave: (data: Omit<BuiltinProviderConfig, 'id'>) => void
  onCancel: () => void
}) {
  const [preset, setPreset] = useState<BuiltinProviderPreset>(
    initial ? inferPreset(initial) : 'anthropic',
  )
  const presetConfig = PROVIDER_PRESETS[preset]
  const [displayName, setDisplayName] = useState(initial?.displayName ?? '')
  const [apiKey, setApiKey] = useState(initial?.apiKey ?? '')
  const [modelName, setModelName] = useState(initial?.model ?? '')
  const [baseURL, setBaseURL] = useState(
    initial?.baseURL ?? presetConfig.baseURL ?? '',
  )
  const [showApiKey, setShowApiKey] = useState(false)
  const [customApiFormat, setCustomApiFormat] = useState<'openai-compat' | 'anthropic'>(
    initial?.type ?? 'openai-compat',
  )

  const [modelList, setModelList] = useState<Array<{ id: string; name: string }>>([])
  const [showModelDropdown, setShowModelDropdown] = useState(false)
  const [modelLoading, setModelLoading] = useState(false)
  const [modelError, setModelError] = useState<string | null>(null)

  const handlePresetChange = useCallback(
    (newPreset: BuiltinProviderPreset) => {
      setPreset(newPreset)
      const cfg = PROVIDER_PRESETS[newPreset]
      if (!displayName.trim() || displayName === PROVIDER_PRESETS[preset].label) {
        setDisplayName(cfg.label)
      }
      setBaseURL(cfg.baseURL ?? '')
      setModelList([])
      setShowModelDropdown(false)
      setModelError(null)
    },
    [displayName, preset],
  )

  const handleFetchModels = useCallback(async () => {
    if (preset === 'anthropic') {
      setModelList(ANTHROPIC_MODELS)
      setShowModelDropdown(true)
      return
    }
    const url = preset === 'custom' ? baseURL.trim() : PROVIDER_PRESETS[preset].baseURL
    if (!url) { setModelError('Base URL is required to search models'); return }
    setModelLoading(true)
    setModelError(null)
    const result = await fetchProviderModels(url, apiKey.trim() || undefined)
    setModelLoading(false)
    if (result.error) {
      setModelError(result.error)
      if (result.models.length > 0) { setModelList(result.models); setShowModelDropdown(true) }
    } else {
      setModelList(result.models)
      setShowModelDropdown(true)
    }
  }, [preset, baseURL, apiKey])

  const handleModelSelect = useCallback((model: { id: string; name: string }) => {
    setModelName(model.id)
    setShowModelDropdown(false)
  }, [])

  const isBaseURLLocked = preset !== 'custom'
  const effectiveType = preset === 'custom' ? customApiFormat : presetConfig.type
  const showBaseURL = effectiveType === 'openai-compat' || preset === 'custom'
  const canSave = displayName.trim().length > 0 && apiKey.trim().length > 0 && modelName.trim().length > 0

  return (
    <div className="space-y-3 rounded-lg border border-border bg-secondary/20 p-3.5">
      <div>
        <label className="text-[11px] text-muted-foreground mb-1 block">Display Name</label>
        <input value={displayName} onChange={(e) => setDisplayName(e.target.value)} placeholder="e.g. My Anthropic Key"
          className="w-full h-8 px-2.5 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors" />
      </div>
      <div>
        <label className="text-[11px] text-muted-foreground mb-1 block">Provider</label>
        <select value={preset} onChange={(e) => handlePresetChange(e.target.value as BuiltinProviderPreset)}
          className="w-full h-8 px-2 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors">
          <option value="anthropic">Anthropic</option>
          <option value="openai">OpenAI</option>
          <option value="openrouter">OpenRouter</option>
          <option value="deepseek">DeepSeek</option>
          <option value="custom">Custom</option>
        </select>
      </div>
      <div>
        <label className="text-[11px] text-muted-foreground mb-1 block">API Key</label>
        <div className="relative">
          <input type={showApiKey ? 'text' : 'password'} value={apiKey} onChange={(e) => setApiKey(e.target.value)} placeholder={presetConfig.placeholder}
            className="w-full h-8 px-2.5 pr-8 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors font-mono" />
          <button type="button" onClick={() => setShowApiKey((v) => !v)} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground">
            {showApiKey ? <EyeOff size={13} /> : <Eye size={13} />}
          </button>
        </div>
      </div>
      <div>
        <label className="text-[11px] text-muted-foreground mb-1 block">Model</label>
        <div className="relative">
          <input value={modelName} onChange={(e) => setModelName(e.target.value)} placeholder={presetConfig.modelPlaceholder}
            className="w-full h-8 px-2.5 pr-16 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors font-mono" />
          <button type="button" onClick={handleFetchModels} disabled={modelLoading} title="Search available models"
            className="absolute right-1.5 top-1/2 -translate-y-1/2 h-6 px-1.5 rounded flex items-center gap-1 text-muted-foreground hover:text-foreground hover:bg-secondary/50 transition-colors disabled:opacity-50">
            {modelLoading ? <Loader2 size={12} className="animate-spin" /> : <><Search size={12} /><ChevronDown size={10} /></>}
          </button>
          {showModelDropdown && modelList.length > 0 && (
            <ModelSearchDropdown models={modelList} onSelect={handleModelSelect} onClose={() => setShowModelDropdown(false)} />
          )}
        </div>
        {modelError && <p className="text-[10px] text-destructive mt-1">{modelError}</p>}
      </div>
      {preset === 'custom' && (
        <div>
          <label className="text-[11px] text-muted-foreground mb-1 block">API Format</label>
          <div className="flex gap-1">
            {(['openai-compat', 'anthropic'] as const).map((fmt) => (
              <button key={fmt} type="button" onClick={() => setCustomApiFormat(fmt)}
                className={cn('flex-1 h-7 text-[11px] rounded-md border transition-colors',
                  customApiFormat === fmt ? 'bg-primary text-primary-foreground border-primary' : 'bg-card text-muted-foreground border-input hover:bg-accent')}>
                {fmt === 'openai-compat' ? 'OpenAI Compatible' : 'Anthropic'}
              </button>
            ))}
          </div>
        </div>
      )}
      {showBaseURL && (
        <div>
          <label className="text-[11px] text-muted-foreground mb-1 block">Base URL{isBaseURLLocked ? '' : ' (required)'}</label>
          <input value={baseURL} onChange={(e) => setBaseURL(e.target.value)} placeholder="https://api.example.com/v1" readOnly={isBaseURLLocked}
            className={cn('w-full h-8 px-2.5 text-[13px] bg-card text-foreground rounded-md border border-input focus:border-ring outline-none transition-colors font-mono', isBaseURLLocked && 'opacity-60 cursor-default')} />
        </div>
      )}
      <div className="flex items-center justify-end gap-2 pt-1">
        <Button variant="ghost" size="sm" onClick={onCancel} className="h-7 px-3 text-[11px]">Cancel</Button>
        <Button size="sm" onClick={() => onSave({
          displayName: displayName.trim(), type: effectiveType, apiKey: apiKey.trim(), model: modelName.trim(), preset,
          ...(showBaseURL && baseURL.trim() ? { baseURL: baseURL.trim() } : {}), enabled: initial?.enabled ?? true,
        })} disabled={!canSave} className="h-7 px-3 text-[11px]">{initial ? 'Save' : 'Add'}</Button>
      </div>
    </div>
  )
}

/* ---------- Builtin Provider Card ---------- */
export function BuiltinProviderCard({ provider }: { provider: BuiltinProviderConfig }) {
  const update = useAgentSettingsStore((s) => s.updateBuiltinProvider)
  const remove = useAgentSettingsStore((s) => s.removeBuiltinProvider)
  const persist = useAgentSettingsStore((s) => s.persist)
  const [editing, setEditing] = useState(false)

  const handleToggle = useCallback((enabled: boolean) => { update(provider.id, { enabled }); persist() }, [provider.id, update, persist])
  const handleRemove = useCallback(() => { remove(provider.id); persist() }, [provider.id, remove, persist])
  const handleSave = useCallback((data: Omit<BuiltinProviderConfig, 'id'>) => { update(provider.id, data); persist(); setEditing(false) }, [provider.id, update, persist])

  if (editing) {
    return <BuiltinProviderForm initial={provider} onSave={handleSave} onCancel={() => setEditing(false)} />
  }

  const masked = provider.apiKey.length > 12 ? provider.apiKey.slice(0, 7) + '***' + provider.apiKey.slice(-3) : '***'

  return (
    <div className="group">
      <div className={cn('flex items-center gap-3 px-3.5 py-2.5 rounded-lg border transition-colors',
        provider.enabled ? 'bg-secondary/30 border-border' : 'border-transparent hover:bg-secondary/20')}>
        <div className={cn('w-9 h-9 rounded-lg flex items-center justify-center shrink-0 transition-colors',
          provider.enabled ? 'bg-foreground/8 text-foreground' : 'bg-secondary text-muted-foreground')}>
          <Key size={18} />
        </div>
        <div className="flex-1 min-w-0">
          <span className="text-[13px] font-medium text-foreground leading-tight block">{provider.displayName}</span>
          <span className="text-[11px] text-muted-foreground leading-tight mt-0.5 block">{provider.model} &middot; {masked}</span>
          {provider.enabled && (
            <span className="text-[11px] text-green-500 leading-tight flex items-center gap-1 mt-0.5"><Check size={10} strokeWidth={2.5} />Ready</span>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <Switch checked={provider.enabled} onCheckedChange={handleToggle} className="mr-1" />
          <Button variant="ghost" size="icon-sm" onClick={() => setEditing(true)} className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"><Pencil size={11} /></Button>
          <Button variant="ghost" size="icon-sm" onClick={handleRemove} className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-destructive"><Trash2 size={11} /></Button>
        </div>
      </div>
    </div>
  )
}

/* ---------- Builtin Providers Section (used in AgentsPage) ---------- */
export function BuiltinProvidersSection() {
  const builtinProviders = useAgentSettingsStore((s) => s.builtinProviders)
  const addBuiltinProvider = useAgentSettingsStore((s) => s.addBuiltinProvider)
  const persist = useAgentSettingsStore((s) => s.persist)
  const [showForm, setShowForm] = useState(false)

  const handleAdd = useCallback(
    (data: Omit<BuiltinProviderConfig, 'id'>) => {
      addBuiltinProvider(data)
      persist()
      setShowForm(false)
    },
    [addBuiltinProvider, persist],
  )

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">Built-in Providers</h3>
        {!showForm && (
          <button
            onClick={() => setShowForm(true)}
            className="text-[11px] text-muted-foreground hover:text-foreground flex items-center gap-1 transition-colors"
          >
            <Plus size={12} /> Add Provider
          </button>
        )}
      </div>
      <p className="text-[11px] text-muted-foreground leading-relaxed">
        Configure API keys directly — no CLI tools required.
      </p>
      {showForm && <BuiltinProviderForm onSave={handleAdd} onCancel={() => setShowForm(false)} />}
      {builtinProviders.map((bp) => (
        <BuiltinProviderCard key={bp.id} provider={bp} />
      ))}
      {!showForm && builtinProviders.length === 0 && (
        <div className="text-center py-6 text-[11px] text-muted-foreground">
          No built-in providers configured yet.
        </div>
      )}
    </div>
  )
}
