#!/usr/bin/env bash
# Stage and verify the immutable, same-run artifact used to update package indexes.

set -euo pipefail

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

required_files() {
    local version=$1
    printf '%s\n' \
        "OpenPencil-${version}-arm64-mac.dmg" \
        "OpenPencil-${version}-x64-mac.dmg" \
        openpencil-desktop-windows-aarch64.zip \
        openpencil-desktop-windows-x86_64.zip \
        op-cli-linux-aarch64.tar.gz \
        op-cli-linux-x86_64.tar.gz \
        op-cli-macos-aarch64.tar.gz \
        op-cli-macos-x86_64.tar.gz \
        op-cli-windows-aarch64.zip \
        op-cli-windows-x86_64.zip
}

validate_version() {
    [[ $1 =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]] || {
        printf 'error: invalid handoff version\n' >&2
        exit 2
    }
}

stage_handoff() {
    local source_dir=$1 destination=$2 version=$3 output_file=$4 file manifest_sha256
    validate_version "$version"
    [[ -d $source_dir && ! -L $source_dir ]] || {
        printf 'error: handoff source is not a regular directory\n' >&2
        exit 1
    }
    [[ ! -e $destination && ! -L $destination ]] || {
        printf 'error: refusing to replace an existing handoff destination\n' >&2
        exit 1
    }
    [[ -f $output_file && ! -L $output_file ]] || {
        printf 'error: handoff output path is not a regular file\n' >&2
        exit 1
    }
    mkdir "$destination"
    while IFS= read -r file; do
        [[ -f $source_dir/$file && ! -L $source_dir/$file ]] || {
            printf 'error: missing handoff asset: %s\n' "$file" >&2
            exit 1
        }
        cp -- "$source_dir/$file" "$destination/"
    done < <(required_files "$version")
    (cd "$destination" && required_files "$version" | xargs sha256sum -- | sort -k2 > SHA256SUMS.txt)
    manifest_sha256=$(sha256sum "$destination/SHA256SUMS.txt" | awk '{print $1}')
    printf 'manifest_sha256=%s\n' "$manifest_sha256" >> "$output_file"
}

