export type SSEEvent =
  | { type: 'text'; content: string }
  | { type: 'thinking'; content: string }
  | { type: 'error'; content: string }
  | { type: 'done' };

export function createSSEResponse(
  producer: (emit: (event: SSEEvent) => void, signal: AbortSignal) => Promise<void>,
): Response {
  const encoder = new TextEncoder();
  const abortController = new AbortController();

  const stream = new ReadableStream({
    async start(controller) {
      const enqueue = (raw: string) => {
        try {
          controller.enqueue(encoder.encode(raw));
        } catch {
          /* closed */
        }
      };

      const emit = (event: SSEEvent) => {
        enqueue(`data: ${JSON.stringify(event)}\n\n`);
      };

      const pingTimer = setInterval(
        () => enqueue(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`),
        5000,
      );

      try {
        await producer(emit, abortController.signal);
        emit({ type: 'done' });
      } catch (error) {
        const msg = error instanceof Error ? error.message : 'Unknown error';
        emit({ type: 'error', content: msg });
      } finally {
        clearInterval(pingTimer);
        try {
          controller.close();
        } catch {
          /* already closed */
        }
      }
    },
    cancel() {
      abortController.abort();
    },
  });

  return new Response(stream, {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    },
  });
}
