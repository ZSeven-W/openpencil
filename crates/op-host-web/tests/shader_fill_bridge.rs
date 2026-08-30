//! Regression coverage for CanvasKit RuntimeEffect shader fills.

use std::io::Write;
use std::process::{Command, Stdio};

fn source(path: &str) -> String {
    std::fs::read_to_string(format!("{}/src/{path}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|error| panic!("failed to read {path}: {error}"))
}

#[test]
fn canvaskit_backend_overrides_both_shader_fill_methods() {
    let backend = source("canvaskit/backend.rs");
    let bindings = source("canvaskit/bindings.rs");
    let ops = source("canvaskit/ops.rs");
    let bridge = source("op_ck_bridge.js");

    for (rust_name, js_name) in [
        ("fill_round_rect_shader", "fillRoundRectShader"),
        (
            "fill_round_rect_shader_per_corner",
            "fillRoundRectShaderPerCorner",
        ),
    ] {
        assert!(
            backend.contains(&format!("fn {rust_name}(")),
            "CanvasKitBackend must override `{rust_name}` instead of using the solid default"
        );
        assert!(
            ops.contains(&format!("fn {rust_name}(")),
            "missing CanvasKit operation `{rust_name}`"
        );
        assert!(
            bindings.contains(&format!("js_name = {js_name}")),
            "missing wasm-bindgen declaration for `{js_name}`"
        );
        assert!(
            bridge.contains(&format!("{js_name}(")),
            "missing JavaScript bridge method `{js_name}`"
        );
    }
}

#[test]
fn runtime_effect_cache_compiles_each_source_once_and_caches_failures() {
    let mut javascript = source("op_ck_bridge.js");
    javascript.push_str(
        r#"
const assert = (condition, message) => { if (!condition) throw new Error(message); };
let compileCount = 0;
let boundUniforms = null;
const effect = {
  getUniformFloatCount: () => 4,
  getUniformCount: () => 1,
  getUniformName: () => 'tint',
  getUniform: () => ({ columns: 4, rows: 1, isInteger: false, slot: 0 }),
  makeShader: (data) => { boundUniforms = Array.from(data); return { delete() {} }; },
};
const CK = {
  RuntimeEffect: {
    Make(source) {
      compileCount += 1;
      return source === 'invalid' ? null : effect;
    },
  },
};
const cache = opCkCreateRuntimeEffectCache(CK);
const first = cache.get('valid');
assert(first === cache.get('valid'), 'the same source must reuse one compiled effect');
assert(cache.get('invalid') === null, 'compile failure must return null');
assert(cache.get('invalid') === null, 'compile failure must be cached');
assert(compileCount === 2, 'valid and invalid sources must each compile exactly once');

const shader = opCkMakeRuntimeShader(
  first,
  ['missing', 'tint'],
  new Float32Array([9, 0.1, 0.2, 0.3, 0.4]),
  new Uint32Array([1, 4]),
);
assert(shader !== null, 'a compiled effect with valid uniforms must build a shader');
assert(
  JSON.stringify(boundUniforms) === JSON.stringify(Array.from(new Float32Array([0.1, 0.2, 0.3, 0.4]))),
  'uniforms must bind by reflected name and slot while unknown names are ignored',
);
"#,
    );

    let mut child = match Command::new("node")
        .args(["--input-type=module"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => panic!("failed to start node for RuntimeEffect cache test: {error}"),
    };
    child
        .stdin
        .take()
        .expect("node stdin is available")
        .write_all(javascript.as_bytes())
        .expect("RuntimeEffect cache test source is writable");
    let output = child
        .wait_with_output()
        .expect("RuntimeEffect cache JavaScript test completes");
    assert!(
        output.status.success(),
        "RuntimeEffect cache JavaScript assertions failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
