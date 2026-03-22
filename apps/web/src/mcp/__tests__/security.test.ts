import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { writeFile, unlink, mkdir } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { sanitizeObject } from '../utils/sanitize'
import { openDocument, invalidateCache } from '../document-manager'
import { handleBatchDesign } from '../tools/batch-design'

const TMP_DIR = join(tmpdir(), 'openpencil-security-tests')

beforeEach(async () => {
  await mkdir(TMP_DIR, { recursive: true })
})

afterEach(async () => {
  // Clean up any test files
  const files = ['proto.op', 'normal.op', 'batch.op', 'batch-proto.op']
  for (const f of files) {
    try {
      const fp = join(TMP_DIR, f)
      invalidateCache(fp)
      await unlink(fp)
    } catch {}
  }
})

// ---------- sanitizeObject ----------

describe('sanitizeObject', () => {
  it('strips __proto__ key', () => {
    const input = JSON.parse('{"__proto__": {"polluted": true}, "safe": 1}')
    const result = sanitizeObject(input)
    expect(result).toEqual({ safe: 1 })
    expect(Object.prototype.hasOwnProperty.call(result, '__proto__')).toBe(false)
  })

  it('strips constructor key', () => {
    const result = sanitizeObject({ constructor: 'bad', ok: true })
    expect(result).toEqual({ ok: true })
  })

  it('strips prototype key', () => {
    const result = sanitizeObject({ prototype: {}, keep: 'yes' })
    expect(result).toEqual({ keep: 'yes' })
  })

  it('works recursively on nested objects', () => {
    const input = JSON.parse(
      '{"a": {"__proto__": {"x": 1}, "b": {"constructor": "c", "d": 2}}}',
    )
    const result = sanitizeObject(input)
    expect(result).toEqual({ a: { b: { d: 2 } } })
  })

  it('preserves arrays', () => {
    const result = sanitizeObject([1, { __proto__: 'x', a: 2 }, 'three'])
    expect(result).toEqual([1, { a: 2 }, 'three'])
  })

  it('preserves primitives', () => {
    expect(sanitizeObject('hello')).toBe('hello')
    expect(sanitizeObject(42)).toBe(42)
    expect(sanitizeObject(null)).toBe(null)
    expect(sanitizeObject(undefined)).toBe(undefined)
  })

  it('preserves normal object keys', () => {
    const obj = { type: 'frame', x: 10, y: 20, children: [] }
    expect(sanitizeObject(obj)).toEqual(obj)
  })
})

// ---------- document-manager openDocument ----------

describe('openDocument', () => {
  it('does not pollute Object.prototype from __proto__ in file', async () => {
    const fp = join(TMP_DIR, 'proto.op')
    const malicious = JSON.stringify({
      version: '1.0.0',
      __proto__: { polluted: true },
      children: [
        {
          id: 'n1',
          type: 'rectangle',
          __proto__: { evil: true },
          x: 0,
          y: 0,
          width: 100,
          height: 100,
        },
      ],
    })
    await writeFile(fp, malicious, 'utf-8')

    const doc = await openDocument(fp)

    // Object.prototype should not be polluted
    expect(({} as any).polluted).toBeUndefined()
    expect(({} as any).evil).toBeUndefined()
    // Doc should still be valid
    expect(doc.version).toBe('1.0.0')
    expect(doc.children.length).toBe(1)
  })

  it('loads normal documents correctly after sanitization', async () => {
    const fp = join(TMP_DIR, 'normal.op')
    const doc = {
      version: '1.0.0',
      children: [
        { id: 'r1', type: 'rectangle', x: 10, y: 20, width: 50, height: 60 },
      ],
    }
    await writeFile(fp, JSON.stringify(doc), 'utf-8')

    const loaded = await openDocument(fp)
    expect(loaded.version).toBe('1.0.0')
    expect(loaded.children[0]).toMatchObject({
      id: 'r1',
      type: 'rectangle',
      x: 10,
      y: 20,
    })
  })
})

// ---------- batch-design (parseJsonArg sanitization) ----------

describe('handleBatchDesign', () => {
  it('executes a normal insert operation', async () => {
    const fp = join(TMP_DIR, 'batch.op')
    await writeFile(
      fp,
      JSON.stringify({ version: '1.0.0', children: [] }),
      'utf-8',
    )

    const result = await handleBatchDesign({
      filePath: fp,
      operations:
        'myRect=I(null, { type: "rectangle", x: 0, y: 0, width: 100, height: 100 })',
    })

    expect(result.results.length).toBe(1)
    expect(result.results[0].binding).toBe('myRect')
    expect(result.nodeCount).toBe(1)
  })

  it('strips __proto__ keys from node data in operations', async () => {
    const fp = join(TMP_DIR, 'batch-proto.op')
    await writeFile(
      fp,
      JSON.stringify({ version: '1.0.0', children: [] }),
      'utf-8',
    )

    const result = await handleBatchDesign({
      filePath: fp,
      operations:
        'bad=I(null, { "type": "rectangle", "__proto__": {"polluted": true}, "x": 0, "y": 0, "width": 50, "height": 50 })',
    })

    expect(result.results.length).toBe(1)
    // Object.prototype must not be polluted
    expect(({} as any).polluted).toBeUndefined()
    expect(result.nodeCount).toBe(1)
  })
})
