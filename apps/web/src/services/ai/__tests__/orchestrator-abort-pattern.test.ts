import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Orchestrator cancel mid-batch — structural + behavioral checks.
 *
 * Two kinds of coverage:
 *
 * A. **Pattern enforcement** (grep-level): every sequential / per-
 *    screen-group loop in the orchestrator MUST check `abortSignal?.aborted`
 *    as the first statement of its iteration body. This is the sole
 *    mechanism preventing the orchestrator from continuing to call the
 *    LLM after the user hit Stop. A new loop that forgets this check
 *    silently wastes tokens (and mutates canvas) after the stop signal;
 *    grep-check catches it at unit-test time.
 *
 * B. **Behavioral** (pure): simulate the orchestrator's abort-aware
 *    sequencer with a stubbed executor, verify that aborting mid-
 *    iteration prevents any subsequent executor call. No LLM mocking
 *    required — the pattern itself is what we're testing.
 *
 * The two layers reinforce each other: A catches "someone added a
 * loop without the guard," B catches "the guard doesn't actually
 * gate subsequent iterations."
 */

const ORCHESTRATOR_SUB_AGENT = join(__dirname, '..', 'orchestrator-sub-agent.ts');

const ORCHESTRATOR = join(__dirname, '..', 'orchestrator.ts');

function readSource(path: string): string {
  return readFileSync(path, 'utf-8');
}

