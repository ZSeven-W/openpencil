//! Markdown parsing for the Design-MD panel now lives in jian
//! `components::markdown` (block structure + inline `**bold**` / `` `code` `` /
//! `#HEX` styling). Re-exported here so the panel's renderer keeps importing it
//! from this path; the panel owns the block painting + scroll layout.

pub(crate) use jian_widgets::components::markdown::{
    parse_blocks, parse_inline, wrap_runs, MdBlock, MdRun,
};
