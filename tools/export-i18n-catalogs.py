#!/usr/bin/env python3
"""Validate Rust i18n tables and export the web-only runtime catalogs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]
I18N_DIR = REPO_ROOT / "crates" / "op-i18n" / "src" / "i18n"
LOCALE_FILE = I18N_DIR.parent / "locale.rs"
EXPECTED_KEY_COUNT = 1697
TABLE_SUFFIXES = ("", "_git", "_panel", "_collab")

# Module stem, Locale variant, and the exact value returned by Locale::code().
# validate_locale_codes() rejects drift between this export contract and Rust.
LOCALE_SPECS = (
    ("en", "EnUs", "en-US"),
    ("zh_cn", "ZhCn", "zh-CN"),
    ("zh_tw", "ZhTw", "zh-TW"),
    ("ja", "Ja", "ja"),
    ("ko", "Ko", "ko"),
    ("fr", "Fr", "fr"),
    ("es", "Es", "es"),
    ("de", "De", "de"),
    ("pt", "Pt", "pt"),
    ("ru", "Ru", "ru"),
    ("hi", "Hi", "hi"),
    ("tr", "Tr", "tr"),
    ("th", "Th", "th"),
    ("vi", "Vi", "vi"),
    ("id", "Id", "id"),
)
EMBEDDED_CODES = frozenset(("en-US", "zh-CN"))
LAZY_LOCALES = tuple(
    (stem, code) for stem, _variant, code in LOCALE_SPECS if code not in EMBEDDED_CODES
)

CONST_DECLARATION = re.compile(
    r"(?m)^[ \t]*const[ \t]+([A-Z][A-Z0-9_]*)[ \t]*:[ \t]*&str[ \t]*=[ \t]*"
)
IDENTIFIER = re.compile(r"[A-Z][A-Z0-9_]*")
LOCALE_CODE_ARM = re.compile(r'Locale::([A-Za-z][A-Za-z0-9]*)\s*=>\s*"([^"]+)"')


class CatalogError(ValueError):
    """A locale source or generated catalog violates the export contract."""


def _location(source_name: str, source: str, offset: int) -> str:
    return f"{source_name}:{source.count(chr(10), 0, offset) + 1}"


def _skip_horizontal(source: str, offset: int) -> int:
    while offset < len(source) and source[offset] in " \t":
        offset += 1
    return offset


def _skip_whitespace(source: str, offset: int) -> int:
    while offset < len(source) and source[offset].isspace():
        offset += 1
    return offset


def _parse_string_literal(
    source: str, offset: int, source_name: str
) -> tuple[str, int]:
    if offset >= len(source) or source[offset] != '"':
        raise CatalogError(
            f"{_location(source_name, source, offset)}: expected a string literal"
        )

    end = offset + 1
    while end < len(source):
        char = source[end]
        if char == "\\":
            end += 2
            continue
        if char == '"':
            token = source[offset : end + 1]
            try:
                # Current tables use JSON-compatible Rust escapes (\n and \"),
                # so this decodes the literal without evaluating Rust code.
                value = json.loads(token)
            except json.JSONDecodeError as error:
                raise CatalogError(
                    f"{_location(source_name, source, offset)}: unsupported Rust "
                    f"string literal: {error.msg}"
                ) from error
            if not isinstance(value, str):
                raise CatalogError(
                    f"{_location(source_name, source, offset)}: value is not a string"
                )
            return value, end + 1
        end += 1

    raise CatalogError(
        f"{_location(source_name, source, offset)}: unterminated string literal"
    )


def _finish_arm(source: str, offset: int, source_name: str) -> None:
    offset = _skip_horizontal(source, offset)
    if offset < len(source) and source[offset] == ",":
        offset = _skip_horizontal(source, offset + 1)
    if offset < len(source) and source[offset] == "\r":
        offset += 1
    if offset < len(source) and source[offset] != "\n":
        excerpt = source[offset : source.find("\n", offset)].strip()
        raise CatalogError(
            f"{_location(source_name, source, offset)}: unsupported tokens after RHS: "
            f"{excerpt!r}"
        )


def _parse_constants(source: str, source_name: str) -> dict[str, str]:
    constants: dict[str, str] = {}
    for declaration in CONST_DECLARATION.finditer(source):
        name = declaration.group(1)
        offset = _skip_whitespace(source, declaration.end())
        value, offset = _parse_string_literal(source, offset, source_name)
        offset = _skip_horizontal(source, offset)
        if offset >= len(source) or source[offset] != ";":
            raise CatalogError(
                f"{_location(source_name, source, offset)}: const {name} must end in ';'"
            )
        _finish_arm(source, offset + 1, source_name)
        if name in constants:
            raise CatalogError(f"{source_name}: duplicate const {name}")
        constants[name] = value
    return constants


def _source_arms(source: str, source_name: str) -> Iterable[tuple[str, int]]:
    """Yield key and RHS offset using the integrity gate's line rules."""
    source_offset = 0
    for line_number, raw_line in enumerate(source.splitlines(keepends=True), start=1):
        leading = len(raw_line) - len(raw_line.lstrip())
        line = raw_line.lstrip()
        if "=>" not in line:
            source_offset += len(raw_line)
            continue

        pattern, rhs = line.split("=>", 1)
        pattern = pattern.strip()
        if pattern == "_":
            source_offset += len(raw_line)
            continue
        if not pattern.startswith('"'):
            raise CatalogError(
                f"{source_name}:{line_number}: non-string match pattern {pattern!r}"
            )
        closing_quote = pattern.find('"', 1)
        if closing_quote < 0:
            raise CatalogError(
                f"{source_name}:{line_number}: unterminated match key {pattern!r}"
            )
        key = pattern[1:closing_quote]
        if pattern[closing_quote + 1 :].strip():
            raise CatalogError(
                f"{source_name}:{line_number}: unsupported match pattern {pattern!r}; "
                "use exactly one quoted key per match arm"
            )
        if not rhs.strip():
            raise CatalogError(f"{source_name}:{line_number}: empty match-arm RHS")

        arrow_offset = line.find("=>")
        yield key, source_offset + leading + arrow_offset + 2
        source_offset += len(raw_line)


