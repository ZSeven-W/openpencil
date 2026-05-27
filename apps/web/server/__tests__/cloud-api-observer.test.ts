import { beforeEach, describe, expect, it, vi } from 'vitest';

describe('cloud-api-observer plugin', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    delete process.env.OPENPENCIL_CLOUD_API_SLOW_MS;
  });

  it('logs slow cloud API responses with request context', async () => {
    process.env.OPENPENCIL_CLOUD_API_SLOW_MS = '1';
    const { default: plugin } = await import('../plugins/cloud-api-observer');
    const hooks = createHooks();

    plugin({ hooks } as any);
    const event = createEvent('https://openpencil.test/api/cloud/codegen-jobs?limit=50');
    hooks.request(event);
    await new Promise((resolve) => setTimeout(resolve, 2));
    hooks.response(new Response('{}', { status: 200 }), event);

    expect(console.warn).toHaveBeenCalledWith(
      expect.stringContaining('[cloud-api-slow] method=GET status=200'),
    );
    expect(console.warn).toHaveBeenCalledWith(
      expect.stringContaining('path=/api/cloud/codegen-jobs?limit=50'),
    );
    expect(console.warn).toHaveBeenCalledWith(
      expect.stringContaining('referer=http://localhost:3001/tasks'),
    );
  });

  it('ignores non-cloud API responses', async () => {
    process.env.OPENPENCIL_CLOUD_API_SLOW_MS = '1';
    const { default: plugin } = await import('../plugins/cloud-api-observer');
    const hooks = createHooks();

    plugin({ hooks } as any);
    const event = createEvent('https://openpencil.test/api/ai/models');
    hooks.request(event);
    await new Promise((resolve) => setTimeout(resolve, 2));
    hooks.response(new Response('{}', { status: 200 }), event);

    expect(console.warn).not.toHaveBeenCalled();
  });
});

function createHooks() {
  const handlers: Record<string, (arg1: any, arg2?: any) => void> = {};
  return {
    hook: vi.fn((name: string, handler: (arg1: any, arg2?: any) => void) => {
      handlers[name] = handler;
    }),
    request: (event: any) => handlers.request?.(event),
    response: (response: Response, event: any) => handlers.response?.(response, event),
  };
}

function createEvent(url: string) {
  const request = new Request(url, {
    headers: { referer: 'http://localhost:3001/tasks' },
  });
  return {
    req: request,
    url: new URL(url),
  };
}
