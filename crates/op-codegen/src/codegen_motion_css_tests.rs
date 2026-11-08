//! HTML motion export tests, carved off `tests.rs` to keep it under the
//! 800-line cap.

use crate::tests::empty_doc;
use crate::{Codegen, Html};

fn html_style_block(html: &str) -> &str {
    let start = html.find("<style>\n").expect("motion style block") + "<style>\n".len();
    let end = html[start..].find("</style>").expect("style close") + start;
    &html[start..end]
}

#[test]
fn html_motion_export_matches_the_golden_css() {
    let doc = jian_ops_schema::load_str(
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
             ],"durationMs":320,"delayMs":60,"easing":"standard","once":true}]}
          ]
        }"#,
    )
    .expect("motion document")
    .value;
    let html = Html.generate(&doc);
    let expected = concat!(
        "@keyframes op-hero-0 {\n",
        "  0% {\n",
        "    opacity: 0;\n",
        "    transform: translate(0px, 16px);\n",
        "    animation-timing-function: cubic-bezier(0.05,0.7,0.1,1);\n",
        "  }\n",
        "  100% {\n",
        "    opacity: 1;\n",
        "    transform: translate(0px, 0px);\n",
        "  }\n",
        "}\n",
        "@keyframes op-card-0 {\n",
        "  0% {\n",
        "    opacity: 0;\n",
        "    transform: translate(0px, 12px);\n",
        "    animation-timing-function: cubic-bezier(0.2,0,0,1);\n",
        "  }\n",
        "  100% {\n",
        "    opacity: 1;\n",
        "    transform: translate(0px, 0px);\n",
        "  }\n",
        "}\n",
        "@media (prefers-reduced-motion: reduce) {\n",
        "  [data-op-motion-node=\"hero\"] {\n",
        "    animation: none;\n",
        "    transition: none;\n",
        "  }\n",
        "  [data-op-motion-node=\"card\"] {\n",
        "    animation: none;\n",
        "    transition: none;\n",
        "  }\n",
        "}\n"
    );
    assert_eq!(html_style_block(&html), expected);
    assert!(html.contains("data-op-motion-node=\"hero\""));
    // The curve lives in the keyframes, so the shorthand times linearly.
    assert!(html.contains("animation:op-hero-0 400ms linear 0ms 1 both"));
    assert!(html.contains("transition:opacity 150ms cubic-bezier(0.2,0,0,1)"));
    // In-view motion waits on its first keyframe; an IntersectionObserver
    // starts it like Preview, instead of a scroll-driven timeline.
    assert!(!html.contains("animation-timeline"));
    assert!(html.contains("animation:op-card-0 320ms linear 60ms 1 both paused"));
    assert!(html.contains("data-op-in-view=\"op-card-0:once\""));
    assert!(html.contains("new IntersectionObserver("));
    assert!(html.contains("{ threshold: 0.1 }"));
}

#[test]
fn html_motion_export_honours_authored_reduced_preference() {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0","motion":"reduced","children":[
          {"type":"rectangle","id":"n1","animations":[
            {"trigger":"mount","keyframes":[{"offset":0,"values":{"opacity":0}},
            {"offset":1,"values":{"opacity":1}}],"durationMs":300}
          ]}
        ]}"#,
    )
    .expect("reduced motion document")
    .value;
    let html = Html.generate(&doc);
    let css = html_style_block(&html);
    assert!(css.contains("/* Authored motion preference: reduced */"));
    assert!(
        css.contains("[data-op-motion-node=\"n1\"] {\n  animation: none;\n  transition: none;\n}")
    );
}

#[test]
fn html_mount_motion_needs_no_observer() {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0","children":[
          {"type":"rectangle","id":"n1","animations":[
            {"trigger":"mount","once":false,"keyframes":[{"offset":0,"values":{"opacity":0}},
            {"offset":1,"values":{"opacity":1}}],"durationMs":300}
          ]}
        ]}"#,
    )
    .expect("mount motion document")
    .value;
    let html = Html.generate(&doc);
    assert!(!html.contains("<script>"));
    assert!(!html.contains("data-op-in-view"));
    assert!(!html.contains("paused"));
}

#[test]
fn html_without_motion_has_no_extra_css() {
    let html = Html.generate(&empty_doc());
    assert!(!html.contains("<style>"));
    assert!(!html.contains("data-op-motion-node"));
    assert!(!html.contains("@keyframes"));
}

#[test]
fn html_motion_keyframes_sit_where_preview_reaches_them() {
    // Preview eases the whole animation, so with easeIn (p²) the keyframe
    // at offset 0.25 is reached half way through, not a quarter of the way.
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0","children":[
          {"type":"rectangle","id":"dot","animations":[
            {"trigger":"mount","keyframes":[
              {"offset":0,"values":{"opacity":0}},
              {"offset":0.25,"values":{"opacity":1}},
              {"offset":1,"values":{"opacity":0.5}}
            ],"durationMs":400,"easing":"easeIn"}
          ]}
        ]}"#,
    )
    .expect("three-stop document")
    .value;
    let css = html_style_block(&Html.generate(&doc)).to_owned();
    assert!(css.contains("  50% {\n    opacity: 1;\n"), "{css}");
    assert!(!css.contains("  25% {"), "{css}");
    assert_eq!(css.matches("animation-timing-function").count(), 2, "{css}");
}

#[test]
fn html_manual_motion_waits_for_an_interaction() {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0","children":[
          {"type":"rectangle","id":"card","animations":[
            {"trigger":"manual","id":"pulse","keyframes":[{"offset":0,"values":{"scaleX":1}},
            {"offset":1,"values":{"scaleX":1.1}}],"durationMs":200},
            {"trigger":"mount","keyframes":[{"offset":0,"values":{"opacity":0}},
            {"offset":1,"values":{"opacity":1}}],"durationMs":300}
          ]},
          {"type":"rectangle","id":"only","animations":[
            {"trigger":"manual","id":"shake","keyframes":[{"offset":0,"values":{"translateX":0}},
            {"offset":1,"values":{"translateX":8}}],"durationMs":200}
          ]}
        ]}"#,
    )
    .expect("manual motion document")
    .value;
    let html = Html.generate(&doc);
    // The keyframes keep their declaration-index names for a later trigger,
    // but only the mount animation plays.
    assert!(html.contains("@keyframes op-card-0 {"));
    assert!(html.contains("animation:op-card-1 300ms linear 0ms 1 both\""));
    assert!(!html.contains("animation:op-card-0"));
    assert!(!html.contains("animation:op-only-0"));
    assert!(!html.contains(";animation:\""));
}
