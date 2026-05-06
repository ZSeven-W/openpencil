import type { ChatAttachment } from '@/services/ai/ai-types';

interface ChatMessageAttachmentsProps {
  attachments: ChatAttachment[];
}

/** Renders 用户消息气泡中的图像附件 */
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
