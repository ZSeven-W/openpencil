import { useState, useEffect, useCallback } from 'react';
import { Component, Diamond, ArrowUpRight, Unlink } from 'lucide-react';
import { useDesignEngine } from '../hooks/use-design-engine.js';
import { useSelection } from '../hooks/use-selection.js';
import { useDocument } from '../hooks/use-document.js';
import { SizeSection } from './sections/size-section.js';
import { LayoutSection } from './sections/layout-section.js';
import { FillSection } from './sections/fill-section.js';
import { StrokeSection } from './sections/stroke-section.js';
import { AppearanceSection } from './sections/appearance-section.js';
import { TextSection } from './sections/text-section.js';
import { TextLayoutSection } from './sections/text-layout-section.js';
import { EffectsSection } from './sections/effects-section.js';
import { ExportSection } from './sections/export-section.js';
import { IconSection } from './sections/icon-section.js';
import { ImageSection } from './sections/image-section.js';
import type { PenNode } from '@zseven-w/pen-types';

/** Properties stored directly on the RefNode (instance-level), not as overrides. */
const INSTANCE_DIRECT_PROPS = new Set([
  'x',
  'y',
  'width',
  'height',
  'name',
  'visible',
  'locked',
  'rotation',
  'opacity',
  'flipX',
  'flipY',
  'enabled',
  'theme',
]);

type RefNode = PenNode & { ref: string; descendants?: Record<string, Record<string, unknown>> };
type PathNode = PenNode & { d?: string; iconId?: string };
type IconFontNode = PenNode & { iconFontFamily?: string; iconFontName?: string };
type ImageNode = PenNode & { src?: string; objectFit?: 'fill' | 'fit' | 'crop' | 'tile' };
type ContainerProps = {
  layout?: 'none' | 'vertical' | 'horizontal';
  justifyContent?: string;
  alignItems?: string;
  gap?: number;
  padding?: number | [number, number] | [number, number, number, number] | string;
  clipContent?: boolean;
};

export interface PropertyPanelProps {
  /** When true, renders without the outer panel wrapper (for embedding in a larger panel). */
  embedded?: boolean;
}

