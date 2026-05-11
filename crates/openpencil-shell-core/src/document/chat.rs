//! Floating AI chat panel state.

/// Floating AI chat panel state — mirrors the TS app's
/// `useAIStore` (messages, input draft, focused flag).
#[derive(Debug, Clone)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub focused: bool,
    /// Which canvas corner the floating chat panel snaps to.
    /// User can drag the panel by its header; on release the
    /// host computes the nearest corner and updates this field.
    pub anchor: ChatAnchor,
    /// Collapsed state — when true the panel paints only the
    /// 36 px header strip (clicking the chevron toggles).
    pub collapsed: bool,
    /// Last user-action timestamp (focus / keystroke) in
    /// milliseconds — drives the caret blink phase via
    /// [`jian_core::anim::blink_visible`]. Reset on focus and on
    /// every key event so the caret reappears immediately when
    /// the user types instead of mid-blink.
    pub caret_anchor_ms: u64,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            focused: false,
            anchor: ChatAnchor::BottomLeft,
            collapsed: false,
            caret_anchor_ms: 0,
        }
    }
}

/// Which corner of the canvas region the AI chat panel sits in.
/// Step 5 P2: 4-corner edge snap on drag release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ChatAnchor {
    /// Pick the nearest corner to the given panel-center point
    /// inside the canvas rect. `(canvas_x0, canvas_y0)` is the
    /// canvas top-left, `(canvas_w, canvas_h)` its size.
    pub fn nearest(
        center: crate::Point2D,
        canvas_x0: f32,
        canvas_y0: f32,
        canvas_w: f32,
        canvas_h: f32,
    ) -> Self {
        let mid_x = canvas_x0 + canvas_w / 2.0;
        let mid_y = canvas_y0 + canvas_h / 2.0;
        let left = center.x < mid_x;
        let top = center.y < mid_y;
        match (top, left) {
            (true, true) => ChatAnchor::TopLeft,
            (true, false) => ChatAnchor::TopRight,
            (false, true) => ChatAnchor::BottomLeft,
            (false, false) => ChatAnchor::BottomRight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatState {
    /// Append the focused input as a new user message + a stub
    /// assistant echo, then clear the buffer. Real AI streaming
    /// lands in Step 6+ (matches TS app's `aiStore.send` flow).
    pub fn send(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }
        let user_msg = ChatMessage {
            role: ChatRole::User,
            content: trimmed.to_string(),
        };
        let echo = ChatMessage {
            role: ChatRole::Assistant,
            content: format!("(stub) Got it — \"{}\"", trimmed),
        };
        self.messages.push(user_msg);
        self.messages.push(echo);
        self.input.clear();
    }
}
