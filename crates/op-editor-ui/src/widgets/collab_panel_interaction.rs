//! Interaction geometry for the collaboration popover.
//!
//! Hover state contains only control identities. Invite codes, endpoints, and
//! admission request keys remain in the ephemeral press hit and are never
//! retained by pointer movement.

use super::*;
use op_editor_core::CollabPanelHover;

impl CollabPanel<'_> {
    pub fn hit_test(&self, panel: Rect, point: Point2D) -> Option<CollabPanelHit> {
        if !panel.contains(point) {
            return None;
        }
        if self.close_rect(panel).contains(point) {
            return Some(CollabPanelHit::Close);
        }
        // The action row paints last. On a height-clamped viewport it may
        // overlap body content, so it must also win hit-testing first.
        for (rect, action) in self.action_rects(panel) {
            if rect.contains(point) {
                return Some(CollabPanelHit::Action(action.action));
            }
        }
        // Body content is clipped above the fixed action row on short
        // viewports. Do not leave clipped controls interactive underneath it.
        if !self.body_clip_rect(panel).contains(point) {
            return Some(CollabPanelHit::Inside);
        }
        let body_top = self.body_top(panel);
        match &self.model.screen {
            CollabPanelScreen::SignInRequired => {
                if self.sign_in_rect(panel, body_top).contains(point) {
                    return Some(CollabPanelHit::OpenSignIn);
                }
            }
            CollabPanelScreen::Join { discovered, .. } => {
                // The clear affordance sits inside the input rect, so it must
                // win before the focus hit.
                if self
                    .clear_join_rect(panel, body_top + 22.0)
                    .is_some_and(|rect| rect.contains(point))
                {
                    return Some(CollabPanelHit::ClearJoinAddress);
                }
                if self.address_rect(panel, body_top + 22.0).contains(point) {
                    return Some(CollabPanelHit::FocusJoinAddress);
                }
                let first_y = body_top + 106.0;
                for (index, endpoint) in discovered.iter().take(MAX_VISIBLE_ENDPOINTS).enumerate() {
                    if self.discovered_rect(panel, first_y, index).contains(point) {
                        return Some(if endpoint.compatible {
                            CollabPanelHit::Action(CollabUiAction::JoinDiscovered {
                                discovery_id: endpoint.discovery_id.clone(),
                            })
                        } else {
                            CollabPanelHit::Inside
                        });
                    }
                }
            }
            CollabPanelScreen::Session {
                invite,
                share_endpoint,
                admission_request,
                ..
            } => {
                if let Some(invite) = invite {
                    if self
                        .invite_copy_rect(panel)
                        .is_some_and(|rect| rect.contains(point))
                    {
                        return Some(CollabPanelHit::CopyInvite(invite.as_str().to_string()));
                    }
                }
                if let Some(endpoint) = share_endpoint {
                    if self
                        .share_endpoint_copy_rect(panel)
                        .is_some_and(|rect| rect.contains(point))
                    {
                        return Some(CollabPanelHit::CopyShareEndpoint(
                            endpoint.as_str().to_string(),
                        ));
                    }
                }
                if let Some(request) = admission_request {
                    for (rect, action) in self.admission_action_rects(panel, body_top, request) {
                        if rect.contains(point) {
                            return Some(CollabPanelHit::Action(action.action));
                        }
                    }
                }
            }
            CollabPanelScreen::Create => {
                for (rect, region) in self.region_option_rects(panel, body_top) {
                    if rect.contains(point) {
                        return Some(CollabPanelHit::Action(CollabUiAction::SetRelayRegion {
                            region,
                        }));
                    }
                }
            }
            // The confirmation screen's two decisions live in the fixed action
            // row, which is hit-tested before this match runs.
            CollabPanelScreen::Unavailable
            | CollabPanelScreen::Home
            | CollabPanelScreen::ConfirmOwner(_)
            | CollabPanelScreen::Progress { .. } => {}
        }
        Some(CollabPanelHit::Inside)
    }

    /// Resolve a paint-only hover identity without retaining any clipboard or
    /// admission payload carried by the corresponding press target.
    pub fn hover_at(&self, panel: Rect, point: Point2D) -> Option<CollabPanelHover> {
        if !panel.contains(point) {
            return None;
        }
        if self.close_rect(panel).contains(point) {
            return Some(CollabPanelHover::Close);
        }
        // Match paint and press ordering when viewport height clamps the card.
        if let Some(hover) = self
            .action_rects(panel)
            .into_iter()
            .find_map(|(rect, action)| rect.contains(point).then_some(action.action))
            .and_then(|action| hover_for_action(&action))
        {
            return Some(hover);
        }
        if !self.body_clip_rect(panel).contains(point) {
            return None;
        }
        let body_top = self.body_top(panel);
        match &self.model.screen {
            CollabPanelScreen::SignInRequired => {
                if self.sign_in_rect(panel, body_top).contains(point) {
                    return Some(CollabPanelHover::OpenSignIn);
                }
            }
            CollabPanelScreen::Join { discovered, .. } => {
                if self
                    .clear_join_rect(panel, body_top + 22.0)
                    .is_some_and(|rect| rect.contains(point))
                {
                    return Some(CollabPanelHover::ClearJoinAddress);
                }
                if self.address_rect(panel, body_top + 22.0).contains(point) {
                    return Some(CollabPanelHover::JoinAddress);
                }
                let first_y = body_top + 106.0;
                for (index, endpoint) in discovered.iter().take(MAX_VISIBLE_ENDPOINTS).enumerate() {
                    if endpoint.compatible
                        && self.discovered_rect(panel, first_y, index).contains(point)
                    {
                        return Some(CollabPanelHover::Discovered(index));
                    }
                }
            }
            CollabPanelScreen::Session {
                admission_request, ..
            } => {
                if self
                    .invite_copy_rect(panel)
                    .is_some_and(|rect| rect.contains(point))
                {
                    return Some(CollabPanelHover::CopyInvite);
                }
                if self
                    .share_endpoint_copy_rect(panel)
                    .is_some_and(|rect| rect.contains(point))
                {
                    return Some(CollabPanelHover::CopyShareEndpoint);
                }
                if let Some(request) = admission_request {
                    for (rect, action) in self.admission_action_rects(panel, body_top, request) {
                        if rect.contains(point) {
                            return hover_for_action(&action.action);
                        }
                    }
                }
            }
            CollabPanelScreen::Create => {
                for (rect, region) in self.region_option_rects(panel, body_top) {
                    if rect.contains(point) {
                        return Some(CollabPanelHover::Region(region));
                    }
                }
            }
            // The confirmation screen's two decisions live in the fixed action
            // row, which is hit-tested before this match runs.
            CollabPanelScreen::Unavailable
            | CollabPanelScreen::Home
            | CollabPanelScreen::ConfirmOwner(_)
            | CollabPanelScreen::Progress { .. } => {}
        }
        None
    }

    pub(super) fn body_top(&self, panel: Rect) -> f32 {
        panel.origin.y
            + HEADER_HEIGHT
            + if self.model.notice.is_some() {
                NOTICE_HEIGHT + 8.0
            } else {
                0.0
            }
    }

    pub(super) fn actions_height(&self) -> f32 {
        if self.model.actions.is_empty() {
            0.0
        } else {
            self.action_height() + PAD
        }
    }

    pub(super) fn action_height(&self) -> f32 {
        if self.ui.touch_chrome() {
            44.0
        } else {
            ACTION_HEIGHT
        }
    }

    /// Visible body area. Actions stay fixed at the bottom; when viewport
    /// height clamps the card, body paint is clipped above that row instead of
    /// drawing through it.
    pub(super) fn body_clip_rect(&self, panel: Rect) -> Rect {
        let top = panel.origin.y + HEADER_HEIGHT;
        let bottom = self
            .action_rects(panel)
            .first()
            .map(|(rect, _)| rect.origin.y - ACTION_GAP)
            .unwrap_or(panel.origin.y + panel.size.y)
            .max(top);
        Rect::xywh(panel.origin.x, top, panel.size.x, bottom - top)
    }

    pub(super) fn session_share_height(&self) -> f32 {
        match &self.model.screen {
            CollabPanelScreen::Session {
                share_endpoint: Some(_),
                ..
            } => SHARE_ENDPOINT_HEIGHT,
            _ => 0.0,
        }
    }

    pub(super) fn session_connection_height(&self) -> f32 {
        match &self.model.screen {
            CollabPanelScreen::Session {
                connection: Some(_),
                ..
            } => CONNECTION_PATH_HEIGHT,
            _ => 0.0,
        }
    }

    pub(super) fn session_invite_height(&self) -> f32 {
        match &self.model.screen {
            CollabPanelScreen::Session {
                invite: Some(_), ..
            } => INVITE_HEIGHT,
            _ => 0.0,
        }
    }

    /// Copy target for the owner-only public invite. The invite remains
    /// redacted in every debug projection and enters the clipboard only from
    /// this explicit pointer gesture.
    pub fn invite_copy_rect(&self, panel: Rect) -> Option<Rect> {
        matches!(
            &self.model.screen,
            CollabPanelScreen::Session {
                invite: Some(_),
                ..
            }
        )
        .then(|| {
            Rect::xywh(
                panel.origin.x + panel.size.x - PAD - 24.0,
                self.body_top(panel) + 58.0 + self.session_connection_height() + 13.0,
                24.0,
                24.0,
            )
        })
    }

    /// Copy target for an owner-only manual share address. Returning `None`
    /// keeps guests and pre-auth screens from manufacturing clipboard data.
    pub fn share_endpoint_copy_rect(&self, panel: Rect) -> Option<Rect> {
        matches!(
            &self.model.screen,
            CollabPanelScreen::Session {
                share_endpoint: Some(_),
                ..
            }
        )
        .then(|| {
            Rect::xywh(
                panel.origin.x + panel.size.x - PAD - 24.0,
                self.body_top(panel)
                    + 58.0
                    + self.session_connection_height()
                    + self.session_invite_height()
                    + 8.0,
                24.0,
                24.0,
            )
        })
    }

    pub(super) fn close_rect(&self, panel: Rect) -> Rect {
        if self.ui.touch_chrome() {
            Rect::xywh(
                panel.origin.x + panel.size.x - 44.0,
                panel.origin.y,
                44.0,
                44.0,
            )
        } else {
            Rect::xywh(
                panel.origin.x + panel.size.x - 38.0,
                panel.origin.y + 8.0,
                30.0,
                28.0,
            )
        }
    }

    pub(super) fn address_rect(&self, panel: Rect, y: f32) -> Rect {
        Rect::xywh(
            panel.origin.x + PAD,
            y,
            panel.size.x - PAD * 2.0,
            INPUT_HEIGHT,
        )
    }

    /// Clear (×) affordance inside the join field. `None` while the field is
    /// empty so an idle input never paints or hit-tests a dead button.
    pub(super) fn clear_join_rect(&self, panel: Rect, y: f32) -> Option<Rect> {
        if self.ui.collab.panel.join_input.text().is_empty() {
            return None;
        }
        let input = self.address_rect(panel, y);
        Some(Rect::xywh(
            input.origin.x + input.size.x - CLEAR_BUTTON_SIZE - 5.0,
            input.origin.y + (input.size.y - CLEAR_BUTTON_SIZE) / 2.0,
            CLEAR_BUTTON_SIZE,
            CLEAR_BUTTON_SIZE,
        ))
    }

    pub(super) fn sign_in_rect(&self, panel: Rect, body_top: f32) -> Rect {
        let height = self.action_height();
        Rect::xywh(
            panel.origin.x + PAD,
            body_top + 40.0,
            panel.size.x - PAD * 2.0,
            height,
        )
    }

    pub(super) fn action_rects(&self, panel: Rect) -> Vec<(Rect, CollabPanelActionModel)> {
        if self.model.actions.is_empty() {
            return Vec::new();
        }
        let count = self.model.actions.len() as f32;
        let available = panel.size.x - PAD * 2.0 - ACTION_GAP * (count - 1.0);
        let width = available / count;
        let height = self.action_height();
        let y = panel.origin.y + panel.size.y - PAD - height;
        self.model
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| {
                (
                    Rect::xywh(
                        panel.origin.x + PAD + index as f32 * (width + ACTION_GAP),
                        y,
                        width,
                        height,
                    ),
                    action.clone(),
                )
            })
            .collect()
    }

    pub(super) fn admission_action_rects(
        &self,
        panel: Rect,
        body_top: f32,
        request: &CollabAdmissionRequestModel,
    ) -> Vec<(Rect, CollabPanelActionModel)> {
        let count = request.actions.len() as f32;
        let available = panel.size.x - PAD * 2.0 - ACTION_GAP * (count - 1.0);
        let width = available / count;
        let y = body_top
            + 81.0
            + self.session_connection_height()
            + self.session_invite_height()
            + self.session_share_height();
        request
            .actions
            .iter()
            .enumerate()
            .map(|(index, action)| {
                (
                    Rect::xywh(
                        panel.origin.x + PAD + index as f32 * (width + ACTION_GAP),
                        y,
                        width,
                        ADMISSION_ACTION_HEIGHT,
                    ),
                    action.clone(),
                )
            })
            .collect()
    }

    /// The two service-region options on the create screen. Empty on every
    /// other screen so stale geometry never hit-tests.
    pub(super) fn region_option_rects(
        &self,
        panel: Rect,
        body_top: f32,
    ) -> Vec<(Rect, op_editor_core::CollabRelayRegion)> {
        if self.model.screen != CollabPanelScreen::Create {
            return Vec::new();
        }
        let width = (panel.size.x - PAD * 2.0 - ACTION_GAP) / 2.0;
        let y = body_top + 66.0;
        [
            op_editor_core::CollabRelayRegion::China,
            op_editor_core::CollabRelayRegion::Global,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, region)| {
            (
                Rect::xywh(
                    panel.origin.x + PAD + index as f32 * (width + ACTION_GAP),
                    y,
                    width,
                    REGION_OPTION_HEIGHT,
                ),
                region,
            )
        })
        .collect()
    }

    pub(super) fn discovered_rect(&self, panel: Rect, first_y: f32, index: usize) -> Rect {
        Rect::xywh(
            panel.origin.x + PAD,
            first_y + index as f32 * ROW_HEIGHT,
            panel.size.x - PAD * 2.0,
            ROW_HEIGHT,
        )
    }

    pub(super) fn action_is_hovered(&self, action: &CollabUiAction) -> bool {
        hover_for_action(action).is_some_and(|hover| self.ui.collab.panel.hover == Some(hover))
    }
}

