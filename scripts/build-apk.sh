#!/bin/sh
# Genuine APK v3 packages using the official OpenWrt SDK host packager.
# This is a static-core packaging path; the feed Makefile uses rust/host instead.
set -eu
umask 022
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
sdk=${1:?Usage: build-apk.sh SDK_ROOT [DIST_DIR]}
out=${2:-"$root/dist"}
mkdir -p "$out"
out=$(CDPATH= cd -- "$out" && pwd)
sdk=$(CDPATH= cd -- "$sdk" && pwd)
apk="$sdk/staging_dir/host/bin/apk"
case "$("$apk" --version)" in 'apk-tools 3.'*) ;; *) echo 'OpenWrt APK v3 host tools required' >&2; exit 1;; esac
[ -f "$out/udp-discovery-relay" ] || { echo 'Run build-static.sh first.' >&2; exit 1; }
# APK package compatibility comes from /etc/apk/arch, not --print-arch,
# which reports the architecture apk-tools itself was compiled for.
arch=${UDR_APK_ARCH:-aarch64_cortex-a53}
case "$arch" in aarch64|aarch64_cortex-a53) ;; *) echo 'Unsupported ARM64 package architecture' >&2; exit 1;; esac
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)
release=$(sed -n 's/^PKG_RELEASE:=//p' "$root/openwrt/udp-discovery-relay/Makefile")
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-0}
case "$SOURCE_DATE_EPOCH" in ''|*[!0-9]*) exit 1;; esac
build=$(mktemp -d)
trap 'rm -rf "$build"' EXIT HUP INT TERM
for name in udp-discovery-relay gl-ui-udp-discovery-relay; do
    payload="$build/$name"
    mkdir -p "$payload/lib/apk/packages"
    cp -R "$root/openwrt/$name/files/." "$payload/"
    if [ "$name" = udp-discovery-relay ]; then
        mkdir -p "$payload/usr/bin"
        cp "$out/udp-discovery-relay" "$payload/usr/bin/"
        chmod 755 "$payload/usr/bin/udp-discovery-relay" "$payload/etc/init.d/udp-discovery-relay" "$payload/usr/libexec/udp-discovery-relay-config"
        depends='lua lua-cjson libuci-lua ubus'
        package_arch=$arch
        printf '/etc/config/udp_discovery_relay\n' > "$payload/lib/apk/packages/$name.conffiles"
        hash=$(sha256sum "$payload/etc/config/udp_discovery_relay" | cut -d' ' -f1)
        printf '/etc/config/udp_discovery_relay %s\n' "$hash" > "$payload/lib/apk/packages/$name.conffiles_static"
    else
        depends="udp-discovery-relay=$version-r$release gl-sdk4-ui-core gl-oui-rpc luci-lib-nixio"
        package_arch=noarch
    fi
    # Same package file-list and lifecycle convention as include/package-pack.mk.
    (cd "$payload" && find . -type f | sed 's|^\.|/|;s|^//|/|' | sort) > "$payload/lib/apk/packages/$name.list"
    cat > "$build/post-install" <<HOOK
#!/bin/sh
[ "\${IPKG_NO_SCRIPT}" = 1 ] && exit 0
[ -s "\${IPKG_INSTROOT}/lib/functions.sh" ] || exit 0
. "\${IPKG_INSTROOT}/lib/functions.sh"
export root="\${IPKG_INSTROOT}"
export pkgname="$name"
default_postinst
HOOK
    if [ "$name" = gl-ui-udp-discovery-relay ]; then
        cat >> "$build/post-install" <<'HOOK'
[ -n "${IPKG_INSTROOT}" ] || { nginx -t -q && nginx -s reload; }
HOOK
    fi
    { printf '#!/bin/sh\nexport PKG_UPGRADE=1\n'; sed '/^#!/d' "$build/post-install"; } > "$build/post-upgrade"
    cat > "$build/pre-deinstall" <<HOOK
#!/bin/sh
[ -s "\${IPKG_INSTROOT}/lib/functions.sh" ] || exit 0
. "\${IPKG_INSTROOT}/lib/functions.sh"
export root="\${IPKG_INSTROOT}"
export pkgname="$name"
default_prerm
HOOK
    cat > "$build/post-deinstall" <<'HOOK'
#!/bin/sh
[ -n "${IPKG_INSTROOT}" ] || { nginx -t -q && nginx -s reload; }
HOOK
    # The core has no web assets and must not invoke nginx during removal.
    [ "$name" != udp-discovery-relay ] || printf '#!/bin/sh\nexit 0\n' > "$build/post-deinstall"
    filename="$name-$version-r$release.apk"
    "$apk" mkpkg --compat 3.0.0_pre1 --compression deflate:9 \
        --info "name:$name" --info "version:$version-r$release" --info "arch:$package_arch" \
        --info 'license:GPL-3.0-only' --info "origin:$name" \
        --info "description:UDP discovery relay - $name" --info "depends:$depends" \
        --info 'tags:openwrt:section=net' \
        --script "post-install:$build/post-install" --script "post-upgrade:$build/post-upgrade" \
        --script "pre-deinstall:$build/pre-deinstall" --script "post-deinstall:$build/post-deinstall" \
        --files "$payload" --output "$out/$filename"
    "$apk" verify --allow-untrusted "$out/$filename"
    (cd "$out" && sha256sum "$filename" > "$filename.sha256")
done
cp "$root/LICENSE" "$out/LICENSE"
echo 'Built unsigned APKv3 packages. Verify provenance and firmware dependencies before installing.'
