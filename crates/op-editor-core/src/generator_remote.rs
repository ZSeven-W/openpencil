//! Remote generator runs — the browser's path to a runtime.
//!
//! The wasm bundle carries no QuickJS, so the browser host installs a
//! runner that asks its daemon (`POST` [`GENERATOR_RUN_ROUTE`]) instead of
//! running the program itself. That runner cannot block, so a miss answers
//! [`GeneratorError::Pending`]: the command builders leave the document
//! untouched, `ui.generator_pending` records what is waiting, and the
//! panel shows "running". When the reply lands the host caches it and
//! calls [`EditorState::apply_remote_generator_result`], which re-runs the
//! SAME command with the SAME spec — the runner now answers from its cache
//! — and records one undo step, exactly like a local regenerate.
//!
//! Native hosts never see `Pending` (their runner is synchronous), so none
//! of this changes their behaviour: `generator_pending` stays `None`.
//!
//! The wire DTOs and reply parsing live here so the daemon route
//! (`op-host-services`) and the browser client (`op-host-web`) share one
//! definition of the protocol.

use jian_ops_schema::node::PenNode;
use serde::{Deserialize, Serialize};

use super::{GeneratorError, GeneratorRequest, GeneratorRunner, GeneratorSpec};
use crate::node_id::NodeId;
use crate::state::EditorState;

/// The daemon route that runs one generator program.
pub const GENERATOR_RUN_ROUTE: &str = "/api/generator/run";

/// A generator run waiting on a remote runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorPending {
    /// The generator being (re)generated — for a new starter, the id it
    /// will take once its first output lands.
    pub node_id: NodeId,
    /// Set when the run creates a NEW generator: the frame + spec the
    /// insert started from, replayed when the result lands.
    pub create: Option<Box<(PenNode, GeneratorSpec)>>,
}

/// `POST` [`GENERATOR_RUN_ROUTE`] body: an owned [`GeneratorRequest`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratorRunWireRequest {
    pub generator_id: String,
    pub spec: GeneratorSpec,
    pub frame: PenNode,
}

impl GeneratorRunWireRequest {
    pub fn from_request(request: &GeneratorRequest<'_>) -> Self {
        Self {
            generator_id: request.generator_id.to_string(),
            spec: request.spec.clone(),
            frame: request.frame.clone(),
        }
    }

    pub fn as_request(&self) -> GeneratorRequest<'_> {
        GeneratorRequest {
            generator_id: &self.generator_id,
            spec: &self.spec,
            frame: &self.frame,
        }
    }
}

/// The reply body for one run: `{"children":[…]}` or `{"error":"…"}`.
pub fn generator_run_reply_body(result: &Result<Vec<PenNode>, GeneratorError>) -> String {
    match result {
        Ok(children) => serde_json::json!({ "children": children }).to_string(),
        Err(error) => serde_json::json!({ "error": error.to_string() }).to_string(),
    }
}

/// A parsed reply, plus whether the failure may go away on retry.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorRunReply {
    pub result: Result<Vec<PenNode>, GeneratorError>,
    /// Transport trouble, a busy or failing server: the same request may
    /// succeed later, so a cache must not keep this answer.
    pub transient: bool,
}

/// Parse a daemon reply. Status 0 is an XHR that never got an answer.
pub fn parse_generator_run_reply(status: u16, body: &str) -> GeneratorRunReply {
    #[derive(Deserialize)]
    struct Wire {
        #[serde(default)]
        children: Option<Vec<PenNode>>,
        #[serde(default)]
        error: Option<String>,
    }
    let transient = status == 0 || status == 429 || status >= 500;
    let parsed = serde_json::from_str::<Wire>(body).ok();
    let result = match parsed {
        Some(Wire {
            children: Some(children),
            ..
        }) if status == 200 => Ok(children),
        Some(Wire {
            error: Some(message),
            ..
        }) => Err(GeneratorError::Remote(message)),
        _ if status == 0 => Err(GeneratorError::Remote(
            "the generator server could not be reached; document unchanged".into(),
        )),
        _ => Err(GeneratorError::Remote(format!(
            "the generator server answered HTTP {status}; document unchanged"
        ))),
    };
    GeneratorRunReply { result, transient }
}

impl EditorState {
    /// Keep `ui.generator_pending` in step with a finished attempt on `id`:
    /// a `Pending` answer parks it (with the create inputs, if any); any
    /// other answer clears a pending entry for the same node.
    pub(super) fn note_generator_pending<T>(
        &mut self,
        id: &NodeId,
        outcome: &Result<T, GeneratorError>,
        create: Option<(PenNode, GeneratorSpec)>,
    ) {
        match outcome {
            Err(GeneratorError::Pending) => {
                self.ui.generator_pending = Some(GeneratorPending {
                    node_id: id.clone(),
                    create: create.map(Box::new),
                });
            }
            _ => {
                if self
                    .ui
                    .generator_pending
                    .as_ref()
                    .is_some_and(|pending| &pending.node_id == id)
                {
                    self.ui.generator_pending = None;
                }
            }
        }
    }

    /// True while generator `id` waits on a remote runtime.
    pub fn generator_is_pending(&self, id: &NodeId) -> bool {
        self.ui
            .generator_pending
            .as_ref()
            .is_some_and(|pending| &pending.node_id == id)
    }

    /// Land a remote result for generator `generator_id` computed from
    /// `spec`. The caller has already made `runner` answer this request
    /// from its cache; this re-runs the same command so the write goes
    /// through the ordinary validation, id checks, and history, as ONE undo
    /// step. Re-applying a byte-identical result is a no-op (`Ok(false)`).
    ///
    /// * A pending starter insert for this id is finished (and selected).
    /// * An existing generator is rewritten to `spec` + the new children.
    /// * A generator deleted meanwhile is ignored.
    pub fn apply_remote_generator_result(
        &mut self,
        generator_id: &NodeId,
        spec: &GeneratorSpec,
        runner: Option<GeneratorRunner>,
    ) -> Result<bool, GeneratorError> {
        let create = match &self.ui.generator_pending {
            Some(pending) if &pending.node_id == generator_id => pending.create.clone(),
            _ => None,
        };
        if let Some(create) = create {
            let (frame, create_spec) = *create;
            if self.locate_generator_node(generator_id).is_none() {
                return self
                    .insert_generator_frame(frame, create_spec, runner)
                    .map(|_| true);
            }
        }
        if self.locate_generator_node(generator_id).is_none() {
            self.note_generator_pending::<()>(generator_id, &Ok(()), None);
            return Ok(false);
        }
        let spec = spec.clone();
        let result = self.generator_rewrite_command(
            generator_id,
            &|stored| {
                *stored = spec.clone();
                Ok(())
            },
            runner,
        );
        self.record_generator_result(generator_id, result, true)
    }
}

#[cfg(test)]
#[path = "generator_remote_tests.rs"]
mod tests;
