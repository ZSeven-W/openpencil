import type { CubicBezier, Easing, EasingPresetV2 } from '@/types/animation'

// Easing preset name → bezier control points
export const EASING_PRESETS: Record<EasingPresetV2, CubicBezier> = {
  linear: [0, 0, 1, 1],
  ease: [0.25, 0.1, 0.25, 1],
  easeIn: [0.42, 0, 1, 1],
  easeOut: [0, 0, 0.58, 1],
  easeInOut: [0.42, 0, 0.58, 1],
  snappy: [0.2, 0, 0, 1],
  bouncy: [0.34, 1.56, 0.64, 1],
  gentle: [0.4, 0, 0.2, 1],
  smooth: [0.25, 0.1, 0.25, 1],
}

// Cache for created easing functions
const easingCache = new Map<string, (t: number) => number>()

/**
 * Create an easing function from cubic bezier control points.
 * Algorithm from bezier-easing (MIT, used by Chrome/Firefox):
 * 1. Pre-compute 11 sample points
 * 2. Newton-Raphson iteration (4 max)
 * 3. Binary subdivision fallback (10 max iterations, 1e-7 precision)
 */
export function createBezierEasing(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): (t: number) => number {
  // Implementation constants
  const NEWTON_ITERATIONS = 4
  const NEWTON_MIN_SLOPE = 0.001
  const SUBDIVISION_PRECISION = 0.0000001
  const SUBDIVISION_MAX_ITERATIONS = 10
  const kSplineTableSize = 11
  const kSampleStepSize = 1.0 / (kSplineTableSize - 1.0)

  // Linear shortcut
  if (x1 === y1 && x2 === y2) return (t) => t

  // Clamp x to [0,1]
  x1 = Math.max(0, Math.min(1, x1))
  x2 = Math.max(0, Math.min(1, x2))

  // Polynomial coefficients
  const aX = 1 - 3 * x2 + 3 * x1
  const bX = 3 * x2 - 6 * x1
  const cX = 3 * x1
  const aY = 1 - 3 * y2 + 3 * y1
  const bY = 3 * y2 - 6 * y1
  const cY = 3 * y1

  function calcBezier(t: number, a: number, b: number, c: number): number {
    return ((a * t + b) * t + c) * t
  }

  function getSlope(t: number, a: number, b: number, c: number): number {
    return 3 * a * t * t + 2 * b * t + c
  }

  // Pre-compute sample table
  const sampleValues = new Float32Array(kSplineTableSize)
  for (let i = 0; i < kSplineTableSize; ++i) {
    sampleValues[i] = calcBezier(i * kSampleStepSize, aX, bX, cX)
  }

  function binarySubdivide(x: number, a: number, b: number): number {
    let currentX: number
    let currentT: number
    let i = 0
    do {
      currentT = a + (b - a) / 2
      currentX = calcBezier(currentT, aX, bX, cX) - x
      if (currentX > 0) b = currentT
      else a = currentT
    } while (
      Math.abs(currentX) > SUBDIVISION_PRECISION &&
      ++i < SUBDIVISION_MAX_ITERATIONS
    )
    return currentT
  }

  function newtonRaphsonIterate(x: number, guessT: number): number {
    for (let i = 0; i < NEWTON_ITERATIONS; ++i) {
      const slope = getSlope(guessT, aX, bX, cX)
      if (slope === 0) return guessT
      const currentX = calcBezier(guessT, aX, bX, cX) - x
      guessT -= currentX / slope
    }
    return guessT
  }

  function getTForX(x: number): number {
    // Find interval in sample table
    let intervalStart = 0
    let currentSample = 1
    const lastSample = kSplineTableSize - 1

    for (
      ;
      currentSample !== lastSample && sampleValues[currentSample] <= x;
      ++currentSample
    ) {
      intervalStart += kSampleStepSize
    }
    --currentSample

    // Interpolate to provide initial guess
    const dist =
      (x - sampleValues[currentSample]) /
      (sampleValues[currentSample + 1] - sampleValues[currentSample])
    const guessForT = intervalStart + dist * kSampleStepSize
    const initialSlope = getSlope(guessForT, aX, bX, cX)

    if (initialSlope >= NEWTON_MIN_SLOPE) {
      return newtonRaphsonIterate(x, guessForT)
    } else if (initialSlope === 0) {
      return guessForT
    } else {
      return binarySubdivide(x, intervalStart, intervalStart + kSampleStepSize)
    }
  }

  return (x: number): number => {
    if (x === 0) return 0
    if (x === 1) return 1
    return calcBezier(getTForX(x), aY, bY, cY)
  }
}

/**
 * Resolve an Easing to a function. Caches by preset name or bezier string.
 */
export function resolveEasing(easing: Easing): (t: number) => number {
  if (typeof easing === 'string') {
    const cached = easingCache.get(easing)
    if (cached) return cached
    const bezier = EASING_PRESETS[easing]
    if (!bezier) return (t) => t // fallback to linear
    const fn = createBezierEasing(...bezier)
    easingCache.set(easing, fn)
    return fn
  }

  const key = easing.join(',')
  const cached = easingCache.get(key)
  if (cached) return cached
  const fn = createBezierEasing(...easing)
  easingCache.set(key, fn)
  return fn
}
