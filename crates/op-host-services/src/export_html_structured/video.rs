//! Structured HTML emission for an image node's video placeholder.

use op_editor_ui::layout_scene::{SceneImageFit, SceneNode};
use op_util::xml_escape::escape_html;
use std::fmt::Write as _;

pub(super) fn emit(out: &mut String, node: &SceneNode) {
    let Some(video) = node.video.as_ref() else {
        return;
    };
    let object_fit = match node.image_fit {
        SceneImageFit::Fit => "contain",
        SceneImageFit::Stretch => "fill",
        SceneImageFit::Fill | SceneImageFit::Crop | SceneImageFit::Tile => "cover",
    };
    let poster = node.image_src.as_deref().filter(|src| {
        src.starts_with("data:") || src.starts_with("http://") || src.starts_with("https://")
    });
    let autoplay = video.autoplay;
    let muted = video.muted || autoplay;
    let loop_video = video.r#loop && !video.hold_last_frame;

    let _ = write!(out, "<video src=\"{}\"", escape_html(video.src.as_ref()));
    if let Some(poster) = poster {
        let _ = write!(out, " poster=\"{}\"", escape_html(poster));
    }
    let _ = write!(
        out,
        " playsinline preload=\"metadata\" style=\"position:absolute;inset:0;width:100%;height:100%;object-fit:{};\"",
        object_fit
    );
    if autoplay {
        out.push_str(" autoplay");
    }
    // holdLastFrame wins over loop: a non-looping video naturally retains its
    // final decoded frame after `ended`, so no script is needed.
    if loop_video {
        out.push_str(" loop");
    }
    // Browsers block autoplay without muted, so autoplay always implies it.
    if muted {
        out.push_str(" muted");
    }
    if video.click_to_replay {
        out.push_str(" onclick=\"this.currentTime=0;this.play()\"");
    }
    out.push_str("></video>");
}

#[cfg(test)]
mod tests {
    use super::emit;
    use op_editor_ui::layout_scene::{NodeKind, SceneImageFit, SceneNode, SceneVideo};

    fn video_node() -> SceneNode {
        let mut node = SceneNode::leaf("video", NodeKind::Rect);
        node.image_src = Some("data:image/png;base64,AA==".into());
        node.image_fit = SceneImageFit::Fit;
        node.video = Some(SceneVideo {
            src: "https://cdn.example/movie.mp4\"quoted".into(),
            autoplay: true,
            r#loop: true,
            muted: false,
            hold_last_frame: false,
            click_to_replay: true,
        });
        node
    }

    #[test]
    fn emits_video_playback_attributes_and_escapes_src() {
        let mut out = String::new();
        emit(&mut out, &video_node());
        assert!(out.contains("<video src=\"https://cdn.example/movie.mp4&quot;quoted\""));
        assert!(out.contains("poster=\"data:image/png"));
        assert!(out.contains("object-fit:contain"));
        assert!(out.contains(" autoplay"));
        assert!(out.contains(" loop"));
        assert!(out.contains(" muted"));
        assert!(out.contains("onclick=\"this.currentTime=0;this.play()\""));
    }

    #[test]
    fn hold_last_frame_suppresses_loop() {
        let mut node = video_node();
        node.video.as_mut().expect("video").hold_last_frame = true;
        let mut out = String::new();
        emit(&mut out, &node);
        assert!(!out.contains(" loop"));
    }
}
