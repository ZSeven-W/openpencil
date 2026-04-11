import { useState, useRef, useEffect, useCallback } from 'react';
import {
  ChevronDown,
  ChevronUp,
  MessageSquare,
  Loader2,
  Plus,
  Maximize2,
  Minimize2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useAIStore } from '@/stores/ai-store';
import type { PanelCorner } from '@/stores/ai-store';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import type { AIProviderType } from '@/types/agent-settings';
import { useChatHandlers } from './ai-chat-handlers';
import { resolveNextModel } from './ai-chat-model-selector';
import { AIChatMessageList } from './ai-chat-message-list';
import { AIChatInput } from './ai-chat-input';

const MIN_WIDTH = 280;
const MIN_HEIGHT = 250;
const MAX_RATIO = 0.8;

type ResizeEdge = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';

const EDGE_CURSORS: Record<ResizeEdge, string> = {
  n: 'cursor-ns-resize',
  s: 'cursor-ns-resize',
  e: 'cursor-ew-resize',
  w: 'cursor-ew-resize',
  ne: 'cursor-nesw-resize',
  sw: 'cursor-nesw-resize',
  nw: 'cursor-nwse-resize',
  se: 'cursor-nwse-resize',
};

const CORNER_CLASSES: Record<PanelCorner, string> = {
  'top-left': 'top-3 left-3',
  'top-right': 'top-3 right-3',
  'bottom-left': 'bottom-3 left-3',
  'bottom-right': 'bottom-3 right-3',
};

/**
 * Minimized AI bar — a compact clickable pill.
 * Parent is responsible for placing it in the layout.
 */
export function AIChatMinimizedBar() {
  const isMinimized = useAIStore((s) => s.isMinimized);
  const toggleMinimize = useAIStore((s) => s.toggleMinimize);

  if (!isMinimized) return null;

  return (
    <button
      type="button"
      onClick={toggleMinimize}
      className="h-8 bg-card border border-border rounded-lg flex items-center gap-1.5 px-3 shadow-lg hover:bg-accent transition-colors"
    >
      <MessageSquare size={13} className="text-muted-foreground" />
      <span className="text-xs text-muted-foreground max-w-[120px] truncate">
        {useAIStore.getState().chatTitle}
      </span>
      <ChevronUp size={12} className="text-muted-foreground" />
    </button>
  );
}

/**
 * Expanded AI chat panel — floating, draggable.
 * Only renders when NOT minimized.
 */
