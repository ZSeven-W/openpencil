//! Press-handler helpers split out of `press.rs` to honor the
//! 800-line cap — node spawn for the active tool + the
//! agent-settings modal press dispatcher.

use super::WidgetHostNative;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// Spawn a fresh node for the active shape / frame / text tool at
    /// `doc_point`. Returns the new node's id when the tool maps to a
    /// creatable kind; `None` for `Select` / `Hand`.
    ///
    /// Delegates to `EditorState::create_node_for_tool` — the host
    /// never builds canonical nodes itself.
    pub(in crate::widget_host) fn create_node_for_active_tool(
        &mut self,
        doc_point: Point2D,
    ) -> Option<op_editor_core::NodeId> {
        // Click-create default size: Text needs room for its
        // placeholder glyphs; shape tools start 1×1 so a drag
        // immediately sizes the node to the cursor.
        let (init_w, init_h) = if matches!(self.editor_state.tool, op_editor_core::Tool::Text) {
            (96.0_f64, 24.0_f64)
        } else {
            (1.0, 1.0)
        };
        let id = self.editor_state.create_node_for_tool(
            self.editor_state.tool,
            &mut self.next_node_id,
            doc_point.x as f64,
            doc_point.y as f64,
            init_w,
            init_h,
        );
        if id.is_some() {
            self.mark_dirty();
        }
        id
    }

    /// Agent-settings modal press dispatcher. Returns true when the
    /// click was consumed by the modal.
    pub(in crate::widget_host) fn dispatch_agent_settings_press(
        &mut self,
        x: f32,
        y: f32,
        vw: f32,
        vh: f32,
    ) -> bool {
        use op_editor_ui::widgets::agent_settings_panel::{AgentSettingsHit, AgentSettingsPanel};
        self.refresh_layout_scene();
        let panel = AgentSettingsPanel::for_editor(&self.editor_state);
        let panel_rect = panel.rect(vw, vh);
        let point = Point2D::new(x, y);
        match panel.hit_test(panel_rect, point) {
            AgentSettingsHit::Close | AgentSettingsHit::Outside => {
                self.commit_settings_focus_if_any();
                self.editor_state.editor_ui.agent_settings_open = false;
                self.editor_state.editor_ui.agent_settings_drag = None;
            }
            AgentSettingsHit::SelectTab(t) => {
                self.commit_settings_focus_if_any();
                self.editor_state.editor_ui.agent_settings.tab = t;
                self.editor_state.editor_ui.agent_settings.scroll_y = 0.0;
            }
            AgentSettingsHit::Connect(p) => {
                // `connected` is indexed by `AgentProvider::ALL` order.
                let idx = op_editor_core::agent_settings::AgentProvider::ALL
                    .iter()
                    .position(|x| *x == p)
                    .unwrap_or(0);
                let v = &mut self.editor_state.editor_ui.agent_settings.connected[idx];
                *v = !*v;
                // Connecting / disconnecting a provider changes which
                // models the chat picker may list — re-derive it from
                // the discovered catalog against the new mask.
                self.editor_state.rebuild_chat_models();
            }
            AgentSettingsHit::ToggleMcpServer => {
                self.commit_settings_focus_if_any();
                let v = &mut self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .running;
                *v = !*v;
            }
            AgentSettingsHit::ToggleMcpCli(cli) => {
                // `mcp_cli_enabled` is indexed by `McpCli::ALL` order.
                let idx = op_editor_core::agent_settings::McpCli::ALL
                    .iter()
                    .position(|x| *x == cli)
                    .unwrap_or(0);
                let v = &mut self.editor_state.editor_ui.agent_settings.mcp_cli_enabled[idx];
                *v = !*v;
            }
            AgentSettingsHit::ToggleImagesAdvanced => {
                let v = &mut self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .images_advanced_open;
                *v = !*v;
            }
            AgentSettingsHit::SetActiveGenConfig(index) => {
                self.commit_settings_focus_if_any();
                if let Some(id) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get(index)
                    .map(|profile| profile.id.clone())
                {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .set_active_image_gen_profile(&id);
                }
            }
            AgentSettingsHit::RemoveGenConfig(index) => {
                self.commit_settings_focus_if_any();
                if let Some(id) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get(index)
                    .map(|profile| profile.id.clone())
                {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .remove_image_gen_profile(&id);
                }
            }
            AgentSettingsHit::ToggleAutoUpdate => {
                let v = &mut self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .auto_update_enabled;
                *v = !*v;
            }
            AgentSettingsHit::FocusMcpPort => {
                self.commit_settings_focus_if_any();
                self.editor_state.editor_ui.agent_settings.focus =
                    Some(op_editor_core::agent_settings::SettingsFocus::McpPort);
                self.editor_state.editor_ui.settings_input_draft = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .mcp_server
                    .port
                    .to_string();
            }
            AgentSettingsHit::FocusBuiltinAgent { index, field } => {
                self.commit_settings_focus_if_any();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get(index)
                {
                    self.editor_state.editor_ui.settings_input_draft = match field {
                        op_editor_core::agent_settings::BuiltinAgentField::DisplayName => {
                            agent.display_name.clone()
                        }
                        op_editor_core::agent_settings::BuiltinAgentField::ApiKey => {
                            agent.api_key.clone()
                        }
                        op_editor_core::agent_settings::BuiltinAgentField::Model => {
                            agent.model.clone()
                        }
                        op_editor_core::agent_settings::BuiltinAgentField::BaseUrl => {
                            agent.base_url.clone()
                        }
                    };
                    self.editor_state.editor_ui.agent_settings.focus = Some(
                        op_editor_core::agent_settings::SettingsFocus::BuiltinAgent {
                            index,
                            field,
                        },
                    );
                }
            }
            AgentSettingsHit::ToggleBuiltinAgentKind(index) => {
                self.commit_settings_focus_if_any();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get_mut(index)
                {
                    use op_editor_core::agent_settings::BuiltinAgentKind;
                    agent.kind = match agent.kind {
                        BuiltinAgentKind::Anthropic => BuiltinAgentKind::OpenAiCompat,
                        BuiltinAgentKind::OpenAiCompat => BuiltinAgentKind::Anthropic,
                    };
                    agent.base_url = agent.kind.default_base_url().to_string();
                    if agent.kind == BuiltinAgentKind::OpenAiCompat
                        && agent.model.starts_with("claude-")
                    {
                        agent.model = "gpt-5.4".into();
                    } else if agent.kind == BuiltinAgentKind::Anthropic
                        && agent.model.starts_with("gpt-")
                    {
                        agent.model = "claude-sonnet-4-5".into();
                    }
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::ToggleBuiltinAgentEnabled(index) => {
                self.commit_settings_focus_if_any();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get_mut(index)
                {
                    agent.enabled = !agent.enabled;
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::EditBuiltinAgent(index) => {
                self.commit_settings_focus_if_any();
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get(index)
                {
                    self.editor_state.editor_ui.settings_input_draft = agent.display_name.clone();
                    self.editor_state.editor_ui.agent_settings.focus = Some(
                        op_editor_core::agent_settings::SettingsFocus::BuiltinAgent {
                            index,
                            field: op_editor_core::agent_settings::BuiltinAgentField::DisplayName,
                        },
                    );
                }
            }
            AgentSettingsHit::RemoveBuiltinAgent(index) => {
                self.commit_settings_focus_if_any();
                let agents = &mut self.editor_state.editor_ui.agent_settings.builtin_agents;
                if index < agents.len() {
                    agents.remove(index);
                    self.editor_state.editor_ui.agent_settings.focus = None;
                    self.editor_state.editor_ui.settings_input_draft.clear();
                    self.editor_state.rebuild_chat_models();
                }
            }
            AgentSettingsHit::AddProvider => {
                let id = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .add_builtin_agent();
                let index = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .iter()
                    .position(|agent| agent.id == id)
                    .unwrap_or(0);
                self.editor_state.editor_ui.agent_settings.focus = Some(
                    op_editor_core::agent_settings::SettingsFocus::BuiltinAgent {
                        index,
                        field: op_editor_core::agent_settings::BuiltinAgentField::ApiKey,
                    },
                );
                self.editor_state.editor_ui.settings_input_draft.clear();
                self.editor_state.rebuild_chat_models();
            }
            AgentSettingsHit::AddAcpAgent
            | AgentSettingsHit::TestImageSearch
            | AgentSettingsHit::Inside => {}
            AgentSettingsHit::AddGenConfig => {
                self.commit_settings_focus_if_any();
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .add_image_gen_profile();
            }
        }
        self.mark_dirty();
        true
    }
}

/// Seed the property-input draft from the panel snapshot for the
/// freshly-focused `PropertyFocus` row. Lives here (not `press.rs`)
/// to keep that file under the 800-line cap.
pub(in crate::widget_host) fn property_focus_initial(
    focus: op_editor_core::PropertyFocus,
    panel: &op_editor_ui::widgets::PropertyPanel,
) -> String {
    use super::helpers::color_to_hex;
    use op_editor_core::PropertyFocus as F;
    match focus {
        F::PositionX => panel.snapshot.x.to_string(),
        F::PositionY => panel.snapshot.y.to_string(),
        F::SizeW => panel.snapshot.width.to_string(),
        F::SizeH => panel.snapshot.height.to_string(),
        F::LayoutGap => format_panel_number(panel.snapshot.layout_gap),
        F::PaddingTop | F::PaddingRight | F::PaddingBottom | F::PaddingLeft => panel
            .snapshot
            .layout_padding
            .value_for(focus)
            .map(format_panel_number)
            .unwrap_or_else(|| "0".to_string()),
        F::Rotation => (panel.snapshot.rotation_deg.round() as i32).to_string(),
        F::PositionR => (panel.snapshot.corner_radius.round() as i32).to_string(),
        F::Opacity => "100".to_string(),
        F::PolygonSides => panel.snapshot.polygon_sides.unwrap_or(3).to_string(),
        F::EllipseStart => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.start_deg)
                .unwrap_or(0.0),
        ),
        F::EllipseSweep => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.sweep_deg)
                .unwrap_or(360.0),
        ),
        F::EllipseInnerRadius => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.inner_percent)
                .unwrap_or(0.0),
        ),
        F::FontSize => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.font_size))
            .unwrap_or_else(|| "16".to_string()),
        F::FontWeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| t.font_weight.to_string())
            .unwrap_or_else(|| "400".to_string()),
        F::LineHeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.line_height_percent))
            .unwrap_or_else(|| "120".to_string()),
        F::LetterSpacing => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.letter_spacing))
            .unwrap_or_else(|| "0".to_string()),
        F::FillOpacity => ((panel.snapshot.fill_opacity * 100.0).round() as i32).to_string(),
        F::FillHex => panel
            .snapshot
            .fill
            .map(color_to_hex)
            .unwrap_or_else(|| "#FFFFFF".to_string()),
        F::StrokeHex => panel
            .snapshot
            .stroke
            .map(|s| color_to_hex(s.color))
            .unwrap_or_else(|| "#000000".to_string()),
        F::StrokeWidth => panel
            .snapshot
            .stroke
            .map(|s| format!("{}", s.width.round() as i32))
            .unwrap_or_else(|| "1".to_string()),
        F::GradientAngle => {
            let a = panel.snapshot.gradient_angle.unwrap_or(0.0);
            if a.fract() == 0.0 {
                format!("{}", a as i32)
            } else {
                format!("{a}")
            }
        }
        F::GradientStopHex(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            // Strip alpha so the input pill matches what paint shows.
            // Per-stop transparency rides through commit invisibly.
            .map(|s| op_editor_ui::widgets::property_panel_fill::stop_hex_rgb_only(&s.hex))
            .unwrap_or_else(|| "#000000".to_string()),
        F::GradientStopOffset(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            .map(|s| ((s.offset * 100.0).round() as i32).to_string())
            .unwrap_or_else(|| "0".to_string()),
    }
}

fn format_panel_number(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.2}")
    }
}

/// Translate a shell-core `ColorTarget` into op-editor-core's — used
/// by `press.rs`'s `OpenColorPicker` branch.
pub(in crate::widget_host) fn color_target(
    t: op_editor_core::ColorTarget,
) -> op_editor_core::ui_draft::ColorTarget {
    match t {
        op_editor_core::ColorTarget::Fill => op_editor_core::ui_draft::ColorTarget::Fill,
        op_editor_core::ColorTarget::Stroke => op_editor_core::ui_draft::ColorTarget::Stroke,
        op_editor_core::ColorTarget::GradientStop(i) => {
            op_editor_core::ui_draft::ColorTarget::GradientStop(i)
        }
        op_editor_core::ColorTarget::EffectColor(i) => {
            op_editor_core::ui_draft::ColorTarget::EffectColor(i)
        }
    }
}
