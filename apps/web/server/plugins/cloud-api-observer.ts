const DEFAULT_SLOW_API_MS = 800;

function getSlowApiThresholdMs(): number {
  const parsed = Number(process.env.OPENPENCIL_CLOUD_API_SLOW_MS);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : DEFAULT_SLOW_API_MS;
}

function isCloudApiPath(pathname: string): boolean {
  return pathname === '/api/cloud' || pathname.startsWith('/api/cloud/');
}

export default (nitro: {
  hooks: {
    hook: (name: 'request' | 'response', handler: (...args: any[]) => void) => void;
  };
}) => {
  const startedAtByRequest = new WeakMap<Request, number>();

  nitro.hooks.hook('request', (event) => {
    if (!isCloudApiPath(event.url.pathname)) return;
    startedAtByRequest.set(event.req, Date.now());
  });

  nitro.hooks.hook('response', (response, event) => {
    if (!isCloudApiPath(event.url.pathname)) return;

    const startedAt = startedAtByRequest.get(event.req);
    if (!startedAt) return;

    const durationMs = Date.now() - startedAt;
    if (durationMs < getSlowApiThresholdMs()) return;

    const referer = event.req.headers.get('referer') ?? '-';
    console.warn(
      [
        '[cloud-api-slow]',
        `method=${event.req.method}`,
        `status=${response.status}`,
        `durationMs=${durationMs}`,
        `path=${event.url.pathname}${event.url.search}`,
        `referer=${referer}`,
      ].join(' '),
    );
  });
};
