//! Bounded vision turns for transports that join their work before returning.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{ChatVisionLlmClient, VisionCallRequest, VisionResponse};

impl ChatVisionLlmClient {
    pub(crate) fn supports_deadline(&self) -> bool {
        self.provider.supports_joined_cancellable_send()
    }

    /// The caller retains its concurrency permit until this returns. Merely
    /// setting a cancellation flag is insufficient: the provider must join its
    /// actual transport work when the iterator ends or is dropped.
    pub(crate) fn validate_before(
        &self,
        req: VisionCallRequest,
        deadline: Instant,
        max_output_tokens: u32,
    ) -> VisionResponse {
        self.validate_before_measured(req, deadline, max_output_tokens)
            .0
    }

    /// Keep preflight failures separate from entered provider calls. This
    /// duration includes collect_response dropping/joining the transport.
    pub(crate) fn validate_before_measured(
        &self,
        req: VisionCallRequest,
        deadline: Instant,
        max_output_tokens: u32,
    ) -> (VisionResponse, Option<Duration>) {
        if !self.supports_deadline() {
            return (
                skipped("vision provider cannot join cancelled transport"),
                None,
            );
        }
        if Instant::now() >= deadline {
            return (
                skipped("vision deadline elapsed before transport start"),
                None,
            );
        }
        let mut request = match self.chat_request(req) {
            Ok(request) => request,
            Err(response) => return (response, None),
        };
        request.max_output_tokens = max_output_tokens;
        // An empty system field makes builtin providers inject generation
        // skills. The complete judge contract stays in user_message (also
        // supported by CLI transports); this prevents unrelated instructions.
        request.system_prompt =
            "Evaluate the supplied image against its original brief. Return only the requested evidence JSON."
                .to_string();
        let timer = CancelAtDeadline::new(deadline);
        if Instant::now() >= deadline {
            return (
                skipped("vision deadline elapsed before transport start"),
                None,
            );
        }
        let provider_started = Instant::now();
        let Some(deltas) = self
            .provider
            .send_cancellable_joined(request, Arc::clone(&timer.cancel))
        else {
            return (
                skipped("vision provider did not supply a joined transport"),
                Some(provider_started.elapsed()),
            );
        };
        // collect_response owns and drops the iterator before returning. A
        // joined provider therefore finishes cleanup before the caller can
        // release its permit, including early Error/Done and panic paths.
        let response = Self::collect_response(deltas);
        let provider_elapsed = provider_started.elapsed();
        let response = if timer.cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
            // Never interpret partial JSON accumulated before cancellation as
            // a completed verdict, even when it happens to be well formed.
            skipped("vision deadline elapsed; incomplete response discarded")
        } else {
            response
        };
        (response, Some(provider_elapsed))
    }
}

fn skipped(reason: &str) -> VisionResponse {
    VisionResponse::Skipped {
        reason: Some(reason.to_string()),
    }
}

struct CancelAtDeadline {
    cancel: Arc<AtomicBool>,
    timer: tokio::task::JoinHandle<()>,
}

impl CancelAtDeadline {
    fn new(deadline: Instant) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancel);
        let timer = crate::chat_runtime::shared_runtime().spawn(async move {
            tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await;
            signal.store(true, Ordering::Release);
        });
        Self { cancel, timer }
    }
}

impl Drop for CancelAtDeadline {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        // This task only holds a timer and flag, never provider work. It is
        // bounded by the caller's acquired transport permits.
        self.timer.abort();
    }
}
