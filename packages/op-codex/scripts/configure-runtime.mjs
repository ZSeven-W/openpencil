import { resolve } from 'node:path';
import { configureRuntime } from '../plugins/openpencil/scripts/runtime-install.mjs';

if (process.argv.length !== 3) {
  throw new Error('Usage: codex-openpencil configure-runtime /absolute/runtime/root (or npm run configure-runtime -- /absolute/runtime/root)');
}
const runtime = await configureRuntime(resolve(process.argv[2]), undefined, { source: 'custom' });
console.log(`Verified and configured Rust OpenPencil ${runtime.version} (${process.platform}-${process.arch}).`);
