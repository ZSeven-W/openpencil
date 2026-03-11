import { useState, useEffect, useCallback } from 'react'
import { Globe, Palette, ArrowRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { STARTER_KITS, getStarterKitById } from '@/vibekit/starter-kits'
import { applyKit } from '@/vibekit/kit-applicator'
import { useVibeKitStore } from '@/stores/vibekit-store'
import ExtractionPreview from './extraction-preview'

type OnboardingPath = 'choose' | 'extract' | 'starter'

interface OnboardingModalProps {
  open: boolean
  onClose: () => void
}

/** Color keys to show as swatch previews in each starter kit card. */
const SWATCH_KEYS = ['color-primary', 'color-secondary', 'color-accent', 'color-bg'] as const

export default function OnboardingModal({ open, onClose }: OnboardingModalProps) {
  const [path, setPath] = useState<OnboardingPath>('choose')

  // Reset when dialog opens
  useEffect(() => {
    if (open) setPath('choose')
  }, [open])

  // Escape key to close
  useEffect(() => {
    if (!open) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (path !== 'choose') {
          setPath('choose')
        } else {
          onClose()
        }
      }
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [open, path, onClose])

  const handlePickKit = useCallback(
    (kitId: string) => {
      const kit = getStarterKitById(kitId)
      if (!kit) return
      applyKit(kit)
      useVibeKitStore.getState().saveKit(kit)
      onClose()
    },
    [onClose],
  )

  if (!open) return null

  // Extraction path — render ExtractionPreview inline
  if (path === 'extract') {
    return (
      <ExtractionPreview
        open
        onClose={() => {
          // If extraction completes, the preview calls onClose which closes itself.
          // We check if a kit was applied — if so, close onboarding too.
          const activeKit = useVibeKitStore.getState().getActiveKit()
          if (activeKit) {
            onClose()
          } else {
            setPath('choose')
          }
        }}
      />
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />
      <div className="relative bg-card border border-border rounded-2xl shadow-2xl p-6 w-full max-w-lg">
        {/* Header */}
        <div className="text-center mb-6">
          <h2 className="text-base font-semibold text-foreground">Choose your style</h2>
          <p className="text-xs text-muted-foreground mt-1">
            Set your brand tokens to get started
          </p>
        </div>

        {/* Path chooser */}
        {path === 'choose' && (
          <div className="grid grid-cols-2 gap-3 mb-4">
            <button
              className="flex flex-col items-center gap-3 p-5 rounded-xl border border-border bg-secondary/50 hover:bg-secondary transition-colors text-center"
              onClick={() => setPath('extract')}
            >
              <Globe size={24} className="text-primary" />
              <div>
                <p className="text-sm font-medium text-foreground">Extract from website</p>
                <p className="text-[11px] text-muted-foreground mt-1">
                  Pull colors and fonts from a URL
                </p>
              </div>
              <ArrowRight size={14} className="text-muted-foreground" />
            </button>
            <button
              className="flex flex-col items-center gap-3 p-5 rounded-xl border border-border bg-secondary/50 hover:bg-secondary transition-colors text-center"
              onClick={() => setPath('starter')}
            >
              <Palette size={24} className="text-primary" />
              <div>
                <p className="text-sm font-medium text-foreground">Pick a style</p>
                <p className="text-[11px] text-muted-foreground mt-1">
                  Start with a pre-built kit
                </p>
              </div>
              <ArrowRight size={14} className="text-muted-foreground" />
            </button>
          </div>
        )}

        {/* Starter kit grid */}
        {path === 'starter' && (
          <div>
            <button
              className="text-xs text-muted-foreground hover:text-foreground mb-3 flex items-center gap-1"
              onClick={() => setPath('choose')}
            >
              <ArrowRight size={12} className="rotate-180" />
              Back
            </button>
            <div className="grid grid-cols-2 gap-3">
              {STARTER_KITS.map((kit) => {
                const headingFont = kit.variables['font-heading']
                const headingFontName = headingFont
                  ? String(headingFont.value).split(',')[0]
                  : 'Inter'

                return (
                  <button
                    key={kit.id}
                    className="p-4 rounded-xl border border-border bg-secondary/30 hover:bg-secondary/60 transition-colors text-left"
                    onClick={() => handlePickKit(kit.id)}
                  >
                    {/* Kit name */}
                    <p className="text-sm font-medium text-foreground mb-2">{kit.name}</p>

                    {/* Color swatches */}
                    <div className="flex gap-1 mb-2">
                      {SWATCH_KEYS.map((key) => {
                        const varDef = kit.variables[key]
                        const color = varDef ? String(varDef.value) : '#ccc'
                        return (
                          <div
                            key={key}
                            className="w-5 h-5 rounded-sm border border-border"
                            style={{ backgroundColor: color }}
                          />
                        )
                      })}
                    </div>

                    {/* Font preview */}
                    <p
                      className="text-[11px] text-muted-foreground truncate"
                      style={{ fontFamily: String(headingFont?.value ?? 'Inter') }}
                    >
                      {headingFontName}
                    </p>
                  </button>
                )
              })}
            </div>
          </div>
        )}

        {/* Skip */}
        {path === 'choose' && (
          <div className="text-center">
            <Button variant="ghost" size="sm" onClick={onClose}>
              Skip for now
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}
