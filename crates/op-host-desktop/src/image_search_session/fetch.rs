//! Re-export shim: the provider fetch ladder moved to
//! `op_image_enrich::net::fetch` (pure code motion, shared with the mobile
//! image-search session); the session and its tests keep their `fetch::…`
//! paths. The moved functions take the shared `WebOpenverseCredentials`
//! directly — desktop call sites unwrap their `OpenverseCredentials`
//! newtype via `as_web()`.

pub(crate) use op_image_enrich::net::fetch::*;
