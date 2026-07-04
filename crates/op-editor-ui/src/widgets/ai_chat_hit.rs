/// What a click inside the AI chat panel resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AIChatHit {
    /// Click landed inside disabled/non-actionable chat chrome. Host
    /// should consume it so the canvas underneath is not affected.
    Inside,
    /// Click landed on the input area — host should focus chat.
    FocusInput,
    /// Press landed on selectable input text at a byte offset.
    SelectInputText(usize),
    /// Click landed on the send affordance.
    Send,
    /// Click landed on the stop affordance shown during a streaming
    /// turn.
    Stop,
    /// Click landed on an example card; `prompt` fills the input.
    Example { index: usize, prompt: String },
    /// Click landed on the header / margin — host should start a
    /// drag so the user can move the panel between canvas corners.
    DragHandle,
    /// Press landed on one of the invisible TS-style resize handles.
    Resize(ChatResizeEdge),
    /// Click on the chevron at the top-left of the header — host
    /// flips the `ChatState::collapsed` flag.
    ToggleCollapse,
    /// Click on the maximize / restore affordance in the header.
    ToggleMaximize,
    /// Click on the plus affordance in the header.
    NewChat,
    /// Click on the model chip (bottom-left of the input toolbar) —
    /// host toggles `ui.chat_model_picker.open` to open / close the
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
    /// Click on the footer Agent Team chip — host cycles
    /// `ChatState::agent_team_size`.
    CycleAgentTeam,
    /// Click on the ⚡ speed chip in its new role as the Parallel Agents chip
    /// — host toggles `EditorUiState::parallel_agents_picker_open`.
    ToggleParallelAgentsPicker,
    /// Click on a row inside the open Parallel Agents picker.
    /// Payload is the selected multiplier (1–6). Host sets
    /// `ChatState::agent_team_size` and closes the picker.
    SetParallelAgents(u32),
    /// Click on the attach button — host opens a file picker and
    /// stages the chosen file via `ChatState::add_attachment`.
    AddAttachment,
    /// Click on a staged-attachment chip — payload is the index
    /// into `chat.pending_attachments` to drop.
    RemoveAttachment(usize),
    /// Click on the selected-count chip's clear target — host clears
    /// the current canvas selection.
    ClearSelection,
    /// Click on a message's thinking-block header — host toggles
    /// `ChatMessage::thinking_collapsed` for that message index.
    ToggleThinking(usize),
    /// Click on a message's tool-calls panel header — host toggles
    /// `ChatMessage::tools_collapsed` for that message index.
    ToggleToolCalls(usize),
    /// Click on a single tool-call card header — host sets only
    /// that card's expanded override.
    SetToolCallCardExpanded(usize, usize, bool),
    /// Click on a single design JSON card header — host sets only
    /// that card's expanded override.
    SetDesignBlockExpanded(usize, usize, bool),
    /// Click on a subtask step-card header — host sets only that
    /// step's expanded override.
    SetActionStepExpanded(usize, usize, bool),
    /// Click on a design JSON card's copy affordance.
    CopyDesignBlock(String),
    /// Click on an expanded design JSON card's apply affordance.
    ApplyDesignBlock(usize, String),
    /// Press on selectable transcript text.
    SelectTranscriptText(usize, usize),
    /// Click on the fixed "Pencil it out" checklist header — host
    /// toggles the checklist body between expanded and collapsed.
    ToggleChecklist,
    /// Click on a checklist row's detail-expand chevron — host toggles
    /// `ChatState::set_checklist_item_expanded` for that row index.
    ToggleChecklistItem(usize),
    /// Click on tab `i`'s body (but not its × close button) — host
    /// calls `state.chat.switch_to(i)`. Wired in MT.3.
    SwitchTab(usize),
    /// Click on tab `i`'s close × glyph — host calls
    /// `state.chat.close_tab(i)`. Wired in MT.3.
    CloseTab(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatResizeEdge {
    N,
    S,
    E,
    W,
    Ne,
    Nw,
    Se,
    Sw,
}

impl From<super::ai_chat_transcript::TranscriptHit> for AIChatHit {
    fn from(hit: super::ai_chat_transcript::TranscriptHit) -> Self {
        match hit {
            super::ai_chat_transcript::TranscriptHit::ToggleThinking(i) => Self::ToggleThinking(i),
            super::ai_chat_transcript::TranscriptHit::ToggleToolCalls(i) => {
                Self::ToggleToolCalls(i)
            }
            super::ai_chat_transcript::TranscriptHit::SetToolCallCardExpanded(
                message_index,
                tool_index,
                expanded,
            ) => Self::SetToolCallCardExpanded(message_index, tool_index, expanded),
            super::ai_chat_transcript::TranscriptHit::SetDesignBlockExpanded(
                message_index,
                block_index,
                expanded,
            ) => Self::SetDesignBlockExpanded(message_index, block_index, expanded),
            super::ai_chat_transcript::TranscriptHit::SetActionStepExpanded(
                message_index,
                step_index,
                expanded,
            ) => Self::SetActionStepExpanded(message_index, step_index, expanded),
            super::ai_chat_transcript::TranscriptHit::CopyDesignBlock(text) => {
                Self::CopyDesignBlock(text)
            }
            super::ai_chat_transcript::TranscriptHit::ApplyDesignBlock(message_index, text) => {
                Self::ApplyDesignBlock(message_index, text)
            }
        }
    }
}
