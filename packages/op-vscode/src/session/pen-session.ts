// PenSession — the document protocol state machine. Translates the postMessage
// bridge stream into VS Code custom-document lifecycle actions. No vscode
// import: every host effect goes through the injected SessionHost, so the whole
// machine is unit-testable. The rules mirror the landed Rust semantics in
// crates/op-editor-core/src/sync_gate.rs and crates/op-host-web/src/vscode_bridge.rs.

import type { BridgeInboundFromPage, BridgeOutboundToPage } from "../protocol/bridge";
import { parseInboundFromPage } from "../protocol/bridge";

export interface SessionHost {
  postToPage(msg: BridgeOutboundToPage): void;
  /** VS Code's CustomDocumentContentChangeEvent has no dirty boolean — each
   *  fire marks the doc dirty; only save/revert clears it. So the host exposes
   *  only the "became dirty" direction; the session tracks the Rust dirty bool
   *  and calls contentChanged() once on the false→true edge. (true→false is not
   *  signalled — an undo back to clean leaving the tab dirty is an accepted
   *  limitation; save/revert clears it.) */
  contentChanged(): void;
  writeFile(bytes: Uint8Array): Promise<void>;
  /** MUST throw on failure. */
  writeBackup(name: string, bytes: Uint8Array): Promise<void>;
  /** Durable fallback for the accept-remote obligation when writeBackup fails
   *  twice; only resolves once bytes are persisted somewhere findable. */
  writeBackupFallback(bytes: Uint8Array): Promise<string>;
  showConflictDialog(serverVersion: number): Promise<"use-local" | "accept-remote" | undefined>;
  showExternalChangeDialog(): Promise<"reload" | "keep-local" | "save-disk-copy" | undefined>;
  /** Timer abstraction for the init retry loop and fallbacks; returns cancel. */
  schedule(fn: () => void, ms: number): () => void;
  warn(message: string): void;
}

export type SessionState = "booting" | "ready" | "open-pending" | "conflict" | "disposed";

export class DisposedError extends Error {
  constructor() {
    super("session disposed");
  }
}
export class NotReadyError extends Error {
  constructor() {
    super("session not ready");
  }
}
export class CancelledError extends Error {
  constructor() {
    super("operation cancelled");
  }
}
export class ConflictPendingError extends Error {
  constructor() {
    super("a sync conflict must be resolved first");
  }
}

