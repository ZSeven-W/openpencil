/**
 * Fail if the extension's 15 locale catalogs have drifted apart.
 *
 * Why this needs a guard rather than a code review: a missing key in one of
 * fifteen `messages.json` files is invisible until somebody running that
 * language hits the one code path that renders it, and what they see then is
 * the raw key (`errorNoIngress`) in the middle of an otherwise translated
 * popup. A *placeholder* that drifted is worse — `chrome.i18n` and the
 * extension's own `i18n.js` both render an out-of-range `$2` as empty string,
 * so "did not answer within  s" ships looking like a rendering bug rather
 * than a translation bug. Neither failure reaches a test, because there is no
 * runtime that loads all fifteen.
 *
 * So the rule is structural, and English is the shape everything else is
 * measured against (it is the manifest's `default_locale`, and the per-key
 * fallback in `i18n.js`):
 *
 * 1. every catalog is valid JSON in Chrome's `{key: {message, …}}` shape;
 * 2. every catalog has exactly English's key set — no extras, none missing;
 * 3. no message is empty or blank;
 * 4. every message carries English's substitution structure: the same set of
 *    positional `$n` slots, the same named `$placeholder$` set, and the same
 *    positional slots inside each named placeholder's `content`;
 * 5. a key that documents itself with `description` in English does so
 *    everywhere (that field is what a translator reads);
 * 6. `_locales/` and `i18n.js`'s `SUPPORTED_LOCALES` name the same locales, so
 *    a catalog can never ship without a row in the language switcher, nor a
 *    row without a catalog;
 * 7. every `__MSG_*__` the manifest references resolves in all of them —
 *    Chrome renders those against the *browser's* language, which is the one
 *    locale the popup's switcher cannot override.
 */

import { readdirSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const extensionDirectory = resolve(scriptDirectory, '..');
const localesDirectory = resolve(extensionDirectory, '_locales');
const BASE = 'en';

const problems = [];
const fail = (message) => problems.push(message);

/* ------------------------------------------------------------ the catalogs */

const directories = readdirSync(localesDirectory, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name)
  .toSorted();

const catalogs = new Map();
for (const locale of directories) {
  const file = resolve(localesDirectory, locale, 'messages.json');
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(file, 'utf8'));
  } catch (error) {
    fail(`${locale}: messages.json is not valid JSON — ${error.message}`);
    continue;
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    fail(`${locale}: messages.json must be a JSON object of message entries`);
    continue;
  }
  catalogs.set(locale, parsed);
}

if (!catalogs.has(BASE)) {
  console.error(`locales: no ${BASE} catalog — nothing to compare against`);
  process.exit(1);
}

/* ------------------------------------------- the switcher list in i18n.js */

/**
 * Read the codes out of `i18n.js`'s `SUPPORTED_LOCALES` literal.
 *
 * Parsed rather than imported because this script runs under plain Node, and
 * that module reaches for `chrome.*` at call time; a static read of the one
 * array literal is both enough and immune to that.
 */
function switcherLocales() {
  const source = readFileSync(resolve(extensionDirectory, 'i18n.js'), 'utf8');
  const start = source.indexOf('export const SUPPORTED_LOCALES = [');
  if (start === -1) return null;
  const end = source.indexOf('];', start);
  if (end === -1) return null;
  const body = source.slice(start, end);
  return [...body.matchAll(/\bcode:\s*'([^']+)'/g)].map((match) => match[1]);
}

const switcher = switcherLocales();
if (switcher === null) {
  fail('i18n.js: could not find the SUPPORTED_LOCALES array literal');
} else {
  for (const code of switcher) {
    if (!catalogs.has(code)) fail(`i18n.js lists "${code}", but _locales/${code} does not exist`);
  }
  for (const locale of catalogs.keys()) {
    if (!switcher.includes(locale)) {
      fail(`_locales/${locale} exists, but i18n.js's SUPPORTED_LOCALES does not list it`);
    }
  }
}

/* ------------------------------------------------- the substitution shape */

