//! The script that starts in-view motion in the exports.
//!
//! Preview starts an in-view animation when at least 10% of its node is
//! inside the visible window: once when the declaration says `once`,
//! otherwise again on every re-entry. CSS scroll timelines scrub with the
//! scroll position instead, so the export keeps in-view animations paused
//! on their first keyframe and lets an IntersectionObserver start them.
//! The observer lives in one function, written so it is valid both as
//! plain JavaScript and as strict TypeScript, so component targets can call
//! it from their own lifecycle hooks.

use super::codegen_motion_css::has_in_view_motion;
use jian_ops_schema::PenDocument;

/// The HTML page's observer `<script>`, or an empty string when nothing
/// animates in view.
pub(super) fn render(doc: &PenDocument) -> String {
    if !has_in_view_motion(doc) {
        return String::new();
    }
    let observer: String = OBSERVE_IN_VIEW_MOTION
        .lines()
        .map(|line| match line {
            "" => "\n".to_owned(),
            line => format!("  {line}\n"),
        })
        .collect();
    format!("<script>\n(() => {{\n{observer}  observeInViewMotion();\n}})();\n</script>\n")
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
