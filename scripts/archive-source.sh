#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
mkdir -p dist
# Explicit source inputs exclude .git, build products, caches and unrelated files.
tar --exclude='__pycache__' --exclude='.DS_Store' -czf "dist/udp-discovery-relay-$version-source.tar.gz" \
    Cargo.toml Cargo.lock rust-toolchain.toml README.md LICENSE .gitignore .github src tests examples openwrt scripts docs
if command -v sha256sum >/dev/null 2>&1; then
    (cd dist && sha256sum "udp-discovery-relay-$version-source.tar.gz" > "udp-discovery-relay-$version-source.tar.gz.sha256")
else
    (cd dist && shasum -a 256 "udp-discovery-relay-$version-source.tar.gz" > "udp-discovery-relay-$version-source.tar.gz.sha256")
fi