export default function AIChatPanel() {
  const { t } = useTranslation();
  const panelRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{ offsetX: number; offsetY: number } | null>(null);
  const resizeRef = useRef<{
    edge: ResizeEdge;
    startX: number;
    startY: number;
    startRect: { left: number; top: number; width: number; height: number };
  } | null>(null);
  const [dragStyle, setDragStyle] = useState<React.CSSProperties | null>(null);

  const messages = useAIStore((s) => s.messages);
  const isStreaming = useAIStore((s) => s.isStreaming);
  const clearMessages = useAIStore((s) => s.clearMessages);
  const panelCorner = useAIStore((s) => s.panelCorner);
  const isMinimized = useAIStore((s) => s.isMinimized);
  const setPanelCorner = useAIStore((s) => s.setPanelCorner);
  const chatTitle = useAIStore((s) => s.chatTitle);
  const toggleMinimize = useAIStore((s) => s.toggleMinimize);
  const isMaximized = useAIStore((s) => s.isMaximized);
  const toggleMaximize = useAIStore((s) => s.toggleMaximize);
  const panelWidth = useAIStore((s) => s.panelWidth);
  const panelHeight = useAIStore((s) => s.panelHeight);
  const setPanelSize = useAIStore((s) => s.setPanelSize);
  const hydrateModelPreference = useAIStore((s) => s.hydrateModelPreference);
  const setModel = useAIStore((s) => s.setModel);
  const availableModels = useAIStore((s) => s.availableModels);
  const setAvailableModels = useAIStore((s) => s.setAvailableModels);
  const setModelGroups = useAIStore((s) => s.setModelGroups);
  const isLoadingModels = useAIStore((s) => s.isLoadingModels);
  const setLoadingModels = useAIStore((s) => s.setLoadingModels);
  const providers = useAgentSettingsStore((s) => s.providers);
  const builtinProviders = useAgentSettingsStore((s) => s.builtinProviders);
  const providersHydrated = useAgentSettingsStore((s) => s.isHydrated);

  const { input, setInput, handleSend } = useChatHandlers();
  const canUseModel = !isLoadingModels && availableModels.length > 0;
  const quickActionsDisabled = !canUseModel || isStreaming;

  // Restore model preference from localStorage on page refresh.
  useEffect(() => {
    hydrateModelPreference();
  }, [hydrateModelPreference]);

  // Build model list from connected CLI providers + enabled built-in providers.
  useEffect(() => {
    if (!providersHydrated) {
      setLoadingModels(true);
      return;
    }

    const providerNames: Record<AIProviderType, string> = {
      anthropic: 'Anthropic',
      openai: 'OpenAI',
      opencode: 'OpenCode',
      copilot: 'GitHub Copilot',
      gemini: 'Google Gemini',
    };

    const connectedProviders = (Object.keys(providers) as AIProviderType[]).filter(
      (p) => providers[p].isConnected && (providers[p].models?.length ?? 0) > 0,
    );

    const groups = connectedProviders.map((p) => ({
      provider: p,
      providerName: providerNames[p],
      models: providers[p].models,
    }));

    for (const bp of builtinProviders) {
      if (!bp.enabled || !bp.apiKey) continue;
      const providerType: AIProviderType = bp.type === 'anthropic' ? 'anthropic' : 'openai';
      groups.push({
        provider: providerType,
        providerName:
          bp.displayName || (bp.type === 'anthropic' ? 'Anthropic (API Key)' : bp.displayName),
        models: [
          {
            value: `builtin:${bp.id}:${bp.model}`,
            displayName: bp.model,
            description: t('builtin.viaApiKey', { name: bp.displayName }),
            provider: providerType,
            builtinProviderId: bp.id,
          },
        ],
      });
    }

    if (groups.length > 0) {
      const flat = groups.flatMap((g) =>
        g.models.map((m) => ({
          value: m.value,
          displayName: m.displayName,
          description: m.description,
        })),
      );
      setModelGroups(groups);
      setAvailableModels(flat);
      const { model: currentModel, preferredModel } = useAIStore.getState();
      const nextModel = resolveNextModel(flat, currentModel, preferredModel);
      if (nextModel && nextModel !== currentModel) {
        setModel(nextModel);
      }
      setLoadingModels(false);
      return;
    }

    setModelGroups([]);
    setAvailableModels([]);
    setLoadingModels(false);
  }, [providers, builtinProviders, providersHydrated]); // eslint-disable-line react-hooks/exhaustive-deps

  // Auto-expand when streaming starts while minimized
  useEffect(() => {
    if (isStreaming && isMinimized) {
      toggleMinimize();
    }
  }, [isStreaming, isMinimized, toggleMinimize]);

  /* --- Drag-to-snap handlers --- */

  const handleDragStart = useCallback((e: React.PointerEvent) => {
    if ((e.target as HTMLElement).closest('button, input, textarea, select')) return;
    const panel = panelRef.current;
    if (!panel) return;
    const panelRect = panel.getBoundingClientRect();
    dragRef.current = {
      offsetX: e.clientX - panelRect.left,
      offsetY: e.clientY - panelRect.top,
    };
    e.currentTarget.setPointerCapture(e.pointerId);
    const container = panel.parentElement!;
    const containerRect = container.getBoundingClientRect();
    setDragStyle({
      left: panelRect.left - containerRect.left,
      top: panelRect.top - containerRect.top,
      right: 'auto',
      bottom: 'auto',
    });
  }, []);

  const handleDragMove = useCallback((e: React.PointerEvent) => {
    if (!dragRef.current) return;
    const panel = panelRef.current;
    if (!panel) return;
    const container = panel.parentElement!;
    const containerRect = container.getBoundingClientRect();
    setDragStyle({
      left: e.clientX - containerRect.left - dragRef.current.offsetX,
      top: e.clientY - containerRect.top - dragRef.current.offsetY,
      right: 'auto',
      bottom: 'auto',
    });
  }, []);

  const handleDragEnd = useCallback(() => {
    if (!dragRef.current) return;
    const panel = panelRef.current;
    if (!panel) return;
    const container = panel.parentElement!;
    const containerRect = container.getBoundingClientRect();
    const panelRect = panel.getBoundingClientRect();
    const centerX = panelRect.left + panelRect.width / 2 - containerRect.left;
    const centerY = panelRect.top + panelRect.height / 2 - containerRect.top;
    const isLeft = centerX < containerRect.width / 2;
    const isTop = centerY < containerRect.height / 2;
    const corner: PanelCorner = isLeft
      ? isTop
        ? 'top-left'
        : 'bottom-left'
      : isTop
        ? 'top-right'
        : 'bottom-right';
    setPanelCorner(corner);
    dragRef.current = null;
    setDragStyle(null);
  }, [setPanelCorner]);

  /* --- Resize handlers (all 8 edges/corners) --- */

  const handleResizePointerDown = useCallback(
    (edge: ResizeEdge) => (e: React.PointerEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const panel = panelRef.current;
      if (!panel) return;
      const rect = panel.getBoundingClientRect();
      const container = panel.parentElement!.getBoundingClientRect();
      resizeRef.current = {
        edge,
        startX: e.clientX,
        startY: e.clientY,
        startRect: {
          left: rect.left - container.left,
          top: rect.top - container.top,
          width: rect.width,
          height: rect.height,
        },
      };
      if (!dragStyle) {
        setDragStyle({
          left: rect.left - container.left,
          top: rect.top - container.top,
        });
      }
      e.currentTarget.setPointerCapture(e.pointerId);
    },
    [dragStyle],
  );

  const handleResizePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!resizeRef.current) return;
      e.preventDefault();
      e.stopPropagation();
      const { edge, startX, startY, startRect } = resizeRef.current;
      const container = panelRef.current?.parentElement;
      if (!container) return;
      const containerRect = container.getBoundingClientRect();
      const maxW = containerRect.width * MAX_RATIO;
      const maxH = containerRect.height * MAX_RATIO;
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;

      let newW = startRect.width;
      let newH = startRect.height;
      let newLeft = startRect.left;
      let newTop = startRect.top;

      if (edge.includes('e')) newW = startRect.width + dx;
      if (edge.includes('w')) {
        newW = startRect.width - dx;
        newLeft = startRect.left + dx;
      }
      if (edge.includes('s')) newH = startRect.height + dy;
      if (edge.includes('n')) {
        newH = startRect.height - dy;
        newTop = startRect.top + dy;
      }

      if (newW < MIN_WIDTH) {
        const diff = MIN_WIDTH - newW;
        newW = MIN_WIDTH;
        if (edge.includes('w')) newLeft -= diff;
      }
      if (newW > maxW) {
        const diff = newW - maxW;
        newW = maxW;
        if (edge.includes('w')) newLeft += diff;
      }
      if (newH < MIN_HEIGHT) {
        const diff = MIN_HEIGHT - newH;
        newH = MIN_HEIGHT;
        if (edge.includes('n')) newTop -= diff;
      }
      if (newH > maxH) {
        const diff = newH - maxH;
        newH = maxH;
        if (edge.includes('n')) newTop += diff;
      }

      setPanelSize(Math.round(newW), Math.round(newH));
      setDragStyle({ left: newLeft, top: newTop });
    },
    [setPanelSize],
  );

  const handleResizePointerUp = useCallback((e: React.PointerEvent) => {
    if (!resizeRef.current) return;
    e.preventDefault();
    e.stopPropagation();
    resizeRef.current = null;
    e.currentTarget.releasePointerCapture(e.pointerId);
  }, []);

  const resizeHandleProps = (edge: ResizeEdge) => ({
    onPointerDown: handleResizePointerDown(edge),
    onPointerMove: handleResizePointerMove,
    onPointerUp: handleResizePointerUp,
  });

  const handleHeaderDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      if ((e.target as HTMLElement).closest('button')) return;
      toggleMaximize();
    },
    [toggleMaximize],
  );

  // Don't render when minimized — the minimized bar is rendered by parent
  if (isMinimized) return null;

  return (
    <div
      ref={panelRef}
      className={cn(
        'absolute z-50 flex flex-col overflow-hidden rounded-xl border border-border bg-card/95 shadow-2xl backdrop-blur-sm',
        isMaximized ? 'inset-3' : !dragStyle && CORNER_CLASSES[panelCorner],
      )}
      style={isMaximized ? undefined : { ...dragStyle, width: panelWidth, height: panelHeight }}
    >
      {/* --- Resize Handles (8 directions, hidden when maximized) --- */}
      {!isMaximized && (
        <>
          <div
            className={cn('absolute -top-1 left-2 right-2 h-2 z-50', EDGE_CURSORS.n)}
            {...resizeHandleProps('n')}
          />
          <div
            className={cn('absolute -bottom-1 left-2 right-2 h-2 z-50', EDGE_CURSORS.s)}
            {...resizeHandleProps('s')}
          />
          <div
            className={cn('absolute -left-1 top-2 bottom-2 w-2 z-50', EDGE_CURSORS.w)}
            {...resizeHandleProps('w')}
          />
          <div
            className={cn('absolute -right-1 top-2 bottom-2 w-2 z-50', EDGE_CURSORS.e)}
            {...resizeHandleProps('e')}
          />
          <div
            className={cn('absolute -top-1 -left-1 w-3 h-3 z-[51]', EDGE_CURSORS.nw)}
            {...resizeHandleProps('nw')}
          />
          <div
            className={cn('absolute -top-1 -right-1 w-3 h-3 z-[51]', EDGE_CURSORS.ne)}
            {...resizeHandleProps('ne')}
          />
          <div
            className={cn('absolute -bottom-1 -left-1 w-3 h-3 z-[51]', EDGE_CURSORS.sw)}
            {...resizeHandleProps('sw')}
          />
          <div
            className={cn('absolute -bottom-1 -right-1 w-3 h-3 z-[51]', EDGE_CURSORS.se)}
            {...resizeHandleProps('se')}
          />
        </>
      )}

      {/* --- Header (draggable, double-click to maximize) --- */}
      <div
        className={cn(
          'flex items-center justify-between px-1 py-1 border-b border-border select-none',
          isMaximized ? '' : 'cursor-grab active:cursor-grabbing',
        )}
        onPointerDown={isMaximized ? undefined : handleDragStart}
        onPointerMove={isMaximized ? undefined : handleDragMove}
        onPointerUp={isMaximized ? undefined : handleDragEnd}
        onDoubleClick={handleHeaderDoubleClick}
      >
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="icon-sm" onClick={toggleMinimize} title={t('ai.collapse')}>
            <ChevronDown size={14} />
          </Button>
          <span
            className="text-sm font-medium text-foreground truncate overflow-hidden text-ellipsis"
            style={{ maxWidth: isMaximized ? 300 : Math.max(80, panelWidth - 140) }}
            title={chatTitle}
          >
            {chatTitle}
          </span>
          {isStreaming && <Loader2 size={13} className="animate-spin text-muted-foreground ml-2" />}
        </div>
        <div className="flex items-center gap-0.5">
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={toggleMaximize}
            title={isMaximized ? t('ai.restore') : t('ai.maximize')}
          >
            {isMaximized ? <Minimize2 size={12} /> : <Maximize2 size={12} />}
          </Button>
          <Button variant="ghost" size="icon-sm" onClick={clearMessages} title={t('ai.newChat')}>
            <Plus size={14} />
          </Button>
        </div>
      </div>

      {/* --- Messages --- */}
      <AIChatMessageList
        messages={messages}
        isStreaming={isStreaming}
        onSend={handleSend}
        quickActionsDisabled={quickActionsDisabled}
      />

      {/* --- Input area --- */}
      <AIChatInput input={input} setInput={setInput} onSend={handleSend} />
    </div>
  );
}
