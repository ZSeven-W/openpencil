//! Normalization for the authoring-only `type:"video"` node alias.

use serde_json::{Map, Value};

const PLAYBACK_KEYS: &[&str] = &[
    "autoplay",
    "loop",
    "muted",
    "holdLastFrame",
    "clickToReplay",
    "videoPrompt",
];

/// Rewrite every authoring video alias in a JSON tree to the canonical
/// image-plus-video shape. The walk is iterative so deeply nested documents
/// cannot overflow the Rust call stack.
pub fn normalize_video_alias(value: &mut Value) -> bool {
    let mut changed = false;
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        match value {
            Value::Object(object) => {
                if object.get("type").and_then(Value::as_str) == Some("video") {
                    normalize_video_object(object);
                    changed = true;
                }
                stack.extend(object.values_mut());
            }
            Value::Array(items) => stack.extend(items.iter_mut()),
            _ => {}
        }
    }
    changed
}

fn normalize_video_object(object: &mut Map<String, Value>) {
    let nested_video = object
        .get("video")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let nested_playback = object
        .get("playback")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let video_src = object
        .get("src")
        .and_then(Value::as_str)
        .or_else(|| object.get("videoSrc").and_then(Value::as_str))
        .or_else(|| nested_video.get("src").and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned();
    let poster = object
        .get("poster")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let mut video = nested_video;
    video.insert("src".into(), Value::String(video_src));
    for key in PLAYBACK_KEYS {
        if !video.contains_key(*key) {
            if let Some(value) = nested_playback.get(*key) {
                video.insert((*key).into(), value.clone());
            }
        }
        if let Some(value) = object.get(*key) {
            video.insert((*key).into(), value.clone());
        }
    }
    // Preserve unknown fields from a nested playback object instead of
    // silently discarding author intent.
    for (key, value) in nested_playback {
        video.entry(key).or_insert(value);
    }

    object.insert("type".into(), Value::String("image".into()));
    object.insert("src".into(), Value::String(poster));
    object.insert("video".into(), Value::Object(video));
    object.remove("poster");
    object.remove("videoSrc");
    object.remove("playback");
    for key in PLAYBACK_KEYS {
        object.remove(*key);
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_video_alias;

    #[test]
    fn nested_aliases_lift_top_level_and_playback_metadata() {
        let mut value = serde_json::json!({
            "type": "frame",
            "id": "root",
            "unknown": "keep",
            "children": [
                {
                    "type": "video",
                    "id": "hero",
                    "src": "hero.mp4",
                    "poster": "hero.png",
                    "autoplay": true,
                    "videoPrompt": "a slow dolly",
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "inner",
                    "children": [{
                        "type": "video",
                        "videoSrc": "fallback.mp4",
                        "poster": "fallback.png",
                        "video": { "src": "nested.mp4", "muted": true },
                        "playback": {
                            "loop": true,
                            "holdLastFrame": true,
                            "clickToReplay": true
                        }
                    }]
                }
            ]
        });

        assert!(normalize_video_alias(&mut value));
        assert_eq!(value["unknown"], "keep");

        let hero = &value["children"][0];
        assert_eq!(hero["type"], "image");
        assert_eq!(hero["src"], "hero.png");
        assert_eq!(hero["video"]["src"], "hero.mp4");
        assert_eq!(hero["video"]["autoplay"], true);
        assert_eq!(hero["video"]["videoPrompt"], "a slow dolly");

        let nested = &value["children"][1]["children"][0];
        assert_eq!(nested["type"], "image");
        assert_eq!(nested["src"], "fallback.png");
        assert_eq!(nested["video"]["src"], "fallback.mp4");
        assert_eq!(nested["video"]["muted"], true);
        assert_eq!(nested["video"]["loop"], true);
        assert_eq!(nested["video"]["holdLastFrame"], true);
        assert_eq!(nested["video"]["clickToReplay"], true);
    }

    #[test]
    fn canonical_image_video_nodes_are_untouched() {
        let mut value = serde_json::json!({
            "type": "image",
            "src": "poster.png",
            "video": { "src": "movie.mp4", "muted": true }
        });
        let before = value.clone();

        assert!(!normalize_video_alias(&mut value));
        assert_eq!(value, before);
    }

    #[test]
    fn missing_alias_sources_default_to_empty_video_and_poster_sources() {
        let mut value = serde_json::json!({
            "type": "video",
            "id": "v",
            "video": { "src": "nested.mp4" }
        });

        normalize_video_alias(&mut value);

        assert_eq!(value["type"], "image");
        assert_eq!(value["src"], "");
        assert_eq!(value["video"]["src"], "nested.mp4");
    }
}
