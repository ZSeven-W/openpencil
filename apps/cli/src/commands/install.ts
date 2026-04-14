/**
 * `op install` — install openpencil-skill for AI coding agents.
 *
 * The skill files are embedded at build time (via skill-bundle.json).
 * If the bundle is empty (e.g. dev build without the skill repo), falls back to git clone.
 *
 * Auto-detects installed agents (Claude Code, Codex, Cursor, Gemini CLI, OpenCode)
 * and installs the skill for each, or use `--target <name>` to install for one.
 */

import { execSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  symlinkSync,
  readFileSync,
  writeFileSync,
  rmSync,
  unlinkSync,
} from 'node:fs';
import { join, dirname } from 'node:path';
import { homedir } from 'node:os';
import bundle from './skill-bundle.json';

const REPO = 'zseven-w/openpencil-skill';
const REPO_URL = `https://github.com/${REPO}.git`;
const SKILL_NAME = 'openpencil-skill';

type Target = 'claude' | 'codex' | 'cursor' | 'gemini' | 'opencode';

const ALL_TARGETS: Target[] = ['claude', 'codex', 'cursor', 'gemini', 'opencode'];

const hasBundledFiles = Object.keys(bundle.files).length > 0;

// --- Helpers ---

function which(cmd: string): string | null {
  try {
    return execSync(`which ${cmd} 2>/dev/null`, { encoding: 'utf-8' }).trim() || null;
  } catch {
    return null;
  }
}

function detectTargets(): Target[] {
  const found: Target[] = [];
  if (which('claude')) found.push('claude');
  if (which('codex')) found.push('codex');
  if (existsSync(join(homedir(), '.cursor'))) found.push('cursor');
  if (which('gemini')) found.push('gemini');
  if (which('opencode')) found.push('opencode');
  return found;
}

function log(msg: string): void {
  process.stderr.write(msg + '\n');
}
function logOk(target: string, msg: string): void {
  log(`  ✓ ${target}: ${msg}`);
}
function logSkip(target: string, msg: string): void {
  log(`  - ${target}: ${msg}`);
}
function logErr(target: string, msg: string): void {
  log(`  ✗ ${target}: ${msg}`);
}

/** Write bundled files to a destination directory. */
function writeBundleTo(dest: string, fileFilter?: (relativePath: string) => boolean): void {
  const files = bundle.files as Record<string, string>;
  for (const [relativePath, content] of Object.entries(files)) {
    if (fileFilter && !fileFilter(relativePath)) continue;
    const fullPath = join(dest, relativePath);
    mkdirSync(dirname(fullPath), { recursive: true });
    writeFileSync(fullPath, content);
  }
}

/** Git clone/update (fallback when no bundle). */
function ensureRepo(dest: string): void {
  if (existsSync(join(dest, '.git'))) {
    execSync('git pull --ff-only 2>/dev/null', { cwd: dest, stdio: 'ignore' });
  } else {
    mkdirSync(dirname(dest), { recursive: true });
    execSync(`git clone --depth 1 ${REPO_URL} "${dest}"`, { stdio: 'ignore' });
  }
}

/** Write bundled files or fall back to git clone. */
function ensureSkillDir(dest: string, fileFilter?: (p: string) => boolean): void {
  if (hasBundledFiles) {
    mkdirSync(dest, { recursive: true });
    writeBundleTo(dest, fileFilter);
  } else {
    ensureRepo(dest);
  }
}

// --- Installers ---

