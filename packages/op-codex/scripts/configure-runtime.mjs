import { resolve } from 'node:path';
import { configureRuntime } from '../plugins/openpencil/scripts/runtime-install.mjs';

if (process.argv.length !== 3) throw new Error('Usage: npm run configure-runtime -- /absolute/runtime/root');
const runtime = await configureRuntime(resolve(process.argv[2]));
console.log(`Verified and configured Rust OpenPencil ${runtime.version} (${process.platform}-${process.arch}).`);
