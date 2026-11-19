import { resolve } from 'node:path';
import { configureRuntime } from '../plugins/openpencil/scripts/runtime-install.mjs';

const raw = process.argv[2];
if (!raw || process.argv.length !== 3) {
  console.error('Usage: npm run configure-runtime -- /absolute/runtime/root');
  process.exit(1);
}
try {
  const runtime = await configureRuntime(resolve(raw), undefined, { source: 'custom' });
  console.log(`Verified and configured Rust OpenPencil ${runtime.version} (${process.platform}-${process.arch}).`);
} catch (error) {
  console.error(`Failed to configure OpenPencil runtime: ${error.message}`);
  process.exit(1);
}
