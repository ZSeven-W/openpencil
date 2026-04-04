#!/usr/bin/env node
// Ensures the Zig NAPI addon binary exists.
// Tries: 1) already built, 2) compile from source, 3) download prebuilt.

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const NAPI_DIR = path.join(__dirname, '..', 'packages', 'agent-native', 'napi');
const ZIG_OUT = path.join(__dirname, '..', 'packages', 'agent-native', 'zig-out', 'napi', 'agent_napi.node');
const BUNDLED = path.join(NAPI_DIR, 'agent_napi.node');

// 1. Already exists?
if (fs.existsSync(ZIG_OUT) || fs.existsSync(BUNDLED)) {
  console.log('[agent-native] Binary found, skipping build.');
  process.exit(0);
}

// Check if submodule is initialized
if (!fs.existsSync(path.join(NAPI_DIR, 'package.json'))) {
  console.log('[agent-native] Submodule not initialized, skipping. Run: git submodule update --init');
  process.exit(0);
}

// 2. Try Zig build
try {
  execSync('zig version', { stdio: 'ignore' });
  console.log('[agent-native] Zig found, building from source...');
  execSync('zig build napi -Doptimize=ReleaseFast', {
    cwd: path.join(__dirname, '..', 'packages', 'agent-native'),
    stdio: 'inherit',
  });
  if (fs.existsSync(ZIG_OUT)) {
    console.log('[agent-native] Build successful.');
    process.exit(0);
  }
} catch {
  console.log('[agent-native] Zig not available, trying prebuilt download...');
}

// 3. Download prebuilt
const REPO = 'ZSeven-W/agent';
const platform = process.platform;
const arch = process.arch;
const assetName = `agent_napi-${platform}-${arch}.node`;

try {
  const releaseJson = execSync(
    `curl -sL https://api.github.com/repos/${REPO}/releases/latest`,
    { encoding: 'utf8' },
  );
  const release = JSON.parse(releaseJson);
  const asset = release.assets?.find((a) => a.name === assetName);
  if (!asset) {
    console.warn(`[agent-native] No prebuilt binary for ${platform}-${arch}. Build manually with: bun run agent:build`);
    process.exit(0);
  }
  console.log(`[agent-native] Downloading ${asset.browser_download_url}...`);
  execSync(`curl -sL -o "${BUNDLED}" "${asset.browser_download_url}"`, { stdio: 'inherit' });
  console.log('[agent-native] Download complete.');
} catch (err) {
  console.warn(`[agent-native] Could not download prebuilt binary: ${err.message}`);
  console.warn('[agent-native] Build manually with: bun run agent:build');
}
