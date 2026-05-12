import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { CloudEditorPage as CloudEditor } from '@/components/cloud/cloud-editor-page';

export const Route = createFileRoute('/editor/$fileId')({
  component: CloudEditorRoutePage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Editor' }],
  }),
});

function CloudEditorRoutePage() {
  const { fileId } = Route.useParams();
  const navigate = useNavigate();
  return <CloudEditor fileId={fileId} navigate={navigate} />;
}
