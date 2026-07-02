// apps/web/src/components/panels/git-panel/git-remote-utils.ts
//
// Small pure helpers for the Phase 6a clone wizard. Stays under ~80 LoC —
// add new helpers only when an existing one wouldn't fit. The desktop side
// owns the canonical parseHost / shouldUseSys; this file is renderer-only
// and intentionally re-derives a tiny subset (auth-mode inference, default
// token usernames) so the form doesn't have to take a round-trip just to
// decide which fields to show.

/**
 * Auth modes the clone wizard renders. `token-or-anon` allows the user to
 * either paste a token + username or leave both blank for an anonymous /
 * public clone. `ssh` requires a previously-imported SSH key (Phase 6c
 * surfaces the picker; Phase 6a only displays a hint).
 */
export type CloneAuthMode = 'token-or-anon' | 'ssh';

/**
 * Infer auth mode from the URL scheme. HTTPS / HTTP → token-or-anon (the
 * server may accept anonymous clones for public repos). Anything else
 * (`git@host:path`, `ssh://`, `git://`, `file://`, ...) → SSH.
 *
 * Empty / null URL defaults to token-or-anon so an unfilled form starts in
 * the most permissive mode.
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
 * Parse the hostname from a git remote URL. Mirrors the desktop-side
 * parseHost in apps/desktop/git/git-engine.ts so the renderer can show the
 * detected host without a round-trip.
 *
 *   https://host/path           → host
 *   ssh://git@host:22/path      → host
 *   git@host:user/repo.git      → host (SCP-style SSH)
 *
 * Returns null for unparseable URLs.
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
 * Default username to send with token auth when the user pastes a token
 * but doesn't supply a username. GitHub and most token-based providers
 * accept any non-empty username when the password slot holds a PAT.
 */
export function defaultTokenUsername(host: string | null): string {
  if (!host) return 'git';
  if (host.endsWith('github.com')) return 'git';
  if (host.endsWith('gitlab.com')) return 'oauth2';
  if (host.endsWith('bitbucket.org')) return 'x-token-auth';
  return 'git';
}

/**
 * Return a provider-specific SSH-key settings URL so the Phase 6c SSH keys
 * view can offer a "open in browser" deeplink after generating or importing
 * a key. The match is case-insensitive on the FULL host (no subdomain
 * traversal) so `api.github.com` or `gitlab.example.com` fall through to
 * null and the caller renders generic "copy the public key" guidance.
 *
 * Returns null when the host is unknown, null, or doesn't match the
 * closed list of supported providers.
 */
export function getProviderSshSettingsUrl(host: string | null): string | null {
  if (!host) return null;
  const normalized = host.toLowerCase();
  if (normalized === 'github.com') return 'https://github.com/settings/keys';
  if (normalized === 'gitlab.com') return 'https://gitlab.com/-/profile/keys';
  return null;
}

/**
 * Return true when the URL is an SSH-style remote (git@host:path or
 * ssh://...). Empty / HTTPS / HTTP returns false. Used by the Phase 6c
 * remote settings view to surface SSH transport gating guidance.
 */
export function isSshRemoteUrl(url: string | null): boolean {
  if (!url) return false;
  const trimmed = url.trim();
  if (trimmed === '') return false;
  if (trimmed.startsWith('ssh://')) return true;
  // SCP-style: user@host:path
  return /^[^@\s]+@[^:\s]+:/.test(trimmed);
}
