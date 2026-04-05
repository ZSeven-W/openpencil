import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

export type UnsavedResult = 'save' | 'discard' | 'cancel';

interface UnsavedChangesDialogProps {
  open: boolean;
  fileName: string;
  onResult: (result: UnsavedResult) => void;
}

export default function UnsavedChangesDialog({
  open,
  fileName,
  onResult,
}: UnsavedChangesDialogProps) {
  const { t } = useTranslation();
  const [visible, setVisible] = useState(false);
  const isLight = typeof document !== 'undefined' && document.documentElement.classList.contains('light');

  useEffect(() => {
    if (open) requestAnimationFrame(() => setVisible(true));
    else setVisible(false);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onResult('cancel');
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open, onResult]);

  if (!open) return null;

  // Theme-aware color tokens
  const theme = isLight
    ? {
        backdrop: 'radial-gradient(ellipse at center, rgba(255,255,255,0.5) 0%, rgba(100,100,120,0.4) 100%)',
        cardBg: 'linear-gradient(145deg, rgba(255,255,255,0.97) 0%, rgba(248,248,252,0.99) 100%)',
        cardBorder: 'rgba(0,0,0,0.08)',
        cardShadow: '0 25px 60px rgba(0,0,0,0.12), 0 8px 20px rgba(0,0,0,0.06)',
        shimmer: 'linear-gradient(90deg, transparent 0%, rgba(0,0,0,0.04) 30%, rgba(0,0,0,0.06) 50%, rgba(0,0,0,0.04) 70%, transparent 100%)',
        iconBg: 'linear-gradient(135deg, rgba(245,158,11,0.12) 0%, rgba(239,68,68,0.08) 100%)',
        iconBorder: 'rgba(245,158,11,0.2)',
        iconShadow: '0 0 15px rgba(245,158,11,0.1)',
        iconPulse: 'radial-gradient(circle, rgba(245,158,11,0.15) 0%, transparent 70%)',
        iconColor: 'text-amber-500',
        titleColor: 'rgba(15,15,20,0.9)',
        messageColor: 'rgba(15,15,20,0.45)',
        saveBg: 'linear-gradient(135deg, #3b82f6 0%, #6366f1 100%)',
        saveShadow: '0 4px 15px rgba(59,130,246,0.25), inset 0 1px 0 rgba(255,255,255,0.2)',
        discardBg: 'rgba(239,68,68,0.06)',
        discardColor: 'rgba(220,38,38,0.85)',
        discardBorder: '1px solid rgba(239,68,68,0.12)',
        cancelBg: 'rgba(0,0,0,0.04)',
        cancelColor: 'rgba(0,0,0,0.45)',
        cancelBorder: '1px solid rgba(0,0,0,0.08)',
        glowOpacity: 0.3,
        glowSpreadOpacity: 0.08,
      }
    : {
        backdrop: 'radial-gradient(ellipse at center, rgba(0,0,0,0.4) 0%, rgba(0,0,0,0.7) 100%)',
        cardBg: 'linear-gradient(145deg, rgba(30,30,35,0.95) 0%, rgba(18,18,22,0.98) 100%)',
        cardBorder: 'rgba(255,255,255,0.08)',
        cardShadow: '0 25px 60px rgba(0,0,0,0.5), inset 0 1px 0 rgba(255,255,255,0.05)',
        shimmer: 'linear-gradient(90deg, transparent 0%, rgba(255,255,255,0.15) 30%, rgba(255,255,255,0.25) 50%, rgba(255,255,255,0.15) 70%, transparent 100%)',
        iconBg: 'linear-gradient(135deg, rgba(245,158,11,0.2) 0%, rgba(239,68,68,0.15) 100%)',
        iconBorder: 'rgba(245,158,11,0.25)',
        iconShadow: '0 0 20px rgba(245,158,11,0.2), inset 0 1px 0 rgba(255,255,255,0.1)',
        iconPulse: 'radial-gradient(circle, rgba(245,158,11,0.3) 0%, transparent 70%)',
        iconColor: 'text-amber-400',
        titleColor: 'rgba(255,255,255,0.92)',
        messageColor: 'rgba(255,255,255,0.45)',
        saveBg: 'linear-gradient(135deg, #3b82f6 0%, #6366f1 100%)',
        saveShadow: '0 4px 15px rgba(59,130,246,0.3), inset 0 1px 0 rgba(255,255,255,0.15)',
        discardBg: 'rgba(239,68,68,0.1)',
        discardColor: 'rgba(248,113,113,0.9)',
        discardBorder: '1px solid rgba(239,68,68,0.15)',
        cancelBg: 'rgba(255,255,255,0.06)',
        cancelColor: 'rgba(255,255,255,0.5)',
        cancelBorder: '1px solid rgba(255,255,255,0.08)',
        glowOpacity: 0.6,
        glowSpreadOpacity: 0.2,
      };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center perspective-[1200px]">
      {/* Backdrop */}
      <div
        className="absolute inset-0 backdrop-blur-md transition-all duration-500"
        style={{ background: theme.backdrop, opacity: visible ? 1 : 0 }}
        onClick={() => onResult('cancel')}
      />

      {/* Dialog card */}
      <div
        className="relative w-[360px] transition-all duration-500 ease-out"
        style={{
          opacity: visible ? 1 : 0,
          transform: visible
            ? 'translateY(0) scale(1) rotateX(0deg)'
            : 'translateY(40px) scale(0.9) rotateX(8deg)',
        }}
      >
        {/* Animated glow border */}
        <div
          className="absolute -inset-[1px] rounded-2xl"
          style={{
            background: 'conic-gradient(from var(--glow-angle, 0deg), #f59e0b, #ef4444, #8b5cf6, #3b82f6, #10b981, #f59e0b)',
            animation: 'glowSpin 4s linear infinite',
            filter: 'blur(2px)',
            opacity: theme.glowOpacity,
          }}
        />
        {/* Outer glow spread */}
        <div
          className="absolute -inset-3 rounded-3xl"
          style={{
            background: 'conic-gradient(from var(--glow-angle, 0deg), #f59e0b, #ef4444, #8b5cf6, #3b82f6, #10b981, #f59e0b)',
            animation: 'glowSpin 4s linear infinite',
            filter: 'blur(20px)',
            opacity: theme.glowSpreadOpacity,
          }}
        />

        {/* Card */}
        <div
          className="relative rounded-2xl overflow-hidden"
          style={{
            background: theme.cardBg,
            border: `1px solid ${theme.cardBorder}`,
            boxShadow: theme.cardShadow,
          }}
        >
          {/* Top shimmer line */}
          <div className="absolute top-0 left-0 right-0 h-px" style={{ background: theme.shimmer }} />

          <div className="relative px-7 pt-7 pb-6">
            {/* Warning icon */}
            <div className="flex justify-center mb-5">
              <div className="relative">
                <div
                  className="absolute inset-0 rounded-full"
                  style={{
                    background: theme.iconPulse,
                    animation: 'pulse 2s ease-in-out infinite',
                    transform: 'scale(2.5)',
                  }}
                />
                <div
                  className="relative h-12 w-12 rounded-full flex items-center justify-center"
                  style={{
                    background: theme.iconBg,
                    boxShadow: theme.iconShadow,
                    border: `1px solid ${theme.iconBorder}`,
                  }}
                >
                  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={theme.iconColor}>
                    <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" />
                    <path d="M12 9v4" />
                    <path d="M12 17h.01" />
                  </svg>
                </div>
              </div>
            </div>

            {/* Title */}
            <h3
              className="text-center text-[15px] font-semibold tracking-tight mb-1.5"
              style={{ color: theme.titleColor }}
            >
              {t('unsaved.title')}
            </h3>

            {/* Message */}
            <p
              className="text-center text-[13px] leading-relaxed mb-7"
              style={{ color: theme.messageColor }}
            >
              {t('unsaved.message', { name: fileName || t('common.untitled') })}
            </p>

            {/* Actions */}
            <div className="flex flex-col gap-2.5">
              {/* Save — hero button */}
              <button
                type="button"
                onClick={() => onResult('save')}
                className="h-10 w-full rounded-xl text-[13px] font-semibold text-white transition-all duration-200 hover:brightness-110 hover:shadow-lg hover:shadow-blue-500/20 active:scale-[0.98]"
                style={{ background: theme.saveBg, boxShadow: theme.saveShadow }}
              >
                {t('common.save')}
              </button>

              {/* Secondary row */}
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={() => onResult('discard')}
                  className="h-9 flex-1 rounded-xl text-[12px] font-medium transition-all duration-200 hover:brightness-125 active:scale-[0.98]"
                  style={{ background: theme.discardBg, color: theme.discardColor, border: theme.discardBorder }}
                >
                  {t('unsaved.dontSave')}
                </button>
                <button
                  type="button"
                  onClick={() => onResult('cancel')}
                  className="h-9 flex-1 rounded-xl text-[12px] font-medium transition-all duration-200 hover:brightness-125 active:scale-[0.98]"
                  style={{ background: theme.cancelBg, color: theme.cancelColor, border: theme.cancelBorder }}
                >
                  {t('common.cancel')}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <style>{`
        @property --glow-angle {
          syntax: "<angle>";
          initial-value: 0deg;
          inherits: false;
        }
        @keyframes glowSpin {
          to { --glow-angle: 360deg; }
        }
      `}</style>
    </div>
  );
}
