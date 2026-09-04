#!/usr/bin/env python3
"""Extract one GitHub artifact only when its ABI-v3 schema is exact and safe."""

from __future__ import annotations

import argparse
import os
import shutil
import stat
import sys
import zipfile
from pathlib import Path, PurePosixPath


TARGETS = {
    "aarch64-apple-darwin": "libop_auth.a",
    "aarch64-apple-ios": "libop_auth.a",
    "aarch64-apple-ios-sim": "libop_auth.a",
    "aarch64-linux-android": "libop_auth.a",
    "aarch64-pc-windows-msvc": "op_auth.lib",
    "aarch64-unknown-linux-gnu": "libop_auth.a",
    "x86_64-apple-darwin": "libop_auth.a",
    "x86_64-linux-android": "libop_auth.a",
    "x86_64-pc-windows-msvc": "op_auth.lib",
    "x86_64-unknown-linux-gnu": "libop_auth.a",
}
MAX_MEMBER_BYTES = 128 * 1024 * 1024
MAX_TOTAL_BYTES = 256 * 1024 * 1024


def fail(message: str) -> None:
    raise ValueError(message)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--zip", required=True, dest="archive")
    parser.add_argument("--target", required=True)
    parser.add_argument("--output-root", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.target not in TARGETS:
        fail(f"unsupported op-auth target: {args.target}")

    archive = Path(args.archive)
    output_root = Path(args.output_root)
    target_root = output_root / args.target
    if not archive.is_file() or archive.is_symlink():
        fail("candidate artifact zip must be a regular non-symlink file")
    if target_root.exists() or target_root.is_symlink():
        fail("candidate target output already exists")

    expected_files = {
        PurePosixPath(args.target, name)
        for name in (
            TARGETS[args.target],
            "ABI_VERSION",
            "CANDIDATE",
            "HARDENING-ATTESTATION",
            "SHA256",
            "VERSION",
        )
    }
    seen: set[PurePosixPath] = set()
    total_size = 0

    with zipfile.ZipFile(archive) as bundle:
        for member in bundle.infolist():
            path = PurePosixPath(member.filename)
            if (
                not member.filename
                or member.filename.startswith("/")
                or "\\" in member.filename
                or any(part in ("", ".", "..") for part in path.parts)
            ):
                fail(f"unsafe candidate archive member: {member.filename!r}")
            if member.flag_bits & 0x1:
                fail(f"encrypted candidate archive member: {member.filename!r}")

            if member.is_dir():
                if path != PurePosixPath(args.target):
                    fail(f"unexpected candidate directory: {member.filename!r}")
                continue
            if path not in expected_files or path in seen:
                fail(f"unexpected or duplicate candidate file: {member.filename!r}")

            unix_mode = member.external_attr >> 16
            file_type = stat.S_IFMT(unix_mode)
            if file_type not in (0, stat.S_IFREG):
                fail(f"candidate member is not a regular file: {member.filename!r}")
            if member.file_size > MAX_MEMBER_BYTES:
                fail(f"candidate member exceeds the size limit: {member.filename!r}")
            total_size += member.file_size
            if total_size > MAX_TOTAL_BYTES:
                fail("candidate artifact exceeds the total extraction size limit")
            seen.add(path)

        if seen != expected_files:
            missing = sorted(str(path) for path in expected_files - seen)
            fail(f"candidate artifact has an incomplete schema; missing={missing}")

        target_root.mkdir(parents=True, mode=0o700)
        for path in sorted(expected_files, key=str):
            member = bundle.getinfo(str(path))
            destination = output_root.joinpath(*path.parts)
            with bundle.open(member) as source, destination.open("xb") as output:
                shutil.copyfileobj(source, output, length=1024 * 1024)
            os.chmod(destination, 0o600)

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
