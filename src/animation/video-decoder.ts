/**
 * MediaBunny WebCodecs video decoder.
 *
 * Replaces HTMLVideoElement pipeline with frame-by-frame WebCodecs decoding.
 * Single-clock architecture: composition RAF loop is sole timing authority.
 *
 * Key design:
 * - CanvasSink for continuous playback (pre-buffered, sequential)
 * - VideoSampleSink.getSample(t) for scrubbing (random access)
 * - advanceFrame() is SYNCHRONOUS — called from RAF loop
 * - Per-decoder AudioContext with lookahead scheduler
 * - Persistent canvas sized to display resolution, not source
 */

import {
  Input,
  ALL_FORMATS,
  BlobSource,
  CanvasSink,
  AudioBufferSink,
  VideoSampleSink,
  type WrappedCanvas,
  type WrappedAudioBuffer,
  type VideoSample,
} from 'mediabunny'

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_FILE_SIZE = 2 * 1024 * 1024 * 1024 // 2GB
const MAX_DIMENSION = 7680 // 8K
const AUDIO_LOOKAHEAD_SEC = 0.1
const AUDIO_POLL_MS = 25
const DEFAULT_DISPLAY_WIDTH = 400
const DEFAULT_DISPLAY_HEIGHT = 225

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Playback state discriminated union — no ambiguous boolean flags. */
type DecoderState =
  | { status: 'idle' }
  | { status: 'playing'; frameIterator: AsyncGenerator<WrappedCanvas, void, unknown> }
  | { status: 'disposed' }

export interface VideoDecoderHandle {
  readonly duration: number
  readonly width: number
  readonly height: number
  readonly hasAudio: boolean
  readonly canvas: HTMLCanvasElement
  get isPlaying(): boolean

