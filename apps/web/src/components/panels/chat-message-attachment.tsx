import type { ChatAttachment } from '@/services/ai/ai-types';

interface ChatMessageAttachmentsProps {
  attachments: ChatAttachment[];
}

/** Renders image attachments in a user message bubble */
export function ChatMessageAttachments({ attachments }: ChatMessageAttachmentsProps) {
  if (attachments.length === 0) return null;

  return (
    <div className="flex flex-wrap gap-1 mb-1.5">
      {attachments.map((att) => (
        <img
          key={att.id}
          src={`data:${att.mediaType};base64,${att.data}`}
          alt={att.name}
          className="max-h-20 rounded object-cover"
        />
      ))}
    </div>
  );
}
