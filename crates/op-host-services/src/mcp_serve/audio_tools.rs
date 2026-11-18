//! Local audio acquisition plus validated, undoable timeline edits.

use op_editor_core::audio_tracks::{audio_tracks, AudioTrack};
use op_editor_core::motion_edit::MotionCommand;
use op_editor_core::walkers::find_node;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) enum AudioTool {
    List,
    Import,
    Generate,
    Update,
    Remove,
}
impl AudioTool {
    pub const ALL: [Self; 5] = [
        Self::List,
        Self::Import,
        Self::Generate,
        Self::Update,
        Self::Remove,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::List => "list_audio_tracks",
            Self::Import => "import_audio",
            Self::Generate => "generate_voiceover",
            Self::Update => "update_audio_track",
            Self::Remove => "remove_audio_track",
        }
    }
    pub fn snapshot(self, state: &EditorState) -> impl McpTool {
        AudioSnapshot {
            tool: self,
            state: state.clone(),
        }
    }
}
struct AudioSnapshot {
    tool: AudioTool,
    state: EditorState,
}
type Args = BTreeMap<String, String>;
type Error = (ToolErrorCode, String);
fn invalid(message: impl Into<String>) -> Error {
    (ToolErrorCode::InvalidArgument, message.into())
}
fn required<'a>(args: &'a Args, key: &str) -> Result<&'a str, Error> {
    args.get(key)
        .map(String::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| invalid(format!("{key} is required")))
}
fn number<T: std::str::FromStr>(args: &Args, key: &str) -> Result<Option<T>, Error> {
    args.get(key)
        .map(|s| s.parse().map_err(|_| invalid(format!("invalid {key}"))))
        .transpose()
}
fn metadata(track: &AudioTrack) -> serde_json::Value {
    json!({"id":track.id,"name":track.name,"durationMs":track.duration_ms,
        "startMs":track.start_ms,"trimStartMs":track.trim_start_ms,"volume":track.volume,
        "muted":track.muted,"trigger":track.trigger,"clipId":track.clip_id,"narration":track.narration,
        "embedded":true})
}
impl McpTool for AudioSnapshot {
    fn name(&self) -> &str {
        self.tool.name()
    }
    fn call(&self, args: &Args) -> ToolOutcome {
        match self.execute(args) {
            Ok(outcome) => outcome,
            Err((code, detail)) => ToolOutcome::Err(code, detail),
        }
    }
}
impl AudioSnapshot {
    fn execute(&self, args: &Args) -> Result<ToolOutcome, Error> {
        let node_id = NodeId::new(required(args, "nodeId")?);
        let node = find_node(self.state.active_children(), &node_id)
            .ok_or_else(|| invalid("node not found on the active page"))?;
        if !matches!(
            node,
            jian_ops_schema::node::PenNode::Frame(_)
                | jian_ops_schema::node::PenNode::Group(_)
                | jian_ops_schema::node::PenNode::Rectangle(_)
        ) {
            return Err(invalid("audio requires a container node"));
        }
        let mut tracks = audio_tracks(node).to_vec();
        let expected = op_editor_core::audio_tracks::audio_tracks_signature(&tracks);
        if matches!(self.tool, AudioTool::List) {
            return Ok(ToolOutcome::OkJson(json!({"nodeId":node_id.as_str(),"tracks":tracks.iter().map(metadata).collect::<Vec<_>>()}).to_string()));
        }
        let id = args
            .get("trackId")
            .cloned()
            .unwrap_or_else(op_audio::new_track_id);
        let index = tracks.iter().position(|t| t.id == id);
        let mut track = match self.tool {
            AudioTool::Import => {
                op_audio::import_audio(Path::new(required(args, "audioPath")?), &id)
                    .map_err(|e| (ToolErrorCode::ToolFailed, e.to_string()))?
            }
            AudioTool::Generate => {
                use op_editor_core::audio_request::VoiceoverProvider;
                let (provider, rate) = match args.get("provider").map(String::as_str) {
                    None | Some("system") => (VoiceoverProvider::System, 200),
                    Some("neural") => (VoiceoverProvider::Neural, 190),
                    Some(_) => return Err(invalid("provider must be system or neural")),
                };
                op_audio::generate_voiceover_with_provider(
                    required(args, "text")?,
                    args.get("voice").map(String::as_str),
                    number(args, "rate")?.unwrap_or(rate),
                    provider,
                    &id,
                )
                .map_err(|error| (ToolErrorCode::ToolFailed, error.to_string()))?
            }
            AudioTool::Update | AudioTool::Remove => tracks
                .get(index.ok_or_else(|| invalid("trackId was not found"))?)
                .cloned()
                .ok_or_else(|| invalid("trackId was not found"))?,
            AudioTool::List => unreachable!(),
        };
        if let Some(previous) = index.and_then(|i| tracks.get(i)) {
            track.start_ms = previous.start_ms;
            track.volume = previous.volume;
            track.name = previous.name.clone();
            track.muted = previous.muted;
            track.trim_start_ms = previous
                .trim_start_ms
                .min(track.duration_ms.saturating_sub(1));
            track.trigger = previous.trigger;
            track.clip_id = previous.clip_id.clone();
        }
        if let Some(start) = number(args, "startMs")? {
            track.start_ms = start;
        }
        if let Some(trim) = number(args, "trimStartMs")? {
            track.trim_start_ms = trim;
        }
        if let Some(volume) = number(args, "volume")? {
            track.volume = volume;
        }
        if let Some(muted) = number(args, "muted")? {
            track.muted = muted;
        }
        if let Some(name) = args.get("name") {
            track.name = name.clone();
        }
        if let Some(trigger) = args.get("trigger") {
            track.trigger = serde_json::from_value(json!(trigger))
                .map_err(|_| invalid("trigger must be mount, inView or manual"))?;
        }
        if let Some(clip) = args.get("clipId") {
            track.clip_id = Some(clip.clone());
        }
        let metadata = metadata(&track);
        if matches!(self.tool, AudioTool::Remove) {
            tracks.remove(index.unwrap());
        } else if let Some(i) = index {
            tracks[i] = track;
        } else {
            tracks.push(track);
        }
        let command = MotionCommand::SetAudioTracksChecked {
            node_id,
            tracks,
            expected,
        };
        let changed = self
            .state
            .motion_command_changes(&command)
            .map_err(|e| invalid(e.to_string()))?;
        let result = json!({"changed":changed,"track":metadata}).to_string();
        if changed {
            Ok(ToolOutcome::OkJsonWithCommand(
                result,
                EditorCommand::Motion(command),
            ))
        } else {
            Ok(ToolOutcome::OkJson(result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state() -> EditorState {
        EditorState::from_document(
            serde_json::from_value(json!({"version":"1.0","children":[{
                "type":"frame","id":"board","audioTracks":[{"id":"voice","name":"Voice",
                    "source":"data:audio/wav;base64,UklGRg==","durationMs":1000}]
            }]}))
            .unwrap(),
        )
    }
    #[test]
    fn audio_listing_never_echoes_embedded_media_bytes() {
        let output = AudioTool::List
            .snapshot(&state())
            .call(&BTreeMap::from([("nodeId".into(), "board".into())]));
        match output {
            ToolOutcome::OkJson(json) => {
                assert!(json.contains("Voice"));
                assert!(!json.contains("UklGRg"));
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
    #[test]
    fn audio_timing_changes_use_the_validated_editor_command() {
        let mut state = state();
        let output = AudioTool::Update.snapshot(&state).call(&BTreeMap::from([
            ("nodeId".into(), "board".into()),
            ("trackId".into(), "voice".into()),
            ("startMs".into(), "500".into()),
            ("volume".into(), "0.5".into()),
        ]));
        match output {
            ToolOutcome::OkJsonWithCommand(_, command) => assert!(state.apply(command)),
            other => panic!("unexpected result: {other:?}"),
        }
        let track = &audio_tracks(&state.doc.children[0])[0];
        assert_eq!(track.start_ms, 500);
        assert_eq!(track.volume, 0.5);
    }
}
