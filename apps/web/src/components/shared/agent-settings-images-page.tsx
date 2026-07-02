import { useState } from 'react';
import {
  Check,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  Loader2,
  Plus,
  Trash2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import type { ImageGenProvider, ImageGenProfile } from '@/types/image-service';
import { MODEL_PLACEHOLDERS } from '@/types/image-service';

type TestStatus = 'idle' | 'testing' | 'valid' | 'invalid';

/* ---------- Shared UI ---------- */

function FieldRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3 mb-2">
      <span className="text-[12px] text-muted-foreground w-[110px] shrink-0">{label}</span>
      <div className="flex-1">{children}</div>
    </div>
  );
}

function TextInput({
  value,
  onChange,
  placeholder,
  type = 'text',
  className,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  className?: string;
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
  );
}

function Collapsible({
  label,
  children,
  defaultOpen = false,
}: {
  label: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);
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
  );
}

function TestStatusBadge({ status }: { status: TestStatus }) {
  if (status === 'idle') return null;
  if (status === 'testing') {
    return <Loader2 size={11} className="animate-spin text-muted-foreground shrink-0" />;
  }
  if (status === 'valid') {
    return <Check size={11} className="text-green-500 shrink-0" />;
  }
  return <span className="text-[10px] text-destructive shrink-0">Invalid</span>;
}

/* ---------- Image Search section ---------- */

function ImageSearchSection() {
  const openverseOAuth = useAgentSettingsStore((s) => s.openverseOAuth);
  const setOpenverseOAuth = useAgentSettingsStore((s) => s.setOpenverseOAuth);
  const persist = useAgentSettingsStore((s) => s.persist);

  const [clientId, setClientId] = useState(openverseOAuth?.clientId ?? '');
  const [clientSecret, setClientSecret] = useState(openverseOAuth?.clientSecret ?? '');
  const [testStatus, setTestStatus] = useState<TestStatus>('idle');

  const handleChange = (field: 'clientId' | 'clientSecret', value: string) => {
    const updated = {
      clientId: field === 'clientId' ? value : clientId,
      clientSecret: field === 'clientSecret' ? value : clientSecret,
    };
    if (field === 'clientId') setClientId(value);
    else setClientSecret(value);

    const hasContent = updated.clientId || updated.clientSecret;
    setOpenverseOAuth(hasContent ? updated : null);
    persist();
  };

  const handleTest = async () => {
    setTestStatus('testing');
    try {
      const res = await fetch('/api/ai/image-service-test', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ service: 'openverse', clientId, clientSecret }),
      });
      setTestStatus(res.ok ? 'valid' : 'invalid');
    } catch {
      setTestStatus('invalid');
    }
  };

  return (
    <div className="mb-6">
      <div className="flex items-center gap-2 mb-3">
        <h3 className="text-[15px] font-semibold text-foreground">Image Search</h3>
        <div className="w-2 h-2 rounded-full bg-green-500 shrink-0" />
        <span className="text-[11px] text-muted-foreground">Ready</span>
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
  );
}

/* ---------- Provider labels ---------- */

const PROVIDER_LABELS: Record<ImageGenProvider, string> = {
  openai: 'OpenAI',
  gemini: 'Google Gemini',
  replicate: 'Replicate',
  custom: 'Custom',
};

/* ---------- Single profile editor ---------- */

function ProfileEditor({
  profile,
  onUpdate,
}: {
  profile: ImageGenProfile;
  onUpdate: (updates: Partial<Omit<ImageGenProfile, 'id'>>) => void;
}) {
  const [testStatus, setTestStatus] = useState<TestStatus>('idle');

  const handleTest = async () => {
    setTestStatus('testing');
    try {
      const res = await fetch('/api/ai/image-service-test', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          service: profile.provider,
          apiKey: profile.apiKey,
          model: profile.model,
          baseUrl: profile.baseUrl,
        }),
      });
      setTestStatus(res.ok ? 'valid' : 'invalid');
    } catch {
      setTestStatus('invalid');
    }
  };

  return (
    <div className="space-y-2">
      <FieldRow label="Name">
        <TextInput
          value={profile.name}
          onChange={(v) => onUpdate({ name: v })}
          placeholder="My Config"
        />
      </FieldRow>

      <FieldRow label="Provider">
        <Select
          value={profile.provider}
          onValueChange={(v) => onUpdate({ provider: v as ImageGenProvider, model: '' })}
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
            value={profile.apiKey}
            onChange={(v) => onUpdate({ apiKey: v })}
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
              disabled={testStatus === 'testing' || !profile.apiKey}
              className="h-6 px-2.5 text-[11px]"
            >
              Test
            </Button>
          </div>
        </div>
      </FieldRow>

      <FieldRow label="Model">
        <TextInput
          value={profile.model}
          onChange={(v) => onUpdate({ model: v })}
          placeholder={MODEL_PLACEHOLDERS[profile.provider]}
        />
      </FieldRow>

      <Collapsible label="Advanced">
        <FieldRow label="Base URL">
          <TextInput
            value={profile.baseUrl ?? ''}
            onChange={(v) => onUpdate({ baseUrl: v || undefined })}
            placeholder="https://api.example.com/v1"
          />
        </FieldRow>
      </Collapsible>
    </div>
  );
}

