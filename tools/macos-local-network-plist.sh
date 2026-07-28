#!/bin/sh
# Canonical macOS local-network privacy metadata for OpenPencil bundles.
#
# Keep the user-facing explanation free of account, document, device, or
# hostname data. Both bundle implementations source this file so their final
# Info.plist values cannot drift.

OPENPENCIL_LOCAL_NETWORK_USAGE_DESCRIPTION='OpenPencil uses your local network to discover and connect to collaboration sessions you start or join.'
OPENPENCIL_BONJOUR_SERVICE='_openpencil-collab._tcp'

openpencil_apply_macos_local_network_plist() (
  plist_path=$1

  /usr/libexec/PlistBuddy \
    -c "Delete :NSLocalNetworkUsageDescription" \
    "$plist_path" 2>/dev/null || true
  /usr/libexec/PlistBuddy \
    -c "Add :NSLocalNetworkUsageDescription string $OPENPENCIL_LOCAL_NETWORK_USAGE_DESCRIPTION" \
    "$plist_path"

  /usr/libexec/PlistBuddy \
    -c "Delete :NSBonjourServices" \
    "$plist_path" 2>/dev/null || true
  /usr/libexec/PlistBuddy \
    -c "Add :NSBonjourServices array" \
    -c "Add :NSBonjourServices:0 string $OPENPENCIL_BONJOUR_SERVICE" \
    "$plist_path"
)
