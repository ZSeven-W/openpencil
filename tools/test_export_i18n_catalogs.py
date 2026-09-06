#!/usr/bin/env python3
"""Tests for the Rust-locale-to-JSON exporter."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


TOOLS_DIR = Path(__file__).resolve().parent
EXPORTER_PATH = TOOLS_DIR / "export-i18n-catalogs.py"


def load_exporter():
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location("export_i18n_catalogs", EXPORTER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load exporter from {EXPORTER_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class ExportI18nCatalogsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.exporter = load_exporter()

    def test_parser_supports_every_current_rhs_form(self) -> None:
        source = r'''
const MULTILINE: &str =
    "first line\nquoted \"value\"";
const INLINE: &str = "constant value";

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "inline" => "plain value",
        "block" => {
            "block value"
        }
        "multilineConst" => MULTILINE,
        "inlineConst" => INLINE,
        _ => return None,
    })
}
'''
        self.assertEqual(
            self.exporter.parse_locale_source(source, "fixture.rs"),
            {
                "inline": "plain value",
                "block": "block value",
                "multilineConst": 'first line\nquoted "value"',
                "inlineConst": "constant value",
            },
        )

    def test_parser_rejects_unsupported_rhs(self) -> None:
        source = '''
pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "unsupported" => concat!("a", "b"),
        _ => return None,
    })
}
'''
        with self.assertRaisesRegex(self.exporter.CatalogError, "unsupported RHS"):
            self.exporter.parse_locale_source(source, "fixture.rs")

    def test_parser_rejects_duplicate_keys(self) -> None:
        source = '''
pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "duplicate" => "first",
        "duplicate" => "second",
        _ => return None,
    })
}
'''
        with self.assertRaisesRegex(self.exporter.CatalogError, "duplicate key"):
            self.exporter.parse_locale_source(source, "fixture.rs")

    def test_parser_rejects_non_file_local_constant(self) -> None:
        source = '''
pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "missing" => DEFINED_IN_ANOTHER_FILE,
        _ => return None,
    })
}
'''
        with self.assertRaisesRegex(self.exporter.CatalogError, "not defined in this file"):
            self.exporter.parse_locale_source(source, "fixture.rs")

    def test_check_mode_does_not_write(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = subprocess.run(
                [sys.executable, str(EXPORTER_PATH), "--check"],
                cwd=root,
                capture_output=True,
                check=False,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(list(root.iterdir()), [])
            self.assertIn("15 locales", result.stdout)
            self.assertIn("1697 keys", result.stdout)

    def test_export_writes_exact_lazy_locale_set_deterministically(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output_dir = Path(temporary) / "i18n"
            self.exporter.export_catalogs(output_dir)
            expected = {f"{code}.json" for _, code in self.exporter.LAZY_LOCALES}
            self.assertEqual({path.name for path in output_dir.iterdir()}, expected)

            first_pass = {path.name: path.read_bytes() for path in output_dir.iterdir()}
            self.exporter.export_catalogs(output_dir)
            second_pass = {path.name: path.read_bytes() for path in output_dir.iterdir()}
            self.assertEqual(first_pass, second_pass)

            for filename, payload in first_pass.items():
                self.assertTrue(payload.endswith(b"\n"), filename)
                catalog = json.loads(payload)
                self.assertEqual(len(catalog), self.exporter.EXPECTED_KEY_COUNT)
                expected_payload = (
                    json.dumps(
                        catalog,
                        ensure_ascii=False,
                        separators=(",", ":"),
                        sort_keys=True,
                    ).encode()
                    + b"\n"
                )
                self.assertEqual(payload, expected_payload, filename)

    def test_export_rejects_unexpected_output_entries(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output_dir = Path(temporary) / "i18n"
            output_dir.mkdir()
            (output_dir / "en-US.json").write_text("{}\n", encoding="utf-8")
            with self.assertRaisesRegex(self.exporter.CatalogError, "unexpected entries"):
                self.exporter.export_catalogs(output_dir)


if __name__ == "__main__":
    unittest.main()
