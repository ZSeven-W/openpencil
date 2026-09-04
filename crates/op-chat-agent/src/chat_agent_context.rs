//! Context hygiene for the builtin tool-executing chat loops.
//!
//! Design loops can call `get_screenshot` more than once.  A screenshot is
//! useful only as the current visual observation; replaying every prior PNG in
//! every later request grows the wire quadratically and can overflow even a
//! million-token model.  This module keeps the latest observation while
//! replacing older inline images with an explicit text tombstone.  Real user
//! and assistant text, tool-call identity, and non-image tool results are left
//! untouched.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use op_ai::chat_provider::ChatToolResult;
use op_orchestrator::{resolve_model_profile, ModelTier};
use serde_json::{json, Value};
use skia_safe::{surfaces, CubicResampler, Data, EncodedImageFormat, Image, Paint, Rect};

pub const ELIDED_SCREENSHOT_TEXT: &str =
    "[Earlier design screenshot elided from context; a newer render supersedes it.]";
pub const OMITTED_SCREENSHOT_TEXT: &str =
    "[Current design screenshot omitted because even the downscaled PNG exceeded the model context budget. Use snapshot_layout or batch_get for a compact structural check.]";
pub const TEXT_ONLY_SCREENSHOT_TEXT: &str =
    "[Screenshot not attached because the selected model only supports text input. Inspect the current design with snapshot_layout or batch_get instead.]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedScreenshot {
    Inline(String),
    TextOnly,
    OverBudget,
}

/// Whether this model accepts inline image input on its chat-completions wire.
///
/// Keep this an explicit deny-list so unknown and known vision models retain
/// the existing multimodal path. GLM's general `glm-*` line is text-only;
/// vision variants are named `glm-<version>v-*` (for example
/// `glm-5v-turbo`). DeepSeek V4 is explicitly text-only. MiniMax M3 is
/// explicitly multimodal and therefore stays enabled.
pub fn model_supports_inline_images(model: &str) -> bool {
    let leaf = model
        .rsplit('/')
        .next()
        .unwrap_or(model)
        .trim()
        .to_ascii_lowercase();
    if leaf.starts_with("deepseek-v4")
        || matches!(leaf.as_str(), "deepseek-chat" | "deepseek-reasoner")
    {
        return false;
    }
    if leaf.starts_with("minimax-m3") {
        return true;
    }
    if leaf.starts_with("glm-") {
        return leaf
            .split('-')
            .nth(1)
            .is_some_and(|version| version.ends_with('v'));
    }
    true
}

/// Convert a successful screenshot result into the model-facing capability
/// decision. Text-only models are rejected before any image block/data URL is
/// built, and over-budget images carry a separate explicit outcome. `None`
/// means this was not a valid successful screenshot result and the ordinary
/// tool-result path should handle it.
pub fn prepare_screenshot_for_context(
    model: &str,
    max_output_tokens: u32,
    tool_name: &str,
    result: &ChatToolResult,
) -> Option<PreparedScreenshot> {
    if tool_name == "get_screenshot" && !result.is_error && !model_supports_inline_images(model) {
        return Some(PreparedScreenshot::TextOnly);
    }
    let base64_png = screenshot_image_base64(tool_name, result)?;
    Some(
        fit_screenshot_to_context(model, max_output_tokens, &base64_png)
            .map(PreparedScreenshot::Inline)
            .unwrap_or(PreparedScreenshot::OverBudget),
    )
}

/// Derive one inline-image character budget from the design turn's explicit
/// output allowance and the existing model tier.  Base64 tokenization varies
/// substantially across OpenAI-compatible providers, so characters are the
/// deterministic wire measure: Full models get at most 64 chars per reserved
/// output token, Standard 42, Basic 21.  The hard ceiling keeps a single image
/// below 384 KiB even when a caller asks for an unusually large completion.
fn inline_screenshot_char_budget(model: &str, max_output_tokens: u32) -> usize {
    let chars_per_output_token = match resolve_model_profile(model).tier {
        ModelTier::Full => 64,
        ModelTier::Standard => 42,
        ModelTier::Basic => 21,
    };
    (max_output_tokens.max(1) as usize)
        .saturating_mul(chars_per_output_token)
        .clamp(32 * 1024, 384 * 1024)
}