/* ---------- Image Generation section ---------- */

function ImageGenerationSection() {
  const profiles = useAgentSettingsStore((s) => s.imageGenProfiles);
  const activeId = useAgentSettingsStore((s) => s.activeImageGenProfileId);
  const addProfile = useAgentSettingsStore((s) => s.addImageGenProfile);
  const updateProfile = useAgentSettingsStore((s) => s.updateImageGenProfile);
  const removeProfile = useAgentSettingsStore((s) => s.removeImageGenProfile);
  const setActive = useAgentSettingsStore((s) => s.setActiveImageGenProfile);
  const persist = useAgentSettingsStore((s) => s.persist);

  const [editingId, setEditingId] = useState<string | null>(null);

  const handleAdd = () => {
    const id = addProfile({
      name: `Config ${profiles.length + 1}`,
      provider: 'openai',
      apiKey: '',
      model: '',
    });
    setEditingId(id);
    persist();
  };

  const handleUpdate = (id: string, updates: Partial<Omit<ImageGenProfile, 'id'>>) => {
    updateProfile(id, updates);
    persist();
  };

  const handleRemove = (id: string) => {
    removeProfile(id);
    if (editingId === id) setEditingId(null);
    persist();
  };

  const handleActivate = (id: string) => {
    setActive(id);
    persist();
  };

  const effectiveActiveId = activeId ?? profiles[0]?.id;

  return (
    <div>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-[15px] font-semibold text-foreground">Image Generation</h3>
        <Button size="sm" variant="outline" onClick={handleAdd} className="h-6 px-2 text-[11px]">
          <Plus size={12} className="mr-1" />
          Add
        </Button>
      </div>

      {profiles.length === 0 ? (
        <p className="text-[11px] text-muted-foreground py-4 text-center">
          No configurations yet. Click "Add" to create one.
        </p>
      ) : (
        <div className="space-y-1.5">
          {profiles.map((profile) => {
            const isActive = profile.id === effectiveActiveId;
            const isEditing = profile.id === editingId;

            return (
              <div key={profile.id}>
                {/* Profile row */}
                <div
                  className={cn(
                    'flex items-center gap-2 h-8 px-2 rounded border transition-colors cursor-pointer',
                    isActive
                      ? 'border-primary/50 bg-primary/5'
                      : 'border-border hover:bg-accent/50',
                  )}
                  onClick={() => setEditingId(isEditing ? null : profile.id)}
                >
                  {/* Active indicator */}
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleActivate(profile.id);
                    }}
                    className={cn(
                      'w-3.5 h-3.5 rounded-full border-2 shrink-0 transition-colors',
                      isActive
                        ? 'border-primary bg-primary'
                        : 'border-muted-foreground/40 hover:border-primary/60',
                    )}
                    title={isActive ? 'Active' : 'Set as active'}
                  >
                    {isActive && (
                      <Check size={8} className="text-primary-foreground m-auto block" />
                    )}
                  </button>

                  {/* Name + provider */}
                  <span className="text-xs text-foreground flex-1 truncate">
                    {profile.name || PROVIDER_LABELS[profile.provider]}
                  </span>
                  <span className="text-[10px] text-muted-foreground shrink-0">
                    {PROVIDER_LABELS[profile.provider]}
                  </span>

                  {/* Expand/Collapse */}
                  {isEditing ? (
                    <ChevronDown size={12} className="text-muted-foreground shrink-0" />
                  ) : (
                    <ChevronRight size={12} className="text-muted-foreground shrink-0" />
                  )}

                  {/* Delete */}
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemove(profile.id);
                    }}
                    className="text-muted-foreground hover:text-destructive transition-colors shrink-0"
                    title="Remove"
                  >
                    <Trash2 size={12} />
                  </button>
                </div>

                {/* Expanded editor */}
                {isEditing && (
                  <div className="mt-2 mb-3 pl-3 border-l-2 border-border">
                    <ProfileEditor
                      profile={profile}
                      onUpdate={(updates) => handleUpdate(profile.id, updates)}
                    />
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/* ---------- Main export ---------- */

export function ImagesPage() {
  return (
    <div>
      <ImageSearchSection />
      <ImageGenerationSection />
    </div>
  );
}