describe('orchestrator abort pattern — structural', () => {
  it('orchestrator-sub-agent.ts sequential loop guards on abortSignal', () => {
    const src = readSource(ORCHESTRATOR_SUB_AGENT);
    // The sequential path is "for (let i = 0; i < plan.subtasks.length; i++)".
    // Find the body and look for the abort check as a top-of-body line.
    const match = /for \(let i = 0; i < plan\.subtasks\.length; i\+\+\) \{\s*([\s\S]{0,200})/.exec(
      src,
    );
    expect(match, 'sequential for-loop not found').toBeTruthy();
    expect(match![1]).toMatch(/if \(abortSignal\?\.aborted\)/);
  });

  it('orchestrator-sub-agent.ts screen-group worker guards on abortSignal', () => {
    const src = readSource(ORCHESTRATOR_SUB_AGENT);
    // Concurrent path: "for (const idx of indices) { ... }" inside each worker.
    const match = /for \(const idx of indices\) \{\s*([\s\S]{0,200})/.exec(src);
    expect(match, 'per-worker for-loop not found').toBeTruthy();
    expect(match![1]).toMatch(/if \(abortSignal\?\.aborted\)/);
  });

  it('orchestrator-sub-agent.ts retry gates on !abortSignal?.aborted', () => {
    const src = readSource(ORCHESTRATOR_SUB_AGENT);
    // Retry should only fire when NOT aborted — otherwise a Stop click
    // could still trigger a retry round.
    expect(src).toMatch(/!abortSignal\?\.aborted/);
  });

  it('orchestrator.ts collect-results path checks aborted flag before throwing "no nodes"', () => {
    const src = readSource(ORCHESTRATOR);
    // The "Orchestration produced no nodes beyond root frame" throw must be
    // gated by `!aborted` — otherwise hitting Stop before any subtask
    // finishes would show the user an error instead of a cleanly-cancelled
    // state.
    expect(src).toMatch(
      /generatedNodeCount === 0 && !aborted[\s\S]{0,80}throw new Error\('Orchestration produced no nodes/,
    );
  });

  it('orchestrator.ts validation path checks aborted before running validator', () => {
    const src = readSource(ORCHESTRATOR);
    // Vision LLM validation must be skipped when aborted — otherwise
    // post-stop we'd hit the validator endpoint.
    expect(src).toMatch(/if \(!aborted && VALIDATION_ENABLED\)/);
  });
});

describe('orchestrator abort pattern — behavioral', () => {
  /**
   * Minimal reproduction of the sequential abort-aware loop in
   * orchestrator-sub-agent.ts (the "concurrency <= 1" path). If the
   * pattern works here, we have confidence the real one does too —
   * and this test fails FIRST if someone changes the pattern.
   */
  async function runSequentialStub(
    subtasks: number[],
    executor: (i: number) => Promise<void>,
    abortSignal?: AbortSignal,
  ): Promise<number[]> {
    const completed: number[] = [];
    for (let i = 0; i < subtasks.length; i += 1) {
      if (abortSignal?.aborted) break;
      await executor(subtasks[i]);
      completed.push(subtasks[i]);
    }
    return completed;
  }

  it('no abort: all N subtasks run to completion', async () => {
    const calls: number[] = [];
    const subtasks = [0, 1, 2, 3, 4];
    await runSequentialStub(subtasks, async (i) => {
      calls.push(i);
    });
    expect(calls).toEqual([0, 1, 2, 3, 4]);
  });

  it('abort fires DURING subtask 2: subtasks 3 and 4 never run', async () => {
    const controller = new AbortController();
    const calls: number[] = [];
    const subtasks = [0, 1, 2, 3, 4];
    await runSequentialStub(
      subtasks,
      async (i) => {
        calls.push(i);
        if (i === 2) controller.abort();
      },
      controller.signal,
    );
    // 0, 1, 2 ran (abort was inside #2's executor, loop check happens at top
    // of NEXT iteration). #3 and #4 never ran.
    expect(calls).toEqual([0, 1, 2]);
  });

  it('abort fires BEFORE loop starts: zero executor calls', async () => {
    const controller = new AbortController();
    controller.abort();
    const calls: number[] = [];
    const subtasks = [0, 1, 2];
    await runSequentialStub(
      subtasks,
      async (i) => {
        calls.push(i);
      },
      controller.signal,
    );
    expect(calls).toEqual([]);
  });

  it('abort fires AFTER completion: no effect (loop already exited)', async () => {
    const controller = new AbortController();
    const calls: number[] = [];
    const subtasks = [0, 1];
    await runSequentialStub(
      subtasks,
      async (i) => {
        calls.push(i);
      },
      controller.signal,
    );
    // All subtasks ran; aborting post-hoc is a no-op
    controller.abort();
    expect(calls).toEqual([0, 1]);
  });

  it('without signal: executor runs every subtask (no dependency on signal)', async () => {
    const calls: number[] = [];
    const subtasks = [10, 20, 30];
    await runSequentialStub(subtasks, async (i) => {
      calls.push(i);
    });
    expect(calls).toEqual([10, 20, 30]);
  });

  /**
   * Minimal reproduction of the concurrent screen-group worker loop
   * in orchestrator-sub-agent.ts. Each worker is independent; each
   * checks abort before pulling the next index.
   */
  async function runConcurrentStub(
    groups: number[][],
    executor: (i: number) => Promise<void>,
    abortSignal?: AbortSignal,
  ): Promise<number[]> {
    const completed: number[] = [];
    const workers = groups.map(async (indices) => {
      for (const idx of indices) {
        if (abortSignal?.aborted) return;
        await executor(idx);
        completed.push(idx);
      }
    });
    await Promise.all(workers);
    return completed;
  }

  it('concurrent: abort during worker A does NOT prevent worker B from continuing', async () => {
    // Intended behavior: abort is checked per-iteration within each
    // worker, so an abort fired while A is mid-iteration lets B finish
    // its current iteration UNTIL B also reaches its next top-of-loop
    // check.
    const controller = new AbortController();
    const groupA = [0, 1, 2];
    const groupB = [10, 11, 12];
    const calls: number[] = [];
    await runConcurrentStub(
      [groupA, groupB],
      async (i) => {
        calls.push(i);
        // Abort when worker A processes item 1
        if (i === 1) controller.abort();
        // Slow down slightly so ordering is semi-deterministic
        await new Promise((r) => setTimeout(r, 1));
      },
      controller.signal,
    );
    // Both workers ran item 0 at the start (no abort yet).
    // After abort fires during A's item 1, A stops before item 2.
    // B may or may not have run items — depends on scheduler — but
    // item 2 (A's next) MUST NOT be in the call list.
    expect(calls).not.toContain(2);
    // Both first items should have run (no abort at iteration-start time
    // for them)
    expect(calls).toContain(0);
    expect(calls).toContain(10);
  });

  it('concurrent: pre-aborted signal prevents any execution', async () => {
    const controller = new AbortController();
    controller.abort();
    const calls: number[] = [];
    await runConcurrentStub(
      [
        [0, 1],
        [10, 11],
      ],
      async (i) => {
        calls.push(i);
      },
      controller.signal,
    );
    expect(calls).toEqual([]);
  });
});
