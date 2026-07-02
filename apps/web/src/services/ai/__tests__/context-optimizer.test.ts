import { describe, expect, it } from 'vitest';
import {
  DEFAULT_MAX_CHARS,
  estimateChatPayloadChars,
  formatChatPayloadTooLargeError,
  MAX_CHAT_REQUEST_CHARS,
  trimChatHistory,
} from '../context-optimizer';

describe('context-optimizer', () => {
  it('removes attachments from historical messages but keeps latest attachment message', () => {
    const messages = [
      {
        role: 'user',
        content: 'first image',
        attachments: [{ name: 'old.png', mediaType: 'image/png', data: 'a'.repeat(1000) }],
      },
      {
        role: 'assistant',
        content: 'looks good',
      },
      {
        role: 'user',
        content: 'second image',
        attachments: [{ name: 'new.png', mediaType: 'image/png', data: 'b'.repeat(2000) }],
      },
    ];

    const trimmed = trimChatHistory(messages);

    expect(trimmed[0]).not.toHaveProperty('attachments');
    expect(trimmed[2].attachments?.[0]?.data.length).toBe(2000);
  });

  it('still truncates oversized text content to the configured char limit', () => {
    const trimmed = trimChatHistory([
      {
        role: 'user',
        content: 'x'.repeat(DEFAULT_MAX_CHARS + 1000),
        attachments: [{ name: 'big.png', mediaType: 'image/png', data: 'b'.repeat(5000) }],
      },
    ]);

    expect(trimmed).toHaveLength(1);
    expect(trimmed[0].content.length).toBeLessThanOrEqual(
      DEFAULT_MAX_CHARS + '[...truncated...]'.length + 2,
    );
    expect(trimmed[0].attachments?.[0]?.data.length).toBe(5000);
  });

  it('estimates payload size from serialized chat body', () => {
    const payloadChars = estimateChatPayloadChars({
      system: 'system',
      messages: [
        {
          role: 'user',
          content: 'hello',
          attachments: [{ name: 'x.png', mediaType: 'image/png', data: 'c'.repeat(4000) }],
        },
      ],
    });

    expect(payloadChars).toBeGreaterThan(4000);
  });

  it('formats actionable payload-too-large error text', () => {
    const message = formatChatPayloadTooLargeError(MAX_CHAT_REQUEST_CHARS + 12345);

    expect(message).toContain('AI input is too large');
    expect(message).toContain('start a new chat');
  });
});
