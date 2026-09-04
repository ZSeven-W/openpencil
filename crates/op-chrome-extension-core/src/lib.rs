//! Logic core of the OpenPencil Chrome extension.
//!
//! Everything the extension decides without asking the browser lives here:
//! which endpoints may be talked to, how a chunked snapshot readback is
//! planned and verified, what the `/mcp` fallback envelope looks like, how a
//! reply is classified, what a download is named, which account hub each
//! region resolves to, what a Hub session reply means, where a capture is
//! allowed to be delivered, and — when that is the account — what the Hub's
//! inbox create request looks like and what its answer means. The extension's
//! JavaScript is left with the parts that genuinely need a browser —
//! `chrome.scripting` / `chrome.tabs` / `chrome.downloads` / `chrome.storage`
//! / `chrome.i18n`, `fetch`, popup DOM updates, and the functions injected
//! into the captured tab.
//!
//! The modules below are plain Rust with no `wasm_bindgen` in sight, so
//! `cargo test -p op-chrome-extension-core` exercises all of it on the host
//! target. [`wasm_api`] is the only thing that knows it is being called from
//! JavaScript, and it does nothing but convert types.

mod account;
mod delivery;
mod design_md;
mod design_md_job;
mod design_md_palette;
mod design_md_render;
mod design_md_validate;
mod endpoint;
mod filename;
mod hub;
mod hub_reply;
mod hub_time;
mod ingress;
mod js_text;
mod op_export;
mod transfer;
mod wasm_api;

#[cfg(test)]
mod account_tests;
#[cfg(test)]
mod delivery_tests;
#[cfg(test)]
mod design_md_optional_tests;
#[cfg(test)]
mod design_md_tests;
#[cfg(test)]
mod endpoint_tests;
#[cfg(test)]
mod filename_tests;
#[cfg(test)]
mod hub_reply_tests;
#[cfg(test)]
mod hub_tests;
#[cfg(test)]
mod ingress_tests;
#[cfg(test)]
mod js_text_tests;
#[cfg(test)]
mod op_export_tests;
#[cfg(test)]
mod transfer_tests;
