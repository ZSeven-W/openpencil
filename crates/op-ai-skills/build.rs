//! `include_dir!` embeds `skills/` at compile time but cargo does not
//! track directory contents by itself — without this, adding or editing
//! a skill .md does NOT rebuild the crate and the binary silently serves
//! the stale corpus (cost a debugging round on 2026-06-10).
fn main() {
    println!("cargo:rerun-if-changed=skills");
}
