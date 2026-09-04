//! Regression coverage for CanvasKit RuntimeEffect shader fills.

use std::io::Write;
use std::process::{Command, Stdio};

fn source(path: &str) -> String {
    std::fs::read_to_string(format!("{}/src/{path}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|error| panic!("failed to read {path}: {error}"))
}

fn run_node_module(javascript: &str, context: &str) {
    let _ = run_node_module_output(javascript, context);
}

fn run_node_module_output(javascript: &str, context: &str) -> Option<String> {
    let mut child = match Command::new("node")
        .args(["--input-type=module"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => panic!("failed to start node for {context}: {error}"),
    };
    child
        .stdin
        .take()
        .expect("node stdin is available")
        .write_all(javascript.as_bytes())
        .unwrap_or_else(|error| panic!("failed to write {context} source: {error}"));
    let output = child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("failed to wait for {context}: {error}"));
    assert!(
        output.status.success(),
        "{context} JavaScript assertions failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
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
    assert!(
        bridge.matches("opCkMakeRuntimeShaderForRect(").count() >= 2,
        "the exported rect-local helper must also be called by the production draw path"
    );
    assert!(
        bridge.contains("entry.effect.makeShader(data, localMatrix)"),
        "CanvasKit RuntimeEffect must receive the computed local matrix"
    );
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
assert(
  JSON.stringify(opCkRuntimeEffectLocalMatrix(
    ['size'], new Float32Array([64, 64]), new Uint32Array([2]), 5, 7, 128, 128,
  )) === JSON.stringify([2, 0, 5, 0, 2, 7, 0, 0, 1]),
  'rect-local matrix must carry both 2x scale and translation',
);
"#,
    );

    run_node_module(&javascript, "RuntimeEffect cache test");
}

#[cfg(feature = "canvaskit")]
fn turbulence_scene_shader() -> op_editor_ui::layout_scene::SceneShader {
    let fixture = r#"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[{
        "type":"rectangle","id":"r","width":64,"height":64,
        "fill":[{"type":"shader","preset":"turbulence",
          "sksl":"AUTHOR_SOURCE_MUST_LOSE",
          "uniforms":{"baseFrequency":[0.08,0.11],"seed":7,"numOctaves":3}}]
      }]}],"children":[]
    }"#;
    let document = jian_ops_schema::load_str(fixture)
        .expect("preset fixture parses")
        .value;
    let scene = op_pen_loader::pen_document_to_layout_scene(
        &document,
        &std::collections::BTreeMap::new(),
        0,
    );
    scene.pages[0].children[0]
        .shader
        .clone()
        .expect("loader expands turbulence")
}

#[cfg(feature = "canvaskit")]
#[test]
fn turbulence_preset_compiles_and_binds_in_real_canvaskit() {
    let shader = turbulence_scene_shader();
    assert!(!shader.sksl.contains("AUTHOR_SOURCE_MUST_LOSE"));

    let names: Vec<&str> = shader
        .uniforms
        .iter()
        .map(|uniform| uniform.name.as_str())
        .collect();
    let values: Vec<f32> = shader
        .uniforms
        .iter()
        .flat_map(|uniform| uniform.values.iter().copied())
        .collect();
    let arities: Vec<usize> = shader
        .uniforms
        .iter()
        .map(|uniform| uniform.values.len())
        .collect();
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/canvaskit");
    let canvas_kit_js = assets.join("canvaskit.js");
    let canvas_kit_wasm = assets.join("canvaskit.wasm");

    let mut javascript = String::from(
        "import { createRequire } from 'module';\nimport fs from 'fs';\n\
         const require = createRequire(import.meta.url);\n",
    );
    javascript.push_str(&source("op_ck_bridge.js"));
    javascript.push_str(&format!(
        r#"
const CanvasKitInit = require({canvas_kit_js});
const CK = await CanvasKitInit({{ wasmBinary: fs.readFileSync({canvas_kit_wasm}) }});
const entry = opCkCreateRuntimeEffectCache(CK).get({sksl});
if (!entry) throw new Error('real CanvasKit RuntimeEffect.Make rejected preset SkSL');
for (const [name, columns] of [['baseFrequency', 2], ['seed', 1], ['size', 2]]) {{
  const info = entry.reflected.get(name);
  if (!info || Number(info.columns) !== columns || Number(info.rows) !== 1) {{
    throw new Error(`missing reflected ${{name}} uniform with arity ${{columns}}`);
  }}
}}
const shader = opCkMakeRuntimeShader(
  entry,
  {names},
  new Float32Array({values}),
  new Uint32Array({arities}),
);
if (!shader) throw new Error('real CanvasKit rejected preset uniform bindings');
if (shader.delete) shader.delete();
if (entry.effect.delete) entry.effect.delete();
"#,
        canvas_kit_js = serde_json::to_string(&canvas_kit_js).expect("JS path serializes"),
        canvas_kit_wasm = serde_json::to_string(&canvas_kit_wasm).expect("WASM path serializes"),
        sksl = serde_json::to_string(&shader.sksl).expect("SkSL serializes"),
        names = serde_json::to_string(&names).expect("uniform names serialize"),
        values = serde_json::to_string(&values).expect("uniform values serialize"),
        arities = serde_json::to_string(&arities).expect("uniform arities serialize"),
    ));
    run_node_module_output(&javascript, "real CanvasKit turbulence compile test")
        .expect("Node is required for the real CanvasKit compile test");
}

