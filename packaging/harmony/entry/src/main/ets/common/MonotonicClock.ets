import { systemDateTime } from '@kit.BasicServicesKit';

/**
 * Shared monotonic engine clock (milliseconds since boot, ACTIVE uptime
 * domain).
 *
 * Every `op_frame` / `op_background_tick` time and every synthetic Cancel
 * must come from THIS clock, never `Date.now()`: ArkUI `TouchEvent.timestamp`
 * is boot-uptime nanoseconds, so the engine's global monotonic clock and the
 * raw touch-event timeline must share one domain — an epoch-based frame
 * pump would jump the clock by the boot offset the moment the first touch
 * arrived, and velocity-sensing recognizers (Swipe) would measure garbage
 * deltas.
 */
export class MonotonicClock {
  /** Milliseconds since boot (ACTIVE). Never wall-clock. */
  static nowMs(): number {
    return systemDateTime.getUptime(systemDateTime.TimeType.ACTIVE, false);
  }
}
