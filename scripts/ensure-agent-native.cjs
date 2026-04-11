#!/usr/bin/env node
// Ensures the Zig NAPI addon binary exists.
//
// Strategy (fastest to slowest):
//   1. Already built / bundled — use it.
//   2. Download prebuilt from the ZSeven-W/agent release whose tag points at
//      the submodule's currently checked-out commit. This means CI and local
//      installs never need Zig installed, as long as whoever bumped the
//      submodule also tagged + published a matching release.
//   3. Build from source with local Zig (slow but authoritative).
//
// Failing all of those is non-fatal — the postinstall wrapper swallows exit
// codes so `bun install` never breaks. Tests / runtime will surface a clear
// "could not locate agent_napi.node" error instead, with instructions.

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const REPO = 'ZSeven-W/agent';
const NAPI_DIR = path.join(__dirname, '..', 'packages', 'agent-native', 'napi');
const AGENT_DIR = path.join(__dirname, '..', 'packages', 'agent-native');
const ZIG_OUT = path.join(AGENT_DIR, 'zig-out', 'napi', 'agent_napi.node');
const BUNDLED = path.join(NAPI_DIR, 'agent_napi.node');

function log(msg) {
  console.log(`[agent-native] ${msg}`);
}

function assetNameForHost() {
  const p = process.platform; // 'darwin' | 'linux' | 'win32'
  const a = process.arch; // 'arm64' | 'x64'
  const os = p === 'darwin' ? 'macos' : p === 'win32' ? 'windows' : 'linux';
  return `agent_napi-${os}-${a}.node`;
}

function readSubmoduleSha() {
  try {
    return execSync('git rev-parse HEAD', {
      cwd: AGENT_DIR,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return null;
  }
}

function ghJson(url) {
  const headers = ['-H', 'Accept: application/vnd.github+json'];
  if (process.env.GITHUB_TOKEN) {
    headers.push('-H', `Authorization: Bearer ${process.env.GITHUB_TOKEN}`);
  }
  const raw = execSync(`curl -sLf ${headers.join(' ')} "${url}"`, { encoding: 'utf8' });
  return JSON.parse(raw);
}

function findMatchingRelease(submoduleSha) {
  if (!submoduleSha) return null;
  // Walk tags (paginated) until we find one whose commit SHA matches the
  // submodule pointer. Tags are listed newest first, so for a freshly bumped
  // submodule we almost always hit on the first page.
  for (let page = 1; page <= 3; page += 1) {
    let tags;
    try {
      tags = ghJson(`https://api.github.com/repos/${REPO}/tags?per_page=30&page=${page}`);
    } catch {
      return null;
    }
    if (!Array.isArray(tags) || tags.length === 0) return null;
    for (const t of tags) {
      if (t?.commit?.sha === submoduleSha) return t.name;
    }
  }
  return null;
}

function downloadPrebuilt(tagName) {
  let release;
  try {
    release = ghJson(`https://api.github.com/repos/${REPO}/releases/tags/${tagName}`);
  } catch (err) {
    log(`No release for tag ${tagName}: ${err.message}`);
    return false;
  }
  const assetName = assetNameForHost();
  const asset = release.assets?.find((a) => a.name === assetName);
  if (!asset) {
    log(
      `Release ${tagName} has no asset ${assetName} (built: ${(release.assets ?? []).map((a) => a.name).join(', ') || 'none'}).`,
    );
    return false;
  }
  log(`Downloading ${assetName} from release ${tagName}…`);
  try {
    fs.mkdirSync(NAPI_DIR, { recursive: true });
    execSync(`curl -sLf -o "${BUNDLED}" "${asset.browser_download_url}"`, { stdio: 'inherit' });
  } catch (err) {
    log(`Download failed: ${err.message}`);
    return false;
  }
  return fs.existsSync(BUNDLED);
}

function buildFromSource() {
  try {
    execSync('zig version', { stdio: 'ignore' });
  } catch {
    log('Zig not installed; cannot build from source.');
    return false;
  }
  log('Building NAPI addon from source (zig build napi)…');
  try {
    execSync('zig build napi -Doptimize=ReleaseFast', {
      cwd: AGENT_DIR,
      stdio: 'inherit',
    });
  } catch (err) {
    log(`Source build failed: ${err.message}`);
    return false;
  }
  return fs.existsSync(ZIG_OUT);
}

function main() {
  // 1. Already have it?
  if (fs.existsSync(ZIG_OUT) || fs.existsSync(BUNDLED)) {
    log('Binary already present, skipping.');
    return 0;
  }

  // Submodule initialized?
  if (!fs.existsSync(path.join(NAPI_DIR, 'package.json'))) {
    log('Submodule not initialized; run `git submodule update --init`. Skipping.');
    return 0;
  }

  // 2. Prebuilt release matching submodule SHA?
  const sha = readSubmoduleSha();
  if (sha) {
    const tag = findMatchingRelease(sha);
    if (tag) {
      if (downloadPrebuilt(tag)) {
        log(`Prebuilt ready at ${BUNDLED}.`);
        return 0;
      }
    } else {
      log(`No release tag matches submodule ${sha.slice(0, 7)}.`);
    }
  }

  // 3. Source build fallback.
  if (buildFromSource()) {
    log(`Built at ${ZIG_OUT}.`);
    return 0;
  }

  log('Could not provision agent_napi.node. Tests / runtime will fail loudly until resolved.');
  log(
    'Options: bump + tag the agent submodule, or install Zig 0.15+ and run `bun run agent:build`.',
  );
  return 0; // Non-fatal: let the wrapper keep install green, real error surfaces at test time.
}

process.exit(main());
