// apps/web/src/components/panels/git-panel/__tests__/git-remote-utils.test.ts
import { describe, it, expect } from 'vitest';
import { defaultTokenUsername, inferCloneAuthMode, parseRemoteHost } from '../git-remote-utils';

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
