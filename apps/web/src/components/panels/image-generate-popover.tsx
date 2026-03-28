import { useState } from 'react';
import { Settings, Sparkles, Loader2 } from 'lucide-react';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';

interface ImageGeneratePopoverProps {
  initialPrompt: string;
  onGenerated: (url: string) => void;
  children: React.ReactNode;
  /** Node dimensions — passed to the API for aspect-ratio-aware generation */
  width?: number;
  height?: number;
}

type State = 'idle' | 'loading' | 'preview' | 'error';

export default function ImageGeneratePopover({
  initialPrompt,
  onGenerated,
  children,
  width,
  height,
}: ImageGeneratePopoverProps) {
  const [open, setOpen] = useState(false);
  const [prompt, setPrompt] = useState(initialPrompt);
  const [state, setState] = useState<State>('idle');
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string>('');

  const activeProfile = useAgentSettingsStore((s) => s.getActiveImageGenProfile());
  const setDialogOpen = useAgentSettingsStore((s) => s.setDialogOpen);

  const isConfigured = !!activeProfile?.apiKey;

  const handleOpenChange = (next: boolean) => {
    setOpen(next);
    if (next) {
      // Reset to idle when reopening
      setPrompt(initialPrompt);
      setState('idle');
      setPreviewUrl(null);
      setErrorMsg('');
    }
  };

  const handleGenerate = async () => {
    if (!activeProfile) return;
    setState('loading');
    setErrorMsg('');
    try {
      const res = await fetch('/api/ai/image-generate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          prompt,
          provider: activeProfile.provider,
          model: activeProfile.model,
          apiKey: activeProfile.apiKey,
          baseUrl: activeProfile.baseUrl,
          ...(width && height ? { width, height } : {}),
        }),
      });
      if (!res.ok) {
        let msg = `HTTP ${res.status}`;
        try {
          const errData = await res.json();
          const raw = errData.message || errData.statusMessage || errData.error || '';
          if (raw) msg = String(raw).slice(0, 200);
        } catch {
          /* use default msg */
        }
        throw new Error(msg);
      }
      const data = (await res.json()) as { url?: string; error?: string };
      if (data.error) throw new Error(data.error);
      if (!data.url) throw new Error('No image URL returned');
      setPreviewUrl(data.url);
      setState('preview');
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setState('error');
    }
  };

  const handleApply = () => {
    if (previewUrl) {
      onGenerated(previewUrl);
      setOpen(false);
    }
  };

  return (
    <Popover open={open} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent className="w-72" side="left" align="start">
        {!isConfigured ? (
          <NotConfiguredView onOpenSettings={() => setDialogOpen(true)} />
        ) : state === 'loading' ? (
          <LoadingView />
        ) : state === 'preview' && previewUrl ? (
          <PreviewView url={previewUrl} onApply={handleApply} onRetry={() => setState('idle')} />
        ) : (
          <IdleView
            prompt={prompt}
            onPromptChange={setPrompt}
            onGenerate={handleGenerate}
            provider={activeProfile!.provider}
            model={activeProfile!.model}
            profileName={activeProfile!.name}
            error={state === 'error' ? errorMsg : undefined}
          />
        )}
      </PopoverContent>
    </Popover>
  );
}

function NotConfiguredView({ onOpenSettings }: { onOpenSettings: () => void }) {
  return (
    <div className="flex flex-col items-center gap-3 py-2 text-center">
      <Settings className="w-8 h-8 text-muted-foreground" />
      <p className="text-xs text-muted-foreground">Image generation not configured</p>
      <Button size="sm" variant="outline" onClick={onOpenSettings}>
        Open Settings
      </Button>
    </div>
  );
}

function LoadingView() {
  return (
    <div className="flex flex-col items-center gap-2 py-4">
      <Loader2 className="w-6 h-6 text-muted-foreground animate-spin" />
      <span className="text-xs text-muted-foreground">Generating...</span>
    </div>
  );
}

function PreviewView({
  url,
  onApply,
  onRetry,
}: {
  url: string;
  onApply: () => void;
  onRetry: () => void;
}) {
  return (
    <div className="flex flex-col gap-3">
      <img
        src={url}
        alt="Generated"
        className="w-full rounded-md border border-border object-cover"
        style={{ maxHeight: 200 }}
      />
      <div className="flex gap-2">
        <Button size="sm" className="flex-1" onClick={onApply}>
          Apply
        </Button>
        <Button size="sm" variant="outline" onClick={onRetry}>
          Retry
        </Button>
      </div>
    </div>
  );
}

function IdleView({
  prompt,
  onPromptChange,
  onGenerate,
  provider,
  model,
  profileName,
  error,
}: {
  prompt: string;
  onPromptChange: (v: string) => void;
  onGenerate: () => void;
  provider: string;
  model: string;
  profileName: string;
  error?: string;
}) {
  return (
    <div className="flex flex-col gap-3">
      <textarea
        rows={2}
        value={prompt}
        onChange={(e) => onPromptChange(e.target.value)}
        placeholder="Describe the image..."
        className="w-full resize-none rounded-md border border-border bg-background px-3 py-2 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
      />
      {error && <p className="text-xs text-destructive line-clamp-4">{error}</p>}
      <Button size="sm" className="w-full" onClick={onGenerate} disabled={!prompt.trim()}>
        <Sparkles className="w-3.5 h-3.5 mr-1.5" />
        Generate
      </Button>
      <p className="text-[10px] text-muted-foreground text-center">
        {profileName} · {provider} · {model || 'default'}
      </p>
    </div>
  );
}