#[cfg(all(
    feature = "canvaskit",
    any(target_os = "macos", target_os = "linux", target_os = "windows")
))]
fn native_turbulence_stats(shader: &op_editor_ui::layout_scene::SceneShader) -> (f64, f64) {
    use op_editor_ui::{Color, Rect};
    use op_host_native::backend::NativeBackend;

    let mut surface = skia_safe::surfaces::raster_n32_premul((64, 64)).expect("native surface");
    let mut backend = NativeBackend::with_dpi(1.0);
    let uniforms: Vec<(&str, &[f32])> = shader
        .uniforms
        .iter()
        .map(|uniform| (uniform.name.as_str(), uniform.values.as_slice()))
        .collect();
    backend.fill_round_rect_shader(
        surface.canvas(),
        Rect::xywh(0.0, 0.0, 64.0, 64.0),
        0.0,
        &shader.sksl,
        &uniforms,
        1.0,
        Color::GREEN,
    );
    let image = surface.image_snapshot();
    let pixels = image
        .peek_pixels()
        .expect("native raster pixels")
        .bytes()
        .expect("native pixel bytes");
    channel_stats(pixels.chunks_exact(4).map(|pixel| pixel[0]))
}

#[cfg(all(
    feature = "canvaskit",
    any(target_os = "macos", target_os = "linux", target_os = "windows")
))]
fn channel_stats(values: impl Iterator<Item = u8>) -> (f64, f64) {
    let values: Vec<f64> = values.map(f64::from).collect();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    (mean, variance)
}

