import { useTranslation } from 'react-i18next';
import { cn } from '@/lib/utils';

const QUICK_ACTIONS = [
  {
    emoji: '📱',
    labelKey: 'ai.quickAction.loginScreen',
    descKey: 'ai.quickAction.loginScreenDesc',
    promptKey: 'ai.quickAction.loginScreenPrompt',
  },
  {
    emoji: '🍕',
    labelKey: 'ai.quickAction.foodApp',
    descKey: 'ai.quickAction.foodAppDesc',
    promptKey: 'ai.quickAction.foodAppPrompt',
  },
  {
    emoji: '⬇️',
    labelKey: 'ai.quickAction.bottomNav',
    descKey: 'ai.quickAction.bottomNavDesc',
    promptKey: 'ai.quickAction.bottomNavPrompt',
  },
  {
    emoji: '🎨',
    labelKey: 'ai.quickAction.colorPalette',
    descKey: 'ai.quickAction.colorPaletteDesc',
    promptKey: 'ai.quickAction.colorPalettePrompt',
  },
];

interface AIChatQuickActionsProps {
  onSend: (prompt: string) => void;
  disabled: boolean;
}

export function AIChatQuickActions({ onSend, disabled }: AIChatQuickActionsProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center justify-center py-6 px-1">
      <p className="text-xs text-muted-foreground mb-4">{t('ai.startDesigning')}</p>
      <div className="grid grid-cols-2 gap-2 w-full">
        {QUICK_ACTIONS.map((action) => (
          <button
            key={action.labelKey}
            type="button"
            onClick={() => onSend(t(action.promptKey))}
            disabled={disabled}
            className={cn(
              'flex flex-col items-start gap-0.5 p-3 rounded-lg border border-border bg-secondary/30 text-left transition-colors',
              disabled ? 'cursor-default opacity-60' : 'hover:bg-secondary hover:border-border/80',
            )}
          >
            <div className="flex items-center gap-1.5">
              <span className="text-sm">{action.emoji}</span>
              <span className="text-xs font-medium text-foreground">{t(action.labelKey)}</span>
            </div>
            <span className="text-[10px] text-muted-foreground leading-tight">
              {t(action.descKey)}
            </span>
          </button>
        ))}
      </div>
      <p className="text-[10px] text-muted-foreground/50 mt-5">{t('ai.tipSelectElements')}</p>
    </div>
  );
}
