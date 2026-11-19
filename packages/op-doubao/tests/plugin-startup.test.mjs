import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { registerNode } from '../scripts/node-registration.mjs';
import { installDoubaoMcpConfig } from '../scripts/host-config.mjs';
import { verifyPluginHost } from '../scripts/plugin-host-check.mjs';
import { CANVAS_URI } from '../plugins/openpencil/scripts/canvas-controller.mjs';

test('installed Doubao launcher initializes MCP with a registered Node outside stripped GUI PATH', async (t) => {
  const home = await mkdtemp(join(tmpdir(), 'op-doubao-startup-'));
  t.after(() => rm(home, { recursive: true, force: true }));
  const stateDir = join(home, '.doubao-openpencil');
  const mcpConfigPath = join(home, 'doubao-mcp.json');
  await registerNode(stateDir, process.execPath);
  await installDoubaoMcpConfig({ mcpConfigPath });

  const result = await verifyPluginHost({
    mcpConfigPath,
    env: {
      ...process.env,
      HOME: home,
      USERPROFILE: home,
      OPENPENCIL_DOUBAO_STATE_DIR: stateDir,
    },
  });

  assert.equal(result.ok, true);
  assert.equal(result.registered, true);
  assert.equal(result.probe.ok, true);
  assert.equal(result.probe.serverInfo.name, 'doubao-openpencil');
  assert.equal(result.probe.resourceUri, CANVAS_URI);
  assert.equal(result.probe.mimeType, 'text/html;profile=mcp-app');
  assert.ok(result.probe.canvasTools.includes('open_canvas'));
  assert.ok(result.probe.canvasTools.includes('canvas_open_browser'));
});