#[cfg(all(
    feature = "canvaskit",
    any(target_os = "macos", target_os = "linux", target_os = "windows")
))]
#[test]
fn turbulence_cpu_rasters_match_statistics_and_web_translation_is_local() {
    let shader = turbulence_scene_shader();
    let names: Vec<&str> = shader
        .uniforms
        .iter()
        .map(|uniform| uniform.name.as_str())
        .collect();
    let values: Vec<f32> = shader
        .uniforms
        .iter()
        .flat_map(|uniform| uniform.values.iter().copied())
        .collect();
    let arities: Vec<usize> = shader
        .uniforms
        .iter()
        .map(|uniform| uniform.values.len())
        .collect();
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/canvaskit");
    let canvas_kit_js = assets.join("canvaskit.js");
    let canvas_kit_wasm = assets.join("canvaskit.wasm");

    let mut javascript = String::from(
        "import { createRequire } from 'module';\nimport fs from 'fs';\n\
         const require = createRequire(import.meta.url);\n",
    );
    javascript.push_str(&source("op_ck_bridge.js"));
    javascript.push_str(&format!(
        r#"
const CanvasKitInit = require({canvas_kit_js});
const CK = await CanvasKitInit({{ wasmBinary: fs.readFileSync({canvas_kit_wasm}) }});
const entry = opCkCreateRuntimeEffectCache(CK).get({sksl});
if (!entry) throw new Error('CanvasKit rejected turbulence SkSL');
const names = {names};
const values = new Float32Array({values});
const arities = new Uint32Array({arities});
const imageInfo = (side) => ({{
  width: side,
  height: side,
  colorType: CK.ColorType.RGBA_8888,
  alphaType: CK.AlphaType.Unpremul,
  colorSpace: CK.ColorSpace.SRGB,
}});
const render = (x, y, surfaceSide) => {{
  const surface = CK.MakeSurface(surfaceSide, surfaceSide);
  if (!surface) throw new Error('CK.MakeSurface failed');
  const canvas = surface.getCanvas();
  canvas.clear(CK.Color(0, 0, 0, 0));
  const shader = opCkMakeRuntimeShaderForRect(
    entry, names, values, arities, x, y, 64, 64,
  );
  if (!shader) throw new Error('CanvasKit makeShaderForRect failed');
  const paint = new CK.Paint();
  paint.setAntiAlias(false);
  paint.setShader(shader);
  canvas.drawRect(CK.LTRBRect(x, y, x + 64, y + 64), paint);
  surface.flush();
  const pixels = canvas.readPixels(0, 0, imageInfo(surfaceSide));
  paint.delete();
  shader.delete();
  surface.delete();
  if (!pixels) throw new Error('CanvasKit readPixels failed');
  return pixels;
}};
const crop = (pixels, surfaceSide, x, y) => {{
  const out = new Uint8Array(64 * 64 * 4);
  for (let row = 0; row < 64; row++) {{
    const start = ((y + row) * surfaceSide + x) * 4;
    out.set(pixels.subarray(start, start + 64 * 4), row * 64 * 4);
  }}
  return out;
}};
const stats = (pixels) => {{
  let sum = 0;
  for (let index = 0; index < pixels.length; index += 4) sum += pixels[index];
  const count = pixels.length / 4;
  const mean = sum / count;
  let squared = 0;
  for (let index = 0; index < pixels.length; index += 4) {{
    const delta = pixels[index] - mean;
    squared += delta * delta;
  }}
  return {{ mean, variance: squared / count }};
}};
const first = crop(render(5, 7, 160), 160, 5, 7);
const second = crop(render(80, 80, 160), 160, 80, 80);
for (let index = 0; index < first.length; index++) {{
  if (first[index] !== second[index]) {{
    throw new Error(`web turbulence swims after translation at byte ${{index}}`);
  }}
}}
console.log('B4_STATS ' + JSON.stringify(stats(render(0, 0, 64))));
entry.effect.delete();
"#,
        canvas_kit_js = serde_json::to_string(&canvas_kit_js).expect("JS path serializes"),
        canvas_kit_wasm = serde_json::to_string(&canvas_kit_wasm).expect("WASM path serializes"),
        sksl = serde_json::to_string(&shader.sksl).expect("SkSL serializes"),
        names = serde_json::to_string(&names).expect("uniform names serialize"),
        values = serde_json::to_string(&values).expect("uniform values serialize"),
        arities = serde_json::to_string(&arities).expect("uniform arities serialize"),
    ));
    let output = run_node_module_output(&javascript, "CanvasKit turbulence raster test")
        .expect("Node is required for the real CanvasKit raster test");
    let encoded = output
        .lines()
        .find_map(|line| line.strip_prefix("B4_STATS "))
        .expect("CanvasKit stats sentinel");
    let web: serde_json::Value = serde_json::from_str(encoded).expect("CanvasKit stats JSON");
    let web_mean = web["mean"].as_f64().expect("web mean");
    let web_variance = web["variance"].as_f64().expect("web variance");
    let (native_mean, native_variance) = native_turbulence_stats(&shader);

    assert!(native_variance > 100.0, "native raster is flat");
    assert!(web_variance > 100.0, "CanvasKit raster is flat");
    assert!(
        (native_mean - web_mean).abs() <= 8.0,
        "mean drift: native={native_mean}, web={web_mean}"
    );
    let variance_drift = (native_variance - web_variance).abs() / native_variance.max(web_variance);
    assert!(
        variance_drift <= 0.15,
        "variance drift: native={native_variance}, web={web_variance}"
    );
}
