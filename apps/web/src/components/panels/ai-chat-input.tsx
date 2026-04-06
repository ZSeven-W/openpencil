import { useState, useRef, useCallback } from 'react';
import { Send, ChevronUp, Paperclip, X, Square, Key } from 'lucide-react';
import { nanoid } from 'nanoid';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useAIStore } from '@/stores/ai-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { PROVIDER_ICON, ConcurrencyButton, ModelDropdown } from './ai-chat-model-selector';

interface AIChatInputProps {
  input: string;
  setInput: (value: string) => void;
  onSend: (text?: string) => void;
}

/**
 * Chat input area: textarea, attachment preview strip, model selector,
 * concurrency button, attachment button, and send/stop button.
 */
export function AIChatInput({ input, setInput, onSend }: AIChatInputProps) {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [modelDropdownOpen, setModelDropdownOpen] = useState(false);

  const isStreaming = useAIStore((s) => s.isStreaming);
  const stopStreaming = useAIStore((s) => s.stopStreaming);
  const model = useAIStore((s) => s.model);
  const availableModels = useAIStore((s) => s.availableModels);
  const modelGroups = useAIStore((s) => s.modelGroups);
  const isLoadingModels = useAIStore((s) => s.isLoadingModels);
  const pendingAttachments = useAIStore((s) => s.pendingAttachments);
  const addPendingAttachment = useAIStore((s) => s.addPendingAttachment);
  const removePendingAttachment = useAIStore((s) => s.removePendingAttachment);
  const selectedIds = useCanvasStore((s) => s.selection.selectedIds);

  const noAvailableModels = !isLoadingModels && availableModels.length === 0;
  const canUseModel = !isLoadingModels && availableModels.length > 0;
  const canSendMessage =
    canUseModel && !isStreaming && (!!input.trim() || pendingAttachments.length > 0);

  const processImageFiles = useCallback(
    (files: File[]) => {
      const maxSize = 5 * 1024 * 1024; // 5MB
      const maxCount = 4;
      const currentCount = useAIStore.getState().pendingAttachments.length;
      const remaining = maxCount - currentCount;
      if (remaining <= 0) return;

      files
        .filter((f) => f.type.startsWith('image/') && f.size <= maxSize)
        .slice(0, remaining)
        .forEach((file) => {
          const reader = new FileReader();
          reader.onload = () => {
            const dataUrl = reader.result as string;
            const base64 = dataUrl.split(',')[1];
            if (!base64) return;
            addPendingAttachment({
              id: nanoid(),
              name: file.name || 'pasted-image.png',
              mediaType: file.type,
              data: base64,
              size: file.size,
            });
          };
          reader.readAsDataURL(file);
        });
    },
    [addPendingAttachment],
  );

  const handleFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (!files) return;
      processImageFiles(Array.from(files));
      // Reset so the same file can be re-selected
      e.target.value = '';
    },
    [processImageFiles],
  );

  const handlePaste = useCallback(
    (e: React.ClipboardEvent) => {
      const files = Array.from(e.clipboardData?.files ?? []);
      const images = files.filter((f) => f.type.startsWith('image/'));
      if (images.length === 0) return;
      e.preventDefault();
      processImageFiles(images);
    },
    [processImageFiles],
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      onSend();
    }
  };

  return (
    <div className="relative border-t border-border bg-card rounded-b-xl">
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        multiple
        className="hidden"
        onChange={handleFileSelect}
      />

      {/* Attachment preview strip */}
      {pendingAttachments.length > 0 && (
        <div className="flex items-center gap-1.5 px-3 pt-2 pb-1 overflow-x-auto">
          {pendingAttachments.map((att) => (
            <div key={att.id} className="relative group shrink-0">
              <img
                src={`data:${att.mediaType};base64,${att.data}`}
                alt={att.name}
                className="w-8 h-8 rounded object-cover border border-border"
              />
              <button
                type="button"
                onClick={() => removePendingAttachment(att.id)}
                className="absolute -top-1 -right-1 w-3.5 h-3.5 rounded-full bg-foreground text-background flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <X size={8} />
              </button>
            </div>
          ))}
        </div>
      )}

      <textarea
        ref={inputRef}
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        onPaste={handlePaste}
        placeholder={isStreaming ? t('ai.generating') : t('ai.designWithAgent')}
        disabled={isStreaming}
        rows={2}
        className="w-full bg-transparent text-sm text-foreground placeholder-muted-foreground px-3.5 pt-3 pb-2 resize-none outline-none max-h-28 min-h-[52px]"
      />

      {/* --- Bottom bar: model selector + concurrency + selected + attach + send --- */}
      <div className="relative flex items-center justify-between px-2 pb-2">
        <div className="flex items-center">
          {/* Model selector */}
          <button
            type="button"
            onClick={() => setModelDropdownOpen((v) => !v)}
            disabled={isLoadingModels || availableModels.length === 0}
            className="flex items-center gap-1.5 text-[11px] text-muted-foreground hover:text-foreground transition-colors px-1.5 py-1 rounded-md hover:bg-secondary"
          >
            {(() => {
              if (model.startsWith('builtin:')) {
                const currentGroup = modelGroups.find((g) =>
                  g.models.some((m) => m.value === model),
                );
                if (currentGroup) {
                  const ProvIcon = PROVIDER_ICON[currentGroup.provider];
                  return ProvIcon ? (
                    <ProvIcon className="w-3.5 h-3.5 shrink-0" />
                  ) : (
                    <Key size={12} className="shrink-0 text-muted-foreground" />
                  );
                }
                return <Key size={12} className="shrink-0 text-muted-foreground" />;
              }
              const currentProvider = modelGroups.find((g) =>
                g.models.some((m) => m.value === model),
              )?.provider;
              if (currentProvider) {
                const ProvIcon = PROVIDER_ICON[currentProvider];
                return <ProvIcon className="w-3.5 h-3.5 shrink-0" />;
              }
              return null;
            })()}
            <span className="truncate max-w-[100px]">
              {isLoadingModels
                ? t('ai.loadingModels')
                : noAvailableModels
                  ? t('ai.noModelsConnected')
                  : (availableModels.find((m) => m.value === model)?.displayName ?? model)}
            </span>
            <ChevronUp size={10} className="shrink-0" />
          </button>

          {/* Concurrency selector */}
          <ConcurrencyButton />

          <span
            className={cn(
              'ml-1 shrink-0 whitespace-nowrap text-[10px] select-none',
              selectedIds.length > 0 ? 'text-muted-foreground/80' : 'text-muted-foreground/40',
            )}
          >
            {t('common.selected', { count: selectedIds.length })}
          </span>
        </div>

        {/* Action icons */}
        <div className="flex items-center gap-0.5">
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => fileInputRef.current?.click()}
            disabled={isStreaming || pendingAttachments.length >= 4}
            title={t('ai.attachImage')}
            className="shrink-0 rounded-lg h-7 w-7"
          >
            <Paperclip size={13} />
          </Button>
          {isStreaming ? (
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={stopStreaming}
              title={t('ai.stopGenerating')}
              className="shrink-0 rounded-lg h-7 w-7 text-destructive hover:text-destructive hover:scale-110 active:scale-95 transition-all duration-150"
            >
              <Square size={10} fill="currentColor" />
            </Button>
          ) : (
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => onSend()}
              disabled={!canSendMessage}
              title={t('ai.sendMessage')}
              className={cn(
                'shrink-0 rounded-lg h-7 w-7 transition-all duration-150',
                canSendMessage
                  ? 'text-primary hover:text-primary hover:scale-110 active:scale-95'
                  : 'text-muted-foreground/30',
              )}
            >
              <Send size={13} />
            </Button>
          )}
        </div>

        {/* Upward model dropdown */}
        <ModelDropdown open={modelDropdownOpen} onClose={() => setModelDropdownOpen(false)} />
      </div>
    </div>
  );
}
