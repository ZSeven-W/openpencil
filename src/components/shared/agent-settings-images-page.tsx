import { useState } from 'react'
import { Check, ChevronDown, ChevronRight, ExternalLink, Loader2 } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import type { ImageGenProvider } from '@/types/image-service'
import { MODEL_PLACEHOLDERS } from '@/types/image-service'

type TestStatus = 'idle' | 'testing' | 'valid' | 'invalid'

/* ---------- Section header ---------- */
function SectionHeader({ title }: { title: string }) {
  return (
    <h3 className="text-[15px] font-semibold text-foreground mb-3">{title}</h3>
  )
}

/* ---------- Field row ---------- */
function FieldRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3 mb-2">
      <span className="text-[12px] text-muted-foreground w-[110px] shrink-0">{label}</span>
      <div className="flex-1">{children}</div>
    </div>
  )
}

/* ---------- Text input ---------- */
function TextInput({
  value,
  onChange,
  placeholder,
  type = 'text',
  className,
}: {
  value: string
  onChange: (v: string) => void
  placeholder?: string
  type?: string
  className?: string
}) {
  return (
    <input
      type={type}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className={cn(
        'h-7 w-full rounded border border-input bg-secondary px-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-ring transition-colors',
        className,
      )}
    />
  )
}

/* ---------- Collapsible section ---------- */
function Collapsible({
  label,
  children,
  defaultOpen = false,
}: {
  label: string
  children: React.ReactNode
  defaultOpen?: boolean
}) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <div>
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors mb-2"
      >
        {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        {label}
      </button>
      {open && <div className="pl-3 border-l border-border/50 space-y-2">{children}</div>}
    </div>
  )
}

/* ---------- Test status indicator ---------- */
function TestStatusBadge({ status }: { status: TestStatus }) {
  if (status === 'idle') return null
  if (status === 'testing') {
    return <Loader2 size={11} className="animate-spin text-muted-foreground shrink-0" />
  }
  if (status === 'valid') {
    return <Check size={11} className="text-green-500 shrink-0" />
  }
  return <span className="text-[10px] text-destructive shrink-0">Invalid</span>
}

/* ---------- Image Search section ---------- */
function ImageSearchSection() {
  const openverseOAuth = useAgentSettingsStore((s) => s.openverseOAuth)
  const setOpenverseOAuth = useAgentSettingsStore((s) => s.setOpenverseOAuth)
  const persist = useAgentSettingsStore((s) => s.persist)

  const [clientId, setClientId] = useState(openverseOAuth?.clientId ?? '')
  const [clientSecret, setClientSecret] = useState(openverseOAuth?.clientSecret ?? '')
  const [testStatus, setTestStatus] = useState<TestStatus>('idle')

  const handleChange = (field: 'clientId' | 'clientSecret', value: string) => {
    const updated = {
      clientId: field === 'clientId' ? value : clientId,
      clientSecret: field === 'clientSecret' ? value : clientSecret,
    }
    if (field === 'clientId') setClientId(value)
    else setClientSecret(value)

    const hasContent = updated.clientId || updated.clientSecret
    setOpenverseOAuth(hasContent ? updated : null)
    persist()
  }

  const handleTest = async () => {
    setTestStatus('testing')
    try {
      const res = await fetch('/api/ai/image-service-test', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ service: 'openverse', clientId, clientSecret }),
      })
      setTestStatus(res.ok ? 'valid' : 'invalid')
    } catch {
      setTestStatus('invalid')
    }
  }

  return (
    <div className="mb-6">
      <SectionHeader title="Image Search" />
      <div className="flex items-center gap-2 px-3.5 py-2.5 rounded-lg border border-border bg-secondary/20 mb-3">
        <div className="w-2 h-2 rounded-full bg-green-500 shrink-0" />
        <span className="text-[13px] text-foreground">Ready</span>
      </div>

      <Collapsible label="Advanced">
        <p className="text-[11px] text-muted-foreground mb-2">
          Openverse OAuth (optional, for higher rate limits)
        </p>

        <FieldRow label="Client ID">
          <TextInput
            value={clientId}
            onChange={(v) => handleChange('clientId', v)}
            placeholder="your-client-id"
          />
        </FieldRow>

        <FieldRow label="Client Secret">
          <TextInput
            value={clientSecret}
            onChange={(v) => handleChange('clientSecret', v)}
            placeholder="your-client-secret"
            type="password"
          />
        </FieldRow>

        <div className="flex items-center justify-between mt-1">
          <a
            href="https://api.openverse.org/v1/auth_tokens/register/"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-[11px] text-blue-500 hover:underline"
          >
            Register at Openverse
            <ExternalLink size={10} />
          </a>
          <div className="flex items-center gap-2">
            <TestStatusBadge status={testStatus} />
            <Button
              size="sm"
              variant="outline"
              onClick={handleTest}
              disabled={testStatus === 'testing' || (!clientId && !clientSecret)}
              className="h-6 px-2.5 text-[11px]"
            >
              Test
            </Button>
          </div>
        </div>
      </Collapsible>
    </div>
  )
}

