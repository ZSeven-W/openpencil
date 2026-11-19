import { randomUUID } from 'node:crypto';
import { existsSync } from 'node:fs';
import { chmod, copyFile, lstat, mkdir, readdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const SERVER_NAME = 'openpencil';
export const PLUGIN_NAME = 'openpencil';
export const PLUGIN_ID = `${PLUGIN_NAME}@local`;
export const SKILL_NAMES = Object.freeze(['openpencil-design', 'openpencil-setup']);
const OWNERSHIP_MARKER = '.openpencil-qoder-managed.json';
const defaultPluginRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../plugins/openpencil');

/**
 * Resolve Qoder's configuration directory following Qoder Desktop's documented order:
 * 1. Explicit configDir argument
 * 2. QODER_CONFIG_DIR / QODERCN_CONFIG_DIR
 * 3. QODER_CLI_HOME + QODER_CONFIG_DIR_NAME (or QODERCN equivalents)
 * 4. Existing ~/.qoder, then ~/.qoder-cn, defaulting to ~/.qoder
 */
export function resolveQoderConfigDir({ configDir, env = process.env, home = homedir() } = {}) {
  if (configDir) return resolve(configDir);
  const direct = env.QODER_CONFIG_DIR?.trim() || env.QODERCN_CONFIG_DIR?.trim();
  if (direct) return resolve(direct);
  if (env.QODER_CLI_HOME?.trim()) {
    return join(resolve(env.QODER_CLI_HOME.trim()), env.QODER_CONFIG_DIR_NAME?.trim() || '.qoder');
  }
  if (env.QODERCN_CLI_HOME?.trim()) {
    return join(resolve(env.QODERCN_CLI_HOME.trim()), env.QODERCN_CONFIG_DIR_NAME?.trim() || '.qoder-cn');
  }
  for (const candidate of ['.qoder', '.qoder-cn']) {
    const dir = join(home, candidate);
    if (existsSync(dir)) return dir;
  }
  return join(home, '.qoder');
}

export function launcherPathForPlugin(pluginRoot = defaultPluginRoot, platform = process.platform) {
  return join(resolve(pluginRoot), 'scripts', platform === 'win32' ? 'launch-mcp.cmd' : 'launch-mcp');
}

export function buildQoderMcpEntry({ pluginRoot = defaultPluginRoot, platform = process.platform, env } = {}) {
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

export function renderQoderConnectorConfig(options = {}) {
  return {
    mcpServers: {
      [SERVER_NAME]: buildQoderMcpEntry(options),
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
  const temp = join(dirname(path), `.qoder-${randomUUID()}.tmp`);
  try {
    await writeFile(temp, JSON.stringify(value, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    await rename(temp, path);
  } finally {
    await rm(temp, { force: true });
  }
}

async function copyTree(sourceDir, destinationDir) {
  await mkdir(destinationDir, { recursive: true, mode: 0o755 });
  const entries = await readdir(sourceDir, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name, 'en'));
  for (const entry of entries) {
    if (entry.name === '.DS_Store' || entry.name === 'Thumbs.db') continue;
    const src = join(sourceDir, entry.name);
    const dst = join(destinationDir, entry.name);
    const info = await lstat(src);
    if (info.isSymbolicLink()) throw new Error(`Plugin bundle must not contain symlinks: ${src}`);
    if (info.isDirectory()) {
      await copyTree(src, dst);
    } else if (info.isFile()) {
      await copyFile(src, dst);
      if (process.platform !== 'win32' && (info.mode & 0o111) !== 0) {
        await chmod(dst, info.mode & 0o777);
      }
    }
  }
}

/**
 * Install the plugin into Qoder's canonical versioned local cache, register
 * openpencil@local in plugins/installed_plugins_v2.json, and enable it in
 * settings.json without clobbering unrelated plugins or settings.
 */
export async function installQoderPlugin({
  configDir,
  pluginRoot = defaultPluginRoot,
  env = process.env,
  home = homedir(),
  version = '0.1.0',
} = {}) {
  const rootDir = resolveQoderConfigDir({ configDir, env, home });
  const installPath = join(rootDir, 'plugins', 'cache', 'local', PLUGIN_NAME, version);
  const pluginsRegistryPath = join(rootDir, 'plugins', 'installed_plugins_v2.json');
  const settingsPath = join(rootDir, 'settings.json');

  if (existsSync(installPath)) {
    const markerPath = join(installPath, OWNERSHIP_MARKER);
    if (!existsSync(markerPath)) {
      throw new Error(`Refusing to overwrite existing unmanaged plugin directory: ${installPath}`);
    }
    await rm(installPath, { recursive: true, force: true });
  }
  await copyTree(resolve(pluginRoot), installPath);
  if (process.platform !== 'win32') {
    await chmod(join(installPath, 'scripts', 'launch-mcp'), 0o755);
  }
  await writeFile(
    join(installPath, OWNERSHIP_MARKER),
    JSON.stringify({ managedBy: '@zseven-w/qoder-openpencil', pluginId: PLUGIN_ID, version }, null, 2) + '\n',
    { mode: 0o644 },
  );

  const registry = await readJsonObject(pluginsRegistryPath);
  const existingPlugins = registry.plugins && typeof registry.plugins === 'object' && !Array.isArray(registry.plugins)
    ? { ...registry.plugins }
    : {};
  const updatedRegistry = {
    ...registry,
    version: 2,
    plugins: {
      ...existingPlugins,
      [PLUGIN_ID]: [{ scope: 'user', installPath, version }],
    },
  };
  await writeJsonAtomic(pluginsRegistryPath, updatedRegistry);

  const settings = await readJsonObject(settingsPath);
  const existingEnabled = settings.enabledPlugins && typeof settings.enabledPlugins === 'object' && !Array.isArray(settings.enabledPlugins)
    ? { ...settings.enabledPlugins }
    : {};
  const updatedSettings = {
    ...settings,
    enabledPlugins: {
      ...existingEnabled,
      [PLUGIN_ID]: true,
    },
  };
  await writeJsonAtomic(settingsPath, updatedSettings);

  return {
    configDir: rootDir,
    pluginId: PLUGIN_ID,
    installPath,
    pluginsRegistryPath,
    settingsPath,
    version,
  };
}

export async function uninstallQoderPlugin({ configDir, env = process.env, home = homedir() } = {}) {
  const rootDir = resolveQoderConfigDir({ configDir, env, home });
  const pluginsRegistryPath = join(rootDir, 'plugins', 'installed_plugins_v2.json');
  const settingsPath = join(rootDir, 'settings.json');
  const removedPaths = [];
  let registryRemoved = false;
  let settingRemoved = false;

  if (existsSync(pluginsRegistryPath)) {
    const registry = await readJsonObject(pluginsRegistryPath);
    if (registry.plugins && typeof registry.plugins === 'object' && Object.hasOwn(registry.plugins, PLUGIN_ID)) {
      const records = Array.isArray(registry.plugins[PLUGIN_ID]) ? registry.plugins[PLUGIN_ID] : [];
      for (const record of records) {
        if (typeof record?.installPath === 'string' && existsSync(join(record.installPath, OWNERSHIP_MARKER))) {
          const info = await lstat(record.installPath);
          if (info.isDirectory() && !info.isSymbolicLink()) {
            await rm(record.installPath, { recursive: true, force: true });
            removedPaths.push(record.installPath);
          }
        }
      }
      const nextPlugins = { ...registry.plugins };
      delete nextPlugins[PLUGIN_ID];
      await writeJsonAtomic(pluginsRegistryPath, { ...registry, version: registry.version ?? 2, plugins: nextPlugins });
      registryRemoved = true;
    }
  }

  if (existsSync(settingsPath)) {
    const settings = await readJsonObject(settingsPath);
    if (settings.enabledPlugins && typeof settings.enabledPlugins === 'object' && Object.hasOwn(settings.enabledPlugins, PLUGIN_ID)) {
      const nextEnabled = { ...settings.enabledPlugins };
      delete nextEnabled[PLUGIN_ID];
      await writeJsonAtomic(settingsPath, { ...settings, enabledPlugins: nextEnabled });
      settingRemoved = true;
    }
  }

  return { configDir: rootDir, pluginId: PLUGIN_ID, registryRemoved, settingRemoved, removedPaths };
}

/** Optional global settings.json mcpServers registration for Qoder IDE / CLI mode. */
export async function installQoderMcpSettings({
  configDir,
  pluginRoot = defaultPluginRoot,
  platform = process.platform,
  env = process.env,
  home = homedir(),
  serverEnv,
} = {}) {
  const rootDir = resolveQoderConfigDir({ configDir, env, home });
  const settingsPath = join(rootDir, 'settings.json');
  const current = await readJsonObject(settingsPath);
  const existingServers = current.mcpServers && typeof current.mcpServers === 'object' && !Array.isArray(current.mcpServers)
    ? { ...current.mcpServers }
    : {};
  const entry = buildQoderMcpEntry({ pluginRoot, platform, env: serverEnv });
  const updated = {
    ...current,
    mcpServers: {
      ...existingServers,
      [SERVER_NAME]: entry,
    },
  };
  await writeJsonAtomic(settingsPath, updated);
  return { configDir: rootDir, settingsPath, serverName: SERVER_NAME, entry };
}

export async function uninstallQoderMcpSettings({ configDir, env = process.env, home = homedir() } = {}) {
  const rootDir = resolveQoderConfigDir({ configDir, env, home });
  const settingsPath = join(rootDir, 'settings.json');
  if (!existsSync(settingsPath)) return { configDir: rootDir, settingsPath, removed: false };
  const current = await readJsonObject(settingsPath);
  if (!current.mcpServers || typeof current.mcpServers !== 'object' || !Object.hasOwn(current.mcpServers, SERVER_NAME)) {
    return { configDir: rootDir, settingsPath, removed: false };
  }
  const nextServers = { ...current.mcpServers };
  delete nextServers[SERVER_NAME];
  await writeJsonAtomic(settingsPath, { ...current, mcpServers: nextServers });
  return { configDir: rootDir, settingsPath, removed: true };
}

export async function installQoderSkills({
  configDir,
  pluginRoot = defaultPluginRoot,
  env = process.env,
  home = homedir(),
  version = '0.1.0',
} = {}) {
  const rootDir = resolveQoderConfigDir({ configDir, env, home });
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
      JSON.stringify({ managedBy: '@zseven-w/qoder-openpencil', skill: skillName, version }, null, 2) + '\n',
      { mode: 0o644 },
    );
    installed.push({ name: skillName, path: target });
  }
  return { configDir: rootDir, skillsDir, installed };
}

export async function uninstallQoderSkills({ configDir, env = process.env, home = homedir() } = {}) {
  const rootDir = resolveQoderConfigDir({ configDir, env, home });
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

export async function inspectQoderRegistration({ configDir, env = process.env, home = homedir() } = {}) {
  const rootDir = resolveQoderConfigDir({ configDir, env, home });
  const pluginsRegistryPath = join(rootDir, 'plugins', 'installed_plugins_v2.json');
  const settingsPath = join(rootDir, 'settings.json');
  const skillsDir = join(rootDir, 'skills');

  let pluginRecord = null;
  if (existsSync(pluginsRegistryPath)) {
    const registry = await readJsonObject(pluginsRegistryPath);
    const records = registry.plugins?.[PLUGIN_ID];
    if (Array.isArray(records) && records.length > 0) pluginRecord = records[0];
  }

  let pluginEnabled = false;
  let mcpEntry = null;
  if (existsSync(settingsPath)) {
    const settings = await readJsonObject(settingsPath);
    pluginEnabled = settings.enabledPlugins?.[PLUGIN_ID] === true;
    if (settings.mcpServers && typeof settings.mcpServers === 'object' && Object.hasOwn(settings.mcpServers, SERVER_NAME)) {
      mcpEntry = settings.mcpServers[SERVER_NAME];
    }
  }

  const skills = SKILL_NAMES.map(name => ({
    name,
    installed: existsSync(join(skillsDir, name, 'SKILL.md')),
    managed: existsSync(join(skillsDir, name, OWNERSHIP_MARKER)),
  }));

  return {
    configDir: rootDir,
    pluginsRegistryPath,
    settingsPath,
    pluginRegistered: pluginRecord !== null,
    pluginEnabled,
    pluginRecord,
    mcpSettingsRegistered: mcpEntry !== null,
    mcpEntry,
    skills,
  };
}
