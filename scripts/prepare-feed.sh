#!/bin/sh
# Stage local locked source for an official OpenWrt SDK with the packages feed.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
sdk=${1:?Usage: prepare-feed.sh /path/to/sdk}
[ -f "$sdk/rules.mk" ] || { echo 'Not an OpenWrt SDK/buildroot' >&2; exit 1; }
[ -f "$sdk/feeds/packages/lang/rust/rust-package.mk" ] || { echo 'Install the matching packages feed (including rust/host) first.' >&2; exit 1; }
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)
package="$sdk/feeds/packages/net/udp-discovery-relay"
[ ! -e "$package" ] || { echo "Destination already exists: $package" >&2; exit 1; }
cp -R "$root/openwrt/udp-discovery-relay" "$package"
mkdir -p "$sdk/dl"
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT HUP INT TERM
mkdir "$temp/udp-discovery-relay-$version"
cp "$root/Cargo.toml" "$root/Cargo.lock" "$root/LICENSE" "$root/README.md" "$temp/udp-discovery-relay-$version/"
cp -R "$root/src" "$root/tests" "$temp/udp-discovery-relay-$version/"
tar -czf "$sdk/dl/udp-discovery-relay-$version.tar.gz" -C "$temp" "udp-discovery-relay-$version"
hash=$(sha256sum "$sdk/dl/udp-discovery-relay-$version.tar.gz" | cut -d' ' -f1)
sed "s/^PKG_HASH:=skip/PKG_HASH:=$hash/" "$package/Makefile" > "$package/Makefile.tmp"
mv "$package/Makefile.tmp" "$package/Makefile"
echo "Prepared locked source $version; run scripts/feeds install udp-discovery-relay, then make package/udp-discovery-relay/compile V=s"
