//! Vue and Svelte motion export tests. Both targets reuse the HTML markup,
//! so their elements carry the inline `animation:` shorthand with the plain
//! keyframe names; these tests prove the emitted `@keyframes` keep exactly
//! those names through each framework's style scoping.

use crate::tests::empty_doc;
use crate::{Codegen, Svelte, Vue};
use jian_ops_schema::PenDocument;

/// A mount animation, an in-view animation that plays once, and a manual
/// animation the export declares but does not start.
fn motion_doc() -> PenDocument {
    jian_ops_schema::load_str(
        r#"{
          "version":"1.0",
          "children":[
            {"type":"rectangle","id":"hero","width":320,"height":180,
             "transition":{"durationMs":150,"easing":"standard","properties":["opacity"]},
             "animations":[{"trigger":"mount","keyframes":[
               {"offset":0,"values":{"opacity":0,"translateY":16}},
               {"offset":1,"values":{"opacity":1,"translateY":0}}
             ],"durationMs":400,"easing":"emphasizedDecelerate","fillMode":"forwards"}]},
            {"type":"rectangle","id":"card","x":0,"y":200,"width":320,"height":96,
             "animations":[{"trigger":"inView","keyframes":[
               {"offset":0,"values":{"opacity":0,"translateY":12}},
               {"offset":1,"values":{"opacity":1,"translateY":0}}
             ],"durationMs":320,"delayMs":60,"easing":"standard","once":true}]},
            {"type":"rectangle","id":"badge","x":0,"y":320,"width":48,"height":48,
             "animations":[{"trigger":"manual","id":"pulse","keyframes":[
               {"offset":0,"values":{"scaleX":1}},
               {"offset":1,"values":{"scaleX":1.1}}
             ],"durationMs":200}]}
          ]
        }"#,
    )
    .expect("motion document")
    .value
}

/// The text between the first `open` and the next `close` after it.
fn block<'a>(source: &'a str, open: &str, close: &str) -> &'a str {
    let start = source
        .find(open)
        .unwrap_or_else(|| panic!("{open}: {source}"))
        + open.len();
    let end = source[start..]
        .find(close)
        .unwrap_or_else(|| panic!("{close}: {source}"))
        + start;
    &source[start..end]
}

/// Keyframe names the inline `animation:` shorthands reference, in markup
/// order.
fn inline_animation_names(markup: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (start, key) in markup.match_indices(";animation:") {
        let rest = &markup[start + key.len()..];
        let end = rest.find(['"', ';']).expect("end of the shorthand");
        for animation in rest[..end].split(", ") {
            let name = animation.split(' ').next().expect("animation name");
            names.push(name.to_owned());
        }
    }
    names
}

/// `@keyframes` names exactly as `css` declares them.
fn keyframe_names(css: &str) -> Vec<String> {
    css.match_indices("@keyframes ")
        .map(|(start, key)| {
            let rest = &css[start + key.len()..];
            rest[..rest.find(' ').expect("keyframes name end")].to_owned()
        })
        .collect()
}

#[test]
fn vue_motion_keyframes_keep_the_names_the_template_references() {
    let vue = Vue.generate(&motion_doc());
    let referenced = inline_animation_names(block(&vue, "<template>\n", "</template>"));
    assert_eq!(referenced, ["op-hero-0", "op-card-0"], "{vue}");

    // The scoped block would hash keyframe names, so it holds none; the
    // motion lives in an unscoped block, where Vue keeps them as written.
    assert!(!block(&vue, "<style scoped>\n", "</style>").contains("@keyframes"));
    let motion = block(&vue, "\n<style>\n", "</style>");
    let declared = keyframe_names(motion);
    assert_eq!(declared, ["op-hero-0", "op-card-0", "op-badge-0"], "{vue}");
    for name in &referenced {
        assert!(declared.contains(name), "{name} has no keyframes: {vue}");
    }
    assert!(motion.contains("[data-op-motion-node=\"hero\"] {"), "{vue}");
}

#[test]
fn vue_without_motion_keeps_a_single_style_block() {
    let vue = Vue.generate(&empty_doc());
    assert_eq!(vue.matches("<style").count(), 1, "{vue}");
    assert!(!vue.contains("@keyframes"), "{vue}");
}

