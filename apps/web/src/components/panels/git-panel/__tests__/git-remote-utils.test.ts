// apps/web/src/components/panels/git-panel/__tests__/git-remote-utils.test.ts
import { describe, it, expect } from 'vitest';
import {
  defaultTokenUsername,
  getProviderSshSettingsUrl,
  inferCloneAuthMode,
  isSshRemoteUrl,
  parseRemoteHost,
} from '../git-remote-utils';

describe('inferCloneAuthMode', () => {
  it('returns token-or-anon for empty input', () => {
    expect(inferCloneAuthMode('')).toBe('token-or-anon');
    expect(inferCloneAuthMode('   ')).toBe('token-or-anon');
  });

  it('returns token-or-anon for HTTPS URLs', () => {
    expect(inferCloneAuthMode('https://github.com/foo/bar.git')).toBe('token-or-anon');
    expect(inferCloneAuthMode('http://gitea.local/foo/bar.git')).toBe('token-or-anon');
  });

  it('returns ssh for git@ SCP-style URLs', () => {
    expect(inferCloneAuthMode('git@github.com:foo/bar.git')).toBe('ssh');
  });

  it('returns ssh for ssh:// URLs', () => {
    expect(inferCloneAuthMode('ssh://git@github.com/foo/bar.git')).toBe('ssh');
  });

  it('returns ssh for any non-http(s) scheme as a safe default', () => {
    expect(inferCloneAuthMode('git://github.com/foo/bar.git')).toBe('ssh');
    expect(inferCloneAuthMode('file:///tmp/repo.git')).toBe('ssh');
  });
});

describe('parseRemoteHost', () => {
  it('parses HTTPS URLs', () => {
    expect(parseRemoteHost('https://github.com/foo/bar.git')).toBe('github.com');
    expect(parseRemoteHost('http://gitea.local:3000/foo/bar.git')).toBe('gitea.local');
  });

  it('parses ssh:// URLs', () => {
    expect(parseRemoteHost('ssh://git@github.com:22/foo/bar.git')).toBe('github.com');
  });

  it('parses SCP-style git@host:path URLs', () => {
    expect(parseRemoteHost('git@github.com:foo/bar.git')).toBe('github.com');
    expect(parseRemoteHost('user@example.com:foo/bar')).toBe('example.com');
  });

  it('returns null for unparseable input', () => {
    expect(parseRemoteHost('')).toBeNull();
    expect(parseRemoteHost('not a url')).toBeNull();
    expect(parseRemoteHost('/local/path/repo.git')).toBeNull();
  });
});

describe('defaultTokenUsername', () => {
  it('returns provider-specific defaults', () => {
    expect(defaultTokenUsername('github.com')).toBe('git');
    expect(defaultTokenUsername('gitlab.com')).toBe('oauth2');
    expect(defaultTokenUsername('bitbucket.org')).toBe('x-token-auth');
  });

  it('returns "git" for unknown hosts and null', () => {
    expect(defaultTokenUsername(null)).toBe('git');
    expect(defaultTokenUsername('git.example.com')).toBe('git');
  });

  it('matches subdomains via endsWith', () => {
    expect(defaultTokenUsername('api.github.com')).toBe('git');
    expect(defaultTokenUsername('gitlab.example.gitlab.com')).toBe('oauth2');
  });
});

describe('getProviderSshSettingsUrl', () => {
  it('returns a github settings URL for github.com (exact match)', () => {
    expect(getProviderSshSettingsUrl('github.com')).toBe('https://github.com/settings/keys');
  });

  it('returns a gitlab settings URL for gitlab.com (exact match)', () => {
    expect(getProviderSshSettingsUrl('gitlab.com')).toBe('https://gitlab.com/-/profile/keys');
  });

  it('is case-insensitive', () => {
    expect(getProviderSshSettingsUrl('GitHub.COM')).toBe('https://github.com/settings/keys');
  });

  it('returns null for subdomains (exact match only)', () => {
    expect(getProviderSshSettingsUrl('api.github.com')).toBeNull();
    expect(getProviderSshSettingsUrl('gitlab.example.com')).toBeNull();
  });

  it('returns null for unknown hosts and null', () => {
    expect(getProviderSshSettingsUrl(null)).toBeNull();
    expect(getProviderSshSettingsUrl('')).toBeNull();
    expect(getProviderSshSettingsUrl('bitbucket.org')).toBeNull();
  });
});

describe('isSshRemoteUrl', () => {
  it('returns true for ssh:// URLs', () => {
    expect(isSshRemoteUrl('ssh://git@github.com/foo/bar.git')).toBe(true);
  });

  it('returns true for SCP-style git@host:path URLs', () => {
    expect(isSshRemoteUrl('git@github.com:foo/bar.git')).toBe(true);
  });

  it('returns false for HTTPS / HTTP URLs', () => {
    expect(isSshRemoteUrl('https://github.com/foo/bar.git')).toBe(false);
    expect(isSshRemoteUrl('http://gitea.local/foo/bar.git')).toBe(false);
  });

  it('returns false for empty / null', () => {
    expect(isSshRemoteUrl(null)).toBe(false);
    expect(isSshRemoteUrl('')).toBe(false);
    expect(isSshRemoteUrl('   ')).toBe(false);
  });
});
