import { describe, expect, it } from 'bun:test'

import { SkiaImageLoader } from '../image-loader'

const originalDocument = globalThis.document
const OriginalImage = globalThis.Image

describe('SkiaImageLoader', () => {
  it('downscales oversized decoded images before creating a CanvasKit image', async () => {
    let drawSize: { width: number; height: number } | null = null
    let imageDataSize: { width: number; height: number } | null = null
    let makeImageSize: { width: number; height: number } | null = null

    ;(globalThis as { document?: Document }).document = {
      createElement(tag: string) {
        expect(tag).toBe('canvas')

        const canvas = {
          width: 0,
          height: 0,
          getContext() {
            return {
              drawImage(_img: unknown, _x: number, _y: number, width: number, height: number) {
                drawSize = { width, height }
              },
              getImageData(_x: number, _y: number, width: number, height: number) {
                imageDataSize = { width, height }
                return { data: new Uint8ClampedArray(width * height * 4) }
              },
            }
          },
        }

        return canvas as unknown as HTMLCanvasElement
      },
    } as Document

    class MockImage {
      naturalWidth = 8192
      naturalHeight = 4096
      width = 8192
      height = 4096
      onload: ((event: Event) => void) | null = null
      onerror: ((event: string | Event) => void) | null = null

      set src(_value: string) {
        queueMicrotask(() => {
          this.onload?.({} as Event)
        })
      }
    }

    ;(globalThis as { Image?: typeof Image }).Image = MockImage as unknown as typeof Image

    try {
      const ck = {
        AlphaType: { Unpremul: 0 },
        ColorType: { RGBA_8888: 0 },
        ColorSpace: { SRGB: 0 },
        MakeImage(info: { width: number; height: number }) {
          makeImageSize = { width: info.width, height: info.height }
          return {
            delete() {},
            width: () => info.width,
            height: () => info.height,
          }
        },
      }

      const loader = new SkiaImageLoader(ck as any)
      const loaded = new Promise<void>((resolve) => {
        loader.setOnLoaded(resolve)
      })

      loader.request('large-image.png')
      await loaded

      expect(drawSize).toEqual({ width: 4096, height: 2048 })
      expect(imageDataSize).toEqual({ width: 4096, height: 2048 })
      expect(makeImageSize).toEqual({ width: 4096, height: 2048 })
      expect(loader.getStatus('large-image.png')).toEqual({ state: 'loaded' })
    } finally {
      if (originalDocument === undefined) {
        delete (globalThis as { document?: Document }).document
      } else {
        ;(globalThis as { document?: Document }).document = originalDocument
      }

      if (OriginalImage === undefined) {
        delete (globalThis as { Image?: typeof Image }).Image
      } else {
        ;(globalThis as { Image?: typeof Image }).Image = OriginalImage
      }
    }
  })
})
