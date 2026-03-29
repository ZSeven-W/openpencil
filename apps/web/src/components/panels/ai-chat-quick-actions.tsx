import { useTranslation } from 'react-i18next';
import { cn } from '@/lib/utils';

export const QUICK_ACTIONS = [
  {
    labelKey: 'ai.quickAction.loginScreen',
    prompt:
      'Design a modern mobile login screen with email input, password input, login button, and social login options',
  },
  {
    labelKey: 'ai.quickAction.foodApp',
    prompt: 'Generate a well-designed food mobile app homepage',
  },
  {
    labelKey: 'ai.quickAction.bottomNav',
    prompt:
      'Design a mobile app bottom navigation bar with 5 tabs: Home, Search, Add, Messages, Profile',
  },
  {
    labelKey: 'ai.quickAction.colorPalette',
    prompt: 'Suggest a modern color palette for a pet care app',
  },
];

interface AIChatQuickActionsProps {
  onSend: (prompt: string) => void;
  disabled: boolean;
}

/**
 * Quick action buttons shown when the chat has no messages yet.
 */
export function AIChatQuickActions({ onSend, disabled }: AIChatQuickActionsProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center justify-center py-4">
      <p className="text-xs text-muted-foreground mb-4">{t('ai.tryExample')}</p>
      <div className="flex flex-col gap-2 w-full px-2">
        {QUICK_ACTIONS.map((action) => (
          <button
            key={action.labelKey}
            type="button"
            onClick={() => onSend(action.prompt)}
            className={cn(
              'text-xs text-left px-3.5 py-1 rounded-full bg-secondary/50 border border-border text-muted-foreground transition-colors',
              disabled ? 'cursor-default' : 'hover:bg-secondary hover:text-foreground',
            )}
          >
            {t(action.labelKey)}
          </button>
        ))}
      </div>
      <p className="text-[10px] text-muted-foreground/50 mt-5">{t('ai.tipSelectElements')}</p>
    </div>
  );
}