/// Keep a screenshot inside the model/profile-derived wire budget.  Small PNGs
/// pass through byte-for-byte. Oversized images are decoded and progressively
/// downscaled (never upscaled), preserving a real visual observation rather
/// than truncating base64 into an invalid image. `None` means decoding or
/// bounded re-encoding failed; callers must send [`OMITTED_SCREENSHOT_TEXT`]
/// instead of replaying the original giant tool result.
pub fn fit_screenshot_to_context(
    model: &str,
    max_output_tokens: u32,
    base64_png: &str,
) -> Option<String> {
    let budget = inline_screenshot_char_budget(model, max_output_tokens);
    if base64_png.len() <= budget {
        return Some(base64_png.to_string());
    }

    let bytes = B64.decode(base64_png).ok()?;
    let src = Image::from_encoded(Data::new_copy(&bytes))?;
    let (width, height) = (src.width(), src.height());
    let longest = width.max(height);
    if width <= 0 || height <= 0 || longest <= 1 {
        return None;
    }

    // Start at a 2048px overview for large artboards. If the source is already
    // below 2048 but compresses poorly, begin at 80% so the first attempt
    // actually shrinks it. Subsequent attempts reduce the long edge by 25%.
    let mut target_longest = if longest > 2048 {
        2048
    } else {
        ((longest as f32 * 0.8).round() as i32).max(1)
    };
    for _ in 0..6 {
        let scale = target_longest as f32 / longest as f32;
        let scaled_width = ((width as f32 * scale).round() as i32).max(1);
        let scaled_height = ((height as f32 * scale).round() as i32).max(1);
        let mut surface = surfaces::raster_n32_premul((scaled_width, scaled_height))?;
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        surface.canvas().draw_image_rect_with_sampling_options(
            &src,
            None,
            Rect::from_xywh(0.0, 0.0, scaled_width as f32, scaled_height as f32),
            CubicResampler::mitchell(),
            &paint,
        );
        let encoded = surface
            .image_snapshot()
            .encode(None, EncodedImageFormat::PNG, 100)?;
        let candidate = B64.encode(encoded.as_bytes());
        if candidate.len() <= budget {
            return Some(candidate);
        }
        if target_longest <= 256 {
            break;
        }
        target_longest = ((target_longest as f32 * 0.75).round() as i32).max(256);
    }
    None
}

/// Extract the PNG from the real canvas-tool result shape.
///
/// Production tool dispatch wraps MCP JSON as
/// `{success:true,data:{image_base64,format}}`; older tests and direct callers
/// may still provide the unwrapped `{image_base64,format}` object.  Supporting
/// both prevents the wrapped base64 from falling through as a giant opaque
/// text tool result.
pub fn screenshot_image_base64(tool_name: &str, result: &ChatToolResult) -> Option<String> {
    if tool_name != "get_screenshot" || result.is_error {
        return None;
    }
    let value = serde_json::from_str::<Value>(&result.content).ok()?;
    let payload = value.get("data").unwrap_or(&value);
    if payload.get("format").and_then(Value::as_str) != Some("png") {
        return None;
    }
    let b64 = payload.get("image_base64").and_then(Value::as_str)?;
    (!b64.is_empty()).then(|| b64.to_string())
}

