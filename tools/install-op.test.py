#!/usr/bin/env python3
"""Exercise the Bash installer with isolated downloads and install directories."""

import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import textwrap
import unittest


REPO = Path(__file__).resolve().parents[1]
SOURCE = (REPO / "scripts/install-op.sh").read_text()
RELEASES = "https://api.github.com/repos/ZSeven-W/openpencil/releases"


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="op-installer-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.installer = self.root / "install-op.sh"
        self.installer.write_text(SOURCE)
        self.requests = self.root / "requests.jsonl"
        self.response = self.root / "response.json"
        self.archive = self.root / "op.tar.gz"
        payload = b"#!/bin/sh\nprintf 'fixture op\\n'\n"
        with tarfile.open(self.archive, "w:gz") as archive:
            member = tarfile.TarInfo("op")
            member.size = len(payload)
            member.mode = 0o755
            archive.addfile(member, io.BytesIO(payload))
        mock_bin = self.root / "mock-bin"
        mock_bin.mkdir()
        curl = mock_bin / "curl"
        curl.write_text(textwrap.dedent("""\
            #!/usr/bin/env python3
            import json, os, pathlib, shutil, sys
            args = sys.argv[1:]
            url = next(arg for arg in args if arg.startswith('https://'))
            with open(os.environ['MOCK_REQUESTS'], 'a') as log:
                log.write(json.dumps(url) + '\\n')
            if url.startswith('https://api.github.com/'):
                if os.environ.get('MOCK_NETWORK_ERROR') == '1':
                    print('curl: (7) fixture connection failure', file=sys.stderr)
                    sys.exit(7)
                sys.stdout.write(pathlib.Path(os.environ['MOCK_RESPONSE']).read_text())
                if '--write-out' in args:
                    sys.stdout.write('\\n' + os.environ['MOCK_STATUS'])
                elif os.environ['MOCK_STATUS'] != '200':
                    sys.exit(22)
            else:
                assert url.startswith('https://github.com/ZSeven-W/openpencil/releases/download/')
                destination = args[args.index('-o') + 1]
                shutil.copyfile(os.environ['MOCK_ARCHIVE'], destination)
            """))
        curl.chmod(0o755)
        self.env = os.environ.copy()
        for name in ("OP_VERSION", "OP_PRERELEASE", "BASH_ENV", "ENV"):
            self.env.pop(name, None)
        self.env.update({
            "PATH": str(mock_bin) + os.pathsep + self.env["PATH"],
            "INSTALL_DIR": str(self.root / "installed"),
            "MOCK_REQUESTS": str(self.requests),
            "MOCK_RESPONSE": str(self.response),
            "MOCK_ARCHIVE": str(self.archive),
            "MOCK_STATUS": "200",
        })
        self.response.write_text(json.dumps({"tag_name": "v1.2.3"}))

    def run_installer(self, piped=False):
        self.requests.write_text("")
        command = [shutil.which("bash")]
        if not piped:
            command.append(str(self.installer))
        return subprocess.run(
            command, input=self.installer.read_text() if piped else None,
            env=self.env, text=True, capture_output=True, timeout=20,
        )

    def urls(self):
        return [json.loads(line) for line in self.requests.read_text().splitlines()]

    def assert_installed(self, result, version):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        target = Path(self.env["INSTALL_DIR"]) / "op"
        self.assertTrue(target.is_file())
        self.assertTrue(os.access(target, os.X_OK))
        self.assertIn(f"Installing op {version} (", result.stdout)
        self.assertIn(f"/download/v{version}/op-cli-", self.urls()[-1])

    def assert_failed_before_download(self, result):
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse((Path(self.env["INSTALL_DIR"]) / "op").exists())
        self.assertTrue(all(url.startswith(RELEASES) for url in self.urls()))
        self.assertEqual(len(self.urls()), 1)

    def test_stable_release_installs(self):
        self.assert_installed(self.run_installer(), "1.2.3")
        self.assertEqual(self.urls()[0], RELEASES + "/latest")

    def test_missing_stable_is_actionable_in_file_and_pipe_invocations(self):
        self.env["MOCK_STATUS"] = "404"
        self.response.write_text('{"message":"Not Found"}')
        for piped in (False, True):
            with self.subTest(piped=piped):
                result = self.run_installer(piped=piped)
                self.assert_failed_before_download(result)
                self.assertIn("no stable OpenPencil release", result.stderr)
                self.assertIn("OP_PRERELEASE=1 bash", result.stderr)
                self.assertIn("OP_VERSION=X.Y.Z", result.stderr)
                self.assertEqual(self.urls(), [RELEASES + "/latest"])

    def test_http_failures_never_fall_back_to_prereleases(self):
        for status in ("403", "429", "500", "503"):
            with self.subTest(status=status):
                self.env["MOCK_STATUS"] = status
                result = self.run_installer()
                self.assert_failed_before_download(result)
                self.assertIn(f"HTTP {status}", result.stderr)
                self.assertNotIn("no stable", result.stderr)
                self.assertEqual(self.urls(), [RELEASES + "/latest"])

    def test_network_failure_stops_without_fallback(self):
        self.env["MOCK_NETWORK_ERROR"] = "1"
        result = self.run_installer()
        self.assert_failed_before_download(result)
        self.assertIn("could not query GitHub releases", result.stderr)
        self.assertIn("fixture connection failure", result.stderr)
        self.assertNotIn("no stable", result.stderr)

    def test_explicit_prerelease_selects_first_release_even_from_a_long_list(self):
        releases = [{"tag_name": "v2.0.0-beta.1"}] + [
            {"tag_name": f"v1.0.{index}"} for index in range(10000)
        ]
        self.response.write_text(json.dumps(releases, indent=2))
        for value in ("1", "true"):
            with self.subTest(value=value):
                self.env["OP_PRERELEASE"] = value
                self.assert_installed(self.run_installer(piped=True), "2.0.0-beta.1")
                self.assertEqual(self.urls()[0], RELEASES)

    def test_empty_and_missing_tags_stop_before_download(self):
        for body, prerelease in (([], "1"), ({}, ""), ({"tag_name": ""}, "")):
            with self.subTest(body=body):
                self.response.write_text(json.dumps(body))
                self.env["OP_PRERELEASE"] = prerelease
                result = self.run_installer()
                self.assert_failed_before_download(result)
                self.assertIn("GitHub returned no release tag", result.stderr)

    def test_prerelease_endpoint_404_is_not_reported_as_missing_stable(self):
        self.env.update(OP_PRERELEASE="1", MOCK_STATUS="404")
        result = self.run_installer()
        self.assert_failed_before_download(result)
        self.assertIn("HTTP 404", result.stderr)
        self.assertNotIn("no stable", result.stderr)

    def test_explicit_version_bypasses_api_and_overrides_stamped_version(self):
        self.installer.write_text(SOURCE.replace('DEFAULT_OP_VERSION=""', 'DEFAULT_OP_VERSION="3.0.0"'))
        self.env.update(OP_VERSION="4.0.0", OP_PRERELEASE="1", MOCK_NETWORK_ERROR="1")
        self.assert_installed(self.run_installer(), "4.0.0")
        self.assertEqual(len(self.urls()), 1)

    def test_release_stamped_version_bypasses_api(self):
        self.installer.write_text(SOURCE.replace('DEFAULT_OP_VERSION=""', 'DEFAULT_OP_VERSION="3.0.0"'))
        self.env.update(OP_PRERELEASE="1", MOCK_NETWORK_ERROR="1")
        self.assert_installed(self.run_installer(), "3.0.0")
        self.assertEqual(len(self.urls()), 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
