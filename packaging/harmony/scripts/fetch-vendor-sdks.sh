#!/usr/bin/env bash
# Fetches the vendored third-party sign-in SDK .har packages (Douyin OpenSDK
# and Alipay AFServiceSDK) into entry/libs/. Run before `ohpm install`; the
# entry module references them as file: dependencies. Checksums pin the exact
# upstream artifacts; bump them deliberately when upgrading SDK versions.
set -euo pipefail

libs_dir="$(cd "$(dirname "$0")/.." && pwd)/entry/libs"
mkdir -p "$libs_dir"

douyin_url="https://artifact.bytedance.com/repository/byted-ohpm/@douyin/opensdk-common-external/-/opensdk-common-external-0.0.5.har"
douyin_sha256="a740810b36d2df5de646b4e8f7d1e8dcf5aff9459fe3d78e8a8bf348691dd7b5"
alipay_url="https://mdn.alipayobjects.com/portal_khlfqg/afts/file/A*RcmpSLB_wy4AAAAAAAAAAAAAAQAAAQ"
alipay_zip_sha256="900b8d587241962c6c0d70048ae08656eec4e16e72e4f27e8433fce8366532aa"

douyin_har="$libs_dir/douyin-opensdk-common-external.har"
if [[ ! -f "$douyin_har" ]]; then
  echo "fetching Douyin OpenSDK har"
  curl -fsSL --retry 2 -o "$douyin_har" "$douyin_url"
  echo "$douyin_sha256  $douyin_har" | shasum -a 256 -c - >/dev/null
fi

alipay_har="$libs_dir/afservicesdk.har"
if [[ ! -f "$alipay_har" ]]; then
  echo "fetching Alipay AFServiceSDK har"
  archive="$libs_dir/afservicesdk.zip"
  curl -fsSL --retry 2 -o "$archive" "$alipay_url"
  echo "$alipay_zip_sha256  $archive" | shasum -a 256 -c - >/dev/null
  staging="$libs_dir/.afservicesdk-staging"
  rm -rf "$staging"
  mkdir -p "$staging"
  unzip -q "$archive" -d "$staging"
  extracted="$(find "$staging" -name "afservicesdk-*.har" -type f | grep -v __MACOSX | head -n 1)"
  if [[ -z "$extracted" ]]; then
    echo "error: afservicesdk har not found inside the archive" >&2
    exit 1
  fi
  mv "$extracted" "$alipay_har"
  rm -rf "$staging" "$archive"
fi

echo "vendor SDK hars ready in $libs_dir"
