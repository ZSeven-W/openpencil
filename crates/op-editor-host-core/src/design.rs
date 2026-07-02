//! Shared design-turn actor state.

use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};

use op_editor_core::{EditorCommand, EditorState};
use op_orchestrator::{DocSink, OrchestratorError, Progress, RunSummary};

/// One in-flight design turn.
pub struct DesignSession {
    delta_rx: Receiver<DesignDelta>,
    cmd_rx: Receiver<DesignCmdReq>,
    finished: bool,
    indicator_epoch: u64,
}

impl Drop for DesignSession {
    fn drop(&mut self) {
        op_editor_core::agent_indicators::end_if_epoch(self.indicator_epoch);
    }
}

/// Progress / completion events emitted by the worker.
pub enum DesignDelta {
    Progress(Progress),
    Done(Result<RunSummary, OrchestratorError>),
}

/// Request from worker to UI to apply one editor mutation or batch boundary.
pub struct DesignCmdReq {
    pub op: DesignCmdOp,
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
        Self {
            delta_rx,
            cmd_rx,
            finished: false,
            indicator_epoch,
        }
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
                    break;
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
}

impl RemoteDocSink {
    pub fn new(cmd_tx: Sender<DesignCmdReq>, initial_state: EditorState) -> Self {
        Self {
            cmd_tx,
            mirror: initial_state,
        }
    }

    fn send_and_wait(&mut self, op: DesignCmdOp) -> bool {
        let (ack_tx, ack_rx) = mpsc::sync_channel::<DesignCmdAck>(1);
        let req = DesignCmdReq { op, ack: ack_tx };
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