/** The positional slots a string uses, `$$` excluded. `"$1 of $2"` → `1,2`. */
function positionalSlots(text) {
  const found = new Set();
  for (let index = 0; index < text.length; index += 1) {
    if (text[index] !== '$') continue;
    const next = text[index + 1];
    if (next === '$') {
      index += 1;
      continue;
    }
    if (next >= '1' && next <= '9') {
      found.add(next);
      index += 1;
    }
  }
  return [...found].toSorted().join(',');
}

/** The named `$placeholder$` references a message makes, lower-cased. */
function namedSlots(text) {
  const found = new Set();
  for (const match of text.matchAll(/\$([A-Za-z0-9_@]+)\$/g)) found.add(match[1].toLowerCase());
  return [...found].toSorted().join(',');
}

/** A key's full substitution contract: message slots plus placeholder bodies. */
function shapeOf(entry) {
  const message = String(entry.message);
  const placeholders = entry.placeholders || {};
  const bodies = Object.keys(placeholders)
    .map(
      (name) =>
        `${name.toLowerCase()}=${positionalSlots(String(placeholders[name].content ?? ''))}`,
    )
    .toSorted()
    .join(' ');
  return `positional[${positionalSlots(message)}] named[${namedSlots(message)}] placeholders{${bodies}}`;
}

/* --------------------------------------------------------------- the checks */

const base = catalogs.get(BASE);
const baseKeys = Object.keys(base);

for (const [locale, catalog] of catalogs) {
  const keys = Object.keys(catalog);
  for (const key of baseKeys) {
    if (!(key in catalog)) fail(`${locale}: missing key "${key}"`);
  }
  for (const key of keys) {
    if (!(key in base)) fail(`${locale}: key "${key}" is not in ${BASE}`);
  }

  for (const key of keys) {
    if (!(key in base)) continue;
    const entry = catalog[key];
    if (entry === null || typeof entry !== 'object' || Array.isArray(entry)) {
      fail(`${locale}: "${key}" must be an object with a "message" field`);
      continue;
    }
    if (typeof entry.message !== 'string') {
      fail(`${locale}: "${key}" has no string "message"`);
      continue;
    }
    if (entry.message.trim() === '') {
      fail(`${locale}: "${key}" has an empty message`);
      continue;
    }

    const expected = shapeOf(base[key]);
    const actual = shapeOf(entry);
    if (actual !== expected) {
      fail(
        `${locale}: "${key}" substitution drift\n    ${BASE}: ${expected}\n    ${locale}: ${actual}`,
      );
    }

    const documented = typeof base[key].description === 'string';
    const has = typeof entry.description === 'string';
    if (documented && !has)
      fail(`${locale}: "${key}" is missing the "description" ${BASE} carries`);
    if (!documented && has) fail(`${locale}: "${key}" has a "description" ${BASE} does not`);
  }

  // Chrome rejects a manifest description over 132 characters, including a
  // localized replacement that only becomes visible during store packaging.
  const description = catalog.extDescription && catalog.extDescription.message;
  if (typeof description === 'string' && [...description].length > 132) {
    fail(`${locale}: "extDescription" exceeds Chrome's 132-character manifest limit`);
  }
}

/* --------------------------------------------------- the manifest's __MSG_ */

const manifest = readFileSync(resolve(extensionDirectory, 'manifest.json'), 'utf8');
const referenced = [
  ...new Set([...manifest.matchAll(/__MSG_([A-Za-z0-9_@]+)__/g)].map((m) => m[1])),
];
for (const key of referenced) {
  for (const [locale, catalog] of catalogs) {
    if (!(key in catalog)) fail(`${locale}: manifest references __MSG_${key}__, which is missing`);
  }
}

const defaultLocale = JSON.parse(manifest).default_locale;
if (defaultLocale !== BASE) {
  fail(`manifest.json: default_locale is "${defaultLocale}", expected "${BASE}"`);
}

/* ------------------------------------------------------------------ verdict */

if (problems.length > 0) {
  for (const problem of problems) console.error(`locales: ${problem}`);
  console.error(`locales: ${problems.length} problem(s) across ${catalogs.size} catalog(s).`);
  process.exit(1);
}

console.log(
  `locales: ok (${catalogs.size} locales × ${baseKeys.length} keys, ` +
    `${referenced.length} manifest __MSG_ key(s) resolved in each)`,
);
console.log(`locales: ${directories.join(' ')}`);
