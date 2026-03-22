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

const SWIFT_RULES: TokenRule[] = [
  { pattern: /\/\/.*/g, className: 'syn-comment' },
  { pattern: /\/\*[\s\S]*?\*\//g, className: 'syn-comment' },
  { pattern: /"(?:[^"\\]|\\.)*"/g, className: 'syn-string' },
  { pattern: /\b(import|struct|var|let|func|return|if|else|for|in|while|switch|case|break|some|self|true|false|nil|class|enum|protocol|extension|guard|throw|try|catch|async|await|private|public|internal|static|mutating|typealias|associatedtype|where)\b/g, className: 'syn-keyword' },
  { pattern: /\b(View|Text|VStack|HStack|ZStack|Color|Font|Image|Rectangle|RoundedRectangle|Circle|Ellipse|Capsule|Path|Spacer|Divider|GeometryReader|ScrollView|Button|AsyncImage|LinearGradient|RadialGradient|Shape|some)\b/g, className: 'syn-tag' },
  { pattern: /\.[a-zA-Z]+(?=\()/g, className: 'syn-attr' },
  { pattern: /\b\d+\.?\d*\b/g, className: 'syn-number' },
  { pattern: /[{}()]/g, className: 'syn-bracket' },
]

const KOTLIN_RULES: TokenRule[] = [
  { pattern: /\/\/.*/g, className: 'syn-comment' },
  { pattern: /\/\*[\s\S]*?\*\//g, className: 'syn-comment' },
  { pattern: /"(?:[^"\\]|\\.)*"/g, className: 'syn-string' },
  { pattern: /\b(package|import|fun|val|var|return|if|else|for|in|while|when|is|class|object|interface|override|private|public|internal|companion|data|sealed|abstract|open|suspend|inline|annotation|true|false|null)\b/g, className: 'syn-keyword' },
  { pattern: /\b(Composable|Modifier|Column|Row|Box|Text|Image|Icon|Canvas|Divider|Spacer|Surface|Card|Scaffold|LazyColumn|LazyRow|Color|FontWeight|FontStyle|TextAlign|TextDecoration|Arrangement|Alignment|RoundedCornerShape|CircleShape|Dp|ContentScale)\b/g, className: 'syn-tag' },
  { pattern: /\.[a-zA-Z]+(?=\()/g, className: 'syn-attr' },
  { pattern: /\b\d+\.?\d*(f|dp|sp)?\b/g, className: 'syn-number' },
  { pattern: /@[A-Za-z]+/g, className: 'syn-attr' },
  { pattern: /[{}()]/g, className: 'syn-bracket' },
]

const DART_RULES: TokenRule[] = [
  { pattern: /\/\/.*/g, className: 'syn-comment' },
  { pattern: /\/\*[\s\S]*?\*\//g, className: 'syn-comment' },
  { pattern: /"(?:[^"\\]|\\.)*"/g, className: 'syn-string' },
  { pattern: /'(?:[^'\\]|\\.)*'/g, className: 'syn-string' },
  { pattern: /\b(import|class|extends|implements|with|mixin|abstract|final|const|var|void|return|if|else|for|in|while|switch|case|break|continue|new|this|super|true|false|null|async|await|static|late|required|override|enum|typedef|try|catch|throw|dynamic)\b/g, className: 'syn-keyword' },
  { pattern: /\b(Widget|StatelessWidget|StatefulWidget|State|BuildContext|Container|Column|Row|Stack|Positioned|Text|Image|SizedBox|Padding|Center|Expanded|Flexible|Scaffold|AppBar|Material|BoxDecoration|BorderRadius|Border|EdgeInsets|Color|TextStyle|FontWeight|FontStyle|TextAlign|BoxFit|MainAxisAlignment|CrossAxisAlignment|CustomPaint|ClipPath|Opacity|Transform|LinearGradient|RadialGradient)\b/g, className: 'syn-tag' },
  { pattern: /\.[a-zA-Z]+(?=\()/g, className: 'syn-attr' },
  { pattern: /@[a-zA-Z]+/g, className: 'syn-attr' },
  { pattern: /\b\d+\.?\d*\b/g, className: 'syn-number' },
  { pattern: /[{}()]/g, className: 'syn-bracket' },
]

export type SyntaxLanguage = 'jsx' | 'html' | 'css' | 'swift' | 'kotlin' | 'dart'

export function highlightCode(code: string, language: SyntaxLanguage): string {
  const ruleMap: Record<SyntaxLanguage, TokenRule[]> = {
    jsx: JSX_RULES,
    html: HTML_RULES,
    css: CSS_RULES,
    swift: SWIFT_RULES,
    kotlin: KOTLIN_RULES,
    dart: DART_RULES,
  }
  const rules = ruleMap[language]
  const tokens = tokenize(code, rules)
  return renderTokens(code, tokens)
}
