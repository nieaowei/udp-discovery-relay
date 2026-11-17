#!/bin/sh
# Build Linux/musl using a real OpenWrt ARM64 SDK toolchain, without changing it.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
sdk=${1:?Usage: build-static.sh SDK_ROOT [OUTPUT_DIR]}
out=${2:-"$root/dist"}
mkdir -p "$out"
out=$(CDPATH= cd -- "$out" && pwd)
sdk=$(CDPATH= cd -- "$sdk" && pwd)
# The SDK contains a single matching aarch64 Cortex-A53 musl toolchain.
set -- "$sdk"/staging_dir/toolchain-aarch64_cortex-a53_gcc-*_musl/bin/aarch64-openwrt-linux-musl-gcc
[ "$#" = 1 ] && [ -x "$1" ] || { echo 'Expected one aarch64_cortex-a53 musl SDK toolchain.' >&2; exit 1; }
linker=$1
export STAGING_DIR="$sdk/staging_dir"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$linker"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$root/target/sdk"}
# Rust's precompiled std requests libunwind even with panic=abort. Use only its
# matching compiler support archive; libc/crt still come from the OpenWrt SDK.
support="$CARGO_TARGET_DIR/udr-linker-support"
mkdir -p "$support"
cp "$(rustc --print sysroot)/lib/rustlib/aarch64-unknown-linux-musl/lib/self-contained/libunwind.a" "$support/"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C target-cpu=cortex-a53 -C target-feature=+crt-static -C link-self-contained=no -L native=$support"
cd "$root"
cargo build --release --locked --target aarch64-unknown-linux-musl
cp "$CARGO_TARGET_DIR/aarch64-unknown-linux-musl/release/udp-discovery-relay" "$out/udp-discovery-relay"
chmod 755 "$out/udp-discovery-relay"
"${linker%-gcc}-readelf" -h "$out/udp-discovery-relay" | grep 'AArch64'
if "${linker%-gcc}-readelf" -l "$out/udp-discovery-relay" | grep -q INTERP; then
    echo 'Expected fully static musl executable, found a dynamic interpreter.' >&2; exit 1
fi
{
    printf 'SDK label: %s\n' "${UDR_SDK_LABEL:-local-sdk-unverified-on-vendor-firmware}"
    printf 'Target: aarch64-unknown-linux-musl; CPU cortex-a53; static\n'
    rustc --version
    "$linker" --version | head -1
    if [ -f "$sdk/version" ]; then cat "$sdk/version"; fi
} > "$out/build-provenance.txt"
chmod 644 "$out/build-provenance.txt"
