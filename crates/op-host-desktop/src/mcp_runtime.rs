//! Desktop-app glue for the live MCP server and terminal integrations.

use super::{mcp_integrations, mcp_live, DesktopApp};

impl DesktopApp {
    pub(crate) fn bootstrap_mcp_runtime_from_settings(&mut self) -> bool {
        let detected_flags = match &self.mcp_integrations_home {
            Some(home) => mcp_integrations::detect_enabled_clis_at_home(home),
            None => mcp_integrations::detect_enabled_clis(),
        };
        let settings = &mut self.host.editor_state_mut().editor_ui.agent_settings;
        let mut changed = false;
        for (idx, detected) in detected_flags.iter().copied().enumerate() {
            if detected && !settings.mcp_cli_enabled[idx] {
                settings.mcp_cli_enabled[idx] = true;
                changed = true;
            }
        }
        let any_cli_enabled = settings.mcp_cli_enabled.iter().any(|enabled| *enabled);
        let port = settings.mcp_server.port;
        if any_cli_enabled && !settings.mcp_server.running {
            settings.mcp_server.running = true;
            changed = true;
        }
        if changed {
            self.host.mark_editor_state_dirty();
        }
        changed |= self.reconcile_mcp_server_from_settings();
        if any_cli_enabled {
            changed |= self.reconcile_mcp_cli_integrations(Some(([false; 6], port)));
        }
        if self.mcp_server_active() {
            changed |= self.request_redraw(false);
        }
        changed
    }

    /// Reconcile the live MCP server against settings + the `--live-mcp`
    /// force flag. Returns `true` when the server's running-state or port
    /// changed (so the caller refreshes the discovery file). A forced
    /// (`op start`) launch starts the server WITHOUT mutating or persisting
    /// the user's settings — the runtime force port is updated to the bound
    /// port instead, so a one-off `op start` never rewrites saved prefs.
    pub(crate) fn reconcile_mcp_server_from_settings(&mut self) -> bool {
        let setting_running = self
            .host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_server
            .running;
        let setting_port = self
            .host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_server
            .port;
        let forced = self.force_live_mcp_port.is_some();
        let desired = setting_running || forced;
        let port = self.force_live_mcp_port.unwrap_or(setting_port);

        if !desired {
            // Report the change so the caller removes the stale port file.
            if let Some(mut server) = self.mcp_server.take() {
                server.stop();
                return true;
            }
            return false;
        }
        if self
            .mcp_server
            .as_ref()
            .is_some_and(|server| server.port() == port)
        {
            return false;
        }
        if let Some(mut server) = self.mcp_server.take() {
            server.stop();
        }
        match mcp_live::McpLiveServer::start(port) {
            Ok(server) => {
                let bound_port = server.port();
                self.mcp_server = Some(server);
                self.note_bound_mcp_port(forced, setting_running, bound_port);
                true
            }
            Err(err) => {
                eprintln!("openpencil-desktop mcp: failed to start on {port}: {err}");
                if port != 0 {
                    match mcp_live::McpLiveServer::start(0) {
                        Ok(server) => {
                            let bound_port = server.port();
                            eprintln!(
                                "openpencil-desktop mcp: fell back to 127.0.0.1:{bound_port}/mcp"
                            );
                            self.mcp_server = Some(server);
                            self.note_bound_mcp_port(forced, setting_running, bound_port);
                            return true;
                        }
                        Err(fallback_err) => {
                            eprintln!(
                                "openpencil-desktop mcp: fallback start failed: {fallback_err}"
                            );
                        }
                    }
                }
                // Couldn't start. Only clear the *persisted* toggle when the
                // user (not the transient force flag) requested it.
                if setting_running && !forced {
                    self.host
                        .editor_state_mut()
                        .editor_ui
                        .agent_settings
                        .mcp_server
                        .running = false;
                    self.host.mark_editor_state_dirty();
                }
                true
            }
        }
    }

