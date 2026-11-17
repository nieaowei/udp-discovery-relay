#!/bin/sh
# Run ON the router after copying the reviewed local APKs and SHA256 files.
# Does not fetch/replace firmware feeds or modify firewall/routing.
set -eu
payload=${1:-.}
[ "$(id -u)" = 0 ] || { echo 'Run as root on the router.' >&2; exit 1; }
case "$(apk --print-arch)" in aarch64|aarch64_cortex-a53) ;; *) echo 'Expected ARM64 OpenWrt APK target.' >&2; exit 1;; esac
grep -qx 'aarch64_cortex-a53' /etc/apk/arch || { echo 'Expected configured aarch64_cortex-a53 APK compatibility.' >&2; exit 1; }
cd "$payload"
set -- ./udp-discovery-relay-*-r*.apk
[ "$#" = 1 ] && [ -f "$1" ] || { echo 'Expected exactly one Core APK.' >&2; exit 1; }
core=$1
sha256sum -c "$core.sha256"
cat build-provenance.txt
apk add --simulate --allow-untrusted "$core"
apk add --allow-untrusted "$core"
/usr/bin/udp-discovery-relay --uci --check
/etc/init.d/udp-discovery-relay status
printf '\nInstalled. Review Core statistics and complete the documented firewall/client acceptance tests.\n'
