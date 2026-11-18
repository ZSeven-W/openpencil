import { execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, readdir, copyFile, chmod } from 'node:fs/promises';
import { join, resolve, relative, sep } from 'node:path';
import { sha256 } from '../plugins/openpencil/scripts/editor-runtime.mjs';
import { packArchive } from '../plugins/openpencil/scripts/runtime-archive.mjs';

const [source, web, destination] = process.argv.slice(2).map(path => resolve(path));
if (!destination) throw new Error('Usage: node scripts/stage-runtime.mjs SOURCE_CHECKOUT WEB_BUNDLE OUTPUT');
const platform = `${process.platform}-${process.arch}`;
const binaryName = process.platform === 'win32' ? 'op-host-web-server.exe' : 'op-host-web-server';
const cargo = await readFile(join(source, 'Cargo.toml'), 'utf8');
const openPencilVersion = /^\[workspace\.package\][\s\S]*?^version = "([^"]+)"/m.exec(cargo)?.[1];
if (!openPencilVersion || !/^\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(openPencilVersion)) {
  throw new Error(`Could not resolve Cargo workspace version in ${join(source, 'Cargo.toml')}.`);
}
const revision = (process.env.OPENPENCIL_REVISION?.trim()
  || execFileSync('git', ['-C', source, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim());
if (!/^[a-f0-9]{40}$/.test(revision)) {
  throw new Error(`Invalid OpenPencil revision: ${revision}`);
}
const root = join(destination, 'runtime');
await mkdir(join(root, 'bin'), { recursive: true });
await copyFile(join(source, 'target/release', binaryName), join(root, 'bin', binaryName));
await chmod(join(root, 'bin', binaryName), 0o755);
const files = new Map();
async function copyTree(from, to) {
  await mkdir(to, { recursive: true });
  for (const entry of await readdir(from, { withFileTypes: true })) {
    if (entry.isSymbolicLink()) throw new Error('Runtime payload cannot contain symlinks.');
    if (entry.isDirectory()) await copyTree(join(from, entry.name), join(to, entry.name));
    else if (entry.isFile() && !/\.(d\.ts|opt\.wasm)$/.test(entry.name)) await copyFile(join(from, entry.name), join(to, entry.name));
  }
}
await copyTree(web, join(root, 'bin/web-bundle'));
await copyFile(join(source, 'LICENSE'), join(root, 'LICENSE'));
await writeFile(join(root, 'NOTICE'), 'OpenPencil Rust editor: https://github.com/ZSeven-W/openpencil\nCanvasKit / Skia and bundled fonts retain their included licenses.\n');
async function collect(dir) {
  for (const entry of (await readdir(dir, { withFileTypes: true })).sort((a,b) => a.name.localeCompare(b.name, 'en'))) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) await collect(path);
    else files.set(relative(root, path).split(sep).join('/'), await readFile(path));
  }
}
await collect(root);
const manifest = { schemaVersion: 1, platform, openPencilVersion, openPencilRevision: revision,
  files: Object.fromEntries([...files].map(([name, bytes]) => [name, sha256(bytes)])) };
const bytes = Buffer.from(JSON.stringify(manifest, null, 2) + '\n');
await writeFile(join(root, 'openpencil-runtime.json'), bytes);
files.set('openpencil-runtime.json', bytes);
const archive = packArchive(files);
const name = `openpencil-runtime-${platform}.tar.gz`;
await writeFile(join(destination, name), archive);
await writeFile(join(destination, `${platform}.json`), JSON.stringify({ platform, asset: name, bytes: archive.length, sha256: sha256(archive) }, null, 2) + '\n');
console.log(`Staged ${platform}: ${files.size} files, ${archive.length} compressed bytes.`);
