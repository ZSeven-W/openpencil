//! Re-export shim: the skia down-scale pass moved to
//! `op_image_enrich::net::downscale` (pure code motion, shared with the
//! mobile image-search session); existing paths stay valid.

pub(crate) use op_image_enrich::net::downscale::*;
