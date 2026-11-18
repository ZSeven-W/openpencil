import { join } from 'node:path';
import { readFile, stat } from 'node:fs/promises';

if (!process.env.CODEX_HOME) throw new Error('Set CODEX_HOME to the isolated install profile to test.');
const { version } = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
process.env.OPENPENCIL_TEST_BRIDGE = join(process.env.CODEX_HOME, 'plugins/cache/zseven-openpencil/openpencil', version, 'scripts/mcp-bridge.mjs');
await stat(process.env.OPENPENCIL_TEST_BRIDGE);
await import('./canvas-smoke.mjs');
