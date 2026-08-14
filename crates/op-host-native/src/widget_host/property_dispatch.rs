//! PropertyPanel action + commit dispatch, split out of `input.rs`
//! to stay under the 800-line cap.
//!
//! The action match itself is shared with the web host in
//! `op_editor_ui::widgets::property_panel_dispatch`; this file keeps
//! the native platform glue — layout-scene sizing resolution, viewport
//! refit, image-crop entry, system-font enumeration, and the desktop
//! codegen clipboard / export intents.

#[path = "property_compositing_dispatch.rs"]
mod property_compositing_dispatch;
#[path = "property_input_dispatch.rs"]
mod property_input_dispatch;

use super::WidgetHostNative;
use op_editor_ui::widgets::property_panel_dispatch as dispatch;
use op_editor_ui::widgets::PropertyPanelAction;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_property_action(&mut self, action: PropertyPanelAction) {
        use dispatch::{
            InstanceLifecycleOutcome, PropertyActionFollowUp as F, PropertyActionOutcome as O,
        };
        use PropertyPanelAction as A;
        if !self.collab_allows_user_action(dispatch::collab_gate_action(&action)) {
            return;
        }
        // ImageTileScale lives in the floating image-fill editor. Any button
        // action may close that editor (or switch away from Tile), so commit
        // its draft before the instance-write scope and before the input can
        // disappear. Regular PropertyPanel presses already blur inputs; this
        // also covers popup-owned actions and direct dispatch in tests/hosts.
        self.commit_image_tile_scale_focus_if_any();
        // The compositing/page actions below use lightweight core
        // mutators. Capture history at the host choke point so an
        // instance redirect is finished before equality is tested and
        // same-value selections never create an empty undo entry.
        let document_before =
            dispatch::updates_document(&action).then(|| self.editor_state.snapshot_for_history());
        // Instance / component lifecycle actions act on the REAL Ref
        // node, so they dispatch BEFORE the instance-write redirect
        // scope below swaps in the merged display node.
        if let InstanceLifecycleOutcome::Handled {
            page_switched,
            select,
        } = dispatch::apply_instance_lifecycle_action(&mut self.editor_state, &action)
        {
            if page_switched {
                self.fit_active_page_after_switch(self.last_viewport_w, self.last_viewport_h);
            }
            if let Some(master) = select {
                self.editor_state.set_single_selection(master);
            }
            self.mark_dirty();
            return;
        }
        // A sizing keyword toggle may temporarily swap an instance's merged
        // display node into the document below. Capture the real canvas size
        // before that scope starts so turning Fill/Hug off freezes exactly
        // what the user sees, without rebuilding a scene from the temporary
        // instance-write representation.
        let resolved_sizing_fallback = match action {
            A::ToggleSizeFillWidth | A::ToggleSizeHugWidth => {
                self.resolved_selected_sizing_axis(true)
            }
            A::ToggleSizeFillHeight | A::ToggleSizeHugHeight => {
                self.resolved_selected_sizing_axis(false)
            }
            _ => None,
        };
        // CHOKE POINT (GAP #10): when the anchor is a Ref, swap in
        // the merged display node so every anchor-keyed mutator below
        // writes into it; `finish_instance_write` then routes the
        // diff onto the RefNode (direct props) / descendants[target]
        // (overrides). See op-editor-core/src/instance_override.rs.
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
        let outcome = dispatch::apply_property_action(
            &mut self.editor_state,
            &action,
            dispatch::PropertyActionContext {
                now_ms: self.now_ms,
                resolved_sizing_fallback,
                image_adjustment_drag: &mut self.image_adjustment_drag,
                effect_radius_drag: &mut self.effect_radius_drag,
            },
        );
        match outcome {
            O::Handled => {}
            O::FollowUp(F::EnterImageCropEdit) => {
                let _ = self.enter_selected_image_crop_edit();
            }
            O::FollowUp(F::ExitImageCropEdit) => {
                self.exit_image_crop_edit();
            }
            O::FollowUp(F::EnsureSystemFontsLoaded) => {
                // Enumerate installed families on first open (TS
                // requests Local Font Access inside the click gesture
                // for the same reason).
                self.ensure_system_fonts_loaded();
            }
            O::HostOwned => self.apply_host_property_action(action),
        }
        if let Some(scope) = instance_scope {
            self.editor_state.finish_instance_write(scope);
        }
        if let Some(before) = document_before {
            if self.editor_state.snapshot_for_history() != before {
                self.editor_state.history_push_past(before);
            }
        }
        // Opening an input-owning Property overlay while the software
        // keyboard is already visible must reveal it immediately. This is
        // idempotent for ordinary actions and complements body-row focus,
        // which is seeded directly by the press tier.
        self.reveal_property_keyboard_owner();
        self.mark_dirty();
    }

    /// The arms the shared dispatcher hands back: image Search /
    /// Generate popovers (host-owned input-selection drag + blur glue),
    /// the effect-param focus seed (needs the host-owned draft commits
    /// first), and the Code panel's platform IO.
    fn apply_host_property_action(&mut self, action: PropertyPanelAction) {
        use PropertyPanelAction as A;
        match action {
            A::MatchImageAspectRatio => self.match_selected_image_aspect_ratio(),
            A::ToggleImageSearchPopover => self.toggle_image_search_popover(),
            A::ToggleImageGeneratePopover => self.toggle_image_generate_popover(),
            A::RunImageSearch => self.run_image_search(),
            A::SelectImageSearchResult(index) => self.select_image_search_result(index),
            A::RunImageGenerate => self.run_image_generate(),
            A::ApplyGeneratedImage => self.apply_generated_image(),
            A::RetryImageGenerate => self.retry_image_generate(),
            A::OpenImageGenSettings => self.open_image_gen_settings(),
            A::FocusEffectParam {
                effect,
                field,
                value,
            } => {
                // Commit whatever draft owned the input before seeding
                // this param's, then re-read the live value.
                self.commit_property_focus_if_any();
                dispatch::focus_effect_param(
                    &mut self.editor_state,
                    effect,
                    field,
                    value,
                    self.now_ms,
                );
            }
            // Code panel actions. SelectFramework / Copy fully work;
            // Generate / Regenerate raise pending flags + flip the
            // phase (drained by the desktop codegen session); Cancel
            // flips the phase AND raises a pending-cancel intent that
            // aborts the in-flight worker; Download / ExportBundle raise
            // pending flags drained by the desktop codegen-export pass
            // (rfd save dialog + fs/zip write).
            A::Codegen(codegen_action) => self.apply_codegen_action(codegen_action),
            // Everything else was applied by the shared dispatcher.
            _ => {}
        }
    }

    fn apply_codegen_action(
        &mut self,
        action: op_editor_ui::widgets::property_panel_action::CodegenAction,
    ) {
        use dispatch::CodegenFollowUp;
        match dispatch::apply_codegen_action(&mut self.editor_state, &action, self.now_ms) {
            CodegenFollowUp::None => {}
            CodegenFollowUp::FrameworkChanged => {
                self.code_selection_drag = None;
            }
            CodegenFollowUp::Copy(code) => {
                // Push the generated code onto the system clipboard via
                // the same queue the MCP-config copy uses; the desktop
                // runner drains `chat.pending_copy_text` into the OS
                // clipboard.
                self.editor_state.chat.queue_copy_text(code);
            }
            CodegenFollowUp::Download => {
                self.editor_state.codegen.pending_download = true;
            }
            CodegenFollowUp::ExportBundle => {
                self.editor_state.codegen.pending_export_bundle = true;
            }
        }
    }
}