  /** Seek to time (async) — for scrubbing. Uses VideoSampleSink.getSample(t). */
  drawFrame(timeSec: number): Promise<void>
  /** Begin continuous playback from time. Starts CanvasSink iterator + audio. */
  startPlayback(fromTimeSec: number): void
  /**
   * Synchronous frame advance — called from RAF loop.
   * MUST be synchronous. Returns true if frame changed.
   * Pre-fetches next frame async; swap is a sync drawImage call.
   */
  advanceFrame(compositionTimeSec: number): boolean
  /** Stop playback, cancel audio, release iterators. */
  stopPlayback(): void
  /** Resize persistent canvas (call when Fabric object is resized). */
  resizeCanvas(displayWidth: number, displayHeight: number): void
  /** Release all resources. Idempotent. */
  dispose(): void
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export async function createVideoDecoder(
  file: File,
  displayWidth = DEFAULT_DISPLAY_WIDTH,
  displayHeight = DEFAULT_DISPLAY_HEIGHT,
): Promise<VideoDecoderHandle | null> {
  // WebCodecs check
  if (typeof VideoDecoder === 'undefined') {
    console.warn('[video-decoder] WebCodecs not available')
    return null
  }

  // Input validation
  if (file.size > MAX_FILE_SIZE) {
    console.warn(`[video-decoder] File too large: ${(file.size / 1e9).toFixed(1)}GB > 2GB limit`)
    return null
  }

  let input: Input | null = null
  try {
    const source = new BlobSource(file)
    input = new Input({ source, formats: ALL_FORMATS })

    const videoTrack = await input.getPrimaryVideoTrack()
    if (!videoTrack) {
      console.warn('[video-decoder] No video track found')
      return null
    }

    if (!await videoTrack.canDecode()) {
      console.warn('[video-decoder] Codec not supported by WebCodecs')
      return null
    }

    const audioTrack = await input.getPrimaryAudioTrack()
    const audioCanDecode = audioTrack ? await audioTrack.canDecode() : false

    const duration = await input.computeDuration()
    const nativeWidth = videoTrack.displayWidth
    const nativeHeight = videoTrack.displayHeight

    // Dimension validation
    if (nativeWidth > MAX_DIMENSION || nativeHeight > MAX_DIMENSION) {
      console.warn(`[video-decoder] Dimensions too large: ${nativeWidth}x${nativeHeight} > ${MAX_DIMENSION}`)
      return null
    }

    // Persistent canvas at DISPLAY resolution (not source)
    const canvas = document.createElement('canvas')
    canvas.width = displayWidth
    canvas.height = displayHeight
    const ctx = canvas.getContext('2d')!

    // Sinks — both open simultaneously on same track
    const canvasSink = new CanvasSink(videoTrack, { poolSize: 2 })
    const sampleSink = new VideoSampleSink(videoTrack)

    // Audio sink
    const audioSink = audioCanDecode && audioTrack
      ? new AudioBufferSink(audioTrack)
      : null

    // ---------------------------------------------------------------------------
    // State
    // ---------------------------------------------------------------------------
    let state: DecoderState = { status: 'idle' }
    let disposed = false

    // Frame double-buffer for synchronous advanceFrame
    let currentFrame: WrappedCanvas | null = null
    let nextFrame: WrappedCanvas | null = null
    let prefetchInFlight = false

    // Audio state
    let audioContext: AudioContext | null = null
    let audioStartTime = 0
    let playbackTimeAtStart = 0
    let audioIterator: AsyncGenerator<WrappedAudioBuffer, void, unknown> | null = null
    const queuedAudioNodes = new Set<AudioBufferSourceNode>()
    let audioSchedulerId: ReturnType<typeof setInterval> | null = null

    // ---------------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------------

    function prefetchNextFrame(): void {
      if (disposed || state.status !== 'playing' || prefetchInFlight) return
      prefetchInFlight = true
      state.frameIterator.next()
        .then((r) => {
          prefetchInFlight = false
          if (disposed || state.status !== 'playing') return
          if (!r.done && r.value) {
            nextFrame = r.value
          } else {
            nextFrame = null // end of stream
          }
        })
        .catch((e) => {
          prefetchInFlight = false
          if (!disposed) console.warn('[video-decoder] Frame prefetch error:', e)
        })
    }

    /** Lookahead audio scheduler — runs every AUDIO_POLL_MS. */
    function startAudioScheduler(): void {
      if (!audioSink || !audioContext) return

      audioIterator = audioSink.buffers(playbackTimeAtStart)
      let audioBufferQueue: WrappedAudioBuffer[] = []
      let fetchingAudio = false

      const fetchMoreAudio = async () => {
        if (fetchingAudio || disposed || state.status !== 'playing' || !audioIterator) return
        fetchingAudio = true
        try {
          const r = await audioIterator.next()
          if (!r.done && r.value && !disposed && state.status === 'playing') {
            audioBufferQueue.push(r.value)
          }
        } catch (e) {
          if (!disposed) console.warn('[video-decoder] Audio fetch error:', e)
        }
        fetchingAudio = false
      }

      audioSchedulerId = setInterval(() => {
        if (disposed || state.status !== 'playing' || !audioContext) {
          if (audioSchedulerId) clearInterval(audioSchedulerId)
          audioSchedulerId = null
          return
        }

        // Schedule any queued buffers within lookahead window
        const now = audioContext.currentTime
        while (audioBufferQueue.length > 0) {
          const wrapped = audioBufferQueue[0]
          const playAt = audioStartTime + wrapped.timestamp - playbackTimeAtStart
          if (playAt > now + AUDIO_LOOKAHEAD_SEC) break // too far ahead

          audioBufferQueue.shift()
          const node = audioContext.createBufferSource()
          node.buffer = wrapped.buffer
          node.connect(audioContext.destination)

          if (playAt >= now) {
            node.start(playAt)
          } else {
            // Catch up — start with offset
            const offset = now - playAt
            if (offset < wrapped.buffer.duration) {
              node.start(now, offset)
            }
            // else: buffer is entirely in the past, skip
          }

          queuedAudioNodes.add(node)
          node.onended = () => queuedAudioNodes.delete(node)
        }

        // Keep fetching ahead
        fetchMoreAudio()
      }, AUDIO_POLL_MS)

      // Kick off initial fetch
      fetchMoreAudio()
    }

    function stopAudio(): void {
      if (audioSchedulerId) {
        clearInterval(audioSchedulerId)
        audioSchedulerId = null
      }
      audioIterator?.return?.(undefined as never)
        .catch(() => {/* iterator cancelled */})
      audioIterator = null
      for (const node of queuedAudioNodes) {
        try { node.stop() } catch { /* already stopped */ }
      }
      queuedAudioNodes.clear()
    }

    // ---------------------------------------------------------------------------
    // Handle
    // ---------------------------------------------------------------------------

    const handle: VideoDecoderHandle = {
      duration,
      width: nativeWidth,
      height: nativeHeight,
      hasAudio: !!audioSink,
      canvas,

      get isPlaying(): boolean {
        return state.status === 'playing'
      },

      async drawFrame(timeSec: number): Promise<void> {
        if (disposed) return
        const sample: VideoSample | null = await sampleSink.getSample(timeSec)
        if (sample && !disposed) {
          sample.draw(ctx, 0, 0, canvas.width, canvas.height)
          sample.close()
        }
      },

      startPlayback(fromTimeSec: number): void {
        if (disposed || state.status === 'playing') return
        playbackTimeAtStart = fromTimeSec

        const frameIterator = canvasSink.canvases(fromTimeSec)
        state = { status: 'playing', frameIterator }
        currentFrame = null
        nextFrame = null
        prefetchInFlight = false

        // Pre-buffer first frame + start prefetching second
        frameIterator.next()
          .then((r) => {
            if (disposed || state.status !== 'playing') return
            if (!r.done && r.value) {
              currentFrame = r.value
              ctx.drawImage(r.value.canvas as HTMLCanvasElement | OffscreenCanvas, 0, 0, canvas.width, canvas.height)
              prefetchNextFrame()
            }
          })
          .catch((e) => {
            if (!disposed) console.warn('[video-decoder] First frame error:', e)
          })

        // Start audio (lazy AudioContext creation from user gesture context)
        if (audioSink) {
          if (!audioContext) {
            try {
              audioContext = new AudioContext()
            } catch (e) {
              console.warn('[video-decoder] AudioContext creation failed:', e)
            }
          }
          if (audioContext) {
            if (audioContext.state === 'suspended') {
              audioContext.resume().catch(() => {})
            }
            audioStartTime = audioContext.currentTime
            startAudioScheduler()
          }
        }
      },

      advanceFrame(compositionTimeSec: number): boolean {
        // MUST be synchronous — called from RAF loop
        if (disposed || state.status !== 'playing' || !currentFrame) return false

        let advanced = false

        // Swap to nextFrame if its timestamp has been reached
        if (nextFrame && nextFrame.timestamp <= compositionTimeSec) {
          currentFrame = nextFrame
          nextFrame = null
          ctx.drawImage(currentFrame.canvas as HTMLCanvasElement | OffscreenCanvas, 0, 0, canvas.width, canvas.height)
          advanced = true
          // Kick off async prefetch for the frame after this one
          prefetchNextFrame()
        }

        return advanced
      },

      stopPlayback(): void {
        if (disposed || state.status !== 'playing') return

        const { frameIterator } = state
        state = { status: 'idle' }
        currentFrame = null
        nextFrame = null
        prefetchInFlight = false

        // Cancel frame iterator
        frameIterator.return(undefined as never)
          .catch(() => {/* iterator cancelled */})

        // Stop audio
        stopAudio()
        if (audioContext) {
          audioContext.suspend().catch(() => {})
        }
      },

      resizeCanvas(dw: number, dh: number): void {
        if (disposed) return
        canvas.width = Math.max(1, Math.round(dw))
        canvas.height = Math.max(1, Math.round(dh))
      },

      dispose(): void {
        if (disposed) return
        disposed = true

        // Stop playback if active
        if (state.status === 'playing') {
          const { frameIterator } = state
          frameIterator.return(undefined as never).catch(() => {})
        }
        state = { status: 'disposed' }
        currentFrame = null
        nextFrame = null

        // Stop audio
        stopAudio()
        audioContext?.close().catch(() => {})
        audioContext = null

        // Release mediabunny resources
        input?.dispose()
      },
    }

    // Draw first frame
    await handle.drawFrame(0)

    return handle
  } catch (e) {
    console.warn('[video-decoder] Failed to create decoder:', e)
    input?.dispose()
    return null
  }
}
