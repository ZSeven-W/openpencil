import type { StateCreator, StoreMutatorIdentifier } from 'zustand'
import type { PlaybackController } from '@/animation/playback-controller'

let playbackController: PlaybackController | null = null

export function setPlaybackControllerRef(
  controller: PlaybackController | null,
): void {
  playbackController = controller
}

type AnimationPauseMiddleware = <
  T,
  Mps extends [StoreMutatorIdentifier, unknown][] = [],
  Mcs extends [StoreMutatorIdentifier, unknown][] = [],
>(
  f: StateCreator<T, Mps, Mcs>,
) => StateCreator<T, Mps, Mcs>

type AnimationPauseMiddlewareImpl = <T>(
  f: StateCreator<T, [], []>,
) => StateCreator<T, [], []>

/**
 * Zustand middleware that auto-pauses playback before any store mutation.
 * Must be synchronous — cancels rAF before the mutation propagates.
 */
const animationPauseMiddlewareImpl: AnimationPauseMiddlewareImpl =
  (config) => (set, get, api) =>
    config(
      (...args) => {
        if (playbackController?.isPlaying()) {
          playbackController.pause()
        }
        ;(set as (...a: unknown[]) => void)(...args)
      },
      get,
      api,
    )

export const animationPauseMiddleware =
  animationPauseMiddlewareImpl as AnimationPauseMiddleware
