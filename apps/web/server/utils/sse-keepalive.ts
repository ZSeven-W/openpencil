export function startSSEKeepAlive(
  send: () => void,
  intervalMs: number,
): ReturnType<typeof setInterval> {
  const tick = () => {
    try {
      send();
    } catch {
      /* stream already closed */
    }
  };

  tick();
  return setInterval(tick, intervalMs);
}
