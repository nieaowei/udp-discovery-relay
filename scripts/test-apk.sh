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
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT HUP INT TERM
mkdir -p "$work/empty" "$work/root/etc/apk"
printf '%s\n' "$arch" > "$work/root/etc/apk/arch"
"$apk" mkpkg --info name:firmware-test-fixture --info version:1 --info arch:noarch \
    --info 'description:Test dependency capabilities only' \
    --info 'provides:lua lua-cjson libuci-lua ubus' \
    --files "$work/empty" --output "$work/deps.apk"
# Assert target payload ownership even when CI builds as an ordinary user.
for package in "$core"; do
    "$apk" adbdump "$package" > "$work/package-metadata"
    awk '/^[[:space:]]+(user|group):/ { seen=1; if ($2 != "root") { print "Non-root APK ownership: " $0 > "/dev/stderr"; bad=1 } } END { exit !seen || bad }' "$work/package-metadata"
done
# A usermode database allows the same disposable transactions without root.
# The test deliberately does not execute service hooks in the fake firmware.
if [ "$(id -u)" = 0 ]; then
    "$apk" --root "$work/root" --initdb --allow-untrusted --no-network --no-scripts add "$work/deps.apk" "$core"
else
    "$apk" --root "$work/root" --initdb --usermode --allow-untrusted --no-network --no-scripts add "$work/deps.apk" "$core"
fi
[ -x "$work/root/usr/bin/udp-discovery-relay" ]
printf '\n# user customization\n' >> "$work/root/etc/config/udp_discovery_relay"
cp "$work/root/etc/config/udp_discovery_relay" "$work/expected"
# Upgrade with a changed packaged default. apk must preserve user edits and
# stage the replacement as .apk-new under its /etc protection policy.
mkdir "$work/upgrade"
"$apk" extract --allow-untrusted --no-chown --destination "$work/upgrade" "$core"
printf '\n# upgraded package default\n' >> "$work/upgrade/etc/config/udp_discovery_relay"
"$apk" mkpkg --info name:udp-discovery-relay --info version:0.1.1-r1 --info "arch:$arch" \
    --info 'depends:lua lua-cjson libuci-lua ubus' --files "$work/upgrade" --output "$work/upgrade.apk"
"$apk" --root "$work/root" --allow-untrusted --no-network --no-scripts add "$work/upgrade.apk"
cmp "$work/expected" "$work/root/etc/config/udp_discovery_relay"
"$apk" --root "$work/root" --allow-untrusted --no-network --no-scripts del udp-discovery-relay
[ ! -e "$work/root/usr/bin/udp-discovery-relay" ]
echo 'PASS Core APK install, config preservation and removal'
