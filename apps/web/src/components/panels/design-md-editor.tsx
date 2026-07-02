import { useState } from 'react';
import { ChevronDown, ChevronRight, Copy } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { cn } from '@/lib/utils';
import type { DesignMdSpec, DesignMdColor } from '@/types/design-md';

type SectionId = 'theme' | 'colors' | 'typography' | 'components' | 'layout' | 'notes';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface DesignMdContentProps {
  designMd: DesignMdSpec;
  onClear: () => void;
  onSyncColor: (color: DesignMdColor) => void;
  onSyncAllColors: () => void;
}

// ---------------------------------------------------------------------------
// DesignMdContent — renders all parsed design.md sections
// ---------------------------------------------------------------------------

export function DesignMdContent({
  designMd,
  onClear,
  onSyncColor,
  onSyncAllColors,
}: DesignMdContentProps) {
  const { t } = useTranslation();
  const [expandedSections, setExpandedSections] = useState<Set<SectionId>>(
    new Set(['theme', 'colors', 'typography']),
  );
  const [copiedHex, setCopiedHex] = useState<string | null>(null);

  const toggleSection = (id: SectionId) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleCopyHex = (hex: string) => {
    navigator.clipboard.writeText(hex);
    setCopiedHex(hex);
    setTimeout(() => setCopiedHex(null), 1500);
  };

  return (
    <div className="p-3 space-y-2">
      {/* Project name */}
      {designMd.projectName && (
        <div className="px-1 pb-1">
          <h3 className="text-sm font-semibold text-foreground">{designMd.projectName}</h3>
        </div>
      )}

      {/* Visual Theme */}
      {designMd.visualTheme && (
        <Section
          title={t('designMd.visualTheme')}
          expanded={expandedSections.has('theme')}
          onToggle={() => toggleSection('theme')}
        >
          <MdText text={designMd.visualTheme} limit={600} />
        </Section>
      )}

      {/* Color Palette */}
      {designMd.colorPalette && designMd.colorPalette.length > 0 && (
        <Section
          title={`${t('designMd.colors')} (${designMd.colorPalette.length})`}
          expanded={expandedSections.has('colors')}
          onToggle={() => toggleSection('colors')}
          action={
            <button
              onClick={(e) => {
                e.stopPropagation();
                onSyncAllColors();
              }}
              className="text-[10px] text-primary hover:text-primary/80 transition-colors"
            >
              {t('designMd.syncAllToVariables')}
            </button>
          }
        >
          <div className="space-y-0.5">
            {designMd.colorPalette.map((color, i) => (
              <div
                key={i}
                className="flex items-center gap-2 px-1.5 py-1 rounded-md hover:bg-muted/50 group transition-colors"
              >
                <div
                  className="w-6 h-6 rounded-md border border-border/60 shrink-0 cursor-pointer shadow-sm"
                  style={{ backgroundColor: color.hex }}
                  onClick={() => handleCopyHex(color.hex)}
                  title={t('designMd.copyHex')}
                />
                <div className="flex-1 min-w-0">
                  <span className="text-[11px] font-medium text-foreground truncate block">
                    {color.name}
                  </span>
                  <span className="text-[10px] text-muted-foreground truncate block">
                    {color.hex} — {color.role}
                  </span>
                </div>
                <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button
                    onClick={() => handleCopyHex(color.hex)}
                    className="p-1 rounded-md hover:bg-muted"
                    title={t('designMd.copyHex')}
                  >
                    <Copy
                      size={11}
                      className={cn(
                        'text-muted-foreground',
                        copiedHex === color.hex && 'text-primary',
                      )}
                    />
                  </button>
                  <button
                    onClick={() => onSyncColor(color)}
                    className="text-[9px] px-1.5 py-0.5 rounded-md hover:bg-muted text-muted-foreground font-medium"
                    title={t('designMd.addAsVariable')}
                  >
                    {t('designMd.addAsVariable')}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </Section>
      )}

      {/* Typography */}
      {designMd.typography && (
        <Section
          title={t('designMd.typography')}
          expanded={expandedSections.has('typography')}
          onToggle={() => toggleSection('typography')}
        >
          {designMd.typography.scale ? (
            <MdText text={designMd.typography.scale} limit={600} />
          ) : (
            <div className="space-y-1.5 text-[11px] text-muted-foreground">
              {designMd.typography.fontFamily && (
                <p>
                  <span className="text-foreground font-medium">{t('designMd.font')}:</span>{' '}
                  {designMd.typography.fontFamily}
                </p>
              )}
              {designMd.typography.headings && (
                <p>
                  <span className="text-foreground font-medium">{t('designMd.headings')}:</span>{' '}
                  {renderInline(designMd.typography.headings)}
                </p>
              )}
              {designMd.typography.body && (
                <p>
                  <span className="text-foreground font-medium">{t('designMd.body')}:</span>{' '}
                  {renderInline(designMd.typography.body)}
                </p>
              )}
            </div>
          )}
        </Section>
      )}

      {/* Component Styles */}
      {designMd.componentStyles && (
        <Section
          title={t('designMd.componentStyles')}
          expanded={expandedSections.has('components')}
          onToggle={() => toggleSection('components')}
        >
          <MdText text={designMd.componentStyles} limit={1000} />
        </Section>
      )}

      {/* Layout Principles */}
      {designMd.layoutPrinciples && (
        <Section
          title={t('designMd.layoutPrinciples')}
          expanded={expandedSections.has('layout')}
          onToggle={() => toggleSection('layout')}
        >
          <MdText text={designMd.layoutPrinciples} limit={1000} />
        </Section>
      )}

      {/* Generation Notes */}
      {designMd.generationNotes && (
        <Section
          title={t('designMd.generationNotes')}
          expanded={expandedSections.has('notes')}
          onToggle={() => toggleSection('notes')}
        >
          <MdText text={designMd.generationNotes} limit={600} />
        </Section>
      )}

      {/* Footer: Remove */}
      <div className="pt-2 pb-1 border-t border-border/30">
        <button
          onClick={onClear}
          className="text-[10px] text-muted-foreground hover:text-destructive transition-colors"
        >
          {t('designMd.remove')}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Lightweight markdown renderer
// ---------------------------------------------------------------------------

function MdText({ text, limit }: { text: string; limit?: number }) {
  const content = limit && text.length > limit ? text.substring(0, limit) + '...' : text;

  // Split into blocks by double newline
  const blocks = content.split(/\n{2,}/);

  return (
    <div className="space-y-2 text-[11px] text-muted-foreground leading-relaxed">
      {blocks.map((block, i) => {
        const trimmed = block.trim();
        if (!trimmed) return null;

        // H3 heading
        if (trimmed.startsWith('### ')) {
          return (
            <h4 key={i} className="text-[11px] font-semibold text-foreground mt-1">
              {renderInline(trimmed.slice(4))}
            </h4>
          );
        }
        // H4 heading
        if (trimmed.startsWith('#### ')) {
          return (
            <h5 key={i} className="text-[11px] font-medium text-foreground">
              {renderInline(trimmed.slice(5))}
            </h5>
          );
        }

        // List block
        const lines = trimmed.split('\n');
        const isList = lines.every((l) => /^\s*[-*]\s/.test(l) || !l.trim());
        if (isList) {
          return (
            <ul key={i} className="space-y-1">
              {lines
                .filter((l) => l.trim())
                .map((line, j) => (
                  <li key={j} className="flex gap-1.5">
                    <span className="text-muted-foreground/50 shrink-0 mt-px">&#8226;</span>
                    <span>{renderInline(line.replace(/^\s*[-*]\s+/, ''))}</span>
                  </li>
                ))}
            </ul>
          );
        }

        // Paragraph
        return <p key={i}>{renderInline(trimmed.replace(/\n/g, ' '))}</p>;
      })}
    </div>
  );
}

/** Render inline markdown: **bold**, *italic*, `code`, #HEX color chips */
function renderInline(text: string): React.ReactNode {
  // Split by markdown inline patterns
  const parts: React.ReactNode[] = [];
  let remaining = text;
  let key = 0;

  while (remaining) {
    // Bold: **text**
    const boldMatch = remaining.match(/\*\*(.+?)\*\*/);
    // Code: `text`
    const codeMatch = remaining.match(/`([^`]+)`/);
    // Hex color: #XXXXXX
    const colorMatch = remaining.match(/#([0-9A-Fa-f]{6})\b/);

    // Find earliest match
    const matches = [
      boldMatch && {
        type: 'bold' as const,
        index: boldMatch.index!,
        length: boldMatch[0].length,
        content: boldMatch[1],
      },
      codeMatch && {
        type: 'code' as const,
        index: codeMatch.index!,
        length: codeMatch[0].length,
        content: codeMatch[1],
      },
      colorMatch && {
        type: 'color' as const,
        index: colorMatch.index!,
        length: colorMatch[0].length,
        content: `#${colorMatch[1]}`,
      },
    ]
      .filter(Boolean)
      .sort((a, b) => a!.index - b!.index);

    if (matches.length === 0) {
      parts.push(remaining);
      break;
    }

    const m = matches[0]!;
    if (m.index > 0) parts.push(remaining.substring(0, m.index));

    switch (m.type) {
      case 'bold':
        parts.push(
          <strong key={key++} className="text-foreground font-medium">
            {m.content}
          </strong>,
        );
        break;
      case 'code':
        parts.push(
          <code
            key={key++}
            className="px-1 py-0.5 rounded bg-muted text-[10px] font-mono text-foreground"
          >
            {m.content}
          </code>,
        );
        break;
      case 'color':
        parts.push(
          <span key={key++} className="inline-flex items-center gap-1">
            <span
              className="inline-block w-2.5 h-2.5 rounded-sm border border-border/50 shrink-0"
              style={{ backgroundColor: m.content }}
            />
            <span className="font-mono text-[10px]">{m.content}</span>
          </span>,
        );
        break;
    }

    remaining = remaining.substring(m.index + m.length);
  }

  return parts.length === 1 && typeof parts[0] === 'string' ? parts[0] : <>{parts}</>;
}

// ---------------------------------------------------------------------------
// Collapsible section
// ---------------------------------------------------------------------------

function Section({
  title,
  expanded,
  onToggle,
  action,
  children,
}: {
  title: string;
  expanded: boolean;
  onToggle: () => void;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border border-border/40 overflow-hidden">
      <button
        onClick={onToggle}
        className="flex items-center justify-between w-full px-2.5 py-2 text-[11px] font-medium text-foreground hover:bg-muted/30 transition-colors"
      >
        <div className="flex items-center gap-1.5">
          {expanded ? (
            <ChevronDown size={12} className="text-muted-foreground" />
          ) : (
            <ChevronRight size={12} className="text-muted-foreground" />
          )}
          {title}
        </div>
        {action && <div onClick={(e) => e.stopPropagation()}>{action}</div>}
      </button>
      {expanded && <div className="px-2.5 pb-2.5 pt-0.5">{children}</div>}
    </div>
  );
}
