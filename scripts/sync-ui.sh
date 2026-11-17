#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ui=${1:-"$root/../gl-ui-udp-discovery-relay"}
[ -f "$ui/files/www/views/gl-sdk4-ui-udp-discovery-relay.common.js" ] || exit 1
cp -R "$ui/files/." "$root/openwrt/gl-ui-udp-discovery-relay/files/"
mkdir -p "$root/openwrt/gl-ui-udp-discovery-relay/scripts"
cp "$ui/Makefile" "$ui/package.json" "$root/openwrt/gl-ui-udp-discovery-relay/"
cp "$ui/scripts/test-ui.cjs" "$root/openwrt/gl-ui-udp-discovery-relay/scripts/"
echo 'Updated embedded UI package snapshot'
