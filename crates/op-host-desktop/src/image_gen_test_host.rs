//! Host pump for the Workbench image-gen profile "Test" button.
//!
//! Same session pattern as `provider_probe_host.rs`: the settings press
//! raises `agent_settings.pending_image_gen_test` (the request seam in
//! op-editor-core), the redraw pump drains it onto a worker thread running
//! `op_host_services::web_image_generate::test_workbench`, and a later
//! frame lands the outcome in the profile's `test_status`.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_editor_core::agent_settings::{ImageGenProfile, ImageTestStatus};

use crate::DesktopApp;

/// One in-flight Workbench status probe.
pub(crate) struct ImageGenTestJob {
    profile_id: String,
    rx: Receiver<ImageTestStatus>,
}

impl ImageGenTestJob {
    fn spawn(profile: ImageGenProfile) -> Self {
        let (tx, rx) = mpsc::channel();
        let profile_id = profile.id.clone();
        std::thread::spawn(move || {
            let status = op_host_services::chat_runtime::block_on_anywhere(async move {
                let client = match reqwest::Client::builder()
                    .use_rustls_tls()
                    .timeout(std::time::Duration::from_secs(20))
                    .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
                    .build()
                {
                    Ok(client) => client,
                    Err(_) => return ImageTestStatus::Invalid,
                };
                op_host_services::web_image_generate::test_workbench(&client, &profile).await
            });
            let _ = tx.send(status);
        });
        Self { profile_id, rx }
    }

    fn poll(&self) -> Option<ImageTestStatus> {
        match self.rx.try_recv() {
            Ok(status) => Some(status),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(ImageTestStatus::Invalid),
        }
    }
}

impl DesktopApp {
    /// Pump: land a finished probe into its profile, then start the next
    /// requested one. Returns true when state changed.
    pub(crate) fn drain_image_gen_test(&mut self) -> bool {
        let mut changed = false;
        if let Some(job) = self.image_gen_test_job.as_ref() {
            if let Some(status) = job.poll() {
                let profile_id = job.profile_id.clone();
                self.image_gen_test_job = None;
                let settings = &mut self.host.editor_state_mut().editor_ui.agent_settings;
                // The user may have removed the profile (or pressed Test
                // again) while the probe ran — only a profile still in its
                // Testing phase accepts the outcome.
                if let Some(profile) = settings
                    .image_gen_profiles
                    .iter_mut()
                    .find(|profile| profile.id == profile_id)
                {
                    if profile.test_status == ImageTestStatus::Testing {
                        profile.test_status = status;
                        changed = true;
                    }
                }
            }
        }
        if self.image_gen_test_job.is_some() {
            return changed;
        }
        let pending = self
            .host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .pending_image_gen_test
            .take();
        if let Some(profile_id) = pending {
            let profile = self
                .host
                .editor_state()
                .editor_ui
                .agent_settings
                .image_gen_profiles
                .iter()
                .find(|profile| profile.id == profile_id)
                .cloned();
            if let Some(profile) = profile {
                self.image_gen_test_job = Some(ImageGenTestJob::spawn(profile));
                changed = true;
            }
        }
        changed
    }
}
