#!/usr/bin/env node
// Provisions the Zig NAPI addon binary by building it from source.
//
// We always build from source on the host so the resulting `agent_napi.node`
// matches the runner's platform/arch. Earlier revisions also tried to download
// a prebuilt from a sibling release repo, but that path was racy: when the
// prebuilt was missing for the current submodule SHA the build fell through
// to source compilation, deposited the binary at `zig-out/napi/...`, and
// electron-builder (which only ships `packages/agent-native/napi/`) silently
// shipped without the addon — every chat request then died at the dynamic
// `@zseven-w/agent-native` import.
//
// Build prerequisite: Zig 0.15+ on PATH. CI workflows install it via
// `mlugg/setup-zig`; local devs install once via their package manager.
//
// Set OPENPENCIL_REQUIRE_AGENT_NATIVE=1 to fail the install when the build
// can't run (electron CI uses this to surface missing prerequisites early).
//
// Set OPENPENCIL_SKIP_AGENT_NATIVE=1 to no-op the script entirely. Useful for
// workflows (npm publish, lint-only CI) that never load the addon at runtime
// and would otherwise pay for a Zig build on every install.
//
// Set ZIG_TARGET to cross-compile for a non-host triple (e.g. on a macOS arm64
// runner build for x86_64-macos with `ZIG_TARGET=x86_64-macos`). Without it
// the build follows the host arch — fine for native runs, wrong when the
// runner doesn't match the artifact you intend to ship.

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const AGENT_DIR = path.join(__dirname, '..', 'packages', 'agent-native');
const NAPI_DIR = path.join(AGENT_DIR, 'napi');
const ZIG_OUT = path.join(AGENT_DIR, 'zig-out', 'napi', 'agent_napi.node');
const BUNDLED = path.join(NAPI_DIR, 'agent_napi.node');
const STRICT = process.env.OPENPENCIL_REQUIRE_AGENT_NATIVE === '1';

function log(msg) {
  console.log(`[agent-native] ${msg}`);
}

function fail(msg) {
  log(msg);
  return STRICT ? 1 : 0;
}

function bundleBinary() {
  fs.mkdirSync(NAPI_DIR, { recursive: true });
  fs.copyFileSync(ZIG_OUT, BUNDLED);
  log(`Bundled binary at ${BUNDLED}.`);
}

function buildFromSource() {
  try {
    execSync('zig version', { stdio: 'ignore' });
  } catch {
    return fail(
      'Zig not installed (need 0.15+). Skipping. Install Zig and re-run `bun run agent:build`.',
    );
  }
  const target = process.env.ZIG_TARGET?.trim();
  const targetFlag = target ? ` -Dtarget=${target}` : '';
  log(`Building NAPI addon (zig build napi -Doptimize=ReleaseFast${targetFlag})…`);
  try {
    execSync(`zig build napi -Doptimize=ReleaseFast${targetFlag}`, {
      cwd: AGENT_DIR,
      stdio: 'inherit',
    });
  } catch (err) {
    return fail(`Zig build failed: ${err.message}`);
  }
  if (!fs.existsSync(ZIG_OUT)) {
    return fail(`Zig build produced no output at ${ZIG_OUT}.`);
  }
  bundleBinary();
  return 0;
}

function main() {
  if (process.env.OPENPENCIL_SKIP_AGENT_NATIVE === '1') {
    log('OPENPENCIL_SKIP_AGENT_NATIVE=1, skipping native binary provisioning.');
    return 0;
  }

  if (!fs.existsSync(path.join(NAPI_DIR, 'package.json'))) {
    return fail('Submodule not initialized; run `git submodule update --init`. Skipping.');
  }

  // Fast path: binary already in place. Make sure both lookup locations are
  // populated so electron-builder (`napi/`) and the runtime loader (which
  // checks `zig-out/` first) both find it without re-running the build.
  if (fs.existsSync(BUNDLED)) {
    log('Binary already present, skipping rebuild.');
    return 0;
  }
  if (fs.existsSync(ZIG_OUT)) {
    log('Binary already built; copying into napi/ for electron-builder.');
    bundleBinary();
    return 0;
  }

  return buildFromSource();
}

process.exit(main());
