//! Injectable system-clipboard snapshot used by desktop paste routing.

/// Every clipboard flavor relevant to Cmd/Ctrl+V.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClipboardPayload {
    pub(crate) text: Option<String>,
    pub(crate) html: Option<String>,
    pub(crate) image: Option<crate::clipboard::ClipboardImage>,
}

impl ClipboardPayload {
    pub(super) fn read_text_system() -> Self {
        Self {
            text: crate::clipboard::read_text_paste(),
            ..Self::default()
        }
    }

    pub(super) fn read_chat_system() -> Self {
        let (text, image) = crate::clipboard::read_chat_paste();
        Self {
            text,
            image,
            ..Self::default()
        }
    }

    pub(super) fn read_canvas_system() -> Self {
        let (html, image) = crate::clipboard::read_canvas_paste();
        Self {
            html,
            image,
            ..Self::default()
        }
    }
}
