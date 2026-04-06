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
  const [stagger, setStagger] = useState(false);
  const isLight =
    typeof document !== 'undefined' && document.documentElement.classList.contains('light');

  useEffect(() => {
    if (open) {
      requestAnimationFrame(() => setVisible(true));
      const timer = setTimeout(() => setStagger(true), 200);
      return () => clearTimeout(timer);
    }
    setVisible(false);
    setStagger(false);
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

  const L = isLight;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 transition-all duration-700 ease-out"
        style={{
          backdropFilter: visible ? 'blur(12px) saturate(1.2)' : 'blur(0px)',
          WebkitBackdropFilter: visible ? 'blur(12px) saturate(1.2)' : 'blur(0px)',
          background: L
            ? `radial-gradient(ellipse at 50% 40%, rgba(255,255,255,0.55) 0%, rgba(120,120,140,0.45) 100%)`
            : `radial-gradient(ellipse at 50% 40%, rgba(10,10,15,0.5) 0%, rgba(0,0,0,0.75) 100%)`,
          opacity: visible ? 1 : 0,
        }}
        onClick={() => onResult('cancel')}
      />

      {/* Card wrapper */}
      <div
        className="relative w-[340px] transition-all duration-600 ease-[cubic-bezier(0.16,1,0.3,1)]"
        style={{
          opacity: visible ? 1 : 0,
          transform: visible ? 'translateY(0) scale(1)' : 'translateY(32px) scale(0.92)',
        }}
      >
        {/* Glow — slow rotating, breathing opacity */}
        <div
          className="absolute -inset-[1px] rounded-[18px]"
          style={{
            background:
              'conic-gradient(from var(--glow-angle, 0deg), #f59e0b88, #ef444488, #8b5cf688, #3b82f688, #10b98188, #f59e0b88)',
            animation: 'glowSpin 6s linear infinite, glowBreath 3s ease-in-out infinite',
            filter: 'blur(1.5px)',
            opacity: L ? 0.35 : 0.55,
          }}
        />
        <div
          className="absolute -inset-2.5 rounded-[22px]"
          style={{
            background:
              'conic-gradient(from var(--glow-angle, 0deg), #f59e0b, #ef4444, #8b5cf6, #3b82f6, #10b981, #f59e0b)',
            animation: 'glowSpin 6s linear infinite, glowBreath 3s ease-in-out infinite',
            filter: 'blur(18px)',
            opacity: L ? 0.06 : 0.15,
          }}
        />

        {/* Glass card */}
        <div
          className="relative rounded-[18px] overflow-hidden"
          style={{
            background: L
              ? 'linear-gradient(160deg, rgba(255,255,255,0.98) 0%, rgba(250,250,254,0.96) 100%)'
              : 'linear-gradient(160deg, rgba(28,28,34,0.97) 0%, rgba(16,16,20,0.99) 100%)',
            border: `1px solid ${L ? 'rgba(0,0,0,0.06)' : 'rgba(255,255,255,0.07)'}`,
            boxShadow: L
              ? '0 20px 50px rgba(0,0,0,0.1), 0 1px 3px rgba(0,0,0,0.04)'
              : '0 20px 50px rgba(0,0,0,0.5), 0 1px 0 rgba(255,255,255,0.03) inset',
          }}
        >
          {/* Top highlight */}
          <div
            className="absolute top-0 inset-x-0 h-px"
            style={{
              background: L
                ? 'linear-gradient(90deg, transparent, rgba(0,0,0,0.04) 30%, rgba(0,0,0,0.06) 50%, rgba(0,0,0,0.04) 70%, transparent)'
                : 'linear-gradient(90deg, transparent, rgba(255,255,255,0.08) 30%, rgba(255,255,255,0.14) 50%, rgba(255,255,255,0.08) 70%, transparent)',
            }}
          />

          {/* Noise grain overlay */}
          <div
            className="absolute inset-0 opacity-[0.025] pointer-events-none mix-blend-overlay"
            style={{
              backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E")`,
            }}
          />

          <div className="relative px-8 pt-8 pb-7">
            {/* Icon — staggered entry */}
            <div
              className="flex justify-center mb-5 transition-all duration-500 ease-out"
              style={{
                opacity: stagger ? 1 : 0,
                transform: stagger ? 'translateY(0) scale(1)' : 'translateY(8px) scale(0.8)',
              }}
            >
              <div className="relative">
                {/* Soft pulse ring */}
                <div
                  className="absolute -inset-3 rounded-full"
                  style={{
                    background: `radial-gradient(circle, ${L ? 'rgba(245,158,11,0.08)' : 'rgba(245,158,11,0.18)'} 0%, transparent 70%)`,
                    animation: 'iconPulse 2.5s ease-in-out infinite',
                  }}
                />
                {/* Outer ring */}
                <div
                  className="h-[52px] w-[52px] rounded-full p-[1px]"
                  style={{
                    background: `linear-gradient(135deg, ${L ? 'rgba(245,158,11,0.2)' : 'rgba(245,158,11,0.3)'}, ${L ? 'rgba(239,68,68,0.1)' : 'rgba(239,68,68,0.2)'})`,
                  }}
                >
                  {/* Inner circle */}
                  <div
                    className="h-full w-full rounded-full flex items-center justify-center"
                    style={{
                      background: L
                        ? 'linear-gradient(135deg, rgba(245,158,11,0.08), rgba(255,250,240,0.9))'
                        : 'linear-gradient(135deg, rgba(245,158,11,0.12), rgba(30,25,20,0.9))',
                      boxShadow: L
                        ? '0 2px 8px rgba(245,158,11,0.08) inset'
                        : '0 2px 12px rgba(245,158,11,0.1) inset, 0 0 1px rgba(255,255,255,0.06) inset',
                    }}
                  >
                    <svg
                      width="21"
                      height="21"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.8"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      className={L ? 'text-amber-500' : 'text-amber-400'}
                    >
                      <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" />
                      <path d="M12 9v4" />
                      <path d="M12 17h.01" />
                    </svg>
                  </div>
                </div>
              </div>
            </div>

            {/* Title — staggered */}
            <h3
              className="text-center text-[15px] font-semibold tracking-[-0.01em] mb-1 transition-all duration-500 delay-75 ease-out"
              style={{
                color: L ? 'rgba(15,15,20,0.88)' : 'rgba(255,255,255,0.9)',
                opacity: stagger ? 1 : 0,
                transform: stagger ? 'translateY(0)' : 'translateY(6px)',
              }}
            >
              {t('unsaved.title')}
            </h3>

            {/* Subtitle — staggered */}
            <p
              className="text-center text-[12.5px] leading-relaxed mb-7 transition-all duration-500 delay-100 ease-out"
              style={{
                color: L ? 'rgba(15,15,20,0.4)' : 'rgba(255,255,255,0.4)',
                opacity: stagger ? 1 : 0,
                transform: stagger ? 'translateY(0)' : 'translateY(6px)',
              }}
            >
              {t('unsaved.message', { name: fileName || t('common.untitled') })}
            </p>

            {/* Buttons — staggered */}
            <div
              className="flex flex-col gap-2 transition-all duration-500 delay-150 ease-out"
              style={{
                opacity: stagger ? 1 : 0,
                transform: stagger ? 'translateY(0)' : 'translateY(8px)',
              }}
            >
              {/* Save — primary */}
              <button
                type="button"
                onClick={() => onResult('save')}
                className="group relative h-[38px] w-full rounded-[10px] text-[13px] font-semibold text-white overflow-hidden transition-all duration-200 active:scale-[0.98]"
                style={{
                  background: 'linear-gradient(135deg, #3b82f6 0%, #6366f1 100%)',
                  boxShadow: L
                    ? '0 2px 10px rgba(59,130,246,0.25), 0 1px 2px rgba(59,130,246,0.1)'
                    : '0 2px 14px rgba(59,130,246,0.3), 0 1px 0 rgba(255,255,255,0.1) inset',
                }}
              >
                {/* Hover shimmer */}
                <div
                  className="absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-300"
                  style={{
                    background:
                      'linear-gradient(135deg, rgba(255,255,255,0.12) 0%, transparent 50%, rgba(255,255,255,0.06) 100%)',
                  }}
                />
                <span className="relative">{t('common.save')}</span>
              </button>

              {/* Secondary row */}
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={() => onResult('discard')}
                  className="group h-[34px] flex-1 rounded-[10px] text-[11.5px] font-medium transition-all duration-200 active:scale-[0.98]"
                  style={{
                    background: L ? 'rgba(239,68,68,0.05)' : 'rgba(239,68,68,0.08)',
                    color: L ? 'rgba(220,38,38,0.8)' : 'rgba(248,113,113,0.85)',
                    border: `1px solid ${L ? 'rgba(239,68,68,0.1)' : 'rgba(239,68,68,0.12)'}`,
                  }}
                >
                  {t('unsaved.dontSave')}
                </button>
                <button
                  type="button"
                  onClick={() => onResult('cancel')}
                  className="group h-[34px] flex-1 rounded-[10px] text-[11.5px] font-medium transition-all duration-200 active:scale-[0.98]"
                  style={{
                    background: L ? 'rgba(0,0,0,0.03)' : 'rgba(255,255,255,0.05)',
                    color: L ? 'rgba(0,0,0,0.4)' : 'rgba(255,255,255,0.45)',
                    border: `1px solid ${L ? 'rgba(0,0,0,0.06)' : 'rgba(255,255,255,0.07)'}`,
                  }}
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
        @keyframes glowBreath {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.6; }
        }
        @keyframes iconPulse {
          0%, 100% { transform: scale(1); opacity: 1; }
          50% { transform: scale(1.3); opacity: 0.5; }
        }
      `}</style>
    </div>
  );
}
