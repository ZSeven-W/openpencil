#!/usr/bin/env python3
"""Cross-crate duplication ratchet for the Rust sources.

The native and web widget hosts were single-sourced in the v0.8.3 campaign
(about 8.5k duplicated lines removed), yet by 2026-11 they had grown back to
about 3.8k duplicated lines per side, and the copies had drifted (the web host
never mirrored the page scroll, so `$scroll` bindings stood still). Nothing
measured it. This script measures copy-paste between crates and fails when it
grows; the 2026-11-07 duplication audit in openpencil-docs explains the
categories it tracks.

Two detectors run over production sources (tests are skipped):

* blocks: comments and blank or brace-only lines are dropped, the rest is
  hashed in sliding windows of WINDOW lines, and windows shared by two files
  are merged into blocks of at least MIN_BLOCK normalized lines;
* functions: functions whose parameter list and body are identical after
  whitespace normalization.

Only duplication between different crates is ratcheted. Copies inside one
crate (locale tables, schema field boilerplate) are left to review.

Usage:
  tools/check-duplication.py                     # compare against the baseline
  tools/check-duplication.py --update-baseline   # record the current state
  tools/check-duplication.py --report            # print the largest offenders
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

WINDOW = 8
MIN_BLOCK = 10
MIN_SUBSTANTIVE = 5
MIN_FUNCTION_BODY = 40
HOST_TWIN = ("op-host-native", "op-host-web")
ROOTS = ("crates", "vendor/jian/crates")
BASELINE = Path("tools/duplication-baseline.json")

SKIP_DIRS = {"target", "node_modules", ".git", "tests", "benches", "examples", "fixtures"}
TEST_FILE = re.compile(r"(^tests?_|_tests?\.rs$|^tests?\.rs$|test_support|test_helpers)")
FUNCTION = re.compile(r"\bfn\s+([a-z_][a-z0-9_]*)\s*(<[^>{]*>)?\s*\(")
TRIVIAL_LINE = re.compile(r"[{}()\[\];,]+")
GENERIC_NAMES = {
    "new", "default", "fmt", "from", "drop", "clone", "eq", "hash", "deref", "as_ref",
    "build", "name", "len", "is_empty", "id", "kind", "apply",
}


def strip_comments(text: str) -> str:
    """Remove `//` and nested `/* */` comments, keeping string contents."""
    out: list[str] = []
    i, n = 0, len(text)
    depth = 0
    in_string = False
    while i < n:
        c = text[i]
        if depth:
            if text.startswith("*/", i):
                depth -= 1
                i += 2
                continue
            if text.startswith("/*", i):
                depth += 1
                i += 2
                continue
            if c == "\n":
                out.append("\n")
            i += 1
            continue
        if in_string:
            out.append(c)
            if c == "\\" and i + 1 < n:
                out.append(text[i + 1])
                i += 2
                continue
            if c == '"':
                in_string = False
            i += 1
            continue
        if text.startswith("//", i):
            end = text.find("\n", i)
            i = n if end < 0 else end
            continue
        if text.startswith("/*", i):
            depth = 1
            i += 2
            continue
        if c == '"':
            in_string = True
        out.append(c)
        i += 1
    return "".join(out)


def crate_of(path: str) -> str:
    parts = Path(path).parts
    index = max(i for i, part in enumerate(parts) if part == "crates")
    return parts[index + 1]


def rust_files(root: Path, roots: tuple[str, ...]) -> list[str]:
    files: list[str] = []
    for relative in roots:
        base = root / relative
        if not base.is_dir():
            raise SystemExit(
                f"check-duplication: {relative} is missing; check out submodules "
                "(git submodule update --init --recursive)"
            )
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
            for name in sorted(filenames):
                if name.endswith(".rs") and not TEST_FILE.search(name):
                    files.append(Path(dirpath, name).relative_to(root).as_posix())
    return files


def normalized_lines(source: str) -> list[tuple[int, str]]:
    lines = []
    for number, raw in enumerate(strip_comments(source).split("\n"), 1):
        line = " ".join(raw.split())
        if not line or TRIVIAL_LINE.fullmatch(line):
            continue
        if line.startswith(("use ", "pub use ", "mod ", "pub mod ", "#[")):
            continue
        lines.append((number, line))
    return lines


def functions(source: str):
    """Yield (name, fingerprint, line) for every function with a body."""
    text = strip_comments(source)
    for match in FUNCTION.finditer(text):
        name = match.group(1)
        if name in GENERIC_NAMES:
            continue
        brace = text.find("{", match.end())
        semi = text.find(";", match.end())
        if brace < 0 or 0 <= semi < brace:
            continue
        depth, end = 0, brace
        while end < len(text):
            if text[end] == "{":
                depth += 1
            elif text[end] == "}":
                depth -= 1
                if depth == 0:
                    break
            end += 1
        body = " ".join(text[brace : end + 1].split())
        if len(body) < MIN_FUNCTION_BODY:
            continue
        params = " ".join(text[match.end() : brace].split())
        digest = hashlib.sha1(f"{params}{body}".encode()).hexdigest()[:16]
        yield name, digest, text.count("\n", 0, match.start()) + 1


@dataclass
class Scan:
    pairs: dict[str, int] = field(default_factory=dict)
    blocks: list[dict] = field(default_factory=list)
    function_groups: dict[str, list[str]] = field(default_factory=dict)

    @property
    def host_twin_lines(self) -> int:
        return self.pairs.get(pair_key(*HOST_TWIN), 0)

    @property
    def cross_crate_block_lines(self) -> int:
        return sum(v for k, v in self.pairs.items() if k != pair_key(*HOST_TWIN))

    def metrics(self) -> dict[str, int]:
        return {
            "host_twin_lines": self.host_twin_lines,
            "cross_crate_block_lines": self.cross_crate_block_lines,
            "cross_crate_identical_functions": len(self.function_groups),
        }


def pair_key(a: str, b: str) -> str:
    first, second = sorted((a, b))
    return f"{first} <-> {second}"


def scan(root: Path, roots: tuple[str, ...] = ROOTS) -> Scan:
    files: dict[str, list[tuple[int, str]]] = {}
    windows: dict[str, list[tuple[str, int]]] = defaultdict(list)
    groups: dict[str, list[tuple[str, str, int]]] = defaultdict(list)
    for path in rust_files(root, roots):
        try:
            source = (root / path).read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        lines = normalized_lines(source)
        files[path] = lines
        for start in range(len(lines) - WINDOW + 1):
            chunk = [line for _, line in lines[start : start + WINDOW]]
            if sum(len(line) > 12 for line in chunk) < MIN_SUBSTANTIVE:
                continue
            windows[hashlib.sha1("\n".join(chunk).encode()).hexdigest()].append((path, start))
        for name, digest, line in functions(source):
            groups[digest].append((name, path, line))

    shared: dict[str, dict[str, set[int]]] = defaultdict(lambda: defaultdict(set))
    for hits in windows.values():
        paths = {path for path, _ in hits}
        if len({crate_of(p) for p in paths}) < 2:
            continue
        for path, start in hits:
            for other in paths:
                if crate_of(other) != crate_of(path):
                    shared[path][other].add(start)

    result = Scan()
    for path in sorted(shared):
        for other in sorted(shared[path]):
            # Count each shared region once, from the side that sorts first.
            if other < path:
                continue
            for first, last in runs(sorted(shared[path][other])):
                length = last - first + WINDOW
                if length < MIN_BLOCK:
                    continue
                lines = files[path]
                key = pair_key(crate_of(path), crate_of(other))
                result.pairs[key] = result.pairs.get(key, 0) + length
                result.blocks.append(
                    {
                        "file": path,
                        "other": other,
                        "line": lines[first][0],
                        "normalized_lines": length,
                    }
                )

    for digest, defs in groups.items():
        if len({crate_of(path) for _, path, _ in defs}) >= 2:
            result.function_groups[digest] = sorted(f"{path}:{line} {name}" for name, path, line in defs)
    result.pairs = dict(sorted(result.pairs.items()))
    return result


def runs(starts: list[int]) -> list[tuple[int, int]]:
    out: list[tuple[int, int]] = []
    for start in starts:
        if out and start <= out[-1][1] + 1:
            out[-1] = (out[-1][0], start)
        else:
            out.append((start, start))
    return out


def baseline_payload(result: Scan) -> dict:
    return {
        "note": "Generated by tools/check-duplication.py --update-baseline. Lower it as duplication is removed.",
        "metrics": result.metrics(),
        "pairs": result.pairs,
        "function_groups": {digest: len(defs) for digest, defs in sorted(result.function_groups.items())},
    }


def compare(result: Scan, baseline: dict) -> tuple[list[str], list[str]]:
    """Return (regressions, improvements) against a baseline payload."""
    regressions: list[str] = []
    improvements: list[str] = []
    old_pairs = baseline.get("pairs", {})
    for key, lines in result.pairs.items():
        before = old_pairs.get(key, 0)
        if lines > before:
            regressions.append(f"{key}: {before} -> {lines} duplicated lines")
        elif lines < before:
            improvements.append(f"{key}: {before} -> {lines}")
    for key, before in old_pairs.items():
        if key not in result.pairs:
            improvements.append(f"{key}: {before} -> 0")
    known: dict[str, int] = baseline.get("function_groups", {})
    for digest, defs in sorted(result.function_groups.items()):
        before = known.get(digest, 0)
        if len(defs) > before:
            regressions.append(
                f"identical function copied across crates ({before} -> {len(defs)} copies): " + ", ".join(defs)
            )
    shrunk = sum(1 for digest, count in known.items() if len(result.function_groups.get(digest, [])) < count)
    if shrunk:
        improvements.append(f"{shrunk} identical-function group(s) shrank or disappeared")
    return regressions, improvements


def report(result: Scan, limit: int = 25) -> None:
    metrics = result.metrics()
    print(json.dumps(metrics, indent=2))
    print("\nlargest crate pairs:")
    for key, lines in sorted(result.pairs.items(), key=lambda item: -item[1])[:limit]:
        print(f"  {lines:6d}  {key}")
    print("\nlargest blocks:")
    for block in sorted(result.blocks, key=lambda b: -b["normalized_lines"])[:limit]:
        print(f"  {block['normalized_lines']:4d}  {block['file']}:{block['line']}  ~  {block['other']}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--update-baseline", action="store_true")
    parser.add_argument("--report", action="store_true")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args(argv)

    root = args.root.resolve()
    result = scan(root)
    if args.report:
        report(result)
        return 0
    baseline_path = root / BASELINE
    if args.update_baseline:
        baseline_path.parent.mkdir(parents=True, exist_ok=True)
        baseline_path.write_text(json.dumps(baseline_payload(result), indent=2) + "\n", encoding="utf-8")
        print(f"check-duplication: baseline written: {json.dumps(result.metrics())}")
        return 0
    if not baseline_path.is_file():
        print(f"::error::{BASELINE} is missing; run tools/check-duplication.py --update-baseline")
        return 1
    regressions, improvements = compare(result, json.loads(baseline_path.read_text(encoding="utf-8")))
    if regressions:
        print("::error::cross-crate duplication grew (see crates/CLAUDE.md, duplication ratchet):")
        for line in regressions:
            print(f"  {line}")
        print(
            "Share the code instead of copying it (op_editor_core::host_*_transitions, "
            "op_editor_ui::widgets::*_flow, or the owning Jian crate). If a copy is truly "
            "unavoidable, justify it in review and run tools/check-duplication.py --update-baseline."
        )
        return 1
    if improvements:
        print("check-duplication: duplication went down; lock it in with --update-baseline:")
        for line in improvements:
            print(f"  {line}")
    print(f"check-duplication OK: {json.dumps(result.metrics())}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
