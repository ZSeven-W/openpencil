//! Prepare timeline audio off-thread; seek and stop with the shared play clock.

use crate::DesktopEvent;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_host_native::WidgetHostNative;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{self, Receiver};
use winit::event_loop::EventLoopProxy;

type Observation = (u64, u64, u64, String, String);

#[derive(Default)]
pub(crate) struct AudioPreviewHost {
    observed: Option<Observation>,
    key: Option<u64>,
    pending: Option<Receiver<Result<Vec<u8>, op_audio::AudioError>>>,
    pcm: Option<Vec<u8>>,
    error: Option<String>,
    player: Option<op_audio::PreviewPlayer>,
    started_clock: Option<(u64, f64)>,
    last_head: f64,
}

impl AudioPreviewHost {
    pub(crate) fn poll(
        &mut self,
        host: &mut WidgetHostNative,
        proxy: Option<EventLoopProxy<DesktopEvent>>,
        now_ms: u64,
    ) -> bool {
        let ui = &host.editor_state().editor_ui;
        if ui.preview.mode || ui.home.visible || ui.works_reader_visible() {
            self.player = None;
            self.started_clock = None;
            let timeline = &mut host.editor_state_mut().editor_ui.motion_timeline;
            timeline.playing = false;
            timeline.play_clock = None;
        }
        host.editor_state_mut().tick_motion_timeline(now_ms);
        let state = host.editor_state();
        let Some(root) = state.motion_timeline_scope() else {
            let changed = self.key.is_some();
            self.clear();
            host.editor_state_mut()
                .editor_ui
                .motion_timeline
                .audio_preview_busy = false;
            return changed;
        };
        let Some(trigger) = state.editor_ui.motion_timeline.clip_trigger() else {
            self.clear();
            host.editor_state_mut()
                .editor_ui
                .motion_timeline
                .audio_preview_busy = false;
            return false;
        };
        let observed = (
            host.document_epoch(),
            state.document_generation(),
            state.document_revision(),
            root.id_str().to_owned(),
            format!("{trigger:?}"),
        );
        let mut changed = false;
        if self.observed.as_ref() != Some(&observed) {
            let tracks = op_audio::matching_tracks(root, &trigger);
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            observed.0.hash(&mut hash);
            observed.1.hash(&mut hash);
            observed.3.hash(&mut hash);
            observed.4.hash(&mut hash);
            let duration = state.motion_timeline_projection().clip_ms.max(1.0) as u64;
            duration.hash(&mut hash);
            for track in &tracks {
                track.id.hash(&mut hash);
                track.source.hash(&mut hash);
                track.start_ms.hash(&mut hash);
                track.trim_start_ms.hash(&mut hash);
                track.duration_ms.hash(&mut hash);
                track.volume.to_bits().hash(&mut hash);
            }
            let key = (!tracks.is_empty()).then(|| hash.finish());
            if self.key != key {
                self.player = None;
                self.pcm = None;
                self.pending = None;
                self.error = None;
                self.started_clock = None;
                self.key = key;
                changed = true;
                if key.is_some() {
                    let root = root.clone();
                    let (tx, rx) = mpsc::channel();
                    match std::thread::Builder::new()
                        .name("op-audio-preview".into())
                        .spawn(move || {
                            let _ = tx.send(op_audio::render_pcm(&root, &trigger, duration));
                            if let Some(proxy) = proxy {
                                let _ = proxy.send_event(DesktopEvent::AudioReady);
                            }
                        }) {
                        Ok(_) => self.pending = Some(rx),
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            }
            self.observed = Some(observed);
        }
        if let Some(pending) = self.pending.take() {
            match pending.try_recv() {
                Ok(Ok(pcm)) => {
                    self.pcm = Some(pcm);
                    changed = true;
                }
                Ok(Err(error)) => {
                    self.error = Some(error.to_string());
                    changed = true;
                }
                Err(mpsc::TryRecvError::Empty) => self.pending = Some(pending),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error = Some("Audio preview stopped unexpectedly".into());
                    changed = true;
                }
            }
        }
        host.editor_state_mut()
            .editor_ui
            .motion_timeline
            .audio_preview_busy = self.pending.is_some();
        let timeline = &host.editor_state().editor_ui.motion_timeline;
        let clock = timeline.play_clock;
        let head = timeline.playhead_ms;
        if !timeline.open || !timeline.playing {
            self.player = None;
            self.started_clock = None;
        } else if self.started_clock != clock || head + 1.0 < self.last_head {
            if let Some(pcm) = &self.pcm {
                self.player = None;
                match op_audio::PreviewPlayer::start(pcm, head as u64) {
                    Ok(player) => {
                        self.player = player;
                        self.started_clock = clock;
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            }
            if let Some(error) = self.error.as_deref() {
                let locale = host.editor_state().editor_ui.locale;
                crate::message_dialog::alert(
                    op_i18n::translate(locale, "audio.title"),
                    error,
                    rfd::MessageLevel::Error,
                );
                host.editor_state_mut().toggle_motion_playback(now_ms);
                changed = true;
            }
        }
        self.last_head = head;
        changed
    }

    fn clear(&mut self) {
        self.observed = None;
        self.key = None;
        self.pending = None;
        self.pcm = None;
        self.error = None;
        self.player = None;
        self.started_clock = None;
        self.last_head = 0.0;
    }
}
