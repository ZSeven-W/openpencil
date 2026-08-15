#!/usr/bin/env python3
"""Compute a canonical SHA-256 handoff digest for an already-verified tree."""

from __future__ import annotations

import argparse
import hashlib
import os
import pathlib
import re
import stat
import sys


SAFE_PATH = re.compile(r"^[A-Za-z0-9._+/-]+$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    result.add_argument("--root", required=True, type=pathlib.Path)
    result.add_argument("--expected")
    return result


def file_digest(path: pathlib.Path) -> tuple[int, str]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            size += len(chunk)
            digest.update(chunk)
    return size, digest.hexdigest()


def tree_digest(root: pathlib.Path) -> str:
    if not root.is_dir() or root.is_symlink():
        raise ValueError("handoff root must be a real directory")

    records: list[tuple[str, int, str]] = []
    for current, directories, files in os.walk(root, followlinks=False):
        current_path = pathlib.Path(current)
        for name in directories:
            directory = current_path / name
            if directory.is_symlink():
                raise ValueError(f"handoff directory is symlinked: {directory}")
        for name in files:
            path = current_path / name
            relative = path.relative_to(root).as_posix()
            if not SAFE_PATH.fullmatch(relative):
                raise ValueError(f"handoff path is not canonical: {relative!r}")
            metadata = path.lstat()
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError(f"handoff entry is not a regular file: {relative}")
            size, digest = file_digest(path)
            records.append((relative, size, digest))

    if not records:
        raise ValueError("handoff tree is empty")

    result = hashlib.sha256(b"op-auth-handoff-tree-v1\0")
    for relative, size, digest in sorted(records):
        record = f"path={relative}\nsize={size}\nsha256={digest}\n"
        result.update(record.encode("ascii"))
    return result.hexdigest()


def main() -> int:
    arguments = parser().parse_args()
    expected = arguments.expected
    if expected is not None and not SHA256.fullmatch(expected):
        parser().error("--expected must be exactly 64 lowercase hexadecimal characters")
    try:
        actual = tree_digest(arguments.root)
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    if expected is not None and actual != expected:
        print("error: handoff tree digest does not match the producing job", file=sys.stderr)
        return 1
    print(actual)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
