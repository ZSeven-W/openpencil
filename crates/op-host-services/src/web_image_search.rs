//! Re-export shim: the browser image-search route's provider ladder moved
//! to `op_image_enrich::net::providers` (pure code motion) so the mobile
//! FFI host resolves design image slots through the same ladder; every
//! existing `op_host_services::web_image_search::…` path stays valid.

pub use op_image_enrich::net::providers::*;