#[test]
fn svelte_motion_keyframes_keep_the_names_the_markup_references() {
    let svelte = Svelte.generate(&motion_doc());
    let markup = block(&svelte, "</script>\n", "\n<style>\n");
    let referenced = inline_animation_names(markup);
    assert_eq!(referenced, ["op-hero-0", "op-card-0"], "{svelte}");

    // Svelte hashes component keyframe names unless they carry `-global-`,
    // which it strips, leaving exactly the names the markup references.
    assert_eq!(svelte.matches("<style").count(), 1, "{svelte}");
    let style = block(&svelte, "\n<style>\n", "</style>");
    let declared = keyframe_names(style);
    assert_eq!(
        declared,
        [
            "-global-op-hero-0",
            "-global-op-card-0",
            "-global-op-badge-0"
        ],
        "{svelte}"
    );
    for name in &referenced {
        assert!(
            declared.contains(&format!("-global-{name}")),
            "{name} has no global keyframes: {svelte}"
        );
    }
    // Selectors opt out of scoping too, so they need no compile-time match.
    assert!(
        style.contains(":global([data-op-motion-node=\"hero\"]) {"),
        "{svelte}"
    );
    assert!(!style.contains("\n  [data-op-motion-node"), "{svelte}");
}

#[test]
fn svelte_without_motion_declares_no_keyframes() {
    let svelte = Svelte.generate(&empty_doc());
    assert!(!svelte.contains("@keyframes"), "{svelte}");
    assert!(!svelte.contains(":global("), "{svelte}");
}

/// A document whose only motion plays on mount, so nothing needs observing.
fn mount_only_doc() -> PenDocument {
    jian_ops_schema::load_str(
        r#"{"version":"1.0","children":[
          {"type":"rectangle","id":"n1","animations":[
            {"trigger":"mount","keyframes":[{"offset":0,"values":{"opacity":0}},
            {"offset":1,"values":{"opacity":1}}],"durationMs":300}
          ]}
        ]}"#,
    )
    .expect("mount motion document")
    .value
}

/// The paused in-view element and the observer that starts it like
/// Preview: at 10% visible, once for `once` motion, otherwise again on
/// every re-entry after rewinding on leave.
fn assert_in_view_motion_starts(output: &str, script: &str) {
    assert!(
        output.contains("animation:op-card-0 320ms linear 60ms 1 both paused"),
        "{output}"
    );
    assert!(
        output.contains("data-op-in-view=\"op-card-0:once\""),
        "{output}"
    );
    for needle in [
        "function observeInViewMotion() {",
        "document.querySelectorAll(\"[data-op-in-view]\")",
        "element.getAttribute(\"data-op-in-view\")",
        "entry.isIntersecting && entry.intersectionRatio >= 0.1",
        "{ threshold: 0.1 }",
        "item instanceof CSSAnimation && item.animationName === name",
        "animation.play();",
        "!visible && once !== \"once\" && done.has(name)",
        "animation.currentTime = 0;",
    ] {
        assert!(script.contains(needle), "{needle}: {script}");
    }
    // The page-level IIFE belongs to the HTML export only.
    assert!(!output.contains("})();"), "{output}");
}

#[test]
fn vue_starts_in_view_motion_when_mounted() {
    let vue = Vue.generate(&motion_doc());
    let script = block(&vue, "<script setup lang=\"ts\">\n", "</script>");
    assert!(
        script.starts_with("// generated\nimport { onBeforeUnmount, onMounted } from \"vue\";\n"),
        "{script}"
    );
    assert_in_view_motion_starts(&vue, script);
    assert!(
        script.contains("onMounted(() => {\n  inViewObserver = observeInViewMotion();\n});\n"),
        "{script}"
    );
    assert!(
        script.contains("onBeforeUnmount(() => inViewObserver?.disconnect());"),
        "{script}"
    );
}

#[test]
fn vue_mount_motion_needs_no_observer() {
    let vue = Vue.generate(&mount_only_doc());
    assert!(
        vue.contains("<script setup lang=\"ts\">\n// generated\n</script>"),
        "{vue}"
    );
    assert!(!vue.contains("data-op-in-view"), "{vue}");
}

#[test]
fn svelte_starts_in_view_motion_when_mounted() {
    let svelte = Svelte.generate(&motion_doc());
    let script = block(&svelte, "<script lang=\"ts\">\n", "</script>");
    assert!(
        script.starts_with("// generated\nimport { onMount } from \"svelte\";\n"),
        "{script}"
    );
    assert_in_view_motion_starts(&svelte, script);
    // The cleanup onMount returns runs when the component is destroyed.
    assert!(
        script.contains(
            "onMount(() => {\n  const observer = observeInViewMotion();\n  \
             return () => observer.disconnect();\n});\n"
        ),
        "{script}"
    );
}

#[test]
fn svelte_mount_motion_needs_no_observer() {
    let svelte = Svelte.generate(&mount_only_doc());
    assert!(
        svelte.contains("<script lang=\"ts\">\n// generated\n</script>"),
        "{svelte}"
    );
    assert!(!svelte.contains("data-op-in-view"), "{svelte}");
}