/* ---------- Image Generation section ---------- */
const PROVIDER_LABELS: Record<ImageGenProvider, string> = {
  openai: 'OpenAI',
  gemini: 'Google Gemini',
  replicate: 'Replicate',
  custom: 'Custom',
}

function ImageGenerationSection() {
  const imageGenConfig = useAgentSettingsStore((s) => s.imageGenConfig)
  const setImageGenConfig = useAgentSettingsStore((s) => s.setImageGenConfig)
  const persist = useAgentSettingsStore((s) => s.persist)

  const [testStatus, setTestStatus] = useState<TestStatus>('idle')

  const update = (updates: Parameters<typeof setImageGenConfig>[0]) => {
    setImageGenConfig(updates)
    persist()
  }

  const handleTest = async () => {
    setTestStatus('testing')
    try {
      const res = await fetch('/api/ai/image-service-test', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          service: 'image-gen',
          provider: imageGenConfig.provider,
          apiKey: imageGenConfig.apiKey,
          model: imageGenConfig.model,
          baseUrl: imageGenConfig.baseUrl,
        }),
      })
      setTestStatus(res.ok ? 'valid' : 'invalid')
    } catch {
      setTestStatus('invalid')
    }
  }

  return (
    <div>
      <SectionHeader title="Image Generation" />

      <FieldRow label="Provider">
        <Select
          value={imageGenConfig.provider}
          onValueChange={(v) => update({ provider: v as ImageGenProvider, model: '' })}
        >
          <SelectTrigger className="h-7 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {(Object.keys(PROVIDER_LABELS) as ImageGenProvider[]).map((p) => (
              <SelectItem key={p} value={p}>
                {PROVIDER_LABELS[p]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </FieldRow>

      <FieldRow label="API Key">
        <div className="flex items-center gap-2">
          <TextInput
            value={imageGenConfig.apiKey}
            onChange={(v) => update({ apiKey: v })}
            placeholder="sk-..."
            type="password"
            className="flex-1"
          />
          <div className="flex items-center gap-1.5 shrink-0">
            <TestStatusBadge status={testStatus} />
            <Button
              size="sm"
              variant="outline"
              onClick={handleTest}
              disabled={testStatus === 'testing' || !imageGenConfig.apiKey}
              className="h-6 px-2.5 text-[11px]"
            >
              Test
            </Button>
          </div>
        </div>
      </FieldRow>

      <FieldRow label="Model">
        <TextInput
          value={imageGenConfig.model}
          onChange={(v) => update({ model: v })}
          placeholder={MODEL_PLACEHOLDERS[imageGenConfig.provider]}
        />
      </FieldRow>

      <Collapsible label="Advanced">
        <FieldRow label="Base URL">
          <TextInput
            value={imageGenConfig.baseUrl ?? ''}
            onChange={(v) => update({ baseUrl: v || undefined })}
            placeholder="https://api.example.com/v1"
          />
        </FieldRow>
      </Collapsible>
    </div>
  )
}

/* ---------- Main export ---------- */
export function ImagesPage() {
  return (
    <div>
      <ImageSearchSection />
      <ImageGenerationSection />
    </div>
  )
}
