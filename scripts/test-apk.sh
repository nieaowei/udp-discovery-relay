#!/bin/sh
# Real package-manager transaction in a disposable root. Firmware dependencies
# are virtual fixtures; no Alpine/OpenWrt packages or host settings are mixed.
set -eu
sdk=${1:?Usage: test-apk.sh SDK_ROOT [DIST_DIR]}
out=${2:-dist}
apk="$sdk/staging_dir/host/bin/apk"
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)
release=$(sed -n 's/^PKG_RELEASE:=//p' "$root/openwrt/udp-discovery-relay/Makefile")
arch=${UDR_APK_ARCH:-aarch64_cortex-a53}
core="$out/udp-discovery-relay-$version-r$release.apk"
ui="$out/gl-ui-udp-discovery-relay-$version-r$release.apk"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
mkdir -p "$work/empty" "$work/root/etc/apk"
printf '%s\n' "$arch" > "$work/root/etc/apk/arch"
"$apk" mkpkg --info name:firmware-test-fixture --info version:1 --info arch:noarch \
    --info 'description:Test dependency capabilities only' \
    --info 'provides:lua lua-cjson libuci-lua ubus gl-sdk4-ui-core gl-oui-rpc luci-lib-nixio' \
    --files "$work/empty" --output "$work/deps.apk"
# The test deliberately does not execute service hooks in the fake firmware.
"$apk" --root "$work/root" --initdb --allow-untrusted --no-network --no-scripts add "$work/deps.apk" "$core" "$ui"
[ -x "$work/root/usr/bin/udp-discovery-relay" ]
[ -f "$work/root/www/views/gl-sdk4-ui-udp-discovery-relay.common.js" ]
"$apk" --root "$work/root" --allow-untrusted --no-network --no-scripts del gl-ui-udp-discovery-relay
[ -x "$work/root/usr/bin/udp-discovery-relay" ]
printf '\n# user customization\n' >> "$work/root/etc/config/udp_discovery_relay"
cp "$work/root/etc/config/udp_discovery_relay" "$work/expected"
# Upgrade with a changed packaged default. apk must preserve user edits and
# stage the replacement as .apk-new under its /etc protection policy.
mkdir "$work/upgrade"
"$apk" extract --allow-untrusted --destination "$work/upgrade" "$core"
printf '\n# upgraded package default\n' >> "$work/upgrade/etc/config/udp_discovery_relay"
"$apk" mkpkg --info name:udp-discovery-relay --info version:0.1.1-r1 --info "arch:$arch" \
    --info 'depends:lua lua-cjson libuci-lua ubus' --files "$work/upgrade" --output "$work/upgrade.apk"
"$apk" --root "$work/root" --allow-untrusted --no-network --no-scripts add "$work/upgrade.apk"
cmp "$work/expected" "$work/root/etc/config/udp_discovery_relay"
"$apk" --root "$work/root" --allow-untrusted --no-network --no-scripts del udp-discovery-relay
[ ! -e "$work/root/usr/bin/udp-discovery-relay" ]
echo 'PASS APK install, UI-only removal, config preservation, core removal'
