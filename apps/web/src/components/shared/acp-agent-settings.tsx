import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, Sparkles, Globe, Terminal, Pencil, Trash2, Plug, Unplug } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import type { AcpAgentConfig } from '@/types/agent-settings';

/* ---------- Shared field wrapper ---------- */
function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <label className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground/70 pl-0.5">
        {label}
      </label>
      {children}
    </div>
  );
}

const inputClass =
  'w-full h-8 px-2.5 text-[12px] bg-background text-foreground rounded-md border border-input focus:border-ring focus:ring-1 focus:ring-ring/20 outline-none transition-all';

/* ---------- Helpers ---------- */

/** Check whether we are running inside the Electron shell */
function isElectron(): boolean {
  return (
    typeof window !== 'undefined' && !!(window as unknown as Record<string, unknown>).electronAPI
  );
}

/** Parse KEY=VALUE lines into a record */
function parseEnvText(text: string): Record<string, string> {
  const env: Record<string, string> = {};
  for (const line of text.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || !trimmed.includes('=')) continue;
    const idx = trimmed.indexOf('=');
    const key = trimmed.slice(0, idx).trim();
    const value = trimmed.slice(idx + 1).trim();
    if (key) env[key] = value;
  }
  return env;
}

/** Serialize a record to KEY=VALUE lines */
function envToText(env?: Record<string, string>): string {
  if (!env) return '';
  return Object.entries(env)
    .map(([k, v]) => `${k}=${v}`)
    .join('\n');
}