const INIT_RETRY_MS = 500;
const INIT_MAX_TRIES = 20;
const ACCEPT_REMOTE_APPLY_TIMEOUT_MS = 10_000;

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (err: unknown) => void;
}
function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (err: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

type SnapshotResult = Extract<BridgeInboundFromPage, { type: "op-bridge/snapshot-result" }>;

export class PenSession {
  private readonly host: SessionHost;
  private readonly token: string;
  private readonly initialDocJson: string;
  private readonly encoder = new TextEncoder();

  private _state: SessionState = "booting";
  private generation = 0;
  private rustDirty = false;
  private booted = false; // first `opened` seen (initial boot completed)
  private requestCounter = 0;

  private initCancel?: () => void;
  private initTries = 0;

  // Resolved by an `opened` message. Registered by boot, revert, and conflict
  // completions that arose during an open flow.
  private pendingOpen?: Deferred<void>;

  // Snapshot serialization: at most one snapshot in flight.
  private queueTail: Promise<unknown> = Promise.resolve();
  // requestId → handlers for the currently-awaited snapshot round trip.
  private readonly resultWaiters = new Map<string, (msg: SnapshotResult) => void>();
  private readonly conflictWaiters = new Map<string, (serverVersion: number) => void>();

  // The single active conflict transaction, if any.
  private conflict?: ConflictTxn;

  // Every live deferred is tracked so dispose() can reject them all.
  private readonly liveWaiters = new Set<Deferred<unknown>>();

  constructor(
    host: SessionHost,
    token: string,
    initialDocJson: string,
    /** The extension's stable MCP endpoint (McpProxy URL) — surfaced by the
     *  editor's MCP settings card instead of the daemon-internal port. */
    private readonly mcpUrl?: string,
  ) {
    this.host = host;
    this.token = token;
    this.initialDocJson = initialDocJson;
  }

  get state(): SessionState {
    return this._state;
  }
  get isRustDirty(): boolean {
    return this.rustDirty;
  }

  /** Begin the init retry loop: post init immediately and re-post every 500ms
   *  (cap 20) until `ready` arrives — postMessage success does not prove the
   *  page received it, and the Rust listener installs late in mount_ck. */
  start(): void {
    if (this._state !== "booting") return;
    this.postInit();
    this.scheduleInitRetry();
  }

  onPageMessage(raw: unknown): void {
    if (this._state === "disposed") return;
    const msg = parseInboundFromPage(raw);
    if (!msg) return;
    switch (msg.type) {
      case "op-bridge/listening":
        this.handleListening();
        break;
      case "op-bridge/ready":
        this.handleReady(msg.generation, msg.revision);
        break;
      case "op-bridge/opened":
        this.handleOpened(msg.generation);
        break;
      case "op-bridge/dirty-changed":
        this.handleDirty(msg.generation, msg.revision, msg.dirty);
        break;
      case "op-bridge/snapshot-result":
        this.handleSnapshotResult(msg);
        break;
      case "op-bridge/snapshot-conflict":
        this.handleSnapshotConflict(msg.requestId, msg.serverVersion);
        break;
      case "op-bridge/sync-conflict":
        this.handleSyncConflict(msg.generation, msg.serverVersion);
        break;
      case "op-bridge/conflict-resolved":
        this.handleConflictResolved(msg.requestId);
        break;
    }
  }

  // ---- lifecycle entry points ----

  save(isCancelled?: () => boolean): Promise<void> {
    return this.enqueueGuarded(async () => {
      if (isCancelled?.()) throw new CancelledError();
      const result = await this.runSnapshot("save");
      const bytes = this.encoder.encode(result.docJson);
      await this.host.writeFile(bytes); // throws → no save-committed
      this.post({
        type: "op-bridge/save-committed",
        generation: result.generation,
        revision: result.revision,
      });
      // A successful save is a clean baseline from VS Code's perspective.
      this.rustDirty = false;
    }, isCancelled);
  }

  backup(isCancelled?: () => boolean): Promise<Uint8Array> {
    return this.enqueueGuarded(async () => {
      if (isCancelled?.()) throw new CancelledError();
      const result = await this.runSnapshot("backup");
      return this.encoder.encode(result.docJson);
    }, isCancelled);
  }

  revert(diskJson: string): Promise<void> {
    if (this._state === "disposed") return Promise.reject(new DisposedError());
    if (!this.isReadyish()) return Promise.reject(new NotReadyError());
    return this.openDocument(diskJson).then(() => {
      this.rustDirty = false;
    });
  }

  async externalFileChanged(diskJson: string, isRustDirty: boolean): Promise<void> {
    if (this._state === "disposed") throw new DisposedError();
    if (!isRustDirty) {
      await this.revert(diskJson);
      return;
    }
    const choice = await this.host.showExternalChangeDialog();
    if (choice === "reload") {
      await this.revert(diskJson);
    } else if (choice === "save-disk-copy") {
      // Persist the DISK bytes durably before keeping the local editor, so both
      // versions survive (same durability discipline as accept-remote).
      await this.persistBytes(`disk-copy-${this.stamp()}`, this.encoder.encode(diskJson));
    }
    // keep-local / undefined → keep the editor as-is.
  }

  dispose(): void {
    if (this._state === "disposed") return;
    this._state = "disposed";
    this.initCancel?.();
    this.initCancel = undefined;
    const err = new DisposedError();
    for (const d of this.liveWaiters) d.reject(err);
    this.liveWaiters.clear();
    this.resultWaiters.clear();
    this.conflictWaiters.clear();
    this.pendingOpen = undefined;
    this.conflict = undefined;
  }

  // ---- init ----

  private postInit(): void {
    this.post({ type: "op-bridge/init", token: this.token, mcpUrl: this.mcpUrl });
  }
  private scheduleInitRetry(): void {
    this.initCancel = this.host.schedule(() => {
      if (this._state !== "booting") return;
      this.initTries += 1;
      if (this.initTries >= INIT_MAX_TRIES) {
        this.initCancel?.();
        this.initCancel = undefined;
        this.host.warn("OpenPencil editor did not become ready");
        return;
      }
      this.postInit();
      this.scheduleInitRetry();
    }, INIT_RETRY_MS);
  }

  // ---- message handlers ----

  private handleListening(): void {
    if (this._state !== "booting") return;
    // The page has installed its receive listener. Re-send init immediately,
    // even when the original finite retry burst already expired while the
    // wasm bundles were loading, and renew that bounded retry window in case
    // the first handoff races a page rebuild.
    this.initCancel?.();
    this.initCancel = undefined;
    this.initTries = 0;
    this.postInit();
    this.scheduleInitRetry();
  }

  private handleReady(generation: number, _revision: number): void {
    if (this._state !== "booting") return; // idempotent (HTML rebuild re-sends)
    this.initCancel?.();
    this.initCancel = undefined;
    this.generation = generation;
    // The host-opened bytes are authoritative — push the boot document and wait
    // for `opened` before serving lifecycle operations.
    void this.openDocument(this.initialDocJson).then(() => {
      this.booted = true;
    });
  }

  private handleOpened(generation: number): void {
    this.generation = generation;
    if (this.pendingOpen) {
      const d = this.pendingOpen;
      this.pendingOpen = undefined;
      this.untrack(d);
      d.resolve();
    }
  }

  private handleDirty(generation: number, _revision: number, dirty: boolean): void {
    this.generation = generation;
    // Steady-state accept-remote completion waits for the first dirty-changed
    // whose generation strictly exceeds the conflict-time generation.
    const c = this.conflict;
    if (c?.awaitGreaterGen !== undefined && generation > c.awaitGreaterGen.gen) {
      const d = c.awaitGreaterGen.deferred;
      c.awaitGreaterGen = undefined;
      this.untrack(d);
      d.resolve();
    }
    if (!this.booted) return; // ignore dirty churn until the initial open lands
    const was = this.rustDirty;
    this.rustDirty = dirty;
    if (!was && dirty) this.host.contentChanged();
  }

  private handleSnapshotResult(msg: SnapshotResult): void {
    this.generation = msg.generation;
    const waiter = this.resultWaiters.get(msg.requestId);
    if (!waiter) {
      this.host.warn(`dropping unmatched snapshot-result ${msg.requestId}`);
      return;
    }
    this.resultWaiters.delete(msg.requestId);
    waiter(msg);
  }

  private handleSnapshotConflict(requestId: string, serverVersion: number): void {
    const waiter = this.conflictWaiters.get(requestId);
    if (waiter) {
      this.conflictWaiters.delete(requestId);
      waiter(serverVersion);
      return;
    }
    // No registered waiter (e.g. a late conflict) — fold into the transaction.
    this.openOrUpdateConflict(serverVersion, /*aroseInOpenFlow*/ this._state === "open-pending");
  }

  private handleSyncConflict(generation: number, serverVersion: number): void {
    this.openOrUpdateConflict(serverVersion, this._state === "open-pending", generation);
  }

  private handleConflictResolved(requestId: string): void {
    const c = this.conflict;
    if (!c || c.resolveRequestId !== requestId) return;
    if (c.awaitResolved) {
      const d = c.awaitResolved;
      c.awaitResolved = undefined;
      this.untrack(d);
      d.resolve();
    } else {
      // Arrived before we were ready to consume it (accept-remote persist in
      // flight) — buffer for later.
      c.resolvedBuffered = true;
    }
  }

  // ---- snapshot round trips ----

  private runSnapshot(purpose: "save" | "backup"): Promise<SnapshotResult> {
    const requestId = this.nextRequestId();
    const result = this.track(deferred<SnapshotResult>());
    this.resultWaiters.set(requestId, (msg) => {
      this.untrack(result);
      result.resolve(msg);
    });
    this.conflictWaiters.set(requestId, (serverVersion) => {
      this.untrack(result);
      // This save/backup's own request conflicted: open the transaction and
      // reject the operation so VS Code surfaces the failed save.
      this.openOrUpdateConflict(serverVersion, this._state === "open-pending");
      result.reject(new ConflictPendingError());
    });
    this.post({ type: "op-bridge/snapshot", purpose, requestId });
    return result.promise;
  }

  // ---- open documents ----

  private openDocument(json: string): Promise<void> {
    this._state = "open-pending";
    const d = this.track(deferred<void>());
    this.pendingOpen = d;
    this.post({ type: "op-bridge/open-document", json });
    return d.promise.then(() => {
      if (this._state === "open-pending") this._state = "ready";
    });
  }

  private isReadyish(): boolean {
    return this._state === "ready";
  }

  // ---- conflict transaction ----

  private openOrUpdateConflict(
    serverVersion: number,
    aroseInOpenFlow: boolean,
    generation = this.generation,
  ): void {
    if (this.conflict) {
      // Single active transaction: a second conflict event only refreshes the
      // stored version; it does not open a second dialog.
      this.conflict.serverVersion = serverVersion;
      return;
    }
    this.conflict = {
      serverVersion,
      aroseInOpenFlow,
      conflictGeneration: generation,
      resolvedBuffered: false,
    };
    this._state = "conflict";
    void this.runConflictTransaction();
  }

  private async runConflictTransaction(): Promise<void> {
    // Loop supports use-local retry-failure re-prompts.
    for (;;) {
      const c = this.conflict;
      if (!c) return;
      const choice = await this.host.showConflictDialog(c.serverVersion);
      if (this._state === "disposed" || this.conflict !== c) return;
      if (choice === undefined) {
        // Dismissed: keep the transaction open; re-entry happens via a later
        // save()/backup() (the Rust conflict latch is consumable and is not
        // re-emitted while the gate stays conflicted).
        return;
      }
      const requestId = this.nextRequestId();
      c.resolveRequestId = requestId;
      if (choice === "use-local") {
        const retry = await this.resolveUseLocal(c, requestId);
        if (retry) continue; // retry-failure re-prompt
        this.closeConflict();
        return;
      }
      // accept-remote
      await this.resolveAcceptRemote(c, requestId);
      this.closeConflict();
      return;
    }
  }

  /** Returns true if a retry-failure snapshot-conflict re-opened the dialog. */
  private async resolveUseLocal(c: ConflictTxn, requestId: string): Promise<boolean> {
    const resolved = this.track(deferred<void>());
    const retried = this.track(deferred<number>());
    c.awaitResolved = resolved;
    if (c.resolvedBuffered) {
      c.resolvedBuffered = false;
      this.untrack(resolved);
      resolved.resolve();
    }
    this.conflictWaiters.set(requestId, (serverVersion) => {
      this.untrack(retried);
      retried.resolve(serverVersion);
    });
    this.post({ type: "op-bridge/resolve-conflict", mode: "use-local", requestId });

    const outcome = await Promise.race([
      resolved.promise.then(() => ({ kind: "resolved" as const })),
      retried.promise.then((serverVersion) => ({ kind: "retry" as const, serverVersion })),
    ]);
    this.conflictWaiters.delete(requestId);
    if (outcome.kind === "retry") {
      c.awaitResolved = undefined;
      this.untrack(resolved);
      c.serverVersion = outcome.serverVersion;
      return true;
    }
    this.untrack(retried);
    // use-local kept OUR document, so there is no remote apply to observe:
    // open-flow still waits `opened`, but steady-state completes immediately.
    await this.awaitCompletion(c, /*waitRemoteApply*/ false);
    return false;
  }

  private async resolveAcceptRemote(c: ConflictTxn, requestId: string): Promise<void> {
    // 1) Wait for the local bytes and persist them durably BEFORE consuming the
    //    (possibly-already-buffered) conflict-resolved.
    const localResult = this.track(deferred<SnapshotResult>());
    this.resultWaiters.set(requestId, (msg) => {
      this.untrack(localResult);
      localResult.resolve(msg);
    });
    this.post({ type: "op-bridge/resolve-conflict", mode: "accept-remote", requestId });
    const local = await localResult.promise;
    await this.persistBytes(`conflict-backup-${this.stamp()}`, this.encoder.encode(local.docJson));

    // 2) Consume conflict-resolved (buffered or awaited).
    if (!c.resolvedBuffered) {
      const resolved = this.track(deferred<void>());
      c.awaitResolved = resolved;
      await resolved.promise;
      c.awaitResolved = undefined;
    } else {
      c.resolvedBuffered = false;
    }

    // 3) Completion bifurcation — accept-remote applies the REMOTE document, so
    //    steady-state waits for that apply (a generation bump).
    await this.awaitCompletion(c, /*waitRemoteApply*/ true);
  }

  /** open-flow conflicts complete on `opened`. Steady-state completion depends
   *  on which side won: accept-remote must observe the remote apply (the first
   *  dirty-changed whose generation exceeds the conflict-time gen, with a
   *  schedule fallback); use-local kept our document, so it completes at once. */
  private async awaitCompletion(c: ConflictTxn, waitRemoteApply: boolean): Promise<void> {
    if (c.aroseInOpenFlow) {
      const d = this.track(deferred<void>());
      this.pendingOpen = d;
      await d.promise;
      return;
    }
    if (!waitRemoteApply) return; // use-local steady-state: done immediately
    const d = this.track(deferred<void>());
    c.awaitGreaterGen = { gen: c.conflictGeneration, deferred: d };
    const cancel = this.host.schedule(() => {
      if (c.awaitGreaterGen?.deferred === d) {
        c.awaitGreaterGen = undefined;
        this.untrack(d);
        this.host.warn("remote document apply not observed; continuing");
        d.resolve();
      }
    }, ACCEPT_REMOTE_APPLY_TIMEOUT_MS);
    try {
      await d.promise;
    } finally {
      cancel();
    }
  }

  private closeConflict(): void {
    this.conflict = undefined;
    if (this._state === "conflict") this._state = "ready";
  }

  // ---- helpers ----

  /** Persist bytes durably: writeBackup, retry once, then the blocking
   *  fallback. Never returns with the bytes only in memory. */
  private async persistBytes(name: string, bytes: Uint8Array): Promise<void> {
    try {
      await this.host.writeBackup(name, bytes);
      return;
    } catch {
      /* retry once */
    }
    try {
      await this.host.writeBackup(name, bytes);
      return;
    } catch {
      await this.host.writeBackupFallback(bytes);
    }
  }

  /** Serialize save/backup jobs; a conflict re-prompts instead of enqueuing. */
  private enqueueGuarded<T>(job: () => Promise<T>, isCancelled?: () => boolean): Promise<T> {
    if (this._state === "disposed") return Promise.reject(new DisposedError());
    if (this._state === "conflict") {
      return this.reenterConflictThen(job);
    }
    if (!this.isReadyish()) return Promise.reject(new NotReadyError());
    const run = this.queueTail.then(() => {
      if (this._state === "disposed") throw new DisposedError();
      if (isCancelled?.()) throw new CancelledError();
      return job();
    });
    this.queueTail = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  }

  /** A save/backup issued while a conflict is open re-prompts the dialog with
   *  the stored serverVersion, then proceeds (or rejects) per resolution. */
  private async reenterConflictThen<T>(job: () => Promise<T>): Promise<T> {
    const c = this.conflict;
    if (!c) return this.enqueueGuarded(job); // resolved meanwhile
    // Drive the same transaction dialog now instead of waiting for a re-emitted
    // sync-conflict (which the consumable Rust latch never sends).
    await this.driveConflictOnce(c);
    if (this._state === "disposed") throw new DisposedError();
    if (this._state === "conflict") throw new ConflictPendingError(); // dismissed again
    return this.enqueueGuarded(job);
  }

  /** Show the dialog once for an already-open transaction (re-entry path). */
  private async driveConflictOnce(c: ConflictTxn): Promise<void> {
    const choice = await this.host.showConflictDialog(c.serverVersion);
    if (this._state === "disposed" || this.conflict !== c) return;
    if (choice === undefined) return; // still dismissed
    const requestId = this.nextRequestId();
    c.resolveRequestId = requestId;
    if (choice === "use-local") {
      const retry = await this.resolveUseLocal(c, requestId);
      if (retry) return this.driveConflictOnce(c);
      this.closeConflict();
      return;
    }
    await this.resolveAcceptRemote(c, requestId);
    this.closeConflict();
  }

  private post(msg: BridgeOutboundToPage): void {
    this.host.postToPage(msg);
  }
  private nextRequestId(): string {
    this.requestCounter += 1;
    return `r${this.requestCounter}`;
  }
  private stamp(): string {
    // Monotonic, deterministic-in-tests: derived from the request counter, not
    // wall-clock, so backup names are stable and unique within a session.
    this.requestCounter += 1;
    return `${this.requestCounter}`;
  }
  private track<T>(d: Deferred<T>): Deferred<T> {
    this.liveWaiters.add(d as Deferred<unknown>);
    return d;
  }
  private untrack<T>(d: Deferred<T>): void {
    this.liveWaiters.delete(d as Deferred<unknown>);
  }
}

interface ConflictTxn {
  serverVersion: number;
  aroseInOpenFlow: boolean;
  conflictGeneration: number;
  resolveRequestId?: string;
  resolvedBuffered: boolean;
  awaitResolved?: Deferred<void>;
  awaitGreaterGen?: { gen: number; deferred: Deferred<void> };
}
