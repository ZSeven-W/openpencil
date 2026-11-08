#!/usr/bin/env python3
"""Tests for tools/check-duplication.py, run with: python3 tools/check-duplication.test.py"""

from __future__ import annotations

import importlib.util
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "check_duplication", Path(__file__).with_name("check-duplication.py")
)
check = importlib.util.module_from_spec(SPEC)
# dataclasses resolve annotations through sys.modules.
sys.modules[SPEC.name] = check
SPEC.loader.exec_module(check)

SHARED_FUNCTION = """
pub fn layout_rows(rows: &[Row], width: f32) -> Vec<Rect> {
    let mut out = Vec::with_capacity(rows.len());
    let mut cursor_y = 0.0_f32;
    for row in rows {
        let height = row.height.max(MIN_ROW_HEIGHT);
        let inset = row.inset.unwrap_or(DEFAULT_INSET);
        let rect = Rect::new(inset, cursor_y, width - inset * 2.0, height);
        out.push(rect);
        cursor_y += height + ROW_GAP;
        if cursor_y > MAX_LIST_HEIGHT {
            tracing::warn!(rows = rows.len(), "row list overflowed");
            break;
        }
    }
    out
}
"""


def write(root: Path, relative: str, text: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class Tree:
    def __init__(self) -> None:
        self._dir = tempfile.TemporaryDirectory()
        self.root = Path(self._dir.name)
        write(self.root, "crates/a/src/lib.rs", "// a\n")
        write(self.root, "vendor/jian/crates/j/src/lib.rs", "// j\n")

    def close(self) -> None:
        self._dir.cleanup()


class ScanTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tree = Tree()
        self.root = self.tree.root

    def tearDown(self) -> None:
        self.tree.close()

    def test_copy_between_crates_is_counted_once_per_pair(self) -> None:
        write(self.root, "crates/a/src/rows.rs", SHARED_FUNCTION)
        write(self.root, "crates/b/src/rows.rs", "// copied\n" + SHARED_FUNCTION)
        result = check.scan(self.root)
        self.assertIn("a <-> b", result.pairs)
        self.assertGreaterEqual(result.pairs["a <-> b"], check.MIN_BLOCK)
        self.assertEqual(len(result.function_groups), 1)
        self.assertEqual(result.metrics()["cross_crate_identical_functions"], 1)

    def test_copy_inside_one_crate_is_not_ratcheted(self) -> None:
        write(self.root, "crates/a/src/one.rs", SHARED_FUNCTION)
        write(self.root, "crates/a/src/two.rs", SHARED_FUNCTION)
        result = check.scan(self.root)
        self.assertEqual(result.pairs, {})
        self.assertEqual(result.function_groups, {})

    def test_test_files_and_comments_are_ignored(self) -> None:
        write(self.root, "crates/a/src/rows_tests.rs", SHARED_FUNCTION)
        commented = "\n".join("// " + line for line in SHARED_FUNCTION.splitlines())
        write(self.root, "crates/b/src/rows.rs", commented)
        write(self.root, "crates/c/src/rows.rs", "/* " + SHARED_FUNCTION + " */")
        self.assertEqual(check.scan(self.root).pairs, {})

    def test_host_twin_is_reported_separately(self) -> None:
        write(self.root, "crates/op-host-native/src/rows.rs", SHARED_FUNCTION)
        write(self.root, "crates/op-host-web/src/rows.rs", SHARED_FUNCTION)
        metrics = check.scan(self.root).metrics()
        self.assertGreaterEqual(metrics["host_twin_lines"], check.MIN_BLOCK)
        self.assertEqual(metrics["cross_crate_block_lines"], 0)

    def test_missing_submodule_fails_with_a_hint(self) -> None:
        (self.root / "vendor/jian/crates/j/src/lib.rs").unlink()
        (self.root / "vendor/jian/crates/j/src").rmdir()
        (self.root / "vendor/jian/crates/j").rmdir()
        (self.root / "vendor/jian/crates").rmdir()
        with self.assertRaises(SystemExit) as raised:
            check.scan(self.root)
        self.assertIn("submodule", str(raised.exception))


class RatchetTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tree = Tree()
        self.root = self.tree.root
        write(self.root, "crates/a/src/rows.rs", SHARED_FUNCTION)
        write(self.root, "crates/b/src/rows.rs", SHARED_FUNCTION)

    def tearDown(self) -> None:
        self.tree.close()

    def run_main(self, *args: str) -> tuple[int, str]:
        buffer = io.StringIO()
        with redirect_stdout(buffer):
            code = check.main(["--root", str(self.root), *args])
        return code, buffer.getvalue()

    def test_unchanged_tree_passes_after_update(self) -> None:
        self.assertEqual(self.run_main("--update-baseline")[0], 0)
        code, output = self.run_main()
        self.assertEqual(code, 0, output)
        self.assertIn("check-duplication OK", output)

    def test_new_copy_fails_and_names_the_function(self) -> None:
        self.run_main("--update-baseline")
        write(self.root, "crates/c/src/rows.rs", SHARED_FUNCTION.replace("layout_rows", "layout_cells"))
        write(self.root, "crates/d/src/rows.rs", SHARED_FUNCTION.replace("layout_rows", "layout_cells"))
        code, output = self.run_main()
        self.assertEqual(code, 1)
        self.assertIn("c <-> d", output)
        self.assertIn("layout_cells", output)

    def test_removed_copy_passes_and_asks_to_lock_it_in(self) -> None:
        self.run_main("--update-baseline")
        write(self.root, "crates/b/src/rows.rs", "// shared now\n")
        code, output = self.run_main()
        self.assertEqual(code, 0, output)
        self.assertIn("--update-baseline", output)

    def test_missing_baseline_fails(self) -> None:
        code, output = self.run_main()
        self.assertEqual(code, 1)
        self.assertIn("missing", output)

    def test_baseline_is_stable_json(self) -> None:
        self.run_main("--update-baseline")
        first = (self.root / check.BASELINE).read_text(encoding="utf-8")
        self.run_main("--update-baseline")
        self.assertEqual(first, (self.root / check.BASELINE).read_text(encoding="utf-8"))
        payload = json.loads(first)
        self.assertEqual(set(payload["metrics"]), {
            "host_twin_lines", "cross_crate_block_lines", "cross_crate_identical_functions",
        })


if __name__ == "__main__":
    unittest.main()
