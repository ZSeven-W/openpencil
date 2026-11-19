import { randomUUID } from 'node:crypto';
import { existsSync } from 'node:fs';
import { copyFile, lstat, mkdir, readdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const SERVER_NAME = 'openpencil';
export const SKILL_NAMES = Object.freeze(['openpencil-design', 'openpencil-setup']);
const OWNERSHIP_MARKER = '.openpencil-workbuddy-managed.json';
const defaultPluginRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../plugins/openpencil');

/** Resolve WorkBuddy's user config directory matching WorkBuddy.app's resolveConfigDir(). */
export function resolveWorkBuddyConfigDir({ configDir, env = process.env, home = homedir() } = {}) {
  if (configDir) return resolve(configDir);
  const explicit = env.WORKBUDDY_CONFIG_DIR?.trim();
  if (explicit) return resolve(explicit);
  const folder = env.WORKBUDDY_DATA_FOLDER_NAME?.trim() || '.workbuddy';
  return join(home, folder);
}

export function launcherPathForPlugin(pluginRoot = defaultPluginRoot, platform = process.platform) {
  return join(resolve(pluginRoot), 'scripts', platform === 'win32' ? 'launch-mcp.cmd' : 'launch-mcp');
}

export function buildWorkBuddyMcpEntry({ pluginRoot = defaultPluginRoot, platform = process.platform, env } = {}) {
  const root = resolve(pluginRoot);
  const entry = {
    type: 'stdio',
    command: launcherPathForPlugin(root, platform),
    args: [],
    cwd: root,
  };
  if (env && Object.keys(env).length > 0) entry.env = env;
  return entry;
}

export function renderWorkBuddyConnectorConfig(options = {}) {
  return {
    mcpServers: {
      [SERVER_NAME]: buildWorkBuddyMcpEntry(options),
    },
  };
}

async function readJsonObject(path) {
  let raw;
  try {
    raw = await readFile(path, 'utf8');
  } catch (error) {
    if (error.code === 'ENOENT') return {};
    throw error;
  }
  if (!raw.trim()) return {};
  const parsed = JSON.parse(raw);
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error(`Expected a JSON object in ${path}.`);
  }
  return parsed;
}