/* ---------- AcpAgentForm ---------- */
export function AcpAgentForm({
  initial,
  onSave,
  onCancel,
}: {
  initial?: AcpAgentConfig;
  onSave: (data: Omit<AcpAgentConfig, 'id'>) => void;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const electron = isElectron();

  const [displayName, setDisplayName] = useState(initial?.displayName ?? '');
  const [connectionType, setConnectionType] = useState<'local' | 'remote'>(
    initial?.connectionType ?? (electron ? 'local' : 'remote'),
  );
  const [command, setCommand] = useState(initial?.command ?? '');
  const [args, setArgs] = useState(initial?.args?.join(', ') ?? '');
  const [envText, setEnvText] = useState(envToText(initial?.env));
  const [url, setUrl] = useState(initial?.url ?? '');

  const canSave =
    displayName.trim().length > 0 &&
    (connectionType === 'local' ? command.trim().length > 0 : url.trim().length > 0);

  const handleSave = useCallback(() => {
    const base = {
      displayName: displayName.trim(),
      connectionType,
      enabled: initial?.enabled ?? true,
    };

    if (connectionType === 'local') {
      const parsedArgs = args
        .split(',')
        .map((a) => a.trim())
        .filter(Boolean);
      const parsedEnv = parseEnvText(envText);
      onSave({
        ...base,
        command: command.trim(),
        args: parsedArgs.length > 0 ? parsedArgs : undefined,
        env: Object.keys(parsedEnv).length > 0 ? parsedEnv : undefined,
      });
    } else {
      onSave({
        ...base,
        url: url.trim(),
      });
    }
  }, [displayName, connectionType, command, args, envText, url, initial?.enabled, onSave]);

  return (
    <div className="rounded-xl border border-border overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-2.5 bg-secondary/30 border-b border-border">
        <div className="h-5 w-5 rounded-md bg-primary/10 flex items-center justify-center">
          <Sparkles size={11} className="text-primary" />
        </div>
        <span className="text-[12px] font-medium text-foreground">
          {initial ? t('common.save') : t('acp.addAgent')}
        </span>
      </div>

      <div className="p-4 space-y-3.5">
        {/* Display Name */}
        <Field label={t('acp.displayName')}>
          <input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder={t('acp.displayNamePlaceholder')}
            className={inputClass}
          />
        </Field>

        {/* Connection Type toggle */}
        <Field label={t('acp.connectionType')}>
          <div className="flex gap-1">
            {electron && (
              <button
                type="button"
                onClick={() => setConnectionType('local')}
                className={cn(
                  'flex-1 h-7 text-[11px] rounded-md border transition-all font-medium flex items-center justify-center gap-1',
                  connectionType === 'local'
                    ? 'bg-primary text-primary-foreground border-primary'
                    : 'bg-background text-muted-foreground border-input hover:bg-accent',
                )}
              >
                <Terminal size={11} />
                {t('acp.local')}
              </button>
            )}
            <button
              type="button"
              onClick={() => setConnectionType('remote')}
              className={cn(
                'flex-1 h-7 text-[11px] rounded-md border transition-all font-medium flex items-center justify-center gap-1',
                connectionType === 'remote'
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'bg-background text-muted-foreground border-input hover:bg-accent',
              )}
            >
              <Globe size={11} />
              {t('acp.remote')}
            </button>
          </div>
        </Field>

        {/* Local-mode fields */}
        {connectionType === 'local' && (
          <>
            <Field label={t('acp.command')}>
              <input
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                placeholder={t('acp.commandPlaceholder')}
                className={cn(inputClass, 'font-mono')}
              />
            </Field>

            <Field label={t('acp.args')}>
              <input
                value={args}
                onChange={(e) => setArgs(e.target.value)}
                placeholder={t('acp.argsPlaceholder')}
                className={cn(inputClass, 'font-mono text-[11px]')}
              />
            </Field>

            <Field label={t('acp.env')}>
              <textarea
                value={envText}
                onChange={(e) => setEnvText(e.target.value)}
                placeholder={t('acp.envPlaceholder')}
                rows={3}
                className={cn(
                  inputClass,
                  'h-auto py-1.5 resize-none font-mono text-[11px] leading-relaxed',
                )}
              />
            </Field>
          </>
        )}

        {/* Remote-mode fields */}
        {connectionType === 'remote' && (
          <Field label={t('acp.url')}>
            <div className="relative">
              <Globe
                size={12}
                className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground/40"
              />
              <input
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder={t('acp.urlPlaceholder')}
                className={cn(inputClass, 'pl-7 font-mono text-[11px]')}
              />
            </div>
          </Field>
        )}

        {/* Actions */}
        <div className="flex items-center justify-end gap-2 pt-1.5 border-t border-border/50">
          <Button
            variant="ghost"
            size="sm"
            onClick={onCancel}
            className="h-7 px-3 text-[11px] rounded-md"
          >
            {t('common.cancel')}
          </Button>
          <Button
            size="sm"
            onClick={handleSave}
            disabled={!canSave}
            className="h-7 px-4 text-[11px] rounded-md"
          >
            {initial ? t('common.save') : t('acp.addAgent')}
          </Button>
        </div>
      </div>
    </div>
  );
}

/* ---------- AcpAgentCard ---------- */
function AcpAgentCard({ agent }: { agent: AcpAgentConfig }) {
  const { t } = useTranslation();
  const update = useAgentSettingsStore((s) => s.updateAcpAgent);
  const remove = useAgentSettingsStore((s) => s.removeAcpAgent);
  const setStatus = useAgentSettingsStore((s) => s.setAcpConnectionStatus);
  const persist = useAgentSettingsStore((s) => s.persist);
  const connectionStatus = useAgentSettingsStore((s) => s.acpConnectionStatus[agent.id]);

  const [editing, setEditing] = useState(false);
  const [connecting, setConnecting] = useState(false);

  const isConnected = connectionStatus?.isConnected ?? false;
  const agentInfo = connectionStatus?.agentInfo;

  const handleConnect = useCallback(async () => {
    setConnecting(true);
    try {
      const res = await fetch('/api/ai/connect-acp', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'connect', agentId: agent.id, config: agent }),
      });
      const data = res.ok ? await res.json() : null;
      if (data?.connected) {
        setStatus(agent.id, {
          isConnected: true,
          agentInfo: data.agentInfo,
        });
      } else {
        const msg = data?.error ?? `HTTP ${res.status}`;
        console.error('[acp] connect failed:', msg);
        alert(`ACP connect failed: ${msg}`);
        setStatus(agent.id, { isConnected: false });
      }
    } catch (err) {
      console.error('[acp] connect error:', err);
      setStatus(agent.id, { isConnected: false });
    } finally {
      setConnecting(false);
    }
  }, [agent, setStatus]);

  const handleDisconnect = useCallback(async () => {
    setConnecting(true);
    try {
      await fetch('/api/ai/connect-acp', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'disconnect', agentId: agent.id }),
      });
      setStatus(agent.id, { isConnected: false });
    } catch {
      // ignore
    } finally {
      setConnecting(false);
    }
  }, [agent.id, setStatus]);

  const handleRemove = useCallback(() => {
    remove(agent.id);
    persist();
  }, [agent.id, remove, persist]);

  const handleSave = useCallback(
    (data: Omit<AcpAgentConfig, 'id'>) => {
      update(agent.id, data);
      persist();
      setEditing(false);
    },
    [agent.id, update, persist],
  );

  if (editing) {
    return <AcpAgentForm initial={agent} onSave={handleSave} onCancel={() => setEditing(false)} />;
  }

  return (
    <div className="group">
      <div
        className={cn(
          'flex items-center gap-3 px-3.5 py-2.5 rounded-lg border transition-colors',
          agent.enabled
            ? 'bg-secondary/30 border-border'
            : 'border-transparent hover:bg-secondary/20',
        )}
      >
        {/* Icon */}
        <div
          className={cn(
            'w-9 h-9 rounded-lg flex items-center justify-center shrink-0 transition-colors',
            agent.enabled
              ? 'bg-foreground/8 text-foreground'
              : 'bg-secondary text-muted-foreground',
          )}
        >
          {agent.connectionType === 'local' ? <Terminal size={18} /> : <Globe size={18} />}
        </div>

        {/* Info */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1.5">
            <span className="text-[13px] font-medium text-foreground leading-tight">
              {agent.displayName}
            </span>
            <span
              className={cn(
                'text-[9px] font-medium uppercase tracking-wider px-1.5 py-0.5 rounded-full',
                agent.connectionType === 'local'
                  ? 'bg-amber-500/10 text-amber-600 dark:text-amber-400'
                  : 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
              )}
            >
              {agent.connectionType === 'local' ? t('acp.local') : t('acp.remote')}
            </span>
          </div>

          {/* Connection status */}
          <div className="flex items-center gap-1 mt-0.5">
            <span
              className={cn(
                'w-1.5 h-1.5 rounded-full shrink-0',
                isConnected ? 'bg-green-500' : 'bg-muted-foreground/30',
              )}
            />
            <span className="text-[11px] text-muted-foreground leading-tight">
              {isConnected
                ? agentInfo
                  ? `${agentInfo.name}${agentInfo.version ? ` v${agentInfo.version}` : ''}`
                  : t('acp.connected')
                : t('acp.notConnected')}
            </span>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1 shrink-0">
          <Button
            variant="ghost"
            size="sm"
            onClick={isConnected ? handleDisconnect : handleConnect}
            disabled={connecting}
            className="h-6 px-2 text-[10px] rounded-md gap-1"
          >
            {isConnected ? (
              <>
                <Unplug size={11} />
                {t('acp.disconnect')}
              </>
            ) : (
              <>
                <Plug size={11} />
                {t('acp.connect')}
              </>
            )}
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => setEditing(true)}
            className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <Pencil size={11} />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleRemove}
            className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-destructive"
          >
            <Trash2 size={11} />
          </Button>
        </div>
      </div>
    </div>
  );
}

