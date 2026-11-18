//! Portable audio-track access and validation, shared by GUI, MCP and export.

use crate::pen_node_ext::PenNodeExt;
pub use jian_ops_schema::audio::{AudioTrack, Narration};
use jian_ops_schema::motion::MotionTrigger;
use jian_ops_schema::node::PenNode;

pub const MAX_AUDIO_TRACKS: usize = 32;
pub const MAX_AUDIO_SOURCE_CHARS: usize = 24 * 1024 * 1024;
pub const MAX_AUDIO_TIMELINE_MS: u64 = 600_000;

/// Optimistic guard for a worker that captured a detached soundtrack list.
pub fn audio_tracks_signature(tracks: &[AudioTrack]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    tracks.len().hash(&mut hash);
    for t in tracks {
        t.id.hash(&mut hash);
        t.name.hash(&mut hash);
        t.source.hash(&mut hash);
        t.duration_ms.hash(&mut hash);
        t.start_ms.hash(&mut hash);
        t.trim_start_ms.hash(&mut hash);
        t.volume.to_bits().hash(&mut hash);
        t.muted.hash(&mut hash);
        std::mem::discriminant(&t.trigger).hash(&mut hash);
        t.clip_id.hash(&mut hash);
        if let Some(n) = &t.narration {
            n.provider.hash(&mut hash);
            n.text.hash(&mut hash);
            n.voice.hash(&mut hash);
            n.rate.hash(&mut hash);
        }
    }
    hash.finish()
}

pub fn audio_tracks(node: &PenNode) -> &[AudioTrack] {
    match node {
        PenNode::Frame(n) => n.container.audio_tracks.as_deref(),
        PenNode::Group(n) => n.container.audio_tracks.as_deref(),
        PenNode::Rectangle(n) => n.container.audio_tracks.as_deref(),
        _ => None,
    }
    .unwrap_or_default()
}

pub fn audio_tracks_mut(node: &mut PenNode) -> Option<&mut Option<Vec<AudioTrack>>> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container.audio_tracks),
        PenNode::Group(n) => Some(&mut n.container.audio_tracks),
        PenNode::Rectangle(n) => Some(&mut n.container.audio_tracks),
        _ => None,
    }
}

pub fn audio_matches(track: &AudioTrack, trigger: &jian_core::motion::ClipTrigger) -> bool {
    use jian_core::motion::ClipTrigger;
    match trigger {
        ClipTrigger::Mount => track.trigger == MotionTrigger::Mount,
        ClipTrigger::InView => track.trigger == MotionTrigger::InView,
        ClipTrigger::Manual(id) => {
            track.trigger == MotionTrigger::Manual && track.clip_id.as_ref() == Some(id)
        }
    }
}

pub fn collect_audio_tracks<'a>(
    root: &'a PenNode,
    trigger: &jian_core::motion::ClipTrigger,
) -> Vec<(&'a str, &'a AudioTrack)> {
    fn walk<'a>(
        node: &'a PenNode,
        trigger: &jian_core::motion::ClipTrigger,
        out: &mut Vec<(&'a str, &'a AudioTrack)>,
    ) {
        if node.base().visible == Some(false) {
            return;
        }
        out.extend(
            audio_tracks(node)
                .iter()
                .filter(|track| audio_matches(track, trigger))
                .map(|track| (node.base().id.as_str(), track)),
        );
        for child in node.children_ref().into_iter().flatten() {
            walk(child, trigger, out);
        }
    }
    let mut tracks = Vec::new();
    walk(root, trigger, &mut tracks);
    tracks
}

pub fn audio_duration_ms(root: &PenNode, trigger: &jian_core::motion::ClipTrigger) -> u64 {
    collect_audio_tracks(root, trigger)
        .into_iter()
        .filter(|(_, track)| !track.muted)
        .map(|(_, track)| {
            track
                .start_ms
                .saturating_add(track.duration_ms.saturating_sub(track.trim_start_ms))
        })
        .max()
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioTrackError {
    EmptyId,
    InvalidSource,
    InvalidTiming,
    InvalidVolume,
    ManualClipMissing,
    TooManyTracks,
    DuplicateId,
    SourceTooLarge,
    InvalidNarration,
}

impl std::fmt::Display for AudioTrackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EmptyId => "audio track id and name must not be empty",
            Self::InvalidSource => "audio source must be an embedded base64 audio data URI",
            Self::InvalidTiming => {
                "audio timing must be within ten minutes with a positive duration"
            }
            Self::InvalidVolume => "audio volume must be a finite number between 0 and 2",
            Self::ManualClipMissing => "manual audio requires a clip id",
            Self::TooManyTracks => "a container supports at most 32 audio tracks",
            Self::DuplicateId => "audio track ids must be unique within a container",
            Self::SourceTooLarge => "embedded audio exceeds the 24 MiB source limit",
            Self::InvalidNarration => "narration needs text and a speech rate between 80 and 400",
        })
    }
}

impl std::error::Error for AudioTrackError {}

pub fn validate_audio_tracks(tracks: &[AudioTrack]) -> Result<(), AudioTrackError> {
    if tracks.len() > MAX_AUDIO_TRACKS {
        return Err(AudioTrackError::TooManyTracks);
    }
    let mut ids = std::collections::HashSet::new();
    let mut source_chars = 0usize;
    for track in tracks {
        if track.id.trim().is_empty() || track.name.trim().is_empty() {
            return Err(AudioTrackError::EmptyId);
        }
        if !ids.insert(&track.id) {
            return Err(AudioTrackError::DuplicateId);
        }
        let Some((header, data)) = track.source.split_once(',') else {
            return Err(AudioTrackError::InvalidSource);
        };
        if !header.starts_with("data:audio/")
            || !header.ends_with(";base64")
            || data.is_empty()
            || !data.len().is_multiple_of(4)
            || !data
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
        {
            return Err(AudioTrackError::InvalidSource);
        }
        source_chars = source_chars.saturating_add(track.source.len());
        if source_chars > MAX_AUDIO_SOURCE_CHARS {
            return Err(AudioTrackError::SourceTooLarge);
        }
        if track.duration_ms == 0
            || track.duration_ms > MAX_AUDIO_TIMELINE_MS
            || track.start_ms > MAX_AUDIO_TIMELINE_MS
            || track.trim_start_ms >= track.duration_ms
            || track
                .start_ms
                .checked_add(track.duration_ms - track.trim_start_ms)
                .is_none_or(|end| end > MAX_AUDIO_TIMELINE_MS)
        {
            return Err(AudioTrackError::InvalidTiming);
        }
        if !track.volume.is_finite() || !(0.0..=2.0).contains(&track.volume) {
            return Err(AudioTrackError::InvalidVolume);
        }
        if track.trigger == MotionTrigger::Manual
            && track
                .clip_id
                .as_deref()
                .is_none_or(|id| id.trim().is_empty())
        {
            return Err(AudioTrackError::ManualClipMissing);
        }
        if track.narration.as_ref().is_some_and(|n| {
            n.text.trim().is_empty()
                || n.text.chars().count() > 10_000
                || !(80..=400).contains(&n.rate)
        }) {
            return Err(AudioTrackError::InvalidNarration);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "audio_tracks_tests.rs"]
mod tests;