    /// Record the actually-bound MCP port. A forced (`--live-mcp`) launch
    /// updates the runtime force port only — never the persisted settings —
    /// so later reconciles see the bound port and don't restart, and a
    /// one-off `op start` leaves saved prefs untouched. A user-enabled
    /// server persists the bound port into settings (existing behavior).
    fn note_bound_mcp_port(&mut self, forced: bool, setting_running: bool, bound: u16) {
        if forced {
            self.force_live_mcp_port = Some(bound);
        } else if setting_running {
            let settings = &mut self
                .host
                .editor_state_mut()
                .editor_ui
                .agent_settings
                .mcp_server;
            if settings.port != bound {
                settings.port = bound;
                self.host.mark_editor_state_dirty();
            }
        }
    }

    pub(crate) fn poll_mcp_server(&mut self) -> bool {
        let Some(server) = self.mcp_server.as_mut() else {
            return false;
        };
        let outcome = server.pump(self.host.editor_state_mut());
        if outcome.layout_dirty {
            self.host.mark_editor_state_dirty();
        }
        outcome.repaint
    }

    pub(crate) fn mcp_server_active(&self) -> bool {
        self.mcp_server.is_some()
    }

    /// Whether the live MCP server received a token-authed `op stop`
    /// (`openpencil/shutdown`) and the editor should quit.
    pub(crate) fn mcp_shutdown_requested(&self) -> bool {
        self.mcp_server
            .as_ref()
            .is_some_and(|server| server.shutdown_requested())
    }

    /// Publish (or retract) the live MCP server's port to the TS-compatible
    /// discovery file `~/.openpencil/.op-mcp-port` so the `op` CLI can find this
    /// editor's on-screen canvas. Called from the app lifecycle (after the
    /// launch bootstrap and after a settings-driven reconcile), NOT from
    /// `reconcile_mcp_server_from_settings` itself — keeping reconcile free
    /// of filesystem side effects so unit tests don't touch the real home
    /// directory. Best-effort; failures are non-fatal.
    pub(crate) fn publish_live_mcp_port(&self) {
        match self.mcp_server.as_ref() {
            Some(server) => crate::mcp_port_file::write(server.port(), server.token()),
            None => crate::mcp_port_file::remove(),
        }
    }

    pub(crate) fn reconcile_mcp_cli_integrations(
        &mut self,
        before: Option<([bool; 6], u16)>,
    ) -> bool {
        let Some((before_flags, before_port)) = before else {
            return false;
        };
        let settings = &self.host.editor_state().editor_ui.agent_settings;
        let after_flags = settings.mcp_cli_enabled;
        let port = settings.mcp_server.port;
        if before_flags == after_flags && before_port == port {
            return false;
        }

        let home_override = self.mcp_integrations_home.clone();
        let mut reverted = false;
        for (idx, cli) in op_editor_core::agent_settings::McpCli::ALL
            .iter()
            .copied()
            .enumerate()
        {
            let flag_changed = before_flags[idx] != after_flags[idx];
            let enabled_port_changed = before_port != port && after_flags[idx];
            if !flag_changed && !enabled_port_changed {
                continue;
            }
            // Test override targets a temp home (env-free); production reads
            // the real home (+ `CODEX_HOME`).
            let result = match &home_override {
                Some(home) => {
                    mcp_integrations::set_cli_enabled_at_home(cli, after_flags[idx], port, home)
                }
                None => mcp_integrations::set_cli_enabled(cli, after_flags[idx], port),
            };
            if let Err(err) = result {
                eprintln!(
                    "openpencil-desktop mcp: failed to update {} integration: {err}",
                    cli.label()
                );
                if flag_changed {
                    self.host
                        .editor_state_mut()
                        .editor_ui
                        .agent_settings
                        .mcp_cli_enabled[idx] = before_flags[idx];
                    self.host.mark_editor_state_dirty();
                    reverted = true;
                }
            }
        }
        reverted
    }
}