fn hover_for_action(action: &CollabUiAction) -> Option<CollabPanelHover> {
    Some(match action {
        CollabUiAction::OpenCreate => CollabPanelHover::OpenCreate,
        CollabUiAction::Start => CollabPanelHover::Start,
        CollabUiAction::StartLan => CollabPanelHover::StartLan,
        CollabUiAction::SetRelayRegion { region } => CollabPanelHover::Region(*region),
        CollabUiAction::OpenJoin => CollabPanelHover::OpenJoin,
        CollabUiAction::BeginDiscovery => CollabPanelHover::BeginDiscovery,
        CollabUiAction::JoinDiscovered { .. } => return None,
        CollabUiAction::JoinAddress { .. } => CollabPanelHover::Connect,
        CollabUiAction::Cancel => CollabPanelHover::Cancel,
        CollabUiAction::Retry => CollabPanelHover::Retry,
        CollabUiAction::Leave => CollabPanelHover::Leave,
        CollabUiAction::DiscardPending => CollabPanelHover::DiscardPending,
        CollabUiAction::ReapplyDiscarded => CollabPanelHover::ReapplyDiscarded,
        CollabUiAction::SaveAsFork => CollabPanelHover::SaveAsFork,
        CollabUiAction::ApproveAdmissionEditor { .. } => CollabPanelHover::ApproveAdmissionEditor,
        CollabUiAction::ApproveAdmissionViewer { .. } => CollabPanelHover::ApproveAdmissionViewer,
        CollabUiAction::RejectAdmission { .. } => CollabPanelHover::RejectAdmission,
        CollabUiAction::ConfirmOwnerIdentity { .. } => CollabPanelHover::ConfirmOwnerIdentity,
        CollabUiAction::RejectOwnerIdentity { .. } => CollabPanelHover::RejectOwnerIdentity,
    })
}
