import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import test from 'node:test';

const require = createRequire(import.meta.url);
const assets = new URL('../assets/canvaskit/', import.meta.url);
const init = require(new URL('canvaskit.js', assets).pathname);
const CK = await init({ wasmBinary: await readFile(new URL('canvaskit.wasm', assets)) });
const source = await readFile(new URL('../src/op_ck_bridge.js', import.meta.url), 'utf8');
const bridge = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);

test('real CanvasKit binds flattened arrays and adjacent uniforms at reflected slots', () => {
  const cache = bridge.opCkCreateRuntimeEffectCache(CK);
  const entry = cache.get('uniform float4 bars[2]; uniform float gain; half4 main(float2 p) { return half4(bars[1].rgb * gain, 1); }');
  assert.ok(entry);
  const shader = bridge.opCkMakeRuntimeShader(
    entry,
    ['bars', 'gain'],
    new Float32Array([9, 9, 9, 9, 0.2, 0.6, 1, 1, 0.5]),
    new Uint32Array([8, 1]),
  );
  assert.ok(shader);
  const surface = CK.MakeSurface(2, 2);
  const paint = new CK.Paint();
  paint.setShader(shader);
  surface.getCanvas().drawRect(CK.LTRBRect(0, 0, 2, 2), paint);
  surface.flush();
  const pixel = surface.getCanvas().readPixels(0, 0, {
    width: 1, height: 1,
    colorType: CK.ColorType.RGBA_8888,
    alphaType: CK.AlphaType.Unpremul,
    colorSpace: CK.ColorSpace.SRGB,
  });
  assert.ok(pixel);
  for (const [index, expected] of [26, 77, 128, 255].entries()) {
    assert.ok(Math.abs(pixel[index] - expected) <= 1, `channel ${index}: ${pixel[index]}`);
  }
  paint.delete();
  shader.delete();
  surface.delete();
  entry.effect.delete();
});