export function PropertyPanel({ embedded }: PropertyPanelProps = {}) {
  const engine = useDesignEngine();
  const selection = useSelection();
  const activeId = selection[0] ?? null;
  // Subscribe to document changes so panel re-renders on node property updates
  void useDocument();

  const node = activeId ? engine.getNodeById(activeId) : undefined;

  const [isEditingName, setIsEditingName] = useState(false);
  const [editName, setEditName] = useState('');
  useEffect(() => {
    setIsEditingName(false);
  }, [activeId]);

  const handleUpdate = useCallback(
    (updates: Partial<PenNode>) => {
      if (!activeId) return;
      const nodeIsInstance = node?.type === 'ref';
      if (nodeIsInstance && node.type === 'ref') {
        const refNode = node as RefNode;
        const refNodeUpdate: Record<string, unknown> = {};
        const overrideProps: Record<string, unknown> = {};

        for (const [key, value] of Object.entries(updates)) {
          if (INSTANCE_DIRECT_PROPS.has(key)) {
            refNodeUpdate[key] = value;
          } else {
            overrideProps[key] = value;
          }
        }

        if (Object.keys(overrideProps).length > 0) {
          const currentDescendants = refNode.descendants ?? {};
          const existing = currentDescendants[refNode.ref] ?? {};
          refNodeUpdate.descendants = {
            ...currentDescendants,
            [refNode.ref]: { ...existing, ...overrideProps },
          };
        }

        if (Object.keys(refNodeUpdate).length > 0) {
          engine.updateNode(activeId, refNodeUpdate as Partial<PenNode>);
        }
      } else {
        engine.updateNode(activeId, updates);
      }
    },
    [activeId, node, engine],
  );

  if (!node) return null;

  const nodeIsReusable =
    'reusable' in node && (node as unknown as Record<string, unknown>).reusable === true;
  const nodeIsInstance = node.type === 'ref';

  // For RefNodes, resolve the referenced component to get visual properties.
  let displayNode = node;
  if (nodeIsInstance) {
    const refNode = node as RefNode;
    const component = engine.getNodeById(refNode.ref);
    if (component) {
      const topOverrides = refNode.descendants?.[refNode.ref] ?? {};
      const merged: Record<string, unknown> = { ...component, ...topOverrides };
      for (const [key, val] of Object.entries(node)) {
        if (key === 'type' || key === 'ref' || key === 'descendants' || key === 'children')
          continue;
        if (val !== undefined) merged[key] = val;
      }
      merged.type = component.type;
      if (!merged.name) merged.name = component.name;
      displayNode = merged as unknown as PenNode;
    }
  }

  const handleGoToComponent = () => {
    if (!nodeIsInstance || node.type !== 'ref') return;
    const refId = (node as RefNode).ref;
    engine.select([refId]);
  };

  const isContainer =
    displayNode.type === 'frame' ||
    displayNode.type === 'group' ||
    displayNode.type === 'rectangle';
  const hasLayout = isContainer;
  const isImage = displayNode.type === 'image';
  const hasFill = displayNode.type !== 'line' && !isImage;
  const hasStroke = !isImage;
  const hasCornerRadius =
    displayNode.type === 'rectangle' ||
    displayNode.type === 'frame' ||
    isImage ||
    displayNode.type === 'polygon' ||
    displayNode.type === 'ellipse';
  const isText = displayNode.type === 'text';
  const isIcon =
    (displayNode.type === 'path' && !!(displayNode as PathNode).iconId) ||
    displayNode.type === 'icon_font';

  const handleNameClick = () => {
    setEditName(node.name ?? node.type);
    setIsEditingName(true);
  };

  const handleNameBlur = () => {
    setIsEditingName(false);
    const trimmed = editName.trim();
    if (trimmed && trimmed !== (node.name ?? node.type)) {
      engine.updateNode(activeId!, { name: trimmed } as Partial<PenNode>);
    }
  };

  const handleNameKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
    if (e.key === 'Escape') {
      setIsEditingName(false);
      setEditName(node.name ?? node.type);
    }
  };

  const content = (
    <>
      {/* Header */}
      <div className="h-8 flex items-center px-2 border-b border-border gap-1 shrink-0">
        {(nodeIsReusable || nodeIsInstance) && (
          <Diamond
            size={12}
            className={`shrink-0 ${nodeIsReusable ? 'text-purple-400' : 'text-[#9281f7]'}`}
          />
        )}
        {isEditingName ? (
          <input
            type="text"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            onBlur={handleNameBlur}
            onKeyDown={handleNameKeyDown}
            className={`text-[11px] font-medium flex-1 min-w-0 bg-secondary rounded px-1.5 py-0.5 border border-ring focus:outline-none ${
              nodeIsReusable
                ? 'text-purple-400'
                : nodeIsInstance
                  ? 'text-[#9281f7]'
                  : 'text-foreground'
            }`}
            autoFocus
          />
        ) : (
          <span
            className={`text-[11px] font-medium flex-1 truncate cursor-text ${
              nodeIsReusable
                ? 'text-purple-400 border border-purple-400/50 rounded px-1.5 py-0.5'
                : nodeIsInstance
                  ? 'text-[#9281f7] border border-dashed border-[#9281f7]/50 rounded px-1.5 py-0.5'
                  : 'text-foreground px-1'
            }`}
            onClick={handleNameClick}
          >
            {node.name ?? node.type}
          </span>
        )}
        {nodeIsInstance && (
          <>
            <button
              type="button"
              title="Go to component"
              className="p-1 rounded hover:bg-accent/50 text-muted-foreground hover:text-foreground shrink-0"
              onClick={handleGoToComponent}
            >
              <ArrowUpRight size={12} />
            </button>
            <button
              type="button"
              title="Detach instance"
              className="p-1 rounded hover:bg-accent/50 text-muted-foreground hover:text-foreground shrink-0"
              onClick={() => activeId && (engine as any).detachComponent?.(activeId)}
            >
              <Unlink size={12} />
            </button>
          </>
        )}
      </div>

      <div className="flex-1 overflow-y-auto">
        {(isContainer || nodeIsInstance) && (
          <div className="px-3 py-2">
            {nodeIsReusable ? (
              <button
                type="button"
                className="w-full h-7 text-xs border border-purple-500/30 rounded gap-1.5 flex items-center justify-center text-purple-400 hover:bg-purple-500/10 transition-colors"
                onClick={() => activeId && (engine as any).detachComponent?.(activeId)}
              >
                <Unlink size={12} />
                Detach Component
              </button>
            ) : (
              <button
                type="button"
                className="w-full h-7 text-xs border border-border rounded gap-1.5 flex items-center justify-center text-foreground hover:bg-accent/50 transition-colors"
                onClick={() => {
                  if (!activeId) return;
                  const eng = engine as any;
                  if (nodeIsInstance) {
                    const newId = eng.detachComponent?.(activeId);
                    if (newId) {
                      eng.makeReusable?.(newId);
                      engine.select([newId]);
                    }
                    return;
                  }
                  eng.makeReusable?.(activeId);
                }}
              >
                <Component size={12} />
                Create Component
              </button>
            )}
          </div>
        )}

        <div className="px-3 py-2">
          <SizeSection
            node={displayNode}
            onUpdate={handleUpdate}
            hasCornerRadius={hasCornerRadius}
            cornerRadius={
              'cornerRadius' in displayNode
                ? (
                    displayNode as PenNode & {
                      cornerRadius?: number | [number, number, number, number];
                    }
                  ).cornerRadius
                : undefined
            }
            hideWH={hasLayout || isText}
          />
        </div>

        {hasLayout && (
          <>
            <div className="h-px bg-border" />
            <div className="px-3 py-2">
              <LayoutSection
                node={displayNode as PenNode & ContainerProps}
                onUpdate={handleUpdate}
              />
            </div>
          </>
        )}

        {isText && displayNode.type === 'text' && (
          <>
            <div className="h-px bg-border" />
            <div className="px-3 py-2">
              <TextLayoutSection
                node={
                  displayNode as PenNode & {
                    textGrowth?: 'auto' | 'fixed-width' | 'fixed-width-height';
                  }
                }
                onUpdate={handleUpdate}
              />
            </div>
          </>
        )}

        {isIcon && (
          <>
            <div className="h-px bg-border" />
            <div className="px-3 py-2">
              <IconSection
                node={displayNode as PathNode | IconFontNode}
                onUpdate={(updates) => handleUpdate(updates as Partial<PenNode>)}
              />
            </div>
          </>
        )}

        {isImage && displayNode.type === 'image' && (
          <>
            <div className="h-px bg-border" />
            <div className="px-3 py-2">
              <ImageSection
                node={displayNode as ImageNode}
                onUpdate={(updates) => handleUpdate(updates as Partial<PenNode>)}
              />
            </div>
          </>
        )}

        <div className="h-px bg-border" />

        <div className="px-3 py-2">
          <AppearanceSection node={displayNode} onUpdate={handleUpdate} />
        </div>

        <div className="h-px bg-border" />

        {hasFill && (
          <>
            <div className="px-3 py-2">
              <FillSection
                fills={
                  'fill' in displayNode
                    ? ((displayNode as PenNode & { fill?: unknown[] }).fill as never)
                    : undefined
                }
                onUpdate={handleUpdate}
              />
            </div>
            <div className="h-px bg-border" />
          </>
        )}

        {hasStroke && (
          <>
            <div className="px-3 py-2">
              <StrokeSection
                stroke={
                  'stroke' in displayNode
                    ? ((displayNode as PenNode & { stroke?: unknown }).stroke as never)
                    : undefined
                }
                onUpdate={handleUpdate}
              />
            </div>
            <div className="h-px bg-border" />
          </>
        )}

        {isText && displayNode.type === 'text' && (
          <div className="px-3 py-2">
            <TextSection
              node={displayNode as PenNode & { fontFamily?: string }}
              onUpdate={handleUpdate}
            />
          </div>
        )}

        <>
          <div className="h-px bg-border" />
          <div className="px-3 py-2">
            <EffectsSection
              effects={
                'effects' in displayNode
                  ? ((displayNode as PenNode & { effects?: unknown[] }).effects as never)
                  : undefined
              }
              onUpdate={handleUpdate}
            />
          </div>
        </>

        <div className="h-px bg-border" />
        <div className="px-3 py-2">
          <ExportSection nodeId={node.id} nodeName={node.name ?? node.type} />
        </div>
      </div>
    </>
  );

  if (embedded) return content;

  return (
    <div className="w-64 bg-card border-l border-border flex flex-col shrink-0">{content}</div>
  );
}
