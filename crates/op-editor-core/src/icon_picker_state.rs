//! Native icon-picker remote search state.
//!
//! The widget layer queues requests here; desktop drains them on a
//! background worker and writes loaded Iconify results back into this
//! cache. Keeping the state in core lets native and web hosts share
//! the same UI contract without putting network code in widgets.

#[derive(Debug, Clone, PartialEq)]
pub struct IconPickerRemoteIcon {
    pub collection: String,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub style: String,
    pub d: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconifyLoadMoreRequest {
    pub query: String,
    pub start: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct IconPickerRemoteState {
    pub query: String,
    pub icons: Vec<IconPickerRemoteIcon>,
    pub total: usize,
    pub next_start: usize,
    pub loading: bool,
    pub error: Option<String>,
}
