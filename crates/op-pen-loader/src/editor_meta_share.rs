//! Wire adapter for [`crate::EditorMeta::share_recipe`].
//!
//! The recipe is written as a small camel-case object:
//!
//! ```json
//! "shareRecipe": {
//!   "brief": "…", "family": "presentation", "device": "mobile",
//!   "ratio": "16:9", "infoKind": "data", "styleGuide": "editorial-dark"
//! }
//! ```
//!
//! Like the scenario tag and the style-guide pin it is a hint, never a
//! reason to refuse a file: an object whose `family` is unknown, a value
//! that is not an object, or `null` reads back as `None`, and an unknown
//! option falls back to that option's default. Every read is sanitized
//! again, because a recipe arriving from someone else's file is exactly
//! as untrusted as one typed here.

use op_editor_core::{HomeDevice, HomeFamily, InfoKind, ShareRecipe, SlideRatio};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WireShareRecipe {
    #[serde(default)]
    brief: String,
    #[serde(default)]
    family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    device: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ratio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    info_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    style_guide: Option<String>,
}

impl From<&ShareRecipe> for WireShareRecipe {
    fn from(recipe: &ShareRecipe) -> Self {
        Self {
            brief: recipe.brief.clone(),
            family: recipe.family.id().to_string(),
            device: Some(recipe.device.id().to_string()),
            ratio: Some(recipe.ratio.id().to_string()),
            info_kind: Some(recipe.info_kind.id().to_string()),
            style_guide: recipe.style_guide.clone(),
        }
    }
}

impl WireShareRecipe {
    fn into_recipe(self) -> Option<ShareRecipe> {
        let family = HomeFamily::from_id(&self.family)?;
        let recipe = ShareRecipe {
            brief: String::new(),
            family,
            device: self
                .device
                .as_deref()
                .and_then(HomeDevice::from_id)
                .unwrap_or_default(),
            ratio: self
                .ratio
                .as_deref()
                .and_then(SlideRatio::from_id)
                .unwrap_or_default(),
            info_kind: self
                .info_kind
                .as_deref()
                .and_then(InfoKind::from_id)
                .unwrap_or_default(),
            style_guide: self.style_guide,
        }
        .with_brief(&self.brief, &[]);
        Some(recipe.sanitized(&[]))
    }
}

pub(crate) fn serialize<S: Serializer>(
    value: &Option<ShareRecipe>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(recipe) => WireShareRecipe::from(recipe).serialize(serializer),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ShareRecipe>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    if !value.is_object() {
        return Ok(None);
    }
    Ok(serde_json::from_value::<WireShareRecipe>(value)
        .ok()
        .and_then(WireShareRecipe::into_recipe))
}

#[cfg(test)]
#[path = "editor_meta_share_tests.rs"]
mod tests;
