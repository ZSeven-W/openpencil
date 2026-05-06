// apps/web/src/components/panels/git-panel/git-remote-utils.ts
//
// Small Phase 6a 克隆向导的纯助手。 Stays 低于 ~80 LoC —
// 仅当现有助手不适合时才添加新助手。 The 桌面端
// 拥有规范的 parseHost / shouldUseSys；该文件仅用于渲染器
// 并有意重新派生一个微小的子集（auth-mode inference，默认
// 令牌用户名），因此表单不必只是为了
// 决定显示哪些字段。

/**
 * Auth 克隆向导呈现的
 * 模式。 `token-or-anon` 允许用户粘贴令牌 + 用户名或将两者留空以进行匿名/公共克隆。 `ssh` 需要先前导入的 SSH
 * 密钥（Phase 6c 显示选择器；Phase 6a 仅显示提示）。
 *
 */
export type CloneAuthMode = 'token-or-anon' | 'ssh';

/**
 * URL 方案中的
 * Infer 身份验证模式。 HTTPS / HTTP → 令牌或匿名（服务器可以接受公共存储库的匿名克隆）。 Anything 否则
 * (`git@host:path`, `ssh://`, `git://`, `file://`, ...) → SSH。 Empty
 *
 * / null URL 默认为令牌或匿名，因此未填写的表单以最宽松的模式启动。
 *
 */
export function inferCloneAuthMode(url: string): CloneAuthMode {
  const trimmed = url.trim();
  if (trimmed === '') return 'token-or-anon';
  if (trimmed.startsWith('https://') || trimmed.startsWith('http://')) {
    return 'token-or-anon';
  }
  return 'ssh';
}

/**
 * Parse 来自 git
 * 远程 URL 的主机名。 Mirrors 是 apps/desktop/git/git-engine.ts 中的桌面端
 * parseHost，因此渲染器无需往返即可显示检测到的主机。 https://host/path → 主机
 *
 * ssh://git@host:22/path → 主机
 * git@host:use
 * r/repo.git → 主机（SCP-style SSH）
 *
 * Returns null 对于不可解析的 URLs。
 */
export function parseRemoteHost(url: string): string | null {
  const trimmed = url.trim();
  if (
    trimmed.startsWith('https://') ||
    trimmed.startsWith('http://') ||
    trimmed.startsWith('ssh://')
  ) {
    try {
      return new URL(trimmed).hostname || null;
    } catch {
      return null;
    }
  }
  const m = trimmed.match(/^[^@\s]+@([^:\s]+):/);
  return m ? m[1] : null;
}

/**
 * 当用户粘贴令牌但不提供用
 * 户名时，Default 用户名与令牌身份验证一起发送。当密码槽包含 PAT 时，GitHub 和大多数基于令牌的提供商接受任何非空用户名。
 *
 */
export function defaultTokenUsername(host: string | null): string {
  if (!host) return 'git';
  if (host.endsWith('github.com')) return 'git';
  if (host.endsWith('gitlab.com')) return 'oauth2';
  if (host.endsWith('bitbucket.org')) return 'x-token-auth';
  return 'git';
}

/**
 * Return 特定于提供
 * 商的 SSH-key 设置 URL，因此 Phase 6c SSH 密钥视图可以在生成或导入密钥后提供“在浏览器中打开”深度链接。 The
 * 匹配在 FULL 主机上不区分大小写（无子域遍历），因此 `api.github.com` 或 `gitlab.example.com` 会变为
 * null，并且调用者会呈现通用的“复制公钥”指导。 Returns null 当主机未知、为 null
 * 或与受支持的提供程序的封闭列表不匹配时。
 *
 *
 *
 */
export function getProviderSshSettingsUrl(host: string | null): string | null {
  if (!host) return null;
  const normalized = host.toLowerCase();
  if (normalized === 'github.com') return 'https://github.com/settings/keys';
  if (normalized === 'gitlab.com') return 'https://gitlab.com/-/profile/keys';
  return null;
}

/**
 * 当 URL 是 SSH
 * 样式的远程（git@host:path 或 ssh://...）时，Return true。 Empty / HTTPS /
 * HTTP 返回 false。 Used 通过 Phase 6c 远程设置视图来表面 SSH 传输门控指导。
 */
export function isSshRemoteUrl(url: string | null): boolean {
  if (!url) return false;
  const trimmed = url.trim();
  if (trimmed === '') return false;
  if (trimmed.startsWith('ssh://')) return true;
  // SCP-风格：用户@主机：路径
  return /^[^@\s]+@[^:\s]+:/.test(trimmed);
}
