#!/usr/bin/env bash
# Fetches the vendored third-party sign-in SDK binaries (Douyin OpenSDK and
# Alipay AFServiceSDK) into packaging/ios/Vendor/. The frameworks are
# proprietary fat static libraries (device arm64 + x86_64 only), so they are
# downloaded on demand instead of being committed; device builds link them
# via the iphoneos-only search paths in project.yml, while simulator builds
# compile the stub branches of the Swift wrappers.
#
# Checksums pin the exact upstream artifacts; bump them deliberately when
# upgrading SDK versions.
set -euo pipefail

vendor_dir="$(cd "$(dirname "$0")/.." && pwd)/Vendor"
mkdir -p "$vendor_dir"

douyin_url="https://sf3-ttcdn-tos.pstatp.com/obj/ies-douyin-opencn/open/DouyinOpenSDK-4.2.5.zip"
douyin_sha256="dc0e7e7e024df20cace9abc97f4cb3bb73c762c7f60fd327c305cf62d03eb71e"
alipay_url="https://mdn.alipayobjects.com/portal_khlfqg/afts/file/A*AY2hR6R_dgoAAAAAAAAAAAAAAQAAAQ"
alipay_sha256="2597a96e3043d2f29dfd81a662d195b4ff75a5fd3d1832b211968e571dfeeb2d"

fetch() {
  local name="$1" url="$2" sha256="$3" framework="$4"
  if [[ -d "$vendor_dir/$framework" ]]; then
    return 0
  fi
  local archive="$vendor_dir/$name.zip"
  echo "fetching $name from $url"
  curl -fsSL --retry 2 -o "$archive" "$url"
  echo "$sha256  $archive" | shasum -a 256 -c - >/dev/null
  local staging="$vendor_dir/.$name-staging"
  rm -rf "$staging"
  mkdir -p "$staging"
  unzip -q "$archive" -d "$staging"
  local extracted
  extracted="$(find "$staging" -maxdepth 2 -name "$framework" -type d | head -n 1)"
  if [[ -z "$extracted" ]]; then
    echo "error: $framework not found inside $name archive" >&2
    exit 1
  fi
  rm -rf "${vendor_dir:?}/$framework"
  mv "$extracted" "$vendor_dir/$framework"
  rm -rf "$staging" "$archive"
}

fetch "DouyinOpenSDK" "$douyin_url" "$douyin_sha256" "DouyinOpenSDK.framework"
fetch "AFServiceSDK" "$alipay_url" "$alipay_sha256" "AFServiceSDK.framework"

# WeChat ships an xcframework; device builds link the ios-arm64 slice like
# the other vendored frameworks (simulator builds compile the Swift stubs).
wechat_url="https://dldir1.qq.com/WechatWebDev/opensdk/XCFramework/OpenSDK2.0.7.zip"
wechat_sha256="5f258a03a91950ed1f0197555aafb2127ad613f8f79413d45751da36eacbf699"
if [[ ! -d "$vendor_dir/WechatOpenSDK.framework" ]]; then
  archive="$vendor_dir/WechatOpenSDK.zip"
  echo "fetching WechatOpenSDK from $wechat_url"
  curl -fsSL --retry 2 -o "$archive" "$wechat_url"
  echo "$wechat_sha256  $archive" | shasum -a 256 -c - >/dev/null
  staging="$vendor_dir/.wechat-staging"
  rm -rf "$staging"
  mkdir -p "$staging"
  unzip -q "$archive" -d "$staging"
  extracted="$(find "$staging" -type d -path "*ios-arm64/WechatOpenSDK.framework" | head -n 1)"
  if [[ -z "$extracted" ]]; then
    echo "error: device WechatOpenSDK.framework not found inside the archive" >&2
    exit 1
  fi
  rm -rf "$vendor_dir/WechatOpenSDK.framework"
  mv "$extracted" "$vendor_dir/WechatOpenSDK.framework"
  rm -rf "$staging" "$archive"
fi
echo "vendor SDKs ready in $vendor_dir"
