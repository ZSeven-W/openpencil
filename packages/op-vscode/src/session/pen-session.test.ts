import { test, expect } from "bun:test";
import type { BridgeOutboundToPage } from "../protocol/bridge";
import {
  CancelledError,
  ConflictPendingError,
  DisposedError,
  NotReadyError,
  PenSession,
  type SessionHost,
} from "./pen-session";

// ---- MockHost: records outbound messages + host effects, with a manual timer
//      queue and controllable dialog/persist outcomes. ----

interface ScheduledTimer {
  fn: () => void;
  ms: number;
  cancelled: boolean;
}

class MockHost implements SessionHost {
  posted: BridgeOutboundToPage[] = [];
  contentChangedCount = 0;
  writes: Uint8Array[] = [];
  backups: { name: string; bytes: Uint8Array }[] = [];
  fallbacks: Uint8Array[] = [];
  warnings: string[] = [];
  timers: ScheduledTimer[] = [];

  // Controllable outcomes:
  writeFileImpl: (bytes: Uint8Array) => Promise<void> = async (b) => {
    this.writes.push(b);
  };
  writeBackupImpl: (name: string, bytes: Uint8Array) => Promise<void> = async (name, bytes) => {
    this.backups.push({ name, bytes });
  };
  writeBackupFallbackImpl: (bytes: Uint8Array) => Promise<string> = async (b) => {
    this.fallbacks.push(b);
    return "/tmp/fallback";
  };
  conflictChoices: Array<"use-local" | "accept-remote" | undefined> = [];
  externalChoices: Array<"reload" | "keep-local" | "save-disk-copy" | undefined> = [];

  postToPage(msg: BridgeOutboundToPage): void {
    this.posted.push(msg);
  }
  contentChanged(): void {
    this.contentChangedCount += 1;
  }
  writeFile(bytes: Uint8Array): Promise<void> {
    return this.writeFileImpl(bytes);
  }
  writeBackup(name: string, bytes: Uint8Array): Promise<void> {
    return this.writeBackupImpl(name, bytes);
  }
  writeBackupFallback(bytes: Uint8Array): Promise<string> {
    return this.writeBackupFallbackImpl(bytes);
  }
  async showConflictDialog(): Promise<"use-local" | "accept-remote" | undefined> {
    return this.conflictChoices.shift();
  }
  async showExternalChangeDialog(): Promise<"reload" | "keep-local" | "save-disk-copy" | undefined> {
    return this.externalChoices.shift();
  }
  schedule(fn: () => void, ms: number): () => void {
    const t: ScheduledTimer = { fn, ms, cancelled: false };
    this.timers.push(t);
    return () => {
      t.cancelled = true;
    };
  }
  warn(message: string): void {
    this.warnings.push(message);
  }

  // Fire the most recently scheduled non-cancelled timer.
  fireLatestTimer(): void {
    for (let i = this.timers.length - 1; i >= 0; i--) {
      if (!this.timers[i].cancelled) {
        this.timers[i].cancelled = true;
        this.timers[i].fn();
        return;
      }
    }
  }
  lastPosted(): BridgeOutboundToPage {
    return this.posted[this.posted.length - 1];
  }
  postedOfType<T extends BridgeOutboundToPage["type"]>(
    type: T,
  ): Extract<BridgeOutboundToPage, { type: T }>[] {
    return this.posted.filter((m) => m.type === type) as Extract<
      BridgeOutboundToPage,
      { type: T }
    >[];
  }
}

const flush = () => new Promise((r) => setTimeout(r, 0));

/** Build a started session and bring it to `ready` (init → ready → opened). */
async function readySession(host = new MockHost()): Promise<{ host: MockHost; s: PenSession }> {
  const s = new PenSession(host, "tok", '{"boot":true}');
  s.start();
  s.onPageMessage(JSON.stringify({ type: "op-bridge/ready", generation: 1, revision: 0 }));
  await flush();
  // boot open sends open-document; complete it with `opened`.
  s.onPageMessage(JSON.stringify({ type: "op-bridge/opened", generation: 1 }));
  await flush();
  return { host, s };
}

function sendResult(s: PenSession, requestId: string, docJson: string, gen = 2, rev = 1): void {
  s.onPageMessage(
    JSON.stringify({
      type: "op-bridge/snapshot-result",
      requestId,
      docJson,
      generation: gen,
      revision: rev,
    }),
  );
}