download_handoff() {
    local artifact_id=$1 expected_digest=$2 repository=$3 run_id=$4 run_attempt=$5
    local api_url=$6 destination=$7 version=$8 token temporary config metadata archive expected_files
    validate_version "$version"
    [[ $artifact_id =~ ^[1-9][0-9]*$ && $run_id =~ ^[1-9][0-9]*$ \
        && $run_attempt =~ ^[1-9][0-9]*$ ]] || {
        printf 'error: invalid same-run artifact identity\n' >&2
        exit 2
    }
    [[ $expected_digest =~ ^[0-9a-f]{64}$ ]] || {
        printf 'error: producer artifact digest is missing or malformed\n' >&2
        exit 2
    }
    [[ $repository =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ \
        && $api_url =~ ^https://[A-Za-z0-9.-]+$ ]] || {
        printf 'error: invalid GitHub API origin or repository\n' >&2
        exit 2
    }
    [[ ! -e $destination && ! -L $destination ]] || {
        printf 'error: refusing to replace an existing artifact destination\n' >&2
        exit 1
    }
    token=${GH_TOKEN:-}
    export -n token
    unset GH_TOKEN
    [[ -n $token ]] || {
        printf 'error: GH_TOKEN is required for the exact artifact download\n' >&2
        exit 2
    }
    umask 077
    temporary=$(mktemp -d)
    config=$temporary/curl.conf
    metadata=$temporary/metadata.json
    archive=$temporary/artifact.zip
    expected_files=$temporary/expected-files
    trap 'rm -rf "$temporary"' EXIT
    {
        printf 'header = "Accept: application/vnd.github+json"\n'
        printf 'header = "Authorization: Bearer %s"\n' "$token"
        printf 'header = "X-GitHub-Api-Version: 2022-11-28"\n'
    } > "$config"
    token=
    local metadata_url="$api_url/repos/$repository/actions/artifacts/$artifact_id"
    curl --fail --location --proto '=https' --tlsv1.2 --silent --show-error \
        --config "$config" --output "$metadata" "$metadata_url"
    python3 - "$metadata" "$artifact_id" "$expected_digest" "$run_id" \
        "package-manager-assets-$run_id-$run_attempt" "$metadata_url/zip" <<'PYTHON'
import json
import pathlib
import sys

path, artifact_id, expected_digest, run_id, name, archive_url = sys.argv[1:]
artifact = json.loads(pathlib.Path(path).read_text())
digest = artifact.get("digest", "")
if digest.startswith("sha256:"):
    digest = digest[7:]
checks = (
    artifact.get("id") == int(artifact_id),
    artifact.get("name") == name,
    artifact.get("expired") is False,
    artifact.get("workflow_run", {}).get("id") == int(run_id),
    artifact.get("archive_download_url") == archive_url,
    digest == expected_digest,
)
if not all(checks):
    raise SystemExit("error: artifact metadata is not bound to this workflow run and producer digest")
PYTHON
    curl --fail --location --proto '=https' --tlsv1.2 --silent --show-error \
        --config "$config" --output "$archive" "$metadata_url/zip"
    [[ $(sha256_file "$archive") == "$expected_digest" ]] || {
        printf 'error: downloaded artifact ZIP differs from the producer digest\n' >&2
        exit 1
    }
    { printf 'SHA256SUMS.txt\n'; required_files "$version"; } | sort > "$expected_files"
    python3 - "$archive" "$destination" "$expected_files" <<'PYTHON'
import pathlib
import shutil
import stat
import sys
import zipfile

archive, destination, expected_file = map(pathlib.Path, sys.argv[1:])
expected = expected_file.read_text().splitlines()
with zipfile.ZipFile(archive) as bundle:
    infos = bundle.infolist()
    names = [item.filename for item in infos]
    if sorted(names) != expected or len(names) != len(set(names)):
        raise SystemExit("error: artifact ZIP file set differs from the reviewed handoff")
    for item in infos:
        mode = item.external_attr >> 16
        if item.is_dir() or stat.S_ISLNK(mode) or not item.filename or "\\" in item.filename:
            raise SystemExit("error: artifact ZIP contains an unsafe member")
    destination.mkdir(mode=0o700)
    for item in infos:
        output = destination / item.filename
        with bundle.open(item) as source, output.open("xb") as target:
            shutil.copyfileobj(source, target)
        output.chmod(0o600)
PYTHON
    rm -rf "$temporary"
    trap - EXIT
}

verify_handoff() {
    local directory=$1 version=$2 artifact_digest=$3 manifest_sha256=$4 env_output=$5 file actual_manifest_sha256
    local -a expected_files actual_files
    validate_version "$version"
    [[ $artifact_digest =~ ^[0-9a-f]{64}$ ]] || {
        printf 'error: same-run artifact digest output is missing or malformed\n' >&2
        exit 1
    }
    [[ $manifest_sha256 =~ ^[0-9a-f]{64}$ ]] || {
        printf 'error: same-run manifest digest output is missing or malformed\n' >&2
        exit 1
    }
    [[ -d $directory && ! -L $directory && -f $env_output && ! -L $env_output ]] || {
        printf 'error: invalid handoff or environment output path\n' >&2
        exit 1
    }
    while IFS= read -r file; do expected_files+=("$file"); done \
        < <({ printf 'SHA256SUMS.txt\n'; required_files "$version"; } | sort)
    while IFS= read -r file; do actual_files+=("$file"); done \
        < <(find "$directory" -mindepth 1 -maxdepth 1 -type f -exec basename {} \; | sort)
    if [[ ${#expected_files[@]} -ne ${#actual_files[@]} ]] \
        || ! diff -u <(printf '%s\n' "${expected_files[@]}") <(printf '%s\n' "${actual_files[@]}"); then
        printf 'error: handoff file set differs from the reviewed contract\n' >&2
        return 1
    fi
    while IFS= read -r file; do
        [[ -f $directory/$file && ! -L $directory/$file ]] || {
            printf 'error: handoff asset is not a regular file: %s\n' "$file" >&2
            exit 1
        }
    done < <(required_files "$version")
    actual_manifest_sha256=$(sha256sum "$directory/SHA256SUMS.txt" | awk '{print $1}')
    [[ $actual_manifest_sha256 == "$manifest_sha256" ]] || {
        printf 'error: handoff manifest is not the one emitted by the producer job\n' >&2
        return 1
    }
    (cd "$directory" && sha256sum --check --strict SHA256SUMS.txt) || {
        printf 'error: handoff manifest verification failed\n' >&2
        return 1
    }
    sha_file() { sha256sum "$directory/$1" | awk '{print $1}'; }
    {
        printf 'MAC_ARM_DMG_SHA=%s\n' "$(sha_file "OpenPencil-${version}-arm64-mac.dmg")"
        printf 'MAC_X64_DMG_SHA=%s\n' "$(sha_file "OpenPencil-${version}-x64-mac.dmg")"
        printf 'CLI_MAC_ARM_SHA=%s\n' "$(sha_file op-cli-macos-aarch64.tar.gz)"
        printf 'CLI_MAC_X64_SHA=%s\n' "$(sha_file op-cli-macos-x86_64.tar.gz)"
        printf 'CLI_LINUX_ARM_SHA=%s\n' "$(sha_file op-cli-linux-aarch64.tar.gz)"
        printf 'CLI_LINUX_X64_SHA=%s\n' "$(sha_file op-cli-linux-x86_64.tar.gz)"
        printf 'DESKTOP_WIN_ARM_SHA=%s\n' "$(sha_file openpencil-desktop-windows-aarch64.zip)"
        printf 'DESKTOP_WIN_X64_SHA=%s\n' "$(sha_file openpencil-desktop-windows-x86_64.zip)"
        printf 'CLI_WIN_ARM_SHA=%s\n' "$(sha_file op-cli-windows-aarch64.zip)"
        printf 'CLI_WIN_X64_SHA=%s\n' "$(sha_file op-cli-windows-x86_64.zip)"
    } >> "$env_output"
}

self_test() {
    local temporary source handoff env_file output_file file digest manifest_digest manifest_line
    temporary=$(mktemp -d)
    source=$temporary/source
    handoff=$temporary/handoff
    env_file=$temporary/env
    output_file=$temporary/output
    mkdir "$source"
    : > "$env_file"
    : > "$output_file"
    while IFS= read -r file; do printf '%s\n' "$file" > "$source/$file"; done < <(required_files 1.2.3)
    stage_handoff "$source" "$handoff" 1.2.3 "$output_file"
    digest=$(printf artifact | sha256sum | awk '{print $1}')
    manifest_line=$(cat "$output_file")
    manifest_digest=${manifest_line#manifest_sha256=}
    verify_handoff "$handoff" 1.2.3 "$digest" "$manifest_digest" "$env_file"
    printf tamper >> "$handoff/op-cli-linux-x86_64.tar.gz"
    if verify_handoff "$handoff" 1.2.3 "$digest" "$manifest_digest" "$env_file" >/dev/null 2>&1; then
        printf 'error: tampered handoff was accepted\n' >&2
        exit 1
    fi
    (cd "$handoff" && required_files 1.2.3 | xargs sha256sum -- | sort -k2 > SHA256SUMS.txt)
    if verify_handoff "$handoff" 1.2.3 "$digest" "$manifest_digest" "$env_file" >/dev/null 2>&1; then
        printf 'error: attacker-rewritten handoff manifest was accepted\n' >&2
        exit 1
    fi
    rm -rf "$temporary"
    printf 'package-manager-handoff.sh: positive and tamper tests passed.\n'
}

case ${1-} in
    stage)
        [[ $# -eq 5 ]] || { printf 'usage: %s stage SOURCE DEST VERSION OUTPUT\n' "$0" >&2; exit 2; }
        stage_handoff "$2" "$3" "$4" "$5"
        ;;
    download)
        [[ $# -eq 9 ]] || {
            printf 'usage: GH_TOKEN=... %s download ID DIGEST REPO RUN_ID ATTEMPT API_URL DEST VERSION\n' "$0" >&2
            exit 2
        }
        download_handoff "$2" "$3" "$4" "$5" "$6" "$7" "$8" "$9"
        ;;
    verify)
        [[ $# -eq 6 ]] || { printf 'usage: %s verify DIR VERSION ARTIFACT_DIGEST MANIFEST_DIGEST ENV_OUTPUT\n' "$0" >&2; exit 2; }
        verify_handoff "$2" "$3" "$4" "$5" "$6"
        ;;
    --self-test)
        [[ $# -eq 1 ]] || exit 2
        self_test
        ;;
    *)
        printf 'usage: %s {stage|download|verify|--self-test} ...\n' "$0" >&2
        exit 2
        ;;
esac
