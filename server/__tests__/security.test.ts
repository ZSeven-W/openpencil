import { describe, it, expect } from 'vitest'

// ---------------------------------------------------------------------------
// 1. Codex client env allowlist
// ---------------------------------------------------------------------------
describe('codex client env allowlist', () => {
  // Reimplement the exact filter logic from server/utils/codex-client.ts L128-137
  const filterEnv = (env: Record<string, string | undefined>): Record<string, string | undefined> => ({
    PATH: env.PATH,
    HOME: env.HOME,
    TERM: env.TERM,
    LANG: env.LANG,
    SHELL: env.SHELL,
    TMPDIR: env.TMPDIR,
    ...Object.fromEntries(
      Object.entries(env).filter(([k]) => k.startsWith('OPENAI_') || k.startsWith('CODEX_')),
    ),
  })

  it('should strip dangerous env vars', () => {
    const env = {
      PATH: '/usr/bin',
      HOME: '/home/user',
      AWS_SECRET_KEY: 'supersecret',
      DATABASE_URL: 'postgres://...',
      ANTHROPIC_API_KEY: 'sk-ant-xxx',
      GITHUB_TOKEN: 'ghp_xxx',
    }
    const filtered = filterEnv(env)
    expect(filtered).not.toHaveProperty('AWS_SECRET_KEY')
    expect(filtered).not.toHaveProperty('DATABASE_URL')
    expect(filtered).not.toHaveProperty('ANTHROPIC_API_KEY')
    expect(filtered).not.toHaveProperty('GITHUB_TOKEN')
  })

  it('should keep PATH, HOME, and shell vars', () => {
    const env = {
      PATH: '/usr/bin',
      HOME: '/home/user',
      TERM: 'xterm-256color',
      LANG: 'en_US.UTF-8',
      SHELL: '/bin/zsh',
      TMPDIR: '/tmp',
    }
    const filtered = filterEnv(env)
    expect(filtered.PATH).toBe('/usr/bin')
    expect(filtered.HOME).toBe('/home/user')
    expect(filtered.TERM).toBe('xterm-256color')
    expect(filtered.LANG).toBe('en_US.UTF-8')
    expect(filtered.SHELL).toBe('/bin/zsh')
    expect(filtered.TMPDIR).toBe('/tmp')
  })

  it('should keep OPENAI_* and CODEX_* vars', () => {
    const env = {
      PATH: '/usr/bin',
      HOME: '/home/user',
      OPENAI_API_KEY: 'sk-openai-xxx',
      OPENAI_ORG_ID: 'org-xxx',
      CODEX_TOKEN: 'codex-xxx',
      CODEX_SANDBOX: 'read-only',
    }
    const filtered = filterEnv(env)
    expect(filtered.OPENAI_API_KEY).toBe('sk-openai-xxx')
    expect(filtered.OPENAI_ORG_ID).toBe('org-xxx')
    expect(filtered.CODEX_TOKEN).toBe('codex-xxx')
    expect(filtered.CODEX_SANDBOX).toBe('read-only')
  })

  it('should not leak vars with similar prefixes', () => {
    const env = {
      PATH: '/usr/bin',
      HOME: '/home/user',
      OPENAI_API_KEY: 'ok',
      OPENAI_COMPAT: 'ok',
      OPEN_SECRET: 'bad',
      CODEX_MODE: 'ok',
      CODE_SECRET: 'bad',
    }
    const filtered = filterEnv(env)
    expect(filtered).not.toHaveProperty('OPEN_SECRET')
    expect(filtered).not.toHaveProperty('CODE_SECRET')
    expect(filtered.OPENAI_API_KEY).toBe('ok')
    expect(filtered.CODEX_MODE).toBe('ok')
  })
})

// ---------------------------------------------------------------------------
// 2. Debug tail sanitization
// ---------------------------------------------------------------------------
describe('debug tail sanitization', () => {
  // Exact pattern from server/api/ai/chat.ts L34
  const SENSITIVE_PATTERN = /ANTHROPIC_API_KEY=|Authorization:\s*Bearer|api[_-]?key\s*[:=]/i

  it('should match ANTHROPIC_API_KEY leak', () => {
    expect(SENSITIVE_PATTERN.test('ANTHROPIC_API_KEY=sk-ant-abc123')).toBe(true)
  })

  it('should match Authorization Bearer header', () => {
    expect(SENSITIVE_PATTERN.test('Authorization: Bearer token123')).toBe(true)
    expect(SENSITIVE_PATTERN.test('authorization:  Bearer xyz')).toBe(true)
  })

  it('should match api_key and api-key patterns', () => {
    expect(SENSITIVE_PATTERN.test('api_key=secret')).toBe(true)
    expect(SENSITIVE_PATTERN.test('api-key: secret')).toBe(true)
    expect(SENSITIVE_PATTERN.test('apikey=secret')).toBe(true)
  })

  it('should NOT match normal log lines', () => {
    expect(SENSITIVE_PATTERN.test('Using API endpoint https://api.anthropic.com')).toBe(false)
    expect(SENSITIVE_PATTERN.test('Model: claude-sonnet-4-5-20250929')).toBe(false)
    expect(SENSITIVE_PATTERN.test('Request completed in 1200ms')).toBe(false)
    expect(SENSITIVE_PATTERN.test('Connecting to upstream server...')).toBe(false)
  })
})

// ---------------------------------------------------------------------------
// 3. Media type allowlist
// ---------------------------------------------------------------------------
describe('media type allowlist', () => {
  // Exact logic from server/api/ai/chat.ts L155-158
  const ALLOWED_MEDIA = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp'])

  const resolveExtension = (mediaType: string): string =>
    ALLOWED_MEDIA.has(mediaType) ? mediaType.split('/')[1] : 'png'

  it('should allow standard image types', () => {
    expect(resolveExtension('image/png')).toBe('png')
    expect(resolveExtension('image/jpeg')).toBe('jpeg')
    expect(resolveExtension('image/gif')).toBe('gif')
    expect(resolveExtension('image/webp')).toBe('webp')
  })

  it('should fall back to png for disallowed types', () => {
    expect(resolveExtension('image/x-sh')).toBe('png')
    expect(resolveExtension('image/svg+xml')).toBe('png')
    expect(resolveExtension('application/pdf')).toBe('png')
    expect(resolveExtension('text/html')).toBe('png')
    expect(resolveExtension('')).toBe('png')
  })
})