test("start posts init and retries until ready, then cancels the timer", async () => {
  const host = new MockHost();
  const s = new PenSession(host, "tok", "{}");
  s.start();
  expect(host.postedOfType("op-bridge/init").length).toBe(1);
  host.fireLatestTimer(); // retry once → second init
  expect(host.postedOfType("op-bridge/init").length).toBe(2);
  s.onPageMessage(JSON.stringify({ type: "op-bridge/ready", generation: 1, revision: 0 }));
  await flush();
  // After ready, firing any leftover timer must not post another init.
  const initsBefore = host.postedOfType("op-bridge/init").length;
  host.fireLatestTimer();
  expect(host.postedOfType("op-bridge/init").length).toBe(initsBefore);
});

test("listening restarts init after the original retry budget expires", () => {
  const host = new MockHost();
  const s = new PenSession(host, "tok", "{}");
  s.start();
  for (let i = 0; i < 20; i++) host.fireLatestTimer();
  expect(host.warnings).toEqual(["OpenPencil editor did not become ready"]);
  const beforeListening = host.postedOfType("op-bridge/init").length;

  s.onPageMessage(JSON.stringify({ type: "op-bridge/listening" }));

  expect(host.postedOfType("op-bridge/init").length).toBe(beforeListening + 1);
  host.fireLatestTimer();
  expect(host.postedOfType("op-bridge/init").length).toBe(beforeListening + 2);
});

test("late listening does not reinitialize a ready or disposed session", async () => {
  const { host, s } = await readySession();
  const readyCount = host.postedOfType("op-bridge/init").length;
  s.onPageMessage(JSON.stringify({ type: "op-bridge/listening" }));
  expect(host.postedOfType("op-bridge/init").length).toBe(readyCount);

  s.dispose();
  s.onPageMessage(JSON.stringify({ type: "op-bridge/listening" }));
  expect(host.postedOfType("op-bridge/init").length).toBe(readyCount);
});

test("boot sends open-document and reaches ready on opened", async () => {
  const { host, s } = await readySession();
  const opens = host.postedOfType("op-bridge/open-document");
  expect(opens.length).toBe(1);
  expect(opens[0].json).toBe('{"boot":true}');
  expect(s.state).toBe("ready");
});

test("save/backup/revert reject before ready", async () => {
  const host = new MockHost();
  const s = new PenSession(host, "tok", "{}");
  s.start();
  await expect(s.save()).rejects.toBeInstanceOf(NotReadyError);
  await expect(s.backup()).rejects.toBeInstanceOf(NotReadyError);
  await expect(s.revert("{}")).rejects.toBeInstanceOf(NotReadyError);
});

test("save happy path emits save-committed with the snapshot-result pair", async () => {
  const { host, s } = await readySession();
  const p = s.save();
  await flush();
  const snap = host.postedOfType("op-bridge/snapshot")[0];
  expect(snap.purpose).toBe("save");
  sendResult(s, snap.requestId, '{"saved":1}', 9, 4);
  await p;
  expect(new TextDecoder().decode(host.writes[0])).toBe('{"saved":1}');
  const committed = host.postedOfType("op-bridge/save-committed")[0];
  expect(committed.generation).toBe(9);
  expect(committed.revision).toBe(4);
});

test("writeFile failure rejects save and does NOT emit save-committed", async () => {
  const { host, s } = await readySession();
  host.writeFileImpl = async () => {
    throw new Error("disk full");
  };
  const p = s.save();
  await flush();
  const snap = host.postedOfType("op-bridge/snapshot")[0];
  sendResult(s, snap.requestId, "{}");
  await expect(p).rejects.toThrow("disk full");
  expect(host.postedOfType("op-bridge/save-committed").length).toBe(0);
});

test("two saves serialize (second snapshot only after first resolves)", async () => {
  const { host, s } = await readySession();
  const p1 = s.save();
  const p2 = s.save();
  await flush();
  expect(host.postedOfType("op-bridge/snapshot").length).toBe(1); // serialized
  const snap1 = host.postedOfType("op-bridge/snapshot")[0];
  sendResult(s, snap1.requestId, "{}");
  await p1;
  await flush();
  expect(host.postedOfType("op-bridge/snapshot").length).toBe(2);
  const snap2 = host.postedOfType("op-bridge/snapshot")[1];
  sendResult(s, snap2.requestId, "{}");
  await p2;
});

