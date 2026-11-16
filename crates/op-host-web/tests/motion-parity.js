// Observe the production CanvasKit surfaces and drive both hosts with one clock.
let now = 0;
let nextFrame = 0;
const frames = new Map();
const surfaces = new Map();
let restore;

export function installFrameClock(wasmBinary) {
  const init = globalThis.CanvasKitInit;
  const ownedWasm = wasmBinary.slice();
  const timeOrigin = performance.now();
  const originalNow = Object.getOwnPropertyDescriptor(performance, 'now');
  const originalRaf = window.requestAnimationFrame;
  const originalCancel = window.cancelAnimationFrame;
  Object.defineProperty(performance, 'now', {configurable: true, value: () => timeOrigin + now});
  window.requestAnimationFrame = callback => {
    frames.set(++nextFrame, callback);
    return nextFrame;
  };
  window.cancelAnimationFrame = id => frames.delete(id);
  globalThis.CanvasKitInit = async options => {
    const ck = await init({...options, wasmBinary: ownedWasm.slice()});
    const make = ck.MakeWebGLCanvasSurface.bind(ck);
    ck.MakeWebGLCanvasSurface = (canvas, colorSpace, attributes) => {
      const surface = make(canvas, colorSpace, {...attributes, preserveDrawingBuffer: true});
      const id = typeof canvas === 'string' ? canvas : canvas.id;
      if (surface) surfaces.set(id, {ck, surface});
      return surface;
    };
    return ck;
  };
  restore = () => {
    if (originalNow) Object.defineProperty(performance, 'now', originalNow);
    else delete performance.now;
    window.requestAnimationFrame = originalRaf;
    window.cancelAnimationFrame = originalCancel;
    globalThis.CanvasKitInit = init;
    surfaces.clear();
    frames.clear();
    now = 0;
  };
}

export function restoreFrameClock() { restore(); }

export function setFrameTime(ms) { now = ms; }

export async function presentFrame() {
  // The Web host first drains input in a Promise task, then requests rAF.
  // Yield to a new browser task so that the whole microtask queue drains.
  await new Promise(resolve => setTimeout(resolve, 0));
  const scheduled = Array.from(frames.keys());
  for (const id of scheduled) {
    const callback = frames.get(id);
    frames.delete(id);
    if (callback) callback(now);
  }
}

export function wheel(canvas, deltaY) {
  const bounds = canvas.getBoundingClientRect();
  canvas.dispatchEvent(new WheelEvent('wheel', {
    clientX: bounds.left + 32, clientY: bounds.top + 32,
    deltaY, deltaMode: WheelEvent.DOM_DELTA_PIXEL, bubbles: true, cancelable: true,
  }));
}

export function frameRgba(id, width, height) {
  const {ck, surface} = surfaces.get(id);
  const image = surface.makeImageSnapshot();
  try {
    return image.readPixels(0, 0, {
      width, height,
      colorType: ck.ColorType.RGBA_8888,
      alphaType: ck.AlphaType.Unpremul,
      colorSpace: ck.ColorSpace.SRGB,
    });
  } finally {
    image.delete();
  }
}