async function writeJsonAtomic(path, value) {
  await mkdir(dirname(path), { recursive: true, mode: 0o700 });
  const temp = join(dirname(path), `.mcp-${randomUUID()}.tmp`);
  try {
    await writeFile(temp, JSON.stringify(value, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    await rename(temp, path);
  } finally {
    await rm(temp, { force: true });
  }
}

export async function installWorkBuddyMcpConfig({
  configDir,
  pluginRoot = defaultPluginRoot,
  platform = process.platform,
  env = process.env,
  home = homedir(),
  serverEnv,
} = {}) {
  const rootDir = resolveWorkBuddyConfigDir({ configDir, env, home });
  const mcpPath = join(rootDir, 'mcp.json');
  const current = await readJsonObject(mcpPath);
  const existingServers = current.mcpServers && typeof current.mcpServers === 'object' && !Array.isArray(current.mcpServers)
    ? { ...current.mcpServers }
    : {};
  const entry = buildWorkBuddyMcpEntry({ pluginRoot, platform, env: serverEnv });
  const updated = {
    ...current,
    mcpServers: {
      ...existingServers,
      [SERVER_NAME]: entry,
    },
  };
  await writeJsonAtomic(mcpPath, updated);
  return { configDir: rootDir, mcpPath, serverName: SERVER_NAME, entry };
}

export async function uninstallWorkBuddyMcpConfig({ configDir, env = process.env, home = homedir() } = {}) {
  const rootDir = resolveWorkBuddyConfigDir({ configDir, env, home });
  const mcpPath = join(rootDir, 'mcp.json');
  if (!existsSync(mcpPath)) return { configDir: rootDir, mcpPath, removed: false };
  const current = await readJsonObject(mcpPath);
  if (!current.mcpServers || typeof current.mcpServers !== 'object' || !Object.hasOwn(current.mcpServers, SERVER_NAME)) {
    return { configDir: rootDir, mcpPath, removed: false };
  }
  const nextServers = { ...current.mcpServers };
  delete nextServers[SERVER_NAME];
  const updated = { ...current, mcpServers: nextServers };
  await writeJsonAtomic(mcpPath, updated);
  return { configDir: rootDir, mcpPath, removed: true };
}

async function copyTree(sourceDir, destinationDir) {
  await mkdir(destinationDir, { recursive: true, mode: 0o755 });
  const entries = await readdir(sourceDir, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name, 'en'));
  for (const entry of entries) {
    const src = join(sourceDir, entry.name);
    const dst = join(destinationDir, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`Skill bundle must not contain symlinks: ${src}`);
    if (entry.isDirectory()) await copyTree(src, dst);
    else if (entry.isFile()) await copyFile(src, dst);
  }
}

export async function installWorkBuddySkills({
  configDir,
  pluginRoot = defaultPluginRoot,
  env = process.env,
  home = homedir(),
  version = '0.1.0',
} = {}) {
  const rootDir = resolveWorkBuddyConfigDir({ configDir, env, home });
  const skillsDir = join(rootDir, 'skills');
  await mkdir(skillsDir, { recursive: true, mode: 0o755 });
  const installed = [];
  for (const skillName of SKILL_NAMES) {
    const source = join(resolve(pluginRoot), 'skills', skillName);
    const target = join(skillsDir, skillName);
    if (existsSync(target)) {
      const markerPath = join(target, OWNERSHIP_MARKER);
      if (!existsSync(markerPath)) {
        throw new Error(`Refusing to overwrite existing unmanaged skill directory: ${target}`);
      }
      await rm(target, { recursive: true, force: true });
    }
    await copyTree(source, target);
    await writeFile(
      join(target, OWNERSHIP_MARKER),
      JSON.stringify({ managedBy: '@zseven-w/workbuddy-openpencil', skill: skillName, version }, null, 2) + '\n',
      { mode: 0o644 },
    );
    installed.push({ name: skillName, path: target });
  }
  return { configDir: rootDir, skillsDir, installed };
}

export async function uninstallWorkBuddySkills({ configDir, env = process.env, home = homedir() } = {}) {
  const rootDir = resolveWorkBuddyConfigDir({ configDir, env, home });
  const skillsDir = join(rootDir, 'skills');
  const removed = [];
  for (const skillName of SKILL_NAMES) {
    const target = join(skillsDir, skillName);
    const markerPath = join(target, OWNERSHIP_MARKER);
    if (!existsSync(target) || !existsSync(markerPath)) continue;
    const info = await lstat(target);
    if (!info.isDirectory() || info.isSymbolicLink()) continue;
    await rm(target, { recursive: true, force: true });
    removed.push(skillName);
  }
  return { configDir: rootDir, skillsDir, removed };
}

export async function inspectWorkBuddyRegistration({ configDir, env = process.env, home = homedir() } = {}) {
  const rootDir = resolveWorkBuddyConfigDir({ configDir, env, home });
  const mcpPath = join(rootDir, 'mcp.json');
  const skillsDir = join(rootDir, 'skills');
  let entry = null;
  if (existsSync(mcpPath)) {
    const config = await readJsonObject(mcpPath);
    if (config.mcpServers && typeof config.mcpServers === 'object' && Object.hasOwn(config.mcpServers, SERVER_NAME)) {
      entry = config.mcpServers[SERVER_NAME];
    }
  }
  const skills = SKILL_NAMES.map(name => ({
    name,
    installed: existsSync(join(skillsDir, name, 'SKILL.md')),
    managed: existsSync(join(skillsDir, name, OWNERSHIP_MARKER)),
  }));
  return { configDir: rootDir, mcpPath, registered: entry !== null, entry, skills };
}
