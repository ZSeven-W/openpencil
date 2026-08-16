#!/usr/bin/env python3
"""Fail closed unless two stdin URLs satisfy the production relay policy.

Input is exactly two NUL-terminated ASCII URLs. Keeping endpoint values off the
command line and out of child-process environments avoids exposing repository
secrets while applying one shared release gate on Linux and macOS.
"""

from __future__ import annotations

import ipaddress
import re
import sys
from urllib.parse import SplitResult, urlsplit


BOOTSTRAP_PATH = "/api/v1/collaboration/bootstrap"
MAX_URL_BYTES = 2_048
DNS_LABEL = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$")
NON_PUBLIC_DNS_SUFFIXES = (
    ".example",
    ".internal",
    ".invalid",
    ".local",
    ".localhost",
    ".test",
)
NAMES = ("CN relay bootstrap", "global relay bootstrap")


def fail(name: str, reason: str) -> None:
    raise ValueError(f"{name} URL {reason}")


def canonical_host(parts: SplitResult, name: str) -> str:
    try:
        host = parts.hostname
        port = parts.port
    except ValueError as error:
        fail(name, f"has an invalid authority ({error})")
    if not host:
        fail(name, "has no host")
    if not host.isascii() or host != host.lower() or "%" in host:
        fail(name, "must use a lowercase canonical ASCII host")
    if port in (0, 443):
        fail(name, "must not use port 0 or an explicit default port 443")

    try:
        address = ipaddress.ip_address(host)
    except ValueError:
        if len(host) > 253 or host.endswith("."):
            fail(name, "has a non-canonical DNS host")
        labels = host.split(".")
        if (
            len(labels) < 2
            or not any(character.isalpha() for character in labels[-1])
            or any(DNS_LABEL.fullmatch(label) is None for label in labels)
            or host == "localhost"
            or any(host.endswith(suffix) for suffix in NON_PUBLIC_DNS_SUFFIXES)
        ):
            fail(name, "must use a canonical public DNS host")
        authority = host
    else:
        if not address.is_global:
            fail(name, "must not use a private, loopback, link-local, or reserved address")
        canonical = address.compressed.lower()
        if host != canonical:
            fail(name, "has a non-canonical IP address")
        authority = f"[{canonical}]" if address.version == 6 else canonical

    if port is not None:
        authority += f":{port}"
    return authority


def validate(value_bytes: bytes, name: str) -> str:
    if not value_bytes or len(value_bytes) > MAX_URL_BYTES:
        fail(name, "is empty or exceeds 2048 ASCII bytes")
    try:
        value = value_bytes.decode("ascii")
    except UnicodeDecodeError:
        fail(name, "must contain ASCII only")
    if value.strip() != value or any(character.isspace() for character in value):
        fail(name, "must not contain whitespace")

    parts = urlsplit(value)
    if parts.scheme != "https":
        fail(name, "must use https")
    if parts.username is not None or parts.password is not None:
        fail(name, "must not contain user information")
    if parts.path != BOOTSTRAP_PATH or parts.query or parts.fragment:
        fail(name, f"must use exactly {BOOTSTRAP_PATH} with no query or fragment")

    authority = canonical_host(parts, name)
    canonical = f"https://{authority}{BOOTSTRAP_PATH}"
    if value != canonical or parts.netloc != authority:
        fail(name, "is not in canonical URL form")
    return value


def validate_pair(payload: bytes) -> None:
    fields = payload.split(b"\0")
    if len(fields) != 3 or fields[-1] != b"":
        raise ValueError("expected exactly two NUL-terminated relay URLs")
    endpoints = tuple(validate(value, name) for value, name in zip(fields[:2], NAMES))
    if endpoints[0] == endpoints[1]:
        raise ValueError("CN and global relay bootstrap URLs must be distinct")


def self_test() -> None:
    valid = (
        b"https://cn.openpencil.dev/api/v1/collaboration/bootstrap",
        b"https://global.openpencil.dev:8443/api/v1/collaboration/bootstrap",
    )
    validate_pair(valid[0] + b"\0" + valid[1] + b"\0")
    invalid = (
        b"http://hub.openpencil.dev/api/v1/collaboration/bootstrap",
        b" https://hub.openpencil.dev/api/v1/collaboration/bootstrap",
        b"https://Hub.openpencil.dev/api/v1/collaboration/bootstrap",
        b"https://hub.openpencil.dev:443/api/v1/collaboration/bootstrap",
        b"https://hub.openpencil.dev:0/api/v1/collaboration/bootstrap",
        b"https://hub.openpencil.dev:65536/api/v1/collaboration/bootstrap",
        b"https://hub.openpencil.dev:/api/v1/collaboration/bootstrap",
        b"https://[fe80::1%25en0]/api/v1/collaboration/bootstrap",
        b"https://hub.openpencil.dev/api/v1/collaboration/%62ootstrap",
        b"https://hub.openpencil.dev/api/v1/collaboration/bootstrap/",
        b"https://user@hub.openpencil.dev/api/v1/collaboration/bootstrap",
        b"https://hub.openpencil.dev/api/v1/collaboration/bootstrap?q=1",
        b"https://hub.openpencil.dev/api/v1/collaboration/bootstrap#fragment",
        b"https://hub.openpencil.dev./api/v1/collaboration/bootstrap",
        b"https://127.1/api/v1/collaboration/bootstrap",
        b"https://0x7f000001/api/v1/collaboration/bootstrap",
        b"https://singlelabel/api/v1/collaboration/bootstrap",
        b"https://127.0.0.1/api/v1/collaboration/bootstrap",
        b"https://10.0.0.1/api/v1/collaboration/bootstrap",
        b"https://hub.local/api/v1/collaboration/bootstrap",
        b"https://hub.example/api/v1/collaboration/bootstrap",
    )
    for endpoint in invalid:
        try:
            validate_pair(endpoint + b"\0" + valid[1] + b"\0")
        except ValueError:
            continue
        raise AssertionError(f"invalid endpoint passed: {endpoint!r}")
    try:
        validate_pair(valid[0] + b"\0" + valid[0] + b"\0")
    except ValueError:
        pass
    else:
        raise AssertionError("duplicate regional endpoints passed")


def main() -> int:
    try:
        if sys.argv[1:] == ["--self-test"]:
            self_test()
            print("check-collab-bootstrap-urls.py: canonical relay policy tests passed.")
            return 0
        if sys.argv[1:]:
            raise ValueError("usage: check-collab-bootstrap-urls.py [--self-test]")
        validate_pair(sys.stdin.buffer.read(MAX_URL_BYTES * 2 + 4))
    except (AssertionError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print("Validated two distinct canonical production relay bootstrap URLs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