test("snapshot-conflict for a save opens the conflict and rejects the save", async () => {
  const { host, s } = await readySession();
  host.conflictChoices = ["use-local"];
  const p = s.save();
  await flush();
  const snap = host.postedOfType("op-bridge/snapshot")[0];
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/snapshot-conflict", requestId: snap.requestId, serverVersion: 5 }),
  );
  await expect(p).rejects.toBeInstanceOf(ConflictPendingError);
  expect(s.state).toBe("conflict");
});

test("already-cancelled queued item rejects with Cancelled when it dequeues", async () => {
  const { host, s } = await readySession();
  // Block the queue with a first save, then enqueue a cancelled second.
  const p1 = s.save();
  let cancelled = false;
  const p2 = s.save(() => cancelled);
  cancelled = true;
  await flush();
  const snap1 = host.postedOfType("op-bridge/snapshot")[0];
  sendResult(s, snap1.requestId, "{}");
  await p1;
  await expect(p2).rejects.toBeInstanceOf(CancelledError);
});

test("dispose rejects in-flight and later calls", async () => {
  const { s } = await readySession();
  const inflight = s.save(); // snapshot posted, awaiting result
  await flush();
  s.dispose();
  await expect(inflight).rejects.toBeInstanceOf(DisposedError);
  expect(s.state).toBe("disposed");
  await expect(s.save()).rejects.toBeInstanceOf(DisposedError);
});

test("sync-conflict → use-local → resolved returns to ready (steady state)", async () => {
  const { host, s } = await readySession();
  host.conflictChoices = ["use-local"];
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/sync-conflict", generation: 1, revision: 0, serverVersion: 3 }),
  );
  await flush();
  const resolve = host.postedOfType("op-bridge/resolve-conflict")[0];
  expect(resolve.mode).toBe("use-local");
  s.onPageMessage(JSON.stringify({ type: "op-bridge/conflict-resolved", requestId: resolve.requestId }));
  await flush();
  expect(s.state).toBe("ready");
});

test("accept-remote buffers early conflict-resolved, persists before consuming, waits opened", async () => {
  // Force the conflict to arise during an open flow so completion waits `opened`.
  const host = new MockHost();
  const s = new PenSession(host, "tok", "{}");
  s.start();
  s.onPageMessage(JSON.stringify({ type: "op-bridge/ready", generation: 1, revision: 0 }));
  await flush();
  // Do NOT send the boot `opened` yet — state is open-pending. Now a conflict.
  host.conflictChoices = ["accept-remote"];
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/sync-conflict", generation: 1, revision: 0, serverVersion: 7 }),
  );
  await flush();
  const resolve = host.postedOfType("op-bridge/resolve-conflict")[0];
  expect(resolve.mode).toBe("accept-remote");
  // conflict-resolved arrives EARLY (before we persist) — must be buffered.
  s.onPageMessage(JSON.stringify({ type: "op-bridge/conflict-resolved", requestId: resolve.requestId }));
  // now the local bytes arrive; persist must happen before completion.
  sendResult(s, resolve.requestId, '{"local":1}');
  await flush();
  expect(host.backups.length).toBe(1); // persisted local bytes
  expect(s.state).toBe("conflict"); // still waiting for opened
  s.onPageMessage(JSON.stringify({ type: "op-bridge/opened", generation: 2 }));
  await flush();
  expect(s.state).toBe("ready");
});

test("accept-remote persist: writeBackup fails twice → fallback blocks then completes (steady state)", async () => {
  const { host, s } = await readySession();
  let attempts = 0;
  host.writeBackupImpl = async () => {
    attempts += 1;
    throw new Error("backup fail");
  };
  host.conflictChoices = ["accept-remote"];
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/sync-conflict", generation: 5, revision: 0, serverVersion: 9 }),
  );
  await flush();
  const resolve = host.postedOfType("op-bridge/resolve-conflict")[0];
  sendResult(s, resolve.requestId, '{"local":2}', 5, 0);
  s.onPageMessage(JSON.stringify({ type: "op-bridge/conflict-resolved", requestId: resolve.requestId }));
  await flush();
  expect(attempts).toBe(2); // tried twice
  expect(host.fallbacks.length).toBe(1); // then fallback
  // steady-state completion: dirty-changed with generation > 5.
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/dirty-changed", generation: 6, revision: 1, dirty: true }),
  );
  await flush();
  expect(s.state).toBe("ready");
});

