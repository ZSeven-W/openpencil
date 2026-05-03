/**
 * In-memory metrics for the element-tool dispatcher.
 *
 * Records how often each `add_X_v0` tool is called, plus the last
 * error message per tool if any. The counters are process-local
 * (no persistence, no networking) — meant to be queried by tests
 * and local A/B harnesses, not exposed to end users.
 *
 * Why in-memory:
 *   - We don't want to bake in a choice of persistence backend
 *     (file / sqlite / redis) at this stage
 *   - Tests can assert call counts deterministically by resetting
 *     between runs
 *   - Per-process state is the right default for a stdio MCP
 *     server (each spawned instance has its own counter, which
 *     matches the "one client, one server" model)
 *
 * If we later want persistent metrics (across server restarts),
 * add a thin serializer on top without touching this module's
 * API.
 */

export interface ElementToolMetrics {
  /** Total number of dispatches for this tool (successful or failed). */
  calls: number;
  /** Total failures (thrown from the handler). */
  errors: number;
  /** Last error message for this tool, if any. */
  lastError?: string;
  /** When the counter last fired (epoch ms). */
  lastCalledAt?: number;
}

const METRICS = new Map<string, ElementToolMetrics>();

/**
 * Record one dispatch. Called by the element-tool dispatcher
 * exactly once per incoming request. `ok` distinguishes success
 * from a thrown handler (either path counts for `calls`).
 */
export function recordElementToolCall(name: string, ok: boolean, errorMessage?: string): void {
  const existing = METRICS.get(name) ?? { calls: 0, errors: 0 };
  existing.calls += 1;
  if (!ok) {
    existing.errors += 1;
    if (errorMessage) existing.lastError = errorMessage;
  }
  existing.lastCalledAt = Date.now();
  METRICS.set(name, existing);
}

/** Read the current counter for one tool (undefined if never called). */
export function getElementToolMetric(name: string): ElementToolMetrics | undefined {
  const m = METRICS.get(name);
  // Return a copy so callers can't mutate our state by reference.
  return m ? { ...m } : undefined;
}

/** Snapshot of every tool ever called, sorted alphabetically by name. */
export function getAllElementToolMetrics(): Record<string, ElementToolMetrics> {
  const out: Record<string, ElementToolMetrics> = {};
  const names = Array.from(METRICS.keys()).sort();
  for (const n of names) {
    out[n] = { ...(METRICS.get(n) as ElementToolMetrics) };
  }
  return out;
}

/**
 * Top-N most-called tools. Ties broken by name (stable). Useful
 * for A/B harnesses that want to eyeball "what did this model
 * actually pick most often on this corpus?"
 */
export function getTopElementToolCalls(
  n: number,
): Array<{ name: string; calls: number; errors: number }> {
  const entries = Array.from(METRICS.entries()).map(([name, m]) => ({
    name,
    calls: m.calls,
    errors: m.errors,
  }));
  entries.sort((a, b) => {
    if (b.calls !== a.calls) return b.calls - a.calls;
    return a.name.localeCompare(b.name);
  });
  return entries.slice(0, Math.max(0, Math.floor(n)));
}

/**
 * Reset all counters. Tests MUST call this in `beforeEach` to
 * stay deterministic under shared process state.
 */
export function resetElementToolMetrics(): void {
  METRICS.clear();
}