/// Replace every previously-inline PNG screenshot in `value` with a compact,
/// explicit marker.  The replacement block uses the same multimodal `text`
/// shape accepted by both Anthropic content arrays and OpenAI content parts.
/// Returns the number of images elided.
pub fn elide_inline_screenshots(value: &mut Value) -> usize {
    match value {
        Value::Array(items) => items.iter_mut().map(elide_inline_screenshots).sum(),
        Value::Object(object) => {
            let source = object.get("source").and_then(Value::as_object);
            let anthropic_png = object.get("type").and_then(Value::as_str) == Some("image")
                && source
                    .and_then(|source| source.get("type"))
                    .and_then(Value::as_str)
                    == Some("base64")
                && source
                    .and_then(|source| source.get("media_type"))
                    .and_then(Value::as_str)
                    == Some("image/png");
            let openai_png = object.get("type").and_then(Value::as_str) == Some("image_url")
                && object
                    .get("image_url")
                    .and_then(Value::as_object)
                    .and_then(|image_url| image_url.get("url"))
                    .and_then(Value::as_str)
                    .is_some_and(|url| url.starts_with("data:image/png;base64,"));
            if anthropic_png || openai_png {
                *value = json!({ "type": "text", "text": ELIDED_SCREENSHOT_TEXT });
                return 1;
            }
            object.values_mut().map(elide_inline_screenshots).sum()
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extractor_accepts_production_envelope_and_legacy_raw_shape() {
        for content in [
            json!({ "image_base64": "PNG", "format": "png" }),
            json!({
                "success": true,
                "data": { "image_base64": "PNG", "format": "png" }
            }),
        ] {
            let result = ChatToolResult {
                content: content.to_string(),
                is_error: false,
            };
            assert_eq!(
                screenshot_image_base64("get_screenshot", &result).as_deref(),
                Some("PNG")
            );
        }
    }

    #[test]
    fn model_image_capabilities_match_documented_provider_models() {
        assert!(!model_supports_inline_images("glm-5.2"));
        assert!(!model_supports_inline_images("ark/glm-5.2"));
        assert!(!model_supports_inline_images("deepseek-v4-pro"));
        assert!(!model_supports_inline_images("deepseek-v4-flash"));
        assert!(!model_supports_inline_images("deepseek-chat"));
        assert!(!model_supports_inline_images("deepseek-reasoner"));
        assert!(model_supports_inline_images("deepseek-vl2"));
        assert!(model_supports_inline_images("deepseek-future-vision"));
        assert!(model_supports_inline_images("MiniMax-M3"));
        assert!(model_supports_inline_images("glm-5v-turbo"));
        assert!(model_supports_inline_images("claude-opus-4.8"));
    }

    #[test]
    fn text_only_production_result_becomes_notice_without_base64() {
        let result = ChatToolResult {
            content: json!({
                "success": true,
                "data": { "image_base64": "SECRET_PNG_PAYLOAD", "format": "png" }
            })
            .to_string(),
            is_error: false,
        };
        assert_eq!(
            prepare_screenshot_for_context("glm-5.2", 6_144, "get_screenshot", &result),
            Some(PreparedScreenshot::TextOnly)
        );
        assert!(!TEXT_ONLY_SCREENSHOT_TEXT.contains("SECRET_PNG_PAYLOAD"));
        assert_eq!(
            prepare_screenshot_for_context("deepseek-v4-pro", 6_144, "get_screenshot", &result),
            Some(PreparedScreenshot::TextOnly)
        );
        let malformed = ChatToolResult {
            content: "data:image/png;base64,SHOULD_NEVER_RIDE_AS_TEXT".to_string(),
            is_error: false,
        };
        assert_eq!(
            prepare_screenshot_for_context("glm-5.2", 6_144, "get_screenshot", &malformed),
            Some(PreparedScreenshot::TextOnly),
            "text-only screenshot calls must never fall back to raw tool text"
        );
        assert_eq!(
            prepare_screenshot_for_context("MiniMax-M3", 6_144, "get_screenshot", &result),
            Some(PreparedScreenshot::Inline("SECRET_PNG_PAYLOAD".to_string()))
        );
    }

    #[test]
    fn screenshot_budget_uses_model_tier_and_never_returns_invalid_truncation() {
        assert!(
            inline_screenshot_char_budget("claude-opus-4.8", 6_144)
                > inline_screenshot_char_budget("minimax-m2.7", 6_144)
        );
        let tiny = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
        assert_eq!(
            fit_screenshot_to_context("claude-opus-4.8", 6_144, tiny).as_deref(),
            Some(tiny),
            "an in-budget PNG must pass through byte-for-byte"
        );

        // Oversized, invalid base64 is omitted rather than sliced into an
        // invalid image or replayed as the original giant text payload.
        let invalid = "!".repeat(400 * 1024);
        assert!(fit_screenshot_to_context("claude-opus-4.8", 6_144, &invalid).is_none());
    }

    #[test]
    fn oversized_real_png_is_downscaled_to_the_profile_budget() {
        let (width, height) = (512_i32, 1200_i32);
        let mut seed = 0x1234_5678_u32;
        let mut rgba = vec![0_u8; width as usize * height as usize * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            pixel.copy_from_slice(&[seed as u8, (seed >> 8) as u8, (seed >> 16) as u8, 255]);
        }
        let info = skia_safe::ImageInfo::new(
            (width, height),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Unpremul,
            None,
        );
        let image =
            skia_safe::images::raster_from_data(&info, Data::new_copy(&rgba), width as usize * 4)
                .expect("raster image");
        let source = image
            .encode(None, EncodedImageFormat::PNG, 100)
            .expect("source PNG");
        let source_b64 = B64.encode(source.as_bytes());
        let budget = inline_screenshot_char_budget("minimax-m3", 6_144);
        assert!(source_b64.len() > budget, "fixture must exceed budget");

        let fitted = fit_screenshot_to_context("minimax-m3", 6_144, &source_b64)
            .expect("valid PNG downscales");
        assert!(fitted.len() <= budget, "{}/{}", fitted.len(), budget);
        let decoded = B64.decode(fitted).expect("fitted base64");
        let fitted_image = Image::from_encoded(Data::new_copy(&decoded)).expect("fitted PNG");
        assert!(fitted_image.height() < height);
        assert!(fitted_image.width() < width);
    }

    #[test]
    fn elision_preserves_text_and_tool_identity_for_both_wires() {
        let mut messages = json!([
            { "role": "user", "content": "Build the coffee landing page" },
            {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "toolu_shot",
                    "content": [
                        { "type": "text", "text": "Rendered design:" },
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": "ANTHROPIC_PNG"
                            }
                        }
                    ]
                }]
            },
            {
                "role": "user",
                "content": [{
                    "type": "image_url",
                    "image_url": { "url": "data:image/png;base64,OPENAI_PNG" }
                }]
            }
        ]);

        assert_eq!(elide_inline_screenshots(&mut messages), 2);
        let wire = messages.to_string();
        assert!(wire.contains("Build the coffee landing page"));
        assert!(wire.contains("toolu_shot"));
        assert!(wire.contains(ELIDED_SCREENSHOT_TEXT));
        assert!(!wire.contains("ANTHROPIC_PNG"));
        assert!(!wire.contains("OPENAI_PNG"));
    }
}
