//! Editor-side motion export choices and the request shared by every host.

use std::collections::BTreeSet;

use jian_core::motion::{Clip, ClipTrigger};
use jian_ops_schema::motion::{MotionPreference, MotionTrigger};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use serde::{Deserialize, Serialize};

use crate::{pen_node_ext::PenNodeExt, walkers::find_node, EditorState};

pub const MAX_MOTION_EXPORT_FRAMES: u32 = 600;
pub const MAX_MP4_DURATION_MS: u64 = 600_000;
const DEFAULT_LOOP_DURATION_MS: u64 = 3_000;

pub fn motion_frame_count(duration_ms: u64, fps: u32) -> u64 {
    duration_ms
        .saturating_mul(u64::from(fps))
        .div_ceil(1_000)
        .saturating_add(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MotionExportFormat {
    Gif,
    Apng,
    #[serde(rename = "png-seq")]
    PngSequence,
    Mp4,
}

impl MotionExportFormat {
    pub const ALL: [Self; 4] = [Self::Gif, Self::Apng, Self::PngSequence, Self::Mp4];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Gif => "GIF",
            Self::Apng => "APNG",
            Self::PngSequence => "PNG ZIP",
            Self::Mp4 => "MP4",
        }
    }

    pub const fn supports_repeat(self) -> bool {
        matches!(self, Self::Gif | Self::Apng)
    }

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::Apng => "apng",
            Self::PngSequence => "zip",
            Self::Mp4 => "mp4",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MotionExportBackground {
    #[default]
    Transparent,
    White,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum MotionExportTrigger {
    Mount,
    InView,
    Manual(String),
}

impl MotionExportTrigger {
    pub fn as_clip_trigger(&self) -> ClipTrigger {
        match self {
            Self::Mount => ClipTrigger::Mount,
            Self::InView => ClipTrigger::InView,
            Self::Manual(id) => ClipTrigger::Manual(id.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionExportChoice {
    pub root_id: String,
    pub root_name: String,
    pub trigger: MotionExportTrigger,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionExportRequest {
    pub root_id: String,
    pub format: MotionExportFormat,
    pub trigger: MotionExportTrigger,
    pub fps: u32,
    pub scale: f32,
    pub duration_ms: Option<u64>,
    pub repeat: bool,
    #[serde(default)]
    pub background: MotionExportBackground,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_motion: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotionExportDialogState {
    pub format: Option<MotionExportFormat>,
    pub mp4_available: bool,
    pub choice: usize,
    pub fps: u32,
    pub duration_ms: Option<u64>,
    pub repeat: bool,
    pub background: MotionExportBackground,
    pub reduced_motion: Option<bool>,
    pub pending: Option<MotionExportRequest>,
}

impl Default for MotionExportDialogState {
    fn default() -> Self {
        Self {
            format: None,
            mp4_available: false,
            choice: 0,
            fps: 30,
            duration_ms: None,
            repeat: true,
            background: MotionExportBackground::Transparent,
            reduced_motion: None,
            pending: None,
        }
    }
}

impl MotionExportDialogState {
    pub fn effective_duration(&self, choice: &MotionExportChoice) -> u64 {
        self.duration_ms
            .or(choice.duration_ms)
            .unwrap_or(DEFAULT_LOOP_DURATION_MS)
    }

    pub fn request(
        &self,
        choices: &[MotionExportChoice],
        scale: f32,
    ) -> Option<MotionExportRequest> {
        let format = self.format?;
        if format == MotionExportFormat::Mp4 && !self.mp4_available {
            return None;
        }
        let choice = choices.get(self.choice).or_else(|| choices.first())?;
        Some(MotionExportRequest {
            root_id: choice.root_id.clone(),
            format,
            trigger: choice.trigger.clone(),
            fps: self.fps,
            scale,
            duration_ms: self.duration_ms.or_else(|| {
                choice
                    .duration_ms
                    .is_none()
                    .then_some(DEFAULT_LOOP_DURATION_MS)
            }),
            repeat: self.repeat,
            background: self.background,
            reduced_motion: self.reduced_motion,
        })
    }
}

/// An explicit export option overrides the authored document preference.
pub fn export_reduced_motion(document: &PenDocument, selected: Option<bool>) -> bool {
    selected.unwrap_or(document.motion == Some(MotionPreference::Reduced))
}

/// Export a single selected subtree, or choose a clip from the page's boards.
pub fn motion_export_choices(state: &EditorState) -> Vec<MotionExportChoice> {
    let children = state.active_children();
    let roots = if state.selection_count() == 1 && state.selection.anchor.is_real() {
        find_node(children, &state.selection.anchor).map_or(&[][..], std::slice::from_ref)
    } else {
        children
    };
    let mut choices = Vec::new();
    for root in roots {
        let mut triggers = BTreeSet::new();
        collect_visible_triggers(root, &mut triggers);
        for trigger in triggers {
            let clip = Clip::build(root, &trigger.as_clip_trigger());
            let audio_ms = crate::audio_tracks::audio_duration_ms(root, &trigger.as_clip_trigger());
            if !clip.is_empty() || audio_ms > 0 {
                choices.push(MotionExportChoice {
                    root_id: root.id_str().to_owned(),
                    root_name: root
                        .base()
                        .name
                        .as_deref()
                        .filter(|name| !name.is_empty())
                        .unwrap_or(root.id_str())
                        .to_owned(),
                    trigger,
                    duration_ms: clip
                        .duration_ms()
                        .map(|ms| ms.max(audio_ms))
                        .or((audio_ms > 0).then_some(audio_ms)),
                });
            }
        }
    }
    choices
}

fn collect_visible_triggers(node: &PenNode, triggers: &mut BTreeSet<MotionExportTrigger>) {
    if node.base().visible == Some(false) {
        return;
    }
    for track in crate::audio_tracks::audio_tracks(node)
        .iter()
        .filter(|track| !track.muted)
    {
        let trigger = match track.trigger {
            MotionTrigger::Mount => MotionExportTrigger::Mount,
            MotionTrigger::InView => MotionExportTrigger::InView,
            MotionTrigger::Manual => {
                let Some(id) = &track.clip_id else {
                    continue;
                };
                MotionExportTrigger::Manual(id.clone())
            }
        };
        triggers.insert(trigger);
    }
    for clip in node.motion_declarations().1.into_iter().flatten() {
        let trigger = match clip.trigger {
            MotionTrigger::Mount => MotionExportTrigger::Mount,
            MotionTrigger::InView => MotionExportTrigger::InView,
            MotionTrigger::Manual => {
                let Some(id) = &clip.id else { continue };
                MotionExportTrigger::Manual(id.clone())
            }
        };
        triggers.insert(trigger);
    }
    for child in node.children().into_iter().flatten() {
        collect_visible_triggers(child, triggers);
    }
}