function installClaude(): void {
  const target = 'Claude Code';
  try {
    if (hasBundledFiles) {
      // Write directly to the plugin cache — no GitHub access needed
      const cacheDir = join(
        homedir(),
        '.claude',
        'plugins',
        'cache',
        SKILL_NAME,
        SKILL_NAME,
        bundle.version,
      );
      mkdirSync(cacheDir, { recursive: true });
      writeBundleTo(cacheDir);

      // Update installed_plugins.json
      const registryPath = join(homedir(), '.claude', 'plugins', 'installed_plugins.json');
      const registry = existsSync(registryPath)
        ? JSON.parse(readFileSync(registryPath, 'utf-8'))
        : { version: 2, plugins: {} };
      const key = `${SKILL_NAME}@${SKILL_NAME}`;
      const now = new Date().toISOString();
      registry.plugins[key] = [
        {
          scope: 'user',
          installPath: cacheDir,
          version: bundle.version,
          installedAt: registry.plugins[key]?.[0]?.installedAt ?? now,
          lastUpdated: now,
        },
      ];
      writeFileSync(registryPath, JSON.stringify(registry, null, 2) + '\n');

      // Register marketplace entry
      const marketplacesPath = join(homedir(), '.claude', 'plugins', 'known_marketplaces.json');
      const marketplaces = existsSync(marketplacesPath)
        ? JSON.parse(readFileSync(marketplacesPath, 'utf-8'))
        : {};
      if (!marketplaces[SKILL_NAME]) {
        marketplaces[SKILL_NAME] = {
          source: { source: 'github', repo: REPO },
          installLocation: join(homedir(), '.claude', 'plugins', 'marketplaces', SKILL_NAME),
          lastUpdated: now,
        };
        writeFileSync(marketplacesPath, JSON.stringify(marketplaces, null, 2) + '\n');
      }
      logOk(target, `installed v${bundle.version} (bundled)`);
    } else {
      // Fallback: use claude CLI
      try {
        execSync(`claude plugin marketplace add ${REPO}`, { stdio: 'ignore' });
      } catch {
        /* already added */
      }
      execSync(`claude plugin install ${SKILL_NAME}@${SKILL_NAME}`, { stdio: 'ignore' });
      logOk(target, 'installed');
    }
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

function installCodex(): void {
  const target = 'Codex';
  try {
    const cloneDir = join(homedir(), '.codex', SKILL_NAME);
    ensureSkillDir(cloneDir);

    const skillsDir = join(homedir(), '.agents', 'skills');
    mkdirSync(skillsDir, { recursive: true });

    const linkPath = join(skillsDir, SKILL_NAME);
    const linkTarget = join(cloneDir, 'skills');
    if (!existsSync(linkPath)) {
      symlinkSync(linkTarget, linkPath);
    }
    logOk(target, hasBundledFiles ? `installed v${bundle.version} (bundled)` : 'installed');
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

function installCursor(): void {
  const target = 'Cursor';
  try {
    const destDir = join(homedir(), '.cursor', 'plugins', SKILL_NAME);
    ensureSkillDir(destDir);
    logOk(target, hasBundledFiles ? `installed v${bundle.version} (bundled)` : 'installed');
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

function installGemini(): void {
  const target = 'Gemini CLI';
  try {
    const destDir = join(homedir(), '.gemini', 'extensions', SKILL_NAME);
    ensureSkillDir(destDir);
    logOk(target, hasBundledFiles ? `installed v${bundle.version} (bundled)` : 'installed');
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

function installOpenCode(): void {
  const target = 'OpenCode';
  try {
    const configPath = join(homedir(), '.config', 'opencode', 'opencode.json');
    const pluginEntry = `${SKILL_NAME}@git+${REPO_URL}`;

    if (existsSync(configPath)) {
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      const plugins: string[] = config.plugin ?? [];
      if (!plugins.some((p: string) => p.includes(SKILL_NAME))) {
        plugins.push(pluginEntry);
        config.plugin = plugins;
        writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n');
        logOk(target, 'added to opencode.json');
      } else {
        logSkip(target, 'already configured');
      }
    } else {
      mkdirSync(join(homedir(), '.config', 'opencode'), { recursive: true });
      writeFileSync(configPath, JSON.stringify({ plugin: [pluginEntry] }, null, 2) + '\n');
      logOk(target, 'created opencode.json');
    }
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

const INSTALLERS: Record<Target, () => void> = {
  claude: installClaude,
  codex: installCodex,
  cursor: installCursor,
  gemini: installGemini,
  opencode: installOpenCode,
};

// --- Uninstallers ---

function uninstallClaude(): void {
  const target = 'Claude Code';
  try {
    if (hasBundledFiles) {
      // Remove from plugin cache
      const cacheParent = join(homedir(), '.claude', 'plugins', 'cache', SKILL_NAME);
      if (existsSync(cacheParent)) rmSync(cacheParent, { recursive: true });

      // Remove from installed_plugins.json
      const registryPath = join(homedir(), '.claude', 'plugins', 'installed_plugins.json');
      if (existsSync(registryPath)) {
        const registry = JSON.parse(readFileSync(registryPath, 'utf-8'));
        delete registry.plugins[`${SKILL_NAME}@${SKILL_NAME}`];
        writeFileSync(registryPath, JSON.stringify(registry, null, 2) + '\n');
      }
      logOk(target, 'uninstalled');
    } else {
      execSync(`claude plugin uninstall ${SKILL_NAME}@${SKILL_NAME}`, { stdio: 'ignore' });
      logOk(target, 'uninstalled');
    }
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

function uninstallCodex(): void {
  const target = 'Codex';
  try {
    const linkPath = join(homedir(), '.agents', 'skills', SKILL_NAME);
    if (existsSync(linkPath)) unlinkSync(linkPath);
    const cloneDir = join(homedir(), '.codex', SKILL_NAME);
    if (existsSync(cloneDir)) rmSync(cloneDir, { recursive: true });
    logOk(target, 'uninstalled');
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

function uninstallCursor(): void {
  const target = 'Cursor';
  try {
    const dir = join(homedir(), '.cursor', 'plugins', SKILL_NAME);
    if (existsSync(dir)) rmSync(dir, { recursive: true });
    logOk(target, 'uninstalled');
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

function uninstallGemini(): void {
  const target = 'Gemini CLI';
  try {
    const dir = join(homedir(), '.gemini', 'extensions', SKILL_NAME);
    if (existsSync(dir)) rmSync(dir, { recursive: true });
    logOk(target, 'uninstalled');
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

function uninstallOpenCode(): void {
  const target = 'OpenCode';
  try {
    const configPath = join(homedir(), '.config', 'opencode', 'opencode.json');
    if (existsSync(configPath)) {
      const config = JSON.parse(readFileSync(configPath, 'utf-8'));
      const plugins: string[] = config.plugin ?? [];
      config.plugin = plugins.filter((p: string) => !p.includes(SKILL_NAME));
      writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n');
      logOk(target, 'removed from opencode.json');
    } else {
      logSkip(target, 'not configured');
    }
  } catch (e) {
    logErr(target, e instanceof Error ? e.message : String(e));
  }
}

const UNINSTALLERS: Record<Target, () => void> = {
  claude: uninstallClaude,
  codex: uninstallCodex,
  cursor: uninstallCursor,
  gemini: uninstallGemini,
  opencode: uninstallOpenCode,
};

// --- Public commands ---

export interface InstallFlags {
  target?: string;
}

function resolveTargets(targetFlag: string | undefined, mode: 'install' | 'uninstall'): Target[] {
  if (targetFlag) {
    const t = targetFlag.toLowerCase() as Target;
    if (!ALL_TARGETS.includes(t)) {
      log(`Unknown target: "${targetFlag}". Available: ${ALL_TARGETS.join(', ')}`);
      process.exit(1);
    }
    return [t];
  }
  const detected = detectTargets();
  if (detected.length === 0 && mode === 'install') {
    log('No supported AI coding agents detected.');
    log(`Supported: ${ALL_TARGETS.join(', ')}`);
    log('Use --target <name> to install for a specific agent.');
    process.exit(1);
  }
  return detected;
}

export function cmdInstall(flags: InstallFlags): void {
  const targets = resolveTargets(flags.target, 'install');
  log(`Installing ${SKILL_NAME} for: ${targets.join(', ')}`);
  if (!hasBundledFiles) log('(no embedded bundle — using git clone fallback)');
  log('');
  for (const t of targets) INSTALLERS[t]();
  log('');
  log('Done. Restart your agent to load the skill.');
}

export function cmdUninstall(flags: InstallFlags): void {
  const targets = resolveTargets(flags.target, 'uninstall');
  log(`Uninstalling ${SKILL_NAME} from: ${targets.join(', ')}`);
  log('');
  for (const t of targets) UNINSTALLERS[t]();
  log('');
  log('Done.');
}
