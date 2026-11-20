import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { registerNode } from '../scripts/node-registration.mjs';
import { installQoderPlugin } from '../scripts/host-config.mjs';
import { verifyPluginHost } from '../scripts/plugin-host-check.mjs';
import { CANVAS_URI } from '../plugins/openpencil/scripts/canvas-controller.mjs';

test('installed Qoder cached plugin launcher initializes MCP with qoder/ui entrypoints under stripped GUI PATH', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-qoder-startup-'));
  t.after(() => rm(home, { recursive: true, force: true, maxRetries: 10, retryDelay: 50 }));
  const stateDir = join(home, '.qoder-openpencil');
  await registerNode(stateDir, process.execPath);
  await installQoderPlugin({ home, env: {}, version: '0.1.0' });

  const result = await verifyPluginHost({
    home,
    env: {
      ...process.env,
      HOME: home,
      USERPROFILE: home,
      OPENPENCIL_QODER_STATE_DIR: stateDir,
    },
  });

  assert.equal(result.ok, true);
  assert.equal(result.pluginRegistered, true);
  assert.equal(result.pluginEnabled, true);
  assert.equal(result.probe.ok, true);
  assert.equal(result.probe.serverInfo.name, 'qoder-openpencil');
  assert.equal(result.probe.resourceUri, CANVAS_URI);
  assert.equal(result.probe.mimeType, 'text/html;profile=mcp-app');
  assert.deepEqual(result.probe.qoderEntrypoints, ['global', 'thread']);
  assert.ok(result.probe.canvasTools.includes('open_canvas'));
  assert.ok(result.probe.canvasTools.includes('canvas_open_browser'));
});
