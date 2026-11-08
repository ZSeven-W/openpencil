//! The script that starts in-view motion in the HTML export.
//!
//! Preview starts an in-view animation when at least 10% of its node is
//! inside the visible window: once when the declaration says `once`,
//! otherwise again on every re-entry. CSS scroll timelines scrub with the
//! scroll position instead, so the export keeps in-view animations paused
//! on their first keyframe and lets this observer start them.

use super::codegen_motion_css::has_in_view_motion;
use jian_ops_schema::PenDocument;

/// The observer script, or an empty string when nothing animates in view.
pub(super) fn render(doc: &PenDocument) -> String {
    if has_in_view_motion(doc) {
        IN_VIEW_SCRIPT.to_owned()
    } else {
        String::new()
    }
}

const IN_VIEW_SCRIPT: &str = r#"<script>
(() => {
  const started = new WeakMap();
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      const element = entry.target;
      const visible = entry.isIntersecting && entry.intersectionRatio >= 0.1;
      const done = started.get(element) ?? new Set();
      started.set(element, done);
      for (const spec of element.dataset.opInView.split(" ")) {
        const [name, once] = spec.split(":");
        const animation = element
          .getAnimations()
          .find((item) => item.animationName === name);
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
})();
</script>
"#;
