let iframe;
let clock;

export async function mountPrototype(html) {
  iframe = document.createElement("iframe");
  iframe.style.cssText = "position:fixed;left:0;top:0;width:160px;height:160px;border:0";
  window.installPrototypeClock = child => {
    const frames = new Map();
    const animations = new Map();
    let now = 0, nextFrame = 0;
    Object.defineProperty(child.performance, "now", {value: () => now});
    child.requestAnimationFrame = fn => { frames.set(++nextFrame, fn); return nextFrame; };
    child.cancelAnimationFrame = id => frames.delete(id);
    const animate = child.Element.prototype.animate;
    const play = child.Animation.prototype.play;
    const cancel = child.Animation.prototype.cancel;
    function freeze(animation) {
      animation.pause();
      animation.currentTime = 0;
      animations.set(animation, now);
    }
    child.Element.prototype.animate = function (...args) {
      const animation = animate.apply(this, args);
      freeze(animation);
      return animation;
    };
    child.Animation.prototype.play = function () { play.call(this); freeze(this); };
    child.Animation.prototype.cancel = function () {
      animations.delete(this);
      return cancel.call(this);
    };
    clock = {
      set time(value) { now = value; },
      advance() {
        for (const [animation, start] of animations) {
          const end = animation.effect.getComputedTiming().endTime;
          animation.currentTime = Math.min(end, now - start);
          if (animation.currentTime >= end) animation.finish();
        }
        const batch = Array.from(frames.entries());
        frames.clear();
        for (const [, callback] of batch) callback(now);
      }
    };
  };
  const hook = '<style>body{margin:0}</style><script>parent.installPrototypeClock(window);</script>';
  const ready = new Promise(resolve => iframe.addEventListener("load", resolve, {once: true}));
  iframe.srcdoc = html.replace("<body>", "<body>" + hook);
  document.body.appendChild(iframe);
  await ready;
}

export function disposePrototype() {
  iframe?.remove();
  iframe = undefined;
  clock = undefined;
  delete window.installPrototypeClock;
}

export function setPrototypeTime(now) { clock.time = now; }
export async function advancePrototypeFrame() {
  clock.advance();
  // Settle animation.finished at the sample boundary before reading styles.
  await new Promise(resolve => setTimeout(resolve, 0));
}
export function tapPrototype(x, y) {
  const child = iframe.contentWindow;
  const target = child.document.elementFromPoint(x, y);
  target.dispatchEvent(new child.MouseEvent("click", {bubbles: true, clientX: x, clientY: y}));
}
export function prototypeRoute() {
  return iframe.contentDocument.querySelector("[data-op-stage]").dataset.opRoute;
}

export async function prototypeRgba() {
  // Freeze the existing browser-computed animation state, then let Chromium
  // rasterize the HTML snapshot. The fixture contains only DOM primitives.
  const doc = iframe.contentDocument;
  const stage = doc.querySelector("[data-op-stage]");
  const snapshot = element => {
    const clone = element.cloneNode(false);
    const style = doc.defaultView.getComputedStyle(element);
    for (const property of ["transform", "opacity", "background-color", "z-index"]) {
      clone.style.setProperty(property, style.getPropertyValue(property));
    }
    clone.style.removeProperty("animation");
    for (const child of element.children) clone.appendChild(snapshot(child));
    return clone;
  };
  const frozen = snapshot(stage);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="160" height="160">
    <foreignObject width="160" height="160">${new XMLSerializer().serializeToString(frozen)}</foreignObject>
  </svg>`;
  const image = new Image();
  image.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
  await image.decode();
  const canvas = document.createElement("canvas");
  canvas.width = 160;
  canvas.height = 160;
  const context = canvas.getContext("2d");
  context.fillStyle = "white";
  context.fillRect(0, 0, 160, 160);
  context.drawImage(image, 0, 0);
  return new Uint8Array(context.getImageData(0, 0, 160, 160).data);
}

export async function capturePrototypePair(time) {
  await window.opPrototypeParityCapture?.(time);
}
