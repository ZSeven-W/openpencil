import { createFileRoute } from '@tanstack/react-router';
import { AppEntryPage } from '@/components/cloud/app-entry-page';

export const Route = createFileRoute('/')({
  component: AppEntryPage,
  head: () => ({
    meta: [{ title: 'OpenPencil - Design as Code' }],
  }),
});
