export function startSSEKeepAlive(
  send: () => void,
  intervalMs: number,
): ReturnType<typeof setInterval> {
  const tick = () => {
    try {
      send();
    } catch {
      /* 流已经关闭 */
    }
  };

  tick();
  return setInterval(tick, intervalMs);
}
