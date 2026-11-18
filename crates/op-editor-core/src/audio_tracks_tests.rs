use super::*;
use crate::motion_edit::MotionCommand;
use crate::{EditorState, NodeId};

fn track() -> AudioTrack {
    serde_json::from_value(serde_json::json!({
        "id":"voice1", "name":"Narration", "source":"data:audio/wav;base64,UklGRg==",
        "durationMs":3000, "startMs":1000, "narration":{"text":"你好","rate":180}
    }))
    .unwrap()
}

fn state() -> EditorState {
    EditorState::from_document(
        serde_json::from_value(serde_json::json!({
            "version":"1.0.0","children":[{"id":"board","type":"frame","width":100,"height":100}]
        }))
        .unwrap(),
    )
}

#[test]
fn embedded_audio_survives_save_load_and_undo() {
    let mut state = state();
    let audio = track();
    assert!(state
        .apply_motion_edits(vec![MotionCommand::SetAudioTracks {
            node_id: NodeId::new("board"),
            tracks: vec![audio.clone()]
        }])
        .unwrap());
    let loaded: jian_ops_schema::PenDocument =
        serde_json::from_str(&serde_json::to_string(&state.doc).unwrap()).unwrap();
    assert_eq!(audio_tracks(&loaded.children[0]), &[audio]);
    assert!(state.undo());
    assert!(audio_tracks(&state.doc.children[0]).is_empty());
    assert!(state.redo());
    assert_eq!(audio_tracks(&state.doc.children[0]).len(), 1);
}

#[test]
fn invalid_audio_leaves_an_entire_multi_edit_unchanged() {
    let mut state = state();
    let original = state.doc.clone();
    let mut bad = track();
    bad.volume = f32::NAN;
    assert!(state
        .apply_motion_edits(vec![
            MotionCommand::SetAudioTracks {
                node_id: NodeId::new("board"),
                tracks: vec![track()]
            },
            MotionCommand::SetAudioTracks {
                node_id: NodeId::new("board"),
                tracks: vec![bad]
            }
        ])
        .is_err());
    assert_eq!(state.doc, original);
    assert_eq!(state.history.past.len(), 0);
}

#[test]
fn audio_validation_rejects_paths_duplicate_ids_and_invalid_trims() {
    let mut bad = track();
    bad.source = "/tmp/private.wav".into();
    assert_eq!(
        validate_audio_tracks(&[bad]),
        Err(AudioTrackError::InvalidSource)
    );
    assert_eq!(
        validate_audio_tracks(&[track(), track()]),
        Err(AudioTrackError::DuplicateId)
    );
    let mut bad = track();
    bad.trim_start_ms = bad.duration_ms;
    assert_eq!(
        validate_audio_tracks(&[bad]),
        Err(AudioTrackError::InvalidTiming)
    );
    let mut bad = track();
    bad.trigger = MotionTrigger::Manual;
    assert_eq!(
        validate_audio_tracks(&[bad]),
        Err(AudioTrackError::ManualClipMissing)
    );
}

#[test]
fn an_audio_only_board_has_an_export_choice_and_timeline_duration() {
    let mut state = state();
    state
        .apply_motion_edits(vec![MotionCommand::SetAudioTracks {
            node_id: NodeId::new("board"),
            tracks: vec![track()],
        }])
        .unwrap();
    let choices = crate::motion_export::motion_export_choices(&state);
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].duration_ms, Some(4000));
    assert_eq!(state.motion_timeline_projection().clip_ms, 4000.0);
}

#[test]
fn a_voiceover_request_freezes_selected_text_target_and_time() {
    let mut state = state();
    if let PenNode::Frame(board) = &mut state.doc.children[0] {
        board.children = Some(vec![serde_json::from_value(serde_json::json!({
            "type":"text","id":"words","content":"你好，我是 Kayshen。"
        }))
        .unwrap()]);
    }
    state.selection.set = vec![NodeId::new("words")];
    state.selection.anchor = NodeId::new("words");
    state.editor_ui.motion_timeline.playhead_ms = 1250.0;
    state.request_native_audio(Some(crate::audio_request::VoiceoverProvider::System));
    state.selection = Default::default();
    let request = state
        .editor_ui
        .motion_timeline
        .audio_request
        .as_ref()
        .unwrap();
    assert_eq!(request.node_id, "board");
    assert_eq!(request.start_ms, 1250);
    assert_eq!(request.text.as_deref(), Some("你好，我是 Kayshen。"));
}

#[test]
fn a_detached_audio_write_cannot_replace_newer_tracks() {
    let mut state = state();
    let expected = audio_tracks_signature(&[]);
    state
        .apply_motion_edits(vec![MotionCommand::SetAudioTracks {
            node_id: NodeId::new("board"),
            tracks: vec![track()],
        }])
        .unwrap();
    let before = state.doc.clone();
    assert!(state
        .apply_motion_edits(vec![MotionCommand::SetAudioTracksChecked {
            node_id: NodeId::new("board"),
            tracks: vec![],
            expected
        }])
        .is_err());
    assert_eq!(state.doc, before);
}

#[test]
fn a_stale_audio_guard_rejects_a_whole_multi_edit() {
    let mut state = state();
    let before = state.doc.clone();
    assert!(state
        .apply_motion_edits(vec![
            MotionCommand::SetAudioTracks {
                node_id: NodeId::new("board"),
                tracks: vec![track()]
            },
            MotionCommand::SetAudioTracksChecked {
                node_id: NodeId::new("board"),
                tracks: vec![],
                expected: audio_tracks_signature(&[])
            },
        ])
        .is_err());
    assert_eq!(state.doc, before);
}
