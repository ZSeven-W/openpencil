//! The script that starts in-view motion in the HTML, Vue and Svelte exports.
//!
//! Preview starts an in-view animation when at least 10% of its node is
//! inside the visible window: once when the declaration says `once`,
//! otherwise again on every re-entry. CSS scroll timelines scrub with the
//! scroll position instead, so the export keeps in-view animations paused
//! on their first keyframe and lets an IntersectionObserver start them.
//! The observer lives in one function, written so it is valid both as
//! plain JavaScript and as strict TypeScript, so component targets can call
//! it from their own lifecycle hooks.

use super::codegen_motion_css::{
    has_in_view_motion, has_in_view_motion_for_roots, has_manual_motion,
};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;

/// The HTML page's observer `<script>`, or an empty string when nothing
/// animates in view.
pub(super) fn render(roots: &[&PenNode], include_hidden: bool) -> String {
    let mut out = String::new();
    if has_manual_motion(roots, include_hidden) {
        out.push_str(MANUAL_MOTION);
    }
    if !has_in_view_motion_for_roots(roots.iter().copied(), include_hidden) {
        return out;
    }
    let observer: String = OBSERVE_IN_VIEW_MOTION
        .lines()
        .map(|line| match line {
            "" => "\n".to_owned(),
            line => format!("  {line}\n"),
        })
        .collect();
    out.push_str(&format!(
        "<script>\n(() => {{\n{observer}  observeInViewMotion();\n}})();\n</script>\n"
    ));
    out
}

/// `<script setup>` statements that start in-view motion once the Vue
/// component is mounted and stop observing when it unmounts, or an empty
/// string when nothing animates in view.
pub(super) fn vue_setup(doc: &PenDocument) -> String {
    if !has_in_view_motion(doc) {
        return String::new();
    }
    format!(
        "import {{ onBeforeUnmount, onMounted }} from \"vue\";\n\n\
         {OBSERVE_IN_VIEW_MOTION}\n\
         let inViewObserver: IntersectionObserver | undefined;\n\
         onMounted(() => {{\n  inViewObserver = observeInViewMotion();\n}});\n\
         onBeforeUnmount(() => inViewObserver?.disconnect());\n"
    )
}

/// Instance-script statements that start in-view motion once the Svelte
/// component is mounted and stop observing when it is destroyed, or an
/// empty string when nothing animates in view.
pub(super) fn svelte_setup(doc: &PenDocument) -> String {
    if !has_in_view_motion(doc) {
        return String::new();
    }
    format!(
        "import {{ onMount }} from \"svelte\";\n\n\
         {OBSERVE_IN_VIEW_MOTION}\n\
         onMount(() => {{\n  const observer = observeInViewMotion();\n  \
         return () => observer.disconnect();\n}});\n"
    )
}

/// Observes every `data-op-in-view` element and plays, or rewinds, the CSS
/// animations its attribute names. Returns the observer so a component can
/// disconnect it.
const OBSERVE_IN_VIEW_MOTION: &str = r#"function observeInViewMotion() {
  const started = new WeakMap();
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      const element = entry.target;
      const visible = entry.isIntersecting && entry.intersectionRatio >= 0.1;
      const done = started.get(element) ?? new Set();
      started.set(element, done);
      for (const spec of (element.getAttribute("data-op-in-view") ?? "").split(" ")) {
        const [name, once] = spec.split(":");
        const animation = element
          .getAnimations()
          .find((item) => item instanceof CSSAnimation && item.animationName === name);
        if (!animation) continue;
        if (visible && !done.has(name)) {
          animation.play();
          done.add(name);
        } else if (!visible && once !== "once" && done.has(name)) {
          animation.pause();
          animation.currentTime = 0;
          done.delete(name);
        }
      }
    }
  }, { threshold: 0.1 });
  for (const element of document.querySelectorAll("[data-op-in-view]")) {
    observer.observe(element);
  }
  return observer;
}
"#;

/// The HTML target exposes named clips to its own dispatcher and callers.
const MANUAL_MOTION: &str = r#"<script>
(() => {
  const running = new WeakMap();
  const selector = "[data-op-manual]";
  function clipTargets(root) {
    const targets = [...root.querySelectorAll(selector)];
    if (root.matches(selector)) targets.unshift(root);
    return targets;
  }

  // A mounted screen starts with fresh manual clips, as in Play.
  window.opResetManualMotion = root => {
    for (const target of clipTargets(root)) {
      const state = running.get(target);
      if (!state) continue;
      for (const animation of state.animations.values()) animation.cancel();
      target.style.animation = state.authored;
      running.delete(target);
    }
  };

  // From an event handler, call opPlayMotion(targetElement, clipId, mode).
  // The target may be a container. Modes: play, restart, reverse or stop.
  window.opPlayMotion = (root, id, mode = "play") => {
    if (!root || !["play", "restart", "reverse", "stop"].includes(mode)) return;
    for (const target of clipTargets(root)) {
      const specs = JSON.parse(target.getAttribute("data-op-manual"));
      const spec = specs.find(([clipId]) => clipId === id);
      if (!spec) continue;
      const [, name, declaration] = spec;
      let state = running.get(target);
      if (!state) {
        state = {authored: target.style.animation, animations: new Map()};
        running.set(target, state);
      }
      let animation = state.animations.get(name);
      if (mode === "stop") {
        animation?.cancel();
        continue;
      }
      if (mode === "play" && animation?.playState === "running") continue;
      if (!animation) {
        const previous = target.style.animation;
        target.style.animation = previous ? previous + ", " + declaration : declaration;
        animation = target.getAnimations().find(item => item.animationName === name);
        if (!animation) {
          target.style.animation = previous;
          continue;
        }
        state.animations.set(name, animation);
      }
      animation.cancel();
      animation.effect.updateTiming({ direction: mode === "reverse" ? "reverse" : "normal" });
      animation.play();
    }
  };
})();
</script>
"#;
