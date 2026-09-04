/**
 * Converts consecutive finger distances into the wheel delta consumed by the
 * editor ABI. The engine applies `exp(delta * 0.0015)`, so this preserves the
 * platform pinch ratio instead of depending on distances measured in pixels.
 *
 * Byte-for-byte the same maths as `PinchZoomDelta.kt` / `PinchZoomDelta.swift`.
 */
export class PinchZoomDelta {
  private static readonly ZOOM_EXPONENT_PER_WHEEL_UNIT: number = 0.0015;
  private static readonly MINIMUM_DISTANCE: number = 0.001;

  static wheelDelta(previousDistance: number, currentDistance: number): number {
    if (
      !Number.isFinite(previousDistance) ||
      !Number.isFinite(currentDistance) ||
      previousDistance <= PinchZoomDelta.MINIMUM_DISTANCE ||
      currentDistance <= PinchZoomDelta.MINIMUM_DISTANCE
    ) {
      return 0;
    }

    const ratio: number = currentDistance / previousDistance;
    if (!Number.isFinite(ratio) || ratio <= 0) {
      return 0;
    }
    const delta: number = Math.log(ratio) / PinchZoomDelta.ZOOM_EXPONENT_PER_WHEEL_UNIT;
    return Number.isFinite(delta) ? delta : 0;
  }
}