def parse_locale_source(source: str, source_name: str) -> dict[str, str]:
    """Parse the literal-only subset used by one canonical locale shard."""
    constants = _parse_constants(source, source_name)
    catalog: dict[str, str] = {}

    for key, rhs_offset in _source_arms(source, source_name):
        offset = _skip_horizontal(source, rhs_offset)
        if offset >= len(source):
            raise CatalogError(f"{source_name}: missing RHS for {key!r}")

        if source[offset] == '"':
            value, offset = _parse_string_literal(source, offset, source_name)
            _finish_arm(source, offset, source_name)
        elif source[offset] == "{":
            value_offset = _skip_whitespace(source, offset + 1)
            value, offset = _parse_string_literal(source, value_offset, source_name)
            offset = _skip_whitespace(source, offset)
            if offset >= len(source) or source[offset] != "}":
                raise CatalogError(
                    f"{_location(source_name, source, offset)}: multiline RHS must "
                    "contain exactly one string literal"
                )
            _finish_arm(source, offset + 1, source_name)
        else:
            identifier = IDENTIFIER.match(source, offset)
            if identifier is None:
                excerpt = source[offset : source.find("\n", offset)].strip()
                raise CatalogError(
                    f"{_location(source_name, source, offset)}: unsupported RHS "
                    f"{excerpt!r}"
                )
            name = identifier.group(0)
            if name not in constants:
                raise CatalogError(
                    f"{_location(source_name, source, offset)}: const {name} is not "
                    "defined in this file"
                )
            value = constants[name]
            _finish_arm(source, identifier.end(), source_name)

        if key in catalog:
            raise CatalogError(f"{source_name}: duplicate key {key!r}")
        catalog[key] = value

    return catalog


