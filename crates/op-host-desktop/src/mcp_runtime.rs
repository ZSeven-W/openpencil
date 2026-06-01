//! Desktop-app glue for the live MCP server and terminal integrations.

use super::{mcp_integrations, mcp_live, DesktopApp};

impl DesktopApp {
    pub(crate) fn bootstrap_mcp_runtime_from_settings(&mut self) -> bool {
        let detected_flags = mcp_integrations::detect_enabled_clis();
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

    pub(crate) fn reconcile_mcp_server_from_settings(&mut self) -> bool {
        let desired = self
            .host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_server
            .running;
        let port = self
            .host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_server
            .port;
        if !desired {
            if let Some(mut server) = self.mcp_server.take() {
                server.stop();
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
                if bound_port != port {
                    self.host
                        .editor_state_mut()
                        .editor_ui
                        .agent_settings
                        .mcp_server
                        .port = bound_port;
                    self.host.mark_editor_state_dirty();
                    true
                } else {
                    false
                }
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
                            let settings = &mut self
                                .host
                                .editor_state_mut()
                                .editor_ui
                                .agent_settings
                                .mcp_server;
                            settings.running = true;
                            settings.port = bound_port;
                            self.host.mark_editor_state_dirty();
                            return true;
                        }
                        Err(fallback_err) => {
                            eprintln!(
                                "openpencil-desktop mcp: fallback start failed: {fallback_err}"
                            );
                        }
                    }
                }
                self.host
                    .editor_state_mut()
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .running = false;
                self.host.mark_editor_state_dirty();
                true
            }
        }
    }

    pub(crate) fn poll_mcp_server(&mut self) -> bool {
        let Some(server) = self.mcp_server.as_mut() else {
            return false;
        };
        let changed = server.pump(self.host.editor_state_mut());
        if changed {
            self.host.mark_editor_state_dirty();
        }
        changed
    }

    pub(crate) fn mcp_server_active(&self) -> bool {
        self.mcp_server.is_some()
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
            if let Err(err) = mcp_integrations::set_cli_enabled(cli, after_flags[idx], port) {
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