/* ---------- AcpAgentSection ---------- */
export function AcpAgentSection() {
  const { t } = useTranslation();
  const acpAgents = useAgentSettingsStore((s) => s.acpAgents);
  const addAcpAgent = useAgentSettingsStore((s) => s.addAcpAgent);
  const persist = useAgentSettingsStore((s) => s.persist);
  const [showForm, setShowForm] = useState(false);

  const handleAdd = useCallback(
    (data: Omit<AcpAgentConfig, 'id'>) => {
      addAcpAgent(data);
      persist();
      setShowForm(false);
    },
    [addAcpAgent, persist],
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">{t('acp.title')}</h3>
        {!showForm && (
          <button
            onClick={() => setShowForm(true)}
            className="text-[11px] text-primary hover:text-primary/80 flex items-center gap-1 transition-colors font-medium"
          >
            <Plus size={12} /> {t('acp.addAgent')}
          </button>
        )}
      </div>
      <p className="text-[11px] text-muted-foreground leading-relaxed">{t('acp.description')}</p>
      {showForm && <AcpAgentForm onSave={handleAdd} onCancel={() => setShowForm(false)} />}
      {acpAgents.map((agent) => (
        <AcpAgentCard key={agent.id} agent={agent} />
      ))}
      {!showForm && acpAgents.length === 0 && (
        <div className="text-center py-6 text-[11px] text-muted-foreground">{t('acp.empty')}</div>
      )}
    </div>
  );
}