test("single transaction: snapshot-conflict then sync-conflict = ONE dialog", async () => {
  const { host, s } = await readySession();
  let dialogs = 0;
  host.showConflictDialog = async () => {
    dialogs += 1;
    return undefined; // dismiss to keep it simple
  };
  const p = s.save();
  await flush();
  const snap = host.postedOfType("op-bridge/snapshot")[0];
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/snapshot-conflict", requestId: snap.requestId, serverVersion: 4 }),
  );
  await expect(p).rejects.toBeInstanceOf(ConflictPendingError);
  // A follow-up sync-conflict during the SAME open transaction must not re-prompt.
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/sync-conflict", generation: 1, revision: 0, serverVersion: 5 }),
  );
  await flush();
  expect(dialogs).toBe(1);
});

test("dismiss then save re-prompts with stored version; use-local lets the save complete", async () => {
  const { host, s } = await readySession();
  // First conflict, dismissed.
  host.conflictChoices = [undefined];
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/sync-conflict", generation: 1, revision: 0, serverVersion: 8 }),
  );
  await flush();
  expect(s.state).toBe("conflict");
  // Now save() re-prompts; choose use-local this time.
  host.conflictChoices = ["use-local"];
  const p = s.save();
  await flush();
  const resolve = host.postedOfType("op-bridge/resolve-conflict").at(-1)!;
  expect(resolve.mode).toBe("use-local");
  s.onPageMessage(JSON.stringify({ type: "op-bridge/conflict-resolved", requestId: resolve.requestId }));
  await flush();
  // conflict cleared → the save proceeds; feed its snapshot result.
  const snap = host.postedOfType("op-bridge/snapshot").at(-1)!;
  sendResult(s, snap.requestId, '{"done":1}');
  await p;
  expect(s.state).toBe("ready");
  expect(host.postedOfType("op-bridge/save-committed").length).toBe(1);
});

test("externalFileChanged: clean → auto revert", async () => {
  const { host, s } = await readySession();
  const p = s.externalFileChanged('{"disk":1}', false);
  await flush();
  const open = host.postedOfType("op-bridge/open-document").at(-1)!;
  expect(open.json).toBe('{"disk":1}');
  s.onPageMessage(JSON.stringify({ type: "op-bridge/opened", generation: 3 }));
  await p;
});

test("externalFileChanged dirty + save-disk-copy persists disk bytes then keeps local", async () => {
  const { host, s } = await readySession();
  host.externalChoices = ["save-disk-copy"];
  await s.externalFileChanged('{"disk":2}', true);
  expect(host.backups.length).toBe(1);
  expect(new TextDecoder().decode(host.backups[0].bytes)).toBe('{"disk":2}');
  // no revert issued
  expect(host.postedOfType("op-bridge/open-document").length).toBe(1); // only the boot open
});

test("unknown / unmatched messages are ignored (no throw, warn on stray result)", async () => {
  const { host, s } = await readySession();
  s.onPageMessage(JSON.stringify({ type: "react-devtools" }));
  s.onPageMessage("not json");
  s.onPageMessage(
    JSON.stringify({
      type: "op-bridge/snapshot-result",
      requestId: "rX",
      docJson: "{}",
      generation: 2,
      revision: 1,
    }),
  );
  expect(host.warnings.some((w) => w.includes("unmatched snapshot-result"))).toBe(true);
  expect(s.state).toBe("ready");
});

test("dirty-changed false→true fires contentChanged once; ignored before boot", async () => {
  const host = new MockHost();
  const s = new PenSession(host, "tok", "{}");
  s.start();
  s.onPageMessage(JSON.stringify({ type: "op-bridge/ready", generation: 1, revision: 0 }));
  await flush();
  // Before the boot `opened`, dirty churn is ignored.
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/dirty-changed", generation: 1, revision: 1, dirty: true }),
  );
  expect(host.contentChangedCount).toBe(0);
  s.onPageMessage(JSON.stringify({ type: "op-bridge/opened", generation: 1 }));
  await flush();
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/dirty-changed", generation: 2, revision: 2, dirty: true }),
  );
  expect(host.contentChangedCount).toBe(1);
  expect(s.isRustDirty).toBe(true);
  // A second dirty:true (no edge) does not re-fire.
  s.onPageMessage(
    JSON.stringify({ type: "op-bridge/dirty-changed", generation: 3, revision: 3, dirty: true }),
  );
  expect(host.contentChangedCount).toBe(1);
});
