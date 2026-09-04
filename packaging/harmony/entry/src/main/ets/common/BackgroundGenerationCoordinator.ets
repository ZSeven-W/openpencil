import { backgroundTaskManager } from '@kit.BackgroundTasksKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import napi from './OpNative';
import { MonotonicClock } from './MonotonicClock';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilBackground';
const TICK_INTERVAL_MS = 100;
const REQUEST_RETRY_MS = 5000;
const NO_REQUEST = -1;

/**
 * Keeps only render-free generation services moving after the surface is
 * suspended. HarmonyOS transient tasks are finite (normally at most three
 * minutes, one minute on low battery), so expiry pauses work for foreground
 * resumption instead of pretending that generation can run indefinitely.
 */
export class BackgroundGenerationCoordinator {
  private engine: number = 0;
  private requestId: number = NO_REQUEST;
  private requestGeneration: number = 0;
  private grantDeadlineMs: number = 0;
  private retryAfterMs: number = 0;
  private tickTimer: number = -1;
  private foreground: boolean = true;
  private preparedActive: boolean = false;
  private pausedNoticePending: boolean = false;
  private destroyed: boolean = false;

  bind(engine: number): void {
    this.engine = engine;
    this.destroyed = false;
    this.foreground = true;
  }

  /** Called by the foreground shell pump as soon as generation becomes active. */
  observeForegroundWork(): void {
    if (!this.foreground || this.destroyed || this.engine === 0) {
      return;
    }
    if (!napi.hasBackgroundWork(this.engine)) {
      this.finishAndCancel();
      return;
    }
    if (this.grantDeadlineMs > 0 && MonotonicClock.nowMs() >= this.grantDeadlineMs) {
      this.onDelayExpiring(this.requestGeneration);
      return;
    }
    this.ensureSuspendDelay();
  }

  /** Snapshot work while still foreground; no grant is first requested here. */
  prepareForBackground(): void {
    if (!this.foreground || this.destroyed || this.engine === 0) {
      return;
    }
    this.preparedActive = napi.hasBackgroundWork(this.engine);
  }

  /** Starts the render-free timer only after the GPU surface is suspended. */
  enterBackground(): void {
    if (!this.foreground || this.destroyed || this.engine === 0) {
      return;
    }
    this.foreground = false;
    const active: boolean = this.preparedActive || napi.hasBackgroundWork(this.engine);
    this.preparedActive = false;
    if (!active) {
      this.finishAndCancel();
      return;
    }
    if (this.requestId === NO_REQUEST) {
      this.pauseUntilForeground('transient-task grant unavailable');
      return;
    }
    this.startTickTimer();
  }

  /** Resume always retires the old grant before arming a fresh foreground one. */
  enterForeground(): void {
    if (this.destroyed || this.engine === 0) {
      return;
    }
    this.foreground = true;
    this.preparedActive = false;
    this.stopTickTimer();
    this.cancelSuspendDelay();
    this.retryAfterMs = 0;
    if (napi.hasBackgroundWork(this.engine)) {
      this.ensureSuspendDelay();
    } else {
      this.pausedNoticePending = false;
    }
  }

  takePausedNotice(): boolean {
    const pending: boolean = this.pausedNoticePending;
    this.pausedNoticePending = false;
    return pending;
  }

  destroy(): void {
    this.destroyed = true;
    this.stopTickTimer();
    this.cancelSuspendDelay();
    this.engine = 0;
    this.preparedActive = false;
  }

  private ensureSuspendDelay(): void {
    const now: number = MonotonicClock.nowMs();
    if (this.requestId !== NO_REQUEST || now < this.retryAfterMs) {
      return;
    }
    const generation: number = this.requestGeneration + 1;
    this.requestGeneration = generation;
    try {
      const info: backgroundTaskManager.DelaySuspendInfo =
        backgroundTaskManager.requestSuspendDelay('Finish OpenPencil generation', () => {
          this.onDelayExpiring(generation);
        });
      if (this.destroyed || generation !== this.requestGeneration) {
        this.cancelRequestId(info.requestId);
        return;
      }
      this.requestId = info.requestId;
      this.grantDeadlineMs = now + Math.max(0, info.actualDelayTime);
      this.retryAfterMs = 0;
      hilog.info(DOMAIN, TAG, 'transient generation grant %{public}d ms', info.actualDelayTime);
    } catch (error) {
      this.retryAfterMs = now + REQUEST_RETRY_MS;
      hilog.warn(DOMAIN, TAG, 'requestSuspendDelay failed: %{public}s', String(error));
    }
  }

  private startTickTimer(): void {
    if (this.tickTimer !== -1) {
      return;
    }
    this.tickTimer = setInterval(() => this.backgroundTick(), TICK_INTERVAL_MS);
  }

  private backgroundTick(): void {
    if (this.foreground || this.destroyed || this.engine === 0) {
      this.stopTickTimer();
      return;
    }
    const now: number = MonotonicClock.nowMs();
    if (this.requestId === NO_REQUEST ||
      (this.grantDeadlineMs > 0 && now >= this.grantDeadlineMs)) {
      this.onDelayExpiring(this.requestGeneration);
      return;
    }
    if (!napi.backgroundTick(this.engine, now)) {
      this.finishAndCancel();
    }
  }

  private onDelayExpiring(generation: number): void {
    if (generation !== this.requestGeneration || this.destroyed) {
      return;
    }
    this.stopTickTimer();
    const active: boolean = this.engine !== 0 && napi.hasBackgroundWork(this.engine);
    // The system calls this only shortly before its hard deadline. Normal
    // 100 ms ticks already persist every applied revision, so release the
    // grant promptly instead of risking a tool batch or file write here.
    this.cancelSuspendDelay();
    if (!this.foreground && active) {
      this.pauseUntilForeground('transient-task time expired');
    }
  }

  private pauseUntilForeground(reason: string): void {
    this.stopTickTimer();
    this.pausedNoticePending = true;
    hilog.warn(DOMAIN, TAG, 'background generation paused: %{public}s', reason);
  }

  private finishAndCancel(): void {
    this.stopTickTimer();
    this.cancelSuspendDelay();
    this.retryAfterMs = 0;
    this.preparedActive = false;
  }

  private cancelSuspendDelay(): void {
    const requestId: number = this.requestId;
    this.requestId = NO_REQUEST;
    this.grantDeadlineMs = 0;
    if (requestId === NO_REQUEST) {
      return;
    }
    this.requestGeneration += 1;
    this.cancelRequestId(requestId);
  }

  private cancelRequestId(requestId: number): void {
    try {
      backgroundTaskManager.cancelSuspendDelay(requestId);
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'cancelSuspendDelay failed: %{public}s', String(error));
    }
  }

  private stopTickTimer(): void {
    if (this.tickTimer === -1) {
      return;
    }
    clearInterval(this.tickTimer);
    this.tickTimer = -1;
  }
}
