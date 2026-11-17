#!/bin/sh
# Remove the independent Core package using normal APK dependency handling.
set -eu
[ "$#" = 0 ] || { echo 'Usage: uninstall-router.sh' >&2; exit 1; }
apk del udp-discovery-relay
