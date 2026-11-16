(() => {
  const stage = document.querySelector("[data-op-stage]");
  const screens = new Map(Array.from(stage?.children ?? [], element =>
    [element.getAttribute("data-op-screen"), element]));
  const targets = new Map(Array.from(document.querySelectorAll("[data-op-target]"), element =>
    [element.getAttribute("data-op-target"), element]));
  const profiles = JSON.parse(document.getElementById("op-screen-motion").textContent);
  const initial = Array.from(screens).find(([, element]) => !element.hidden);
  const history = initial ? [{ path: initial[0], back: null }] : [];
  let transition = [];
  let generation = 0;
  let active = initial?.[1];

  function paintCurrentScreen() {
    for (const element of screens.values()) {
      element.hidden = element !== active;
      element.inert = element !== active;
      element.style.zIndex = "";
    }
  }

  function show(path, effect) {
    const next = screens.get(path);
    if (!next || next === active) return;
    const id = ++generation;
    for (const animation of transition) animation.cancel();
    transition = [];
    paintCurrentScreen();
    const previous = active;
    active = next;
    stage.style.width = next.style.width;
    stage.style.height = next.style.height;
    paintCurrentScreen();
    const motion = profiles[effect];
    if (!previous || !motion?.duration || stage.hasAttribute("data-op-reduced") ||
        matchMedia("(prefers-reduced-motion: reduce)").matches ||
        typeof next.animate !== "function") return;

    previous.hidden = false;
    previous.style.zIndex = motion.outgoing_on_top ? "2" : "1";
    next.style.zIndex = motion.outgoing_on_top ? "1" : "2";
    const width = stage.clientWidth;
    const height = stage.clientHeight;
    transition = [previous, next].map((element, index) => element.animate(
      motion.frames[index].map(([x, y, opacity]) => ({
        transform: `translate(${x * width}px, ${y * height}px)`,
        opacity,
      })),
      { duration: motion.duration, fill: "both" }
    ));
    Promise.allSettled(transition.map(animation => animation.finished)).then(() => {
      if (id !== generation) return;
      for (const animation of transition) animation.cancel();
      transition = [];
      paintCurrentScreen();
    });
  }

  function runActions(element) {
    if (stage?.contains(element) && !active?.contains(element)) return;
    for (const action of JSON.parse(element.getAttribute("data-op-tap"))) {
      switch (action.op) {
        case "motion": {
          const target = targets.get(action.target);
          if (target && (!stage?.contains(target) || active?.contains(target))) {
            window.opPlayMotion?.(target, action.animation, action.mode);
          }
          break;
        }
        case "push":
        case "replace": {
          if (!screens.has(action.path)) break;
          if (action.op === "replace") history.pop();
          history.push({ path: action.path, back: action.back });
          show(action.path, action.effect);
          break;
        }
        case "pop": {
          if (history.length < 2) break;
          const leaving = history.pop();
          show(history[history.length - 1].path, action.effect ?? leaving.back);
          break;
        }
      }
    }
  }

  document.addEventListener("click", event => {
    const element = event.target instanceof Element ? event.target.closest("[data-op-tap]") : null;
    if (element) runActions(element);
  });
  document.addEventListener("keydown", event => {
    if (event.key !== "Enter" && event.key !== " ") return;
    const element = event.target;
    if (element?.getAttribute("role") === "button" && element.hasAttribute("data-op-tap")) {
      event.preventDefault();
      runActions(element);
    }
  });
})();
