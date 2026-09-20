#!/usr/bin/env python3
"""Capture a live page to an OpenPencil web snapshot (v1 JSON).

Joins the two halves that already exist upstream: drives a pinned Chromium
with Playwright, executes the bundled snapshot extractor
(``crates/op-html/assets/snapshot-extractor.js``) in the page, saves the
downloaded ``snapshot.json``, and optionally imports it to a native ``.op``
document via ``op import:snapshot``.

The caller serves the application (including authentication and readiness);
this tool only captures a ready URL. Single purpose by design: no server
lifecycle, no profiles, no baselines.

Environment:
  OPENPENCIL_SNAPSHOT_EXTRACTOR  path to snapshot-extractor.js
                                 (default: sibling of the op-html crate asset
                                 when run from source, else required)
  OPENPENCIL_CHROMIUM            Chromium executable
                                 (default: ``chromium`` from PATH)
  OPENPENCIL_OP_BIN              ``op`` binary for optional ``--to-op``
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

SOURCE_TREE_EXTRACTOR = (
    Path(__file__).resolve().parents[1]
    / "crates"
    / "op-html"
    / "assets"
    / "snapshot-extractor.js"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True, help="Live page URL to capture")
    parser.add_argument("--out", required=True, type=Path,
                        help="Fresh output path for snapshot.json")
    parser.add_argument("--viewport", default="1280x720",
                        help="WIDTHxHEIGHT (default 1280x720)")
    parser.add_argument("--root-selector", default=None,
                        help="CSS selector narrowing capture to one subtree "
                             "(via openpencilSnapshotOptions.root)")
    parser.add_argument("--setup-js", type=Path, default=None,
                        help="Optional JS file evaluated before extraction "
                             "(e.g. wait for hydration)")
    parser.add_argument("--wait-ms", type=int, default=0,
                        help="Extra settle delay before extraction")
    parser.add_argument("--browser-executable", default=None,
                        help="Chromium executable (default $OPENPENCIL_CHROMIUM "
                             "or PATH chromium)")
    parser.add_argument("--to-op", type=Path, default=None,
                        help="Also import the snapshot to this .op path via "
                             "``op import:snapshot``")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        width, height = (int(part) for part in args.viewport.lower().split("x"))
    except ValueError:
        print(f"invalid --viewport {args.viewport!r}, want WIDTHxHEIGHT",
              file=sys.stderr)
        return 2
    if args.out.exists():
        print(f"output must be fresh: {args.out}", file=sys.stderr)
        return 2
    extractor = Path(os.environ.get("OPENPENCIL_SNAPSHOT_EXTRACTOR", ""))
    if not extractor.is_file():
        extractor = SOURCE_TREE_EXTRACTOR
    if not extractor.is_file():
        print("snapshot extractor not found; set OPENPENCIL_SNAPSHOT_EXTRACTOR",
              file=sys.stderr)
        return 2
    extractor_js = extractor.read_text(encoding="utf-8")
    setup_js = args.setup_js.read_text(encoding="utf-8") if args.setup_js else None

    from playwright.sync_api import sync_playwright

    executable = (
        args.browser_executable
        or os.environ.get("OPENPENCIL_CHROMIUM")
    )
    # --no-sandbox and --disable-dev-shm-usage keep headless Chromium
    # working inside Nix sandboxes and containerized CI.
    launch: dict = {"headless": True,
                    "args": ["--no-sandbox", "--disable-dev-shm-usage"]}
    if executable:
        launch["executable_path"] = executable
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(**launch)
        page = browser.new_context(
            viewport={"width": width, "height": height}).new_page()
        page.goto(args.url, wait_until="domcontentloaded")
        if setup_js:
            page.evaluate(setup_js)
        if args.wait_ms:
            page.wait_for_timeout(args.wait_ms)
        if args.root_selector:
            page.evaluate(
                "selector => { globalThis.openpencilSnapshotOptions = "
                "{ root: document.querySelector(selector) };"
                " if (!globalThis.openpencilSnapshotOptions.root)"
                " throw new Error('No element matched selector: ' + selector); }",
                args.root_selector)
        with page.expect_download() as download:
            page.evaluate(extractor_js)
        snapshot_path = download.value.path()
        payload = json.loads(Path(snapshot_path).read_text(encoding="utf-8"))
        browser.close()

    if payload.get("version") != 1 or not isinstance(payload.get("root"), dict):
        print("captured snapshot failed schema check (version==1 with root)",
              file=sys.stderr)
        return 1
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(payload, indent=2) + "\n",
                        encoding="utf-8")
    print(f"snapshot: {args.out} "
          f"({payload.get('title', '')!r} "
          f"{payload.get('viewport', {})})")

    if args.to_op is not None:
        op_bin = os.environ.get("OPENPENCIL_OP_BIN", "op")
        result = subprocess.run(
            [op_bin, "import:snapshot", str(args.out),
             "--out", str(args.to_op)],
            capture_output=True, text=True)
        if result.returncode != 0:
            print(result.stdout + result.stderr, file=sys.stderr)
            return 1
        print(f"document: {args.to_op}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
