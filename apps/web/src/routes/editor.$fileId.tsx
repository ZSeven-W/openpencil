import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { z } from 'zod';
import { CloudEditorPage as CloudEditor } from '@/components/cloud/cloud-editor-page';

export const Route = createFileRoute('/editor/$fileId')({
  validateSearch: z.object({
    generationId: z.string().optional(),
  }),
  component: CloudEditorRoutePage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Editor' }],
  }),
});

function CloudEditorRoutePage() {
  const { fileId } = Route.useParams();
  const { generationId } = Route.useSearch();
  const navigate = useNavigate();
  return <CloudEditor fileId={fileId} generationId={generationId} navigate={navigate} />;
}
