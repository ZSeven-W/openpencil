  const program = JSON.parse(document.getElementById("op-prototype").textContent);
  const stage = document.querySelector("[data-op-stage]");
  const screens = new Map(Array.from(stage?.children ?? [], element =>
    [element.getAttribute("data-op-screen"), element]));
  const nodes = new Map(Array.from(document.querySelectorAll("[data-op-target]"), element => {
    const id = element.getAttribute("data-op-target");
    const spec = own(program.nodes, id);
    return [id, {id, element, spec, values: Object.assign(Object.create(null), spec?.state)}];
  }));
  const byElement = new Map(Array.from(nodes.values(), node => [node.element, node]));
  const app = Object.assign(Object.create(null), program.state);
  const vars = Object.assign(Object.create(null), program.vars);
  const pages = new Map(Object.entries(program.pages).map(([key, value]) =>
    [key, Object.assign(Object.create(null), value)]));
  const initial = Array.from(screens).find(([, element]) => !element.hidden);
  const history = initial ? [{path: initial[0], effect: null}] : [];
  let active = initial?.[1];
  let mountedDepth = history.length;
  let lastEffect = null;
  const route = {path: initial?.[0] ?? "/", params: {}, query: {}, stack: history.map(entry => entry.path)};
  if (stage) stage.dataset.opRoute = route.path;

  let renderedTransition = [];
  let transitionGeneration = 0;
  let framePending = false;
  let parkedInput = null;
  const waiting = [];

  function activeNode(node) {
    return !stage?.contains(node.element) || active?.contains(node.element);
  }

  function paintCurrentScreen() {
    for (const element of screens.values()) {
      element.hidden = element !== active;
      element.inert = element !== active;
      element.style.zIndex = "";
    }
  }

  function runLifecycle(root, hook) {
    for (const node of nodes.values()) {
      if (root ? root.contains(node.element) : !stage?.contains(node.element)) {
        const chain = node.spec?.events[hook];
        if (chain) run(chain, {node, event: {nodeId: node.id}});
      }
    }
  }

  function show(path, effect) {
    const next = screens.get(path);
    if (!next || next === active) return;
    const id = ++transitionGeneration;
    parkedInput = null;
    for (const animation of renderedTransition) animation.cancel();
    renderedTransition = [];
    paintCurrentScreen();
    const previous = active;
    if (previous) {
      runLifecycle(previous, "onUnmount");
      for (const animation of previous.getAnimations({subtree: true})) animation.pause();
    }
    window.opResetManualMotion?.(next);
    active = next;
    stage.dataset.opRoute = path;
    stage.style.width = next.style.width;
    stage.style.height = next.style.height;
    paintCurrentScreen();
    runLifecycle(next, "onMount");
    const motion = program.profiles[effect];
    if (!previous || !motion?.duration || program.reduced ||
        matchMedia("(prefers-reduced-motion: reduce)").matches ||
        typeof next.animate !== "function") return;

    previous.hidden = false;
    previous.style.zIndex = motion.outgoing_on_top ? "2" : "1";
    next.style.zIndex = motion.outgoing_on_top ? "1" : "2";
    const width = stage.clientWidth, height = stage.clientHeight;
    renderedTransition = [previous, next].map((element, index) => element.animate(
      motion.frames[index].map(([x, y, opacity]) => ({
        transform: `translate(${x * width}px, ${y * height}px)`, opacity,
      })),
      {duration: motion.duration, fill: "both"}
    ));
    Promise.allSettled(renderedTransition.map(animation => animation.finished)).then(() => {
      if (id !== transitionGeneration) return;
      for (const animation of renderedTransition) animation.cancel();
      renderedTransition = [];
      paintCurrentScreen();
      const pending = parkedInput;
      parkedInput = null;
      if (pending) pending();
    });
  }

  function reconcileRoute() {
    if (!history.length) return;
    const path = history.at(-1).path;
    const kind = history.length > mountedDepth ? 0 : history.length < mountedDepth ? 2 : 1;
    route.stack = history.map(entry => entry.path);
    if (route.path === path) return;
    route.path = path;
    mountedDepth = history.length;
    show(path, (lastEffect ?? program.defaults)[kind]);
  }

  function getScope(context, scope) {
    switch (scope) {
      case "$app": return app;
      case "$page": return pages.get(context.node.spec?.page);
      case "$self": return context.node.values;
      case "$vars": return vars;
      case "$route": return route;
      case "$event": return context.event;
      case "$viewport": return {width: stage?.clientWidth ?? innerWidth,
        height: stage?.clientHeight ?? innerHeight, dpr: devicePixelRatio};
      default: return null;
    }
  }

  function stateScope(context, field) {
    const local = getScope(context, "$self");
    if (field === undefined || Object.hasOwn(local, field)) return local;
    const page = getScope(context, "$page");
    return page && Object.hasOwn(page, field) ? page : app;
  }

  function evalIn(ops, context) {
    return evaluate(ops, path => {
      const [scope, ...fields] = path.split(".");
      const value = scope === "$state" ? stateScope(context, fields[0]) : getScope(context, scope);
      return fields.reduce((value, field) => object(value) ? own(value, field) : null, value);
    });
  }

  function write(context, [scope, key], value) {
    const state = getScope(context, scope);
    if (state) state[key] = value;
    requestFrame();
  }

  function* execute(actions, context) {
    for (const action of actions) {
      switch (action.op) {
        case "set":
          for (const [path, ops] of action.writes) write(context, path, evalIn(ops, context));
          break;
        case "toggle": {
          const [scope, key] = action.target;
          const value = own(getScope(context, scope), key);
          if (typeof value !== "boolean") throw new Error("toggle requires a bool");
          write(context, action.target, !value);
          break;
        }
        case "delete": write(context, action.target, null); break;
        case "reset_state": {
          const scope = getScope(context, action.scope);
          for (const key of Object.keys(scope ?? {})) delete scope[key];
          requestFrame();
          break;
        }
        case "if":
          yield* execute(truthy(evalIn(action.test, context)) ? action.then : action.else, context);
          break;
        case "delay": if (action.ms) yield action.ms; break;
        case "parallel": yield action.branches; break;
        case "push": case "replace": case "reset": {
          const path = evalIn(action.path, context);
          if (!screens.has(path)) break;
          if (action.op === "reset") history.length = 0;
          if (action.op === "replace") history.pop();
          history.push({path, effect: action.effect});
          lastEffect = action.effect;
          requestFrame();
          break;
        }
        case "pop":
          if (history.length > 1) {
            const leaving = history.pop();
            lastEffect = action.effect ?? leaving.effect;
            requestFrame();
          }
          break;
        case "motion": {
          const target = nodes.get(action.target);
          if (target && activeNode(target)) {
            window.opPlayMotion?.(target.element, action.animation, action.mode);
          }
          break;
        }
        case "hide": case "show": case "toggle_visibility": {
          const target = nodes.get(action.target);
          if (target && activeNode(target)) {
            target.element.hidden = action.op === "hide" ||
              (action.op === "toggle_visibility" && !target.element.hidden);
          }
          break;
        }
        case "focus": {
          const target = nodes.get(action.target);
          if (target && activeNode(target)) {
            (target.element.querySelector("input,textarea,select,button") ?? target.element).focus();
          }
          break;
        }
        case "blur": document.activeElement?.blur(); break;
      }
    }
  }

  function run(actions, context, complete = () => {}) {
    const chain = execute(actions, context);
    function resume() {
      let next;
      try { next = chain.next(); } catch (error) {
        console.warn("OpenPencil HTML action:", error.message);
        complete();
        return;
      }
      if (next.done) {
        complete();
      } else if (typeof next.value === "number") {
        waiting.push({at: performance.now() + next.value, resume});
        requestFrame();
      } else {
        let remaining = next.value.length;
        if (!remaining) resume();
        for (const branch of next.value) run(branch, context, () => {
          if (--remaining === 0) resume();
        });
      }
    }
    resume();
  }

  const geometry = new Map([
    ["x", "left"], ["y", "top"], ["width", "width"], ["height", "height"],
    ["cornerRadius", "borderRadius"]
  ]);
  function inputWithin(element) {
    return element.querySelector(":scope > input, :scope > textarea, :scope > select");
  }
  function applyBindings() {
    for (const node of nodes.values()) {
      if (!activeNode(node)) continue;
      const element = node.element;
      const input = inputWithin(element);
      for (const [name, ops] of Object.entries(node.spec?.bindings ?? {})) {
        const value = evalIn(ops, {node, event: {}});
        if (name === "visible" && typeof value === "boolean") element.hidden = !value;
        else if (name === "opacity" && typeof value === "number") element.style.opacity = value;
        else if (geometry.has(name) && typeof value === "number") element.style[geometry.get(name)] = `${value}px`;
        else if (name === "content" && element.tagName === "SPAN") element.textContent = display(value);
        else if (name === "disabled" && input && typeof value === "boolean") input.disabled = value;
        else if ((name === "value" || name === "bind:value") && input) {
          if (input.type === "checkbox") input.checked = Boolean(value);
          else if (input.value !== display(value)) input.value = display(value);
        } else if (name === "fill" || name === "stroke") {
          const color = Array.isArray(value) ? value.find(paint => paint?.type === "solid")?.color : value;
          if (typeof color === "string" && CSS.supports("color", color)) {
            if (name === "stroke") element.style.borderColor = color;
            else element.style[element.tagName === "SPAN" ? "color" : "backgroundColor"] = color;
          }
        }
      }
    }
  }

  function requestFrame() {
    if (!framePending) {
      framePending = true;
      requestAnimationFrame(tick);
    }
  }
  function tick() {
    framePending = false;
    const now = performance.now();
    const due = waiting.filter(wait => wait.at <= now);
    for (const wait of due) waiting.splice(waiting.indexOf(wait), 1);
    for (const wait of due) wait.resume();
    reconcileRoute();
    applyBindings();
    if (waiting.length) requestFrame();
  }

  function dispatch(target, hook, payload) {
    for (let element = target; element instanceof Element; element = element.parentElement) {
      const node = byElement.get(element);
      const chain = node?.spec?.events[hook];
      if (!chain || !activeNode(node) || inputWithin(element)?.disabled) continue;
      const executeEvent = () => run(chain, {node, event: {...payload, nodeId: node.id}});
      if (renderedTransition.length) {
        if (hook === "onTap" || hook === "onSubmit") parkedInput ??= executeEvent;
      } else executeEvent();
      return true;
    }
    return false;
  }

  const point = event => ({x: event.clientX, y: event.clientY, button: event.button});
  document.addEventListener("click", event => dispatch(event.target, "onTap", point(event)));
  for (const [event, hook] of [["pointerover", "onHoverEnter"], ["pointerout", "onHoverLeave"]]) {
    document.addEventListener(event, input => {
      const element = input.target instanceof Element ? input.target.closest("[data-op-target]") : null;
      if (element && !element.contains(input.relatedTarget)) dispatch(element, hook, point(input));
    });
  }
  document.addEventListener("wheel", event =>
    dispatch(event.target, "onScroll", {deltaX: event.deltaX, deltaY: -event.deltaY}));
  document.addEventListener("focusin", event => dispatch(event.target, "onFocus", {}));
  document.addEventListener("focusout", event => dispatch(event.target, "onBlur", {}));
  document.addEventListener("input", event => {
    const input = event.target;
    if (!("value" in input)) return;
    const value = input.type === "checkbox" ? input.checked : input.type === "number"
      ? (Number.isFinite(input.valueAsNumber) ? input.valueAsNumber : null) : input.value;
    dispatch(input, "onChange", {value});
  });
  document.addEventListener("keydown", event => {
    const element = event.target;
    dispatch(element, "onKey", {key: event.key});
    if (event.key === "Enter" && "value" in element) {
      if (dispatch(element, "onSubmit", {value: element.value, key: event.key})) event.preventDefault();
    } else if ((event.key === "Enter" || event.key === " ") && element?.hasAttribute("data-op-tap")) {
      event.preventDefault();
      dispatch(element, "onTap", {key: event.key});
    }
  });
  runLifecycle(active, "onMount");
  if (active) runLifecycle(null, "onMount");
  applyBindings();
