import { useEffect } from 'react';
import { HeadContent, Outlet, Scripts, createRootRoute } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';

import '@/i18n';
import { detectLanguagePostHydration } from '@/i18n';
import appCss from '../styles.css?url';

export const Route = createRootRoute({
  head: () => ({
    meta: [
      {
        charSet: 'utf-8',
      },
      {
        name: 'viewport',
        content: 'width=device-width, initial-scale=1',
      },
      {
        title: 'OpenPencil',
      },
    ],
    links: [
      {
        rel: 'stylesheet',
        href: appCss,
      },
    ],
  }),
  component: RootComponent,
  notFoundComponent: NotFoundComponent,
  shellComponent: RootDocument,
});

function NotFoundComponent() {
  const { t } = useTranslation();
  return (
    <div className="min-h-screen flex items-center justify-center text-muted-foreground">
      <p>{t('notFound.message')}</p>
    </div>
  );
}

function RootComponent() {
  return <Outlet />;
}

function RootDocument({ children }: { children: React.ReactNode }) {
  const { i18n } = useTranslation();

  useEffect(() => {
    void detectLanguagePostHydration();
  }, []);

  return (
    <html lang={i18n.language} suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  );
}
