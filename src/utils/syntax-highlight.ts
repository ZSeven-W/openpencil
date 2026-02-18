/**
 * Lightweight syntax highlighter for JSX, HTML, and CSS.
 * Produces HTML strings with color spans.
 */

interface TokenRule {
  pattern: RegExp
  className: string
}

const JSX_RULES: TokenRule[] = [
  // Multi-line comments
  { pattern: /\/\*[\s\S]*?\*\//g, className: 'syn-comment' },
  // Single-line comments
  { pattern: /\/\/.*/g, className: 'syn-comment' },
  // JSX self-closing tags: <Tag ... />
  { pattern: /<\/?[A-Za-z][A-Za-z0-9.]*/g, className: 'syn-tag' },
  // Closing bracket
  { pattern: /\/?>/g, className: 'syn-tag' },
  // Strings (double and single quoted)
  { pattern: /"(?:[^"\\]|\\.)*"/g, className: 'syn-string' },
  { pattern: /'(?:[^'\\]|\\.)*'/g, className: 'syn-string' },
  // Template literals
  { pattern: /`(?:[^`\\]|\\.)*`/g, className: 'syn-string' },
  // Keywords
  { pattern: /\b(export|function|return|const|let|var|import|from|default|if|else|for|while|switch|case|break|continue|new|this|class|extends|typeof|instanceof|void|null|undefined|true|false)\b/g, className: 'syn-keyword' },
  // Attribute names (word followed by =)
  { pattern: /\b[a-zA-Z-]+(?==)/g, className: 'syn-attr' },
  // Numbers
  { pattern: /\b\d+\.?\d*\b/g, className: 'syn-number' },
  // Curly braces in JSX
  { pattern: /[{}]/g, className: 'syn-bracket' },
]

const HTML_RULES: TokenRule[] = [
  // Comments
  { pattern: /<!--[\s\S]*?-->/g, className: 'syn-comment' },
  // Tags
  { pattern: /<\/?[a-zA-Z][a-zA-Z0-9-]*/g, className: 'syn-tag' },
  { pattern: /\/?>/g, className: 'syn-tag' },
  // Attribute values
  { pattern: /"(?:[^"\\]|\\.)*"/g, className: 'syn-string' },
  { pattern: /'(?:[^'\\]|\\.)*'/g, className: 'syn-string' },
  // Attribute names
  { pattern: /\b[a-zA-Z-]+(?==)/g, className: 'syn-attr' },
]

const CSS_RULES: TokenRule[] = [
  // Comments
  { pattern: /\/\*[\s\S]*?\*\//g, className: 'syn-comment' },
  // Selectors (class/id/element)
  { pattern: /[.#]?[a-zA-Z_-][a-zA-Z0-9_-]*(?=\s*\{)/g, className: 'syn-tag' },
  // Property names
  { pattern: /[a-zA-Z-]+(?=\s*:)/g, className: 'syn-attr' },
  // Strings
  { pattern: /"(?:[^"\\]|\\.)*"/g, className: 'syn-string' },
  { pattern: /'(?:[^'\\]|\\.)*'/g, className: 'syn-string' },
  // Numbers with units
  { pattern: /\b\d+\.?\d*(px|em|rem|%|deg|vh|vw|s|ms)?\b/g, className: 'syn-number' },
  // Colors
  { pattern: /#[0-9a-fA-F]{3,8}\b/g, className: 'syn-string' },
  // Braces
  { pattern: /[{}]/g, className: 'syn-bracket' },
  // Keywords
  { pattern: /\b(important|inherit|initial|unset|none|auto|solid|dashed|flex|grid|block|inline|absolute|relative|fixed|sticky)\b/g, className: 'syn-keyword' },
]

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

interface Token {
  start: number
  end: number
  className: string
}

function tokenize(code: string, rules: TokenRule[]): Token[] {
  const tokens: Token[] = []
  for (const rule of rules) {
    const regex = new RegExp(rule.pattern.source, rule.pattern.flags)
    let match: RegExpExecArray | null
    while ((match = regex.exec(code)) !== null) {
      tokens.push({
        start: match.index,
        end: match.index + match[0].length,
        className: rule.className,
      })
    }
  }
  // Sort by start position; earlier rules win ties (priority order)
  tokens.sort((a, b) => a.start - b.start)

  // Remove overlapping tokens (first match wins)
  const filtered: Token[] = []
  let lastEnd = 0
  for (const token of tokens) {
    if (token.start >= lastEnd) {
      filtered.push(token)
      lastEnd = token.end
    }
  }
  return filtered
}

function renderTokens(code: string, tokens: Token[]): string {
  let result = ''
  let pos = 0
  for (const token of tokens) {
    if (token.start > pos) {
      result += escapeHtml(code.slice(pos, token.start))
    }
    result += `<span class="${token.className}">${escapeHtml(code.slice(token.start, token.end))}</span>`
    pos = token.end
  }
  if (pos < code.length) {
    result += escapeHtml(code.slice(pos))
  }
  return result
}

export type SyntaxLanguage = 'jsx' | 'html' | 'css'

export function highlightCode(code: string, language: SyntaxLanguage): string {
  const rules = language === 'jsx' ? JSX_RULES : language === 'html' ? HTML_RULES : CSS_RULES
  const tokens = tokenize(code, rules)
  return renderTokens(code, tokens)
}