def validate_locale_codes(locale_file: Path = LOCALE_FILE) -> None:
    source = locale_file.read_text(encoding="utf-8")
    code_start = source.find("pub const fn code")
    code_end = source.find("/// Parse a BCP-47", code_start)
    if code_start < 0 or code_end < 0:
        raise CatalogError(f"{locale_file}: cannot locate Locale::code()")
    actual = dict(LOCALE_CODE_ARM.findall(source[code_start:code_end]))
    expected = {variant: code for _stem, variant, code in LOCALE_SPECS}
    if actual != expected:
        raise CatalogError(
            f"{locale_file}: Locale::code() differs from exporter mapping; "
            f"expected={expected!r}, actual={actual!r}"
        )


def load_catalogs(i18n_dir: Path = I18N_DIR) -> dict[str, dict[str, str]]:
    validate_locale_codes(i18n_dir.parent / "locale.rs")
    catalogs: dict[str, dict[str, str]] = {}
    for stem, _variant, _code in LOCALE_SPECS:
        catalog: dict[str, str] = {}
        for suffix in TABLE_SUFFIXES:
            source_path = i18n_dir / f"{stem}{suffix}.rs"
            source = source_path.read_text(encoding="utf-8")
            shard = parse_locale_source(source, str(source_path))
            duplicates = catalog.keys() & shard.keys()
            if duplicates:
                raise CatalogError(
                    f"locale {stem!r} contains duplicate keys across shards: "
                    f"{sorted(duplicates)!r}"
                )
            catalog.update(shard)
        catalogs[stem] = catalog

    english_keys = set(catalogs["en"])
    if len(english_keys) != EXPECTED_KEY_COUNT:
        raise CatalogError(
            f"English catalog has {len(english_keys)} keys; expected "
            f"{EXPECTED_KEY_COUNT}"
        )
    for stem, catalog in catalogs.items():
        actual = set(catalog)
        if actual != english_keys:
            missing = sorted(english_keys - actual)
            extra = sorted(actual - english_keys)
            raise CatalogError(
                f"locale {stem!r} differs from English; missing={missing!r}, "
                f"extra={extra!r}"
            )
    return catalogs


def export_catalogs(output_dir: Path) -> None:
    catalogs = load_catalogs()
    expected_files = {f"{code}.json" for _stem, code in LAZY_LOCALES}
    output_dir.mkdir(parents=True, exist_ok=True)
    unexpected = {path.name for path in output_dir.iterdir()} - expected_files
    if unexpected:
        raise CatalogError(
            f"{output_dir}: refusing to mix generated catalogs with unexpected "
            f"entries {sorted(unexpected)!r}"
        )

    for stem, code in LAZY_LOCALES:
        output_path = output_dir / f"{code}.json"
        payload = json.dumps(
            catalogs[stem], ensure_ascii=False, separators=(",", ":"), sort_keys=True
        )
        temporary_path = output_dir / f".{code}.json.tmp"
        temporary_path.write_text(f"{payload}\n", encoding="utf-8", newline="\n")
        temporary_path.replace(output_path)

    actual_files = {path.name for path in output_dir.iterdir()}
    if actual_files != expected_files:
        raise CatalogError(
            f"{output_dir}: expected exactly {sorted(expected_files)!r}, "
            f"found {sorted(actual_files)!r}"
        )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--check",
        action="store_true",
        help="validate all 15 canonical tables without writing files",
    )
    mode.add_argument(
        "--output-dir",
        type=Path,
        metavar="DIR",
        help="write the 13 non-embedded runtime catalogs into DIR",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        if args.check:
            load_catalogs()
            print(
                f"PASS: 15 locales have exactly {EXPECTED_KEY_COUNT} keys; "
                "no files written"
            )
        else:
            export_catalogs(args.output_dir)
            print(
                f"PASS: wrote {len(LAZY_LOCALES)} lazy locale catalogs to "
                f"{args.output_dir}"
            )
    except (CatalogError, OSError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
