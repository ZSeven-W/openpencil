//! `pen2op` — convert a plaintext canonical `.pen` (or legacy `.op`) file into
//! a current-schema `.op` file by routing it THROUGH the canonical
//! `load_canonical` + `normalize_legacy_doc` path the desktop uses to open
//! files. A naive rename does NOT work: the normalize step (version 2.x →
//! rewrite, image nodes missing `src`, `fill`-as-string, etc.) is required for
//! the file to load in current OpenPencil.
//!
//! Usage: `pen2op <in.pen> <out.op>`
//!
//! The output is `serde_json::to_string_pretty(&result.value)` — the same
//! canonical-PenDocument serialize path `op-smoke` uses to dump `state.doc`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(in_path), Some(out_path)) = (args.next(), args.next()) else {
        eprintln!("usage: pen2op <in.pen> <out.op>");
        return ExitCode::from(2);
    };

    let src = match std::fs::read_to_string(&in_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[pen2op] read {in_path} failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Load THROUGH the canonical loader so the legacy-tolerance normalize runs.
    let loaded = match op_pen_loader::load_canonical(&src) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("[pen2op] load_canonical({in_path}) failed: {e:?}");
            return ExitCode::FAILURE;
        }
    };

    if !loaded.warnings.is_empty() {
        eprintln!(
            "[pen2op] {in_path}: {} load warning(s): {:?}",
            loaded.warnings.len(),
            loaded.warnings
        );
    }

    // Same serialize path as op-smoke's `state.doc` dump.
    let json = match serde_json::to_string_pretty(&loaded.value) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[pen2op] serialize {in_path} failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = std::fs::write(&out_path, &json) {
        eprintln!("[pen2op] write {out_path} failed: {e}");
        return ExitCode::FAILURE;
    }

    // Node count: walk the serialized JSON counting objects bearing a `type`
    // key (matches how the loader's normalize pass identifies PenNodes).
    let node_count = serde_json::from_str::<serde_json::Value>(&json)
        .map(|v| count_typed_objects(&v))
        .unwrap_or(0);

    println!(
        "OK {in_path} -> {out_path} nodes={node_count} version={}",
        loaded.value.version
    );
    ExitCode::SUCCESS
}

/// Count objects carrying a `"type"` key anywhere in the tree (iterative walk,
/// no recursion, so deep trees don't blow the stack).
fn count_typed_objects(root: &serde_json::Value) -> usize {
    use serde_json::Value;
    let mut count = 0usize;
    let mut stack = vec![root];
    while let Some(v) = stack.pop() {
        match v {
            Value::Object(map) => {
                if map.contains_key("type") {
                    count += 1;
                }
                for child in map.values() {
                    stack.push(child);
                }
            }
            Value::Array(arr) => stack.extend(arr.iter()),
            _ => {}
        }
    }
    count
}
