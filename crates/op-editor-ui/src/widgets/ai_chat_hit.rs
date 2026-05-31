/// What a click inside the AI chat panel resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AIChatHit {
    /// Click landed on the input area — host should focus chat.
    FocusInput,
    /// Click landed on the send affordance.
    Send,
    /// Click landed on an example card; payload is the example's
    /// title (host fills the input with this).
    Example(String),
    /// Click landed on the header / margin — host should start a
    /// drag so the user can move the panel between canvas corners.
    DragHandle,
    /// Click on the chevron at the top-left of the header — host
    /// flips the `ChatState::collapsed` flag.
    ToggleCollapse,
    /// Click on the maximize / restore affordance in the header.
    ToggleMaximize,
    /// Click on the plus affordance in the header.
    NewChat,
    /// Click on the model chip (bottom-left of the input toolbar) —
    /// host toggles `ui.chat_model_picker_open` to open / close the
    /// model dropdown.
    ToggleModelPicker,
    /// Click on a model row in the open picker dropdown — payload
    /// is the index into `chat.available_models`
    /// (`Document::select_chat_model`).
    SelectModel(usize),
    /// Click landed inside the model-picker search/header area.
    /// The picker owns keyboard input while open, so this consumes
    /// the click without closing the dropdown.
    FocusModelSearch,
    /// Click on the clear affordance inside the model-picker search.
    ClearModelSearch,
    /// Click on the thinking-mode chip — host cycles
    /// `ChatState::thinking_mode`.
    CycleThinking,
    /// Click on the effort chip — host cycles
    /// `ChatState::effort_level`.
    CycleEffort,
    /// Click on the attach button — host opens a file picker and
    /// stages the chosen file via `ChatState::add_attachment`.
    AddAttachment,
    /// Click on a staged-attachment chip — payload is the index
    /// into `chat.pending_attachments` to drop.
    RemoveAttachment(usize),
    /// Click on a message's thinking-block header — host toggles
    /// `ChatMessage::thinking_collapsed` for that message index.
    ToggleThinking(usize),
    /// Click on a message's tool-calls panel header — host toggles
    /// `ChatMessage::tools_collapsed` for that message index.
    ToggleToolCalls(usize),
    /// Click on the fixed "Pencil it out" checklist header — host
    /// toggles the checklist body between expanded and collapsed.
    ToggleChecklist,
}
