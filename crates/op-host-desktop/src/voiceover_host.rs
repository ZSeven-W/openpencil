//! Native timeline audio: pick files or synthesize selected text on a worker.

use crate::{DesktopApp, DesktopEvent};
use op_editor_core::audio_request::NativeAudioRequest;
use op_editor_core::audio_tracks::{audio_tracks, AudioTrack};
use op_editor_core::motion_edit::MotionCommand;
use op_editor_core::walkers::find_node;
use op_editor_core::{
    CollabDocumentMutation, CollabEditSource, CollabGateAction, CollabUnsupportedFeature, NodeId,
};
use std::sync::mpsc::{self, Receiver};

pub(crate) struct AudioJob {
    epoch: u64,
    request: NativeAudioRequest,
    result: Receiver<Result<AudioTrack, op_audio::AudioError>>,
}

fn allowed(app: &mut DesktopApp) -> bool {
    app.host.gate_collaboration_action(
        CollabGateAction::Document(CollabDocumentMutation::Unsupported(
            CollabUnsupportedFeature::UnsupportedNodeProperty,
        )),
        CollabEditSource::User,
    )
}

fn show_error(app: &DesktopApp, detail: &str) {
    let title = op_i18n::translate(app.host.editor_state().editor_ui.locale, "audio.title");
    crate::message_dialog::alert(title, detail, rfd::MessageLevel::Error);
}

pub(crate) fn drain(app: &mut DesktopApp) -> bool {
    let mut changed = false;
    if let Some(job) = app.voiceover_job.take() {
        match job.result.try_recv() {
            Ok(result) => {
                app.host
                    .editor_state_mut()
                    .editor_ui
                    .motion_timeline
                    .audio_busy = false;
                if job.epoch == app.host.document_epoch()
                    && job.request.document_generation
                        == app.host.editor_state().document_generation()
                    && app
                        .host
                        .editor_state()
                        .audio_request_target_exists(&job.request)
                    && allowed(app)
                {
                    match result {
                        Ok(mut track) => {
                            track.start_ms = job.request.start_ms;
                            (track.trigger, track.clip_id) = match &job.request.trigger {
                                jian_core::motion::ClipTrigger::Mount => {
                                    (jian_ops_schema::motion::MotionTrigger::Mount, None)
                                }
                                jian_core::motion::ClipTrigger::InView => {
                                    (jian_ops_schema::motion::MotionTrigger::InView, None)
                                }
                                jian_core::motion::ClipTrigger::Manual(id) => (
                                    jian_ops_schema::motion::MotionTrigger::Manual,
                                    Some(id.clone()),
                                ),
                            };
                            let state = app.host.editor_state_mut();
                            let id = NodeId::new(&job.request.node_id);
                            let mut tracks = find_node(state.active_children(), &id)
                                .map(audio_tracks)
                                .unwrap_or_default()
                                .to_vec();
                            tracks.push(track);
                            if let Err(error) =
                                state.apply_motion_edits(vec![MotionCommand::SetAudioTracks {
                                    node_id: id,
                                    tracks,
                                }])
                            {
                                show_error(app, &error.to_string());
                            }
                        }
                        Err(error) => show_error(app, &error.to_string()),
                    }
                }
                changed = true;
            }
            Err(mpsc::TryRecvError::Empty) => app.voiceover_job = Some(job),
            Err(mpsc::TryRecvError::Disconnected) => {
                app.host
                    .editor_state_mut()
                    .editor_ui
                    .motion_timeline
                    .audio_busy = false;
                show_error(app, "The audio worker stopped before returning a result.");
                changed = true;
            }
        }
    }
    if app.voiceover_job.is_none() {
        if let Some(request) = app
            .host
            .editor_state_mut()
            .editor_ui
            .motion_timeline
            .audio_request
            .take()
        {
            changed = true;
            if request.document_generation != app.host.editor_state().document_generation()
                || !allowed(app)
            {
                return true;
            }
            if request.text.as_ref().is_some_and(|s| s.trim().is_empty()) {
                show_error(
                    app,
                    op_i18n::translate(
                        app.host.editor_state().editor_ui.locale,
                        "audio.selectText",
                    ),
                );
                return true;
            }
            let path = if request.text.is_none() {
                rfd::FileDialog::new()
                    .set_title(op_i18n::translate(
                        app.host.editor_state().editor_ui.locale,
                        "audio.import",
                    ))
                    .add_filter(
                        "Audio",
                        &["wav", "mp3", "m4a", "aac", "aif", "aiff", "ogg", "flac"],
                    )
                    .pick_file()
            } else {
                None
            };
            if request.text.is_none() && path.is_none() {
                return true;
            }
            let (tx, rx) = mpsc::channel();
            let text = request.text.clone();
            let provider = request.provider;
            let proxy = app.mcp_wake_proxy.clone();
            match std::thread::Builder::new()
                .name("op-voiceover".into())
                .spawn(move || {
                    let id = op_audio::new_track_id();
                    let result = match (text, provider, path) {
                        (Some(text), Some(provider), _) => {
                            let rate = match provider {
                                op_editor_core::audio_request::VoiceoverProvider::System => 200,
                                op_editor_core::audio_request::VoiceoverProvider::Neural => 190,
                            };
                            op_audio::generate_voiceover_with_provider(
                                &text, None, rate, provider, &id,
                            )
                            .map(|mut track| {
                                track.name = text.chars().take(24).collect();
                                track
                            })
                        }
                        (_, _, Some(path)) => op_audio::import_audio(&path, &id),
                        _ => return,
                    };
                    let _ = tx.send(result);
                    if let Some(proxy) = proxy {
                        let _ = proxy.send_event(DesktopEvent::AudioReady);
                    }
                }) {
                Ok(_) => {
                    app.host
                        .editor_state_mut()
                        .editor_ui
                        .motion_timeline
                        .audio_busy = true;
                    app.voiceover_job = Some(AudioJob {
                        epoch: app.host.document_epoch(),
                        request,
                        result: rx,
                    });
                }
                Err(error) => show_error(app, &error.to_string()),
            }
        }
    }
    if changed {
        app.host.mark_editor_state_dirty();
    }
    changed
}
