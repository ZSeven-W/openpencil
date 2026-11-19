import { randomUUID } from 'node:crypto';
import { existsSync } from 'node:fs';
import { copyFile, lstat, mkdir, readdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { stateRoot } from '../plugins/openpencil/scripts/editor-runtime.mjs';

export const SERVER_NAME = 'openpencil';
export const SKILL_NAMES = Object.freeze(['openpencil-design', 'openpencil-setup']);
const OWNERSHIP_MARKER = '.openpencil-doubao-managed.json';
const defaultPluginRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../plugins/openpencil');

export function resolveDoubaoMcpConfigPath({ mcpConfigPath, env = process.env } = {}) {
  const candidate = mcpConfigPath || env.DOUBAO_MCP_CONFIG?.trim();
  return candidate ? resolve(candidate) : null;
}

export function resolveDoubaoSkillsDir({ skillsDir, env = process.env } = {}) {
  const candidate = skillsDir || env.DOUBAO_SKILLS_DIR?.trim();
  return candidate ? resolve(candidate) : null;
}

export function launcherPathForPlugin(pluginRoot = defaultPluginRoot, platform = process.platform) {
  return join(resolve(pluginRoot), 'scripts', platform === 'win32' ? 'launch-mcp.cmd' : 'launch-mcp');
}

export function buildDoubaoMcpEntry({ pluginRoot = defaultPluginRoot, platform = process.platform, env } = {}) {
  const root = resolve(pluginRoot);
  const entry = {
    type: 'stdio',
    command: launcherPathForPlugin(root, platform),
    args: [],
    cwd: root,
    disabled: false,
  };
  if (env && Object.keys(env).length > 0) entry.env = env;
  return entry;
}

export function renderDoubaoConnectorConfig(options = {}) {
  return {
    mcpServers: {
      [SERVER_NAME]: buildDoubaoMcpEntry(options),
    },
  };
}

/** Return structured form fields and human-readable text for Doubao Desktop's manual connector form. */
export function renderDoubaoConnectorForm(options = {}) {
  const entry = buildDoubaoMcpEntry(options);
  const fields = {
    name: SERVER_NAME,
    transport: 'stdio',
    command: entry.command,
    args: entry.args,
    cwd: entry.cwd,
    env: entry.env || {},
  };
  const text = [
    '豆包电脑版 · 手动添加自定义连接器（技能·连接器·伙伴 → 连接器 → 手动添加）',
    `  名称 (Name):          ${fields.name}`,
    `  传输方式 (Transport): 本地命令 / stdio`,
    `  命令 (Command):       ${fields.command}`,
    `  参数 (Args):          （留空）`,
    `  工作目录 (CWD，如有): ${fields.cwd}`,
    `  环境变量 (Env):       （无需填写，Node 22+ 路径与画布令牌已本地托管）`,
  ].join('\n');
  return { fields, text };
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
  const temp = join(dirname(path), `.doubao-${randomUUID()}.tmp`);
  try {
    await writeFile(temp, JSON.stringify(value, null, 2) + '\n', { flag: 'wx', mode: 0o600 });
    await rename(temp, path);
  } finally {
    await rm(temp, { force: true });
  }
}

/** Write ready-to-paste connector JSON under private state exports. */
export async function exportDoubaoConnectorConfig({
  pluginRoot = defaultPluginRoot,
  platform = process.platform,
  stateDir = stateRoot(),
} = {}) {
  const outputPath = join(resolve(stateDir), 'exports', 'doubao-connector.json');
  const config = renderDoubaoConnectorConfig({ pluginRoot, platform });
  await writeJsonAtomic(outputPath, config);
  return { outputPath, config };
}

export async function installDoubaoMcpConfig({
  mcpConfigPath,
  pluginRoot = defaultPluginRoot,
  platform = process.platform,
  env = process.env,
  serverEnv,
} = {}) {
  const targetPath = resolveDoubaoMcpConfigPath({ mcpConfigPath, env });
  if (!targetPath) throw new Error('Pass --mcp-config /path/to/mcp.json or set DOUBAO_MCP_CONFIG.');
  const current = await readJsonObject(targetPath);
  const existingServers = current.mcpServers && typeof current.mcpServers === 'object' && !Array.isArray(current.mcpServers)
    ? { ...current.mcpServers }
    : {};
  const entry = buildDoubaoMcpEntry({ pluginRoot, platform, env: serverEnv });
  const updated = {
    ...current,
    mcpServers: {
      ...existingServers,
      [SERVER_NAME]: entry,
    },
  };
  await writeJsonAtomic(targetPath, updated);
  return { mcpPath: targetPath, serverName: SERVER_NAME, entry };
}

export async function uninstallDoubaoMcpConfig({ mcpConfigPath, env = process.env } = {}) {
  const targetPath = resolveDoubaoMcpConfigPath({ mcpConfigPath, env });
  if (!targetPath || !existsSync(targetPath)) return { mcpPath: targetPath, removed: false };
  const current = await readJsonObject(targetPath);
  if (!current.mcpServers || typeof current.mcpServers !== 'object' || !Object.hasOwn(current.mcpServers, SERVER_NAME)) {
    return { mcpPath: targetPath, removed: false };
  }
  const nextServers = { ...current.mcpServers };
  delete nextServers[SERVER_NAME];
  await writeJsonAtomic(targetPath, { ...current, mcpServers: nextServers });
  return { mcpPath: targetPath, removed: true };
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

export async function installDoubaoSkills({
  skillsDir,
  pluginRoot = defaultPluginRoot,
  env = process.env,
  version = '0.1.0',
} = {}) {
  const targetRoot = resolveDoubaoSkillsDir({ skillsDir, env });
  if (!targetRoot) throw new Error('Pass --skills-dir /path/to/skills or set DOUBAO_SKILLS_DIR.');
  await mkdir(targetRoot, { recursive: true, mode: 0o755 });
  const installed = [];
  for (const skillName of SKILL_NAMES) {
    const source = join(resolve(pluginRoot), 'skills', skillName);
    const target = join(targetRoot, skillName);
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
      JSON.stringify({ managedBy: '@zseven-w/doubao-openpencil', skill: skillName, version }, null, 2) + '\n',
      { mode: 0o644 },
    );
    installed.push({ name: skillName, path: target });
  }
  return { skillsDir: targetRoot, installed };
}

export async function uninstallDoubaoSkills({ skillsDir, env = process.env } = {}) {
  const targetRoot = resolveDoubaoSkillsDir({ skillsDir, env });
  if (!targetRoot) return { skillsDir: null, removed: [] };
  const removed = [];
  for (const skillName of SKILL_NAMES) {
    const target = join(targetRoot, skillName);
    const markerPath = join(target, OWNERSHIP_MARKER);
    if (!existsSync(target) || !existsSync(markerPath)) continue;
    const info = await lstat(target);
    if (!info.isDirectory() || info.isSymbolicLink()) continue;
    await rm(target, { recursive: true, force: true });
    removed.push(skillName);
  }
  return { skillsDir: targetRoot, removed };
}

export async function inspectDoubaoRegistration({ mcpConfigPath, skillsDir, env = process.env } = {}) {
  const mcpPath = resolveDoubaoMcpConfigPath({ mcpConfigPath, env });
  const targetSkillsDir = resolveDoubaoSkillsDir({ skillsDir, env });
  let entry = null;
  if (mcpPath && existsSync(mcpPath)) {
    const config = await readJsonObject(mcpPath);
    if (config.mcpServers && typeof config.mcpServers === 'object' && Object.hasOwn(config.mcpServers, SERVER_NAME)) {
      entry = config.mcpServers[SERVER_NAME];
    }
  }
  const skills = targetSkillsDir
    ? SKILL_NAMES.map(name => ({
        name,
        installed: existsSync(join(targetSkillsDir, name, 'SKILL.md')),
        managed: existsSync(join(targetSkillsDir, name, OWNERSHIP_MARKER)),
      }))
    : [];
  return { mcpPath, registered: entry !== null, entry, skillsDir: targetSkillsDir, skills };
}
