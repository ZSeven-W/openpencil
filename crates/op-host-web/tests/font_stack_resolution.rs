//! CanvasKit named-font stack resolver coverage.

fn bridge_source() -> String {
    std::fs::read_to_string(format!(
        "{}/src/op_ck_bridge.js",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("CanvasKit bridge source is readable")
}

#[test]
fn canvaskit_named_font_stack_resolution_runs_in_javascript() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut source = bridge_source();
    source.push_str(
        r#"
const assert = (condition, message) => { if (!condition) throw new Error(message); };
const imported = new Map([
  ['inter', { tf: 'imported-inter' }],
  ['later face', { tf: 'imported-later' }],
  ['acme, sans', { tf: 'imported-comma' }],
]);
const system = new Map([
  ['inter', { tf: 'system-inter' }],
  ['system first', { tf: 'system-first' }],
]);
const bundled = new Map([
  ['inter', { tf: 'bundled-inter' }],
  ['outfit', { tf: 'bundled-outfit' }],
]);

const parsed = opCkParseFontFamilyStack('"Acme, Sans", Missing, \'Later Face\'');
assert(JSON.stringify(parsed) === JSON.stringify(['Acme, Sans', 'Missing', 'Later Face']),
  'quoted family commas must not split the CSS stack');

let resolved = opCkResolveRegisteredTypeface('SYSTEM FIRST, Later Face', imported, system, bundled);
assert(resolved && resolved.source === 'system' && resolved.tf === 'system-first',
  'an earlier exact system family must beat a later imported family');

resolved = opCkResolveRegisteredTypeface('InTeR', imported, system, bundled);
assert(resolved && resolved.source === 'imported' && resolved.tf === 'imported-inter',
  'matching must ignore ASCII case and imported must beat system for one candidate');

resolved = opCkResolveRegisteredTypeface('Missing, "ACME, SANS"', imported, system, bundled);
assert(resolved && resolved.familyKey === 'acme, sans' && resolved.tf === 'imported-comma',
  'an unavailable candidate must fall through to the next quoted candidate');

resolved = opCkResolveRegisteredTypeface('Missing, UI-SANS-SERIF, Later Face', imported, system, bundled);
assert(resolved === null,
  'a generic candidate must terminate named lookup and use browser fallback');

// The bundled registry is the fallback below both named registries: it answers
// only when imported and system miss, and never outranks them.
resolved = opCkResolveRegisteredTypeface('Missing, OuTfIt', imported, system, bundled);
assert(resolved && resolved.source === 'bundled' && resolved.tf === 'bundled-outfit',
  'a bundled family must resolve when imported and system both miss, ignoring case');
assert(resolved.key === 'bundled:outfit', 'the bundled cache key must name its registry');

resolved = opCkResolveRegisteredTypeface('Inter', new Map(), system, bundled);
assert(resolved && resolved.source === 'system' && resolved.tf === 'system-inter',
  'a system face must still beat a bundled face of the same name');

resolved = opCkResolveRegisteredTypeface('Outfit', imported, system);
assert(resolved === null,
  'the bundled registry is optional — a 3-arg caller must still resolve');
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
        Err(error) => panic!("failed to start node for CanvasKit bridge test: {error}"),
    };
    child
        .stdin
        .take()
        .expect("node stdin is available")
        .write_all(source.as_bytes())
        .expect("CanvasKit bridge test source is writable");
    let output = child
        .wait_with_output()
        .expect("CanvasKit bridge JavaScript test completes");
    assert!(
        output.status.success(),
        "CanvasKit font stack JavaScript assertions failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn canvaskit_named_font_stack_resolver_is_wired_to_system_registry() {
    let source = bridge_source();

    for marker in [
        "const systemTypefacesByFamily = new Map()",
        "const bundledTypefaces = new Map()",
        "opCkResolveRegisteredTypeface(family, importedTypefaces, systemTypefacesByFamily, bundledTypefaces)",
        "systemTypefacesByFamily.set(key, entry)",
        "registerBundledFont(family, bytes)",
        "bundledTypefaces.set(key, { tf, family: String(family || '') })",
        "registeredCoverageSegments(familyEntry.key, familyEntry.tf, sz, t)",
    ] {
        assert!(
            source.contains(marker),
            "CanvasKit exact-family selection must preserve `{marker}`"
        );
    }
}
