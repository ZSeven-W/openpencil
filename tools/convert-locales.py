#!/usr/bin/env python3
"""Convert openpencil/apps/web/src/i18n/locales/*.ts files into
Rust match-arm tables under crates/openpencil-shell-core/src/i18n/<locale>.rs.

Each TS file exports a single object literal of the form:
    'key.name': 'value',
    'key.with.escape': 'a\\nb',
    'key.with.unicode': '…',

We emit a `pub fn lookup(key: &str) -> Option<&'static str>` that
returns the value for known keys and None otherwise.

Multi-line values aren't used in the TS source so a per-line regex
is sufficient.
"""

import re
import sys
from pathlib import Path

TS_DIR = Path("apps/web/src/i18n/locales")
RS_DIR = Path("crates/openpencil-shell-core/src/i18n")

# Map TS file basename → Rust file basename (sanitised).
LOCALES = {
    "en": "en",
    "zh": "zh_cn",
    "zh-tw": "zh_tw",
    "ja": "ja",
    "ko": "ko",
    "fr": "fr",
    "es": "es",
    "de": "de",
    "pt": "pt",
    "ru": "ru",
    "hi": "hi",
    "tr": "tr",
    "th": "th",
    "vi": "vi",
    "id": "id",
}

KEY_RE = re.compile(r"^\s*'([^']+)':\s*'((?:[^'\\]|\\.)*)',?\s*$")


def parse_ts(path: Path) -> list[tuple[str, str]]:
    rows = []
    for line in path.read_text(encoding="utf-8").splitlines():
        m = KEY_RE.match(line)
        if not m:
            continue
        key, value = m.group(1), m.group(2)
        # TS uses \u escapes + \n / \t — Python read_text already
        # decoded the file as UTF-8, but the JS-style \uXXXX
        # escapes are still literal. Decode them.
        value = (
            value
            .replace("\\\\", "\x00")  # placeholder
            .replace("\\'", "'")
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r")
        )
        # Decode \uXXXX
        def _u(m):
            return chr(int(m.group(1), 16))
        value = re.sub(r"\\u([0-9A-Fa-f]{4})", _u, value)
        value = value.replace("\x00", "\\")
        rows.append((key, value))
    return rows


def rust_escape(s: str) -> str:
    out = []
    for c in s:
        if c == "\\":
            out.append("\\\\")
        elif c == "\"":
            out.append("\\\"")
        elif c == "\n":
            out.append("\\n")
        elif c == "\t":
            out.append("\\t")
        elif c == "\r":
            out.append("\\r")
        elif ord(c) < 0x20:
            out.append(f"\\u{{{ord(c):04x}}}")
        else:
            out.append(c)
    return "".join(out)


def emit_rust(rows: list[tuple[str, str]], rs_module: str, locale_label: str) -> str:
    lines = [
        "//! Generated from `apps/web/src/i18n/locales/{}.ts` —".format(locale_label),
        "//! do NOT hand-edit. Re-run `tools/convert-locales.py`.",
        "",
        "pub fn lookup(key: &str) -> Option<&'static str> {",
        "    Some(match key {",
    ]
    seen: set[str] = set()
    for key, value in rows:
        if key in seen:
            continue
        seen.add(key)
        lines.append(f"        \"{rust_escape(key)}\" => \"{rust_escape(value)}\",")
    lines.append("        _ => return None,")
    lines.append("    })")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    RS_DIR.mkdir(parents=True, exist_ok=True)
    for ts_name, rs_name in LOCALES.items():
        ts_path = TS_DIR / f"{ts_name}.ts"
        if not ts_path.exists():
            print(f"missing: {ts_path}", file=sys.stderr)
            continue
        rows = parse_ts(ts_path)
        rs_path = RS_DIR / f"{rs_name}.rs"
        rs_path.write_text(emit_rust(rows, rs_name, ts_name), encoding="utf-8")
        print(f"{ts_name} → {rs_path} ({len(rows)} keys)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
