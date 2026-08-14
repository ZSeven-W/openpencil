//! Shared design-turn actor state.

use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};

use op_editor_core::{EditorCommand, EditorState};
use op_orchestrator::{AbortFlag, DocSink, OrchestratorError, Progress, RunSummary};

/// One in-flight design turn.
pub struct DesignSession {
    delta_rx: Receiver<DesignDelta>,
    cmd_rx: Receiver<DesignCmdReq>,
    abort: AbortFlag,
    finished: bool,
    indicator_epoch: u64,
}

impl Drop for DesignSession {
    fn drop(&mut self) {
        if self.finished {
            // Natural completion: let the queued reveals play out; the
            // overlay clears itself once the last one lands.
            op_editor_core::agent_indicators::finish_if_epoch(self.indicator_epoch);
        } else {
            // Dropping the UI-side receiver must also stop the orchestrator
            // and any concurrent screen-group LLM calls that share this flag.
            // Closing the channels alone only prevents later UI delivery; it
            // does not ask an in-flight provider request to return early.
            self.abort.set();
            // Aborted / discarded mid-run: tear the overlay down at once.
            op_editor_core::agent_indicators::end_if_epoch(self.indicator_epoch);
        }
    }
}

/// Progress / completion events emitted by the worker. A progress value may be
/// [`Progress::WorkerScoped`]; the transport deliberately forwards that
/// screen-group envelope intact instead of flattening it into the primary
/// design stream.
pub enum DesignDelta {
    Progress(Progress),
    Done(Result<RunSummary, OrchestratorError>),
}

/// Request from worker to UI to apply one editor mutation or batch boundary.
pub struct DesignCmdReq {
    pub op: DesignCmdOp,
    /// Page the design turn started on. Hosts apply every command against
    /// this page even when the user switches pages while the worker runs.
    pub target_page_id: Option<String>,
    pub ack: SyncSender<DesignCmdAck>,
}

/// What the worker is asking the UI to do.
pub enum DesignCmdOp {
    Apply(EditorCommand),
    BeginUndoBatch,
    EndUndoBatch,
}

/// UI's reply to one [`DesignCmdReq`].
pub struct DesignCmdAck {
    pub applied: bool,
    pub new_state: EditorState,
}

/// Result of one non-blocking progress drain.
pub struct DesignPoll {
    pub progress: Vec<Progress>,
    pub summary: Option<Result<RunSummary, OrchestratorError>>,
    pub finished: bool,
}

impl DesignSession {
    pub fn from_channels(delta_rx: Receiver<DesignDelta>, cmd_rx: Receiver<DesignCmdReq>) -> Self {
        Self::from_channels_with_epoch(delta_rx, cmd_rx, 0)
    }

    pub fn from_channels_with_epoch(
        delta_rx: Receiver<DesignDelta>,
        cmd_rx: Receiver<DesignCmdReq>,
        indicator_epoch: u64,
    ) -> Self {
        Self::from_channels_with_epoch_and_abort(
            delta_rx,
            cmd_rx,
            indicator_epoch,
            AbortFlag::new(),
        )
    }

    /// Build a session that shares its cancellation flag with the worker.
    /// Production launchers use this constructor; channel-only tests can keep
    /// using [`Self::from_channels`] unchanged.
    pub fn from_channels_with_epoch_and_abort(
        delta_rx: Receiver<DesignDelta>,
        cmd_rx: Receiver<DesignCmdReq>,
        indicator_epoch: u64,
        abort: AbortFlag,
    ) -> Self {
        Self {
            delta_rx,
            cmd_rx,
            abort,
            finished: false,
            indicator_epoch,
        }
    }

    /// Explicitly request cancellation before the session is dropped.
    pub fn abort(&self) {
        self.abort.set();
    }

    /// Drain every progress delta ready right now. Non-blocking.
    pub fn poll_progress(&mut self) -> DesignPoll {
        let mut progress = Vec::new();
        let mut summary = None;
        loop {
            match self.delta_rx.try_recv() {
                Ok(DesignDelta::Progress(p)) => progress.push(p),
                Ok(DesignDelta::Done(r)) => {
                    self.finished = true;
                    summary = Some(r);
                    // A terminal event must not hide progress that is already
                    // queued behind it. The desktop clears the session as soon
                    // as `finished` is true, so drain the ready queue fully in
                    // this same poll.
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        DesignPoll {
            progress,
            summary,
            finished: self.finished,
        }
    }

    /// Drain every pending apply request.
    pub fn drain_cmd_requests(&mut self) -> Vec<DesignCmdReq> {
        let mut out = Vec::new();
        while let Ok(req) = self.cmd_rx.try_recv() {
            out.push(req);
        }
        out
    }
}

/// Worker-side `DocSink` impl that forwards mutations to the UI thread.
pub struct RemoteDocSink {
    cmd_tx: Sender<DesignCmdReq>,
    mirror: EditorState,
    target_page_id: Option<String>,
}

impl RemoteDocSink {
    pub fn new(cmd_tx: Sender<DesignCmdReq>, initial_state: EditorState) -> Self {
        let target_page_id = initial_state
            .doc
            .pages
            .as_ref()
            .and_then(|pages| pages.get(initial_state.ui.active_page_index))
            .map(|page| page.id.clone())
            .or_else(|| Some("0".into()));
        Self {
            cmd_tx,
            mirror: initial_state,
            target_page_id,
        }
    }

    fn send_and_wait(&mut self, op: DesignCmdOp) -> bool {
        let (ack_tx, ack_rx) = mpsc::sync_channel::<DesignCmdAck>(1);
        let req = DesignCmdReq {
            op,
            target_page_id: self.target_page_id.clone(),
            ack: ack_tx,
        };
        if self.cmd_tx.send(req).is_err() {
            return false;
        }
        match ack_rx.recv() {
            Ok(ack) => {
                self.mirror = ack.new_state;
                ack.applied
            }
            Err(_) => false,
        }
    }
}

impl DocSink for RemoteDocSink {
    fn state(&self) -> &EditorState {
        &self.mirror
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.send_and_wait(DesignCmdOp::Apply(cmd))
    }

    fn begin_undo_batch(&mut self) {
        let _ = self.send_and_wait(DesignCmdOp::BeginUndoBatch);
    }

    fn end_undo_batch(&mut self) {
        let _ = self.send_and_wait(DesignCmdOp::EndUndoBatch);
    }
}
