#!/bin/sh
# Build the Core APK v3 with the official OpenWrt SDK packager.
set -eu
if [ "$(id -u)" != 0 ]; then
    command -v fakeroot >/dev/null 2>&1 || { echo 'Non-root APK packaging requires fakeroot.' >&2; exit 1; }
    exec fakeroot -- "$0" "$@"
fi
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
arch=${UDR_APK_ARCH:-aarch64_cortex-a53}
case "$arch" in aarch64|aarch64_cortex-a53) ;; *) echo 'Unsupported ARM64 package architecture' >&2; exit 1;; esac
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)
release=$(sed -n 's/^PKG_RELEASE:=//p' "$root/openwrt/udp-discovery-relay/Makefile")
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-0}
case "$SOURCE_DATE_EPOCH" in ''|*[!0-9]*) exit 1;; esac
build=$(mktemp -d)
trap 'rm -rf "$build"' EXIT HUP INT TERM
name=udp-discovery-relay
payload="$build/root"
mkdir -p "$payload/lib/apk/packages" "$payload/usr/bin"
cp -R "$root/openwrt/$name/files/." "$payload/"
cp "$out/udp-discovery-relay" "$payload/usr/bin/"
chmod 755 "$payload/usr/bin/udp-discovery-relay" "$payload/etc/init.d/udp-discovery-relay" "$payload/usr/libexec/udp-discovery-relay-config"
printf '/etc/config/udp_discovery_relay\n' > "$payload/lib/apk/packages/$name.conffiles"
hash=$(sha256sum "$payload/etc/config/udp_discovery_relay" | cut -d' ' -f1)
printf '/etc/config/udp_discovery_relay %s\n' "$hash" > "$payload/lib/apk/packages/$name.conffiles_static"
(cd "$payload" && find . -type f | sed 's|^\.|/|;s|^//|/|' | sort) > "$payload/lib/apk/packages/$name.list"
cat > "$build/post-install" <<'HOOK'
#!/bin/sh
[ "${IPKG_NO_SCRIPT}" = 1 ] && exit 0
[ -s "${IPKG_INSTROOT}/lib/functions.sh" ] || exit 0
. "${IPKG_INSTROOT}/lib/functions.sh"
export root="${IPKG_INSTROOT}"
export pkgname=udp-discovery-relay
default_postinst
HOOK
{ printf '#!/bin/sh\nexport PKG_UPGRADE=1\n'; sed '/^#!/d' "$build/post-install"; } > "$build/post-upgrade"
cat > "$build/pre-deinstall" <<'HOOK'
#!/bin/sh
[ -s "${IPKG_INSTROOT}/lib/functions.sh" ] || exit 0
. "${IPKG_INSTROOT}/lib/functions.sh"
export root="${IPKG_INSTROOT}"
export pkgname=udp-discovery-relay
default_prerm
HOOK
filename="$name-$version-r$release.apk"
"$apk" mkpkg --compat 3.0.0_pre1 --compression deflate:9 \
    --info "name:$name" --info "version:$version-r$release" --info "arch:$arch" \
    --info 'license:GPL-3.0-only' --info "origin:$name" \
    --info 'description:Source-preserving UDP discovery relay' \
    --info 'depends:lua lua-cjson libuci-lua ubus' --info 'tags:openwrt:section=net' \
    --script "post-install:$build/post-install" --script "post-upgrade:$build/post-upgrade" \
    --script "pre-deinstall:$build/pre-deinstall" \
    --files "$payload" --output "$build/$filename"
"$apk" adbdump "$build/$filename" > "$build/metadata"
if ! awk '/^[[:space:]]+(user|group):/ { seen=1; if ($2 != "root") { print "Non-root APK ownership: " $0 > "/dev/stderr"; bad=1 } } END { exit !seen || bad }' "$build/metadata"; then
    echo 'SDK apk did not record root ownership; rerun packaging as real root (sudo or a root container).' >&2
    exit 1
fi
"$apk" verify --allow-untrusted "$build/$filename"
mv "$build/$filename" "$out/$filename"
(cd "$out" && sha256sum "$filename" > "$filename.sha256")
cp "$root/LICENSE" "$out/LICENSE"
echo 'Built unsigned Core APKv3. Verify provenance and target dependencies before installing.'
