import { createFileRoute, Link } from '@tanstack/react-router'
import { useTranslation } from 'react-i18next'
import { PenTool, Plus } from 'lucide-react'


export const Route = createFileRoute('/')({
  component: LandingPage,
  head: () => ({
    meta: [{ title: 'OpenPencil - Design as Code' }],
  }),
})

function LandingPage() {
  const { t } = useTranslation()
  return (
    <div className="min-h-screen bg-background flex flex-col items-center justify-center text-foreground">
      <div className="text-center mb-12">
        <div className="flex items-center justify-center gap-3 mb-4">
          <PenTool size={40} className="text-primary" />
          <h1 className="text-5xl font-bold tracking-tight">
            {t('landing.open')}
            <span className="text-primary">{t('landing.pencil')}</span>
          </h1>
        </div>
        <p className="text-xl text-muted-foreground">
          {t('landing.tagline')}
        </p>
      </div>

      <div className="flex gap-4">
        <Link
          to="/editor"
          className="inline-flex items-center justify-center gap-2 h-10 rounded-md bg-primary px-8 text-sm font-medium text-primary-foreground shadow transition-colors hover:bg-primary/90"
        >
          <Plus size={18} />
          {t('landing.newDesign')}
        </Link>
      </div>

      <p className="mt-8 text-sm text-muted-foreground">
        {t('landing.shortcutHint', { key1: 'Ctrl', key2: 'N' })}
      </p>
    </div>
  )
}
