#!/bin/sh
# Default: remove UI only. --core also removes the independent daemon.
set -eu
case "${1:-}" in ''|--core) ;; *) echo 'Usage: uninstall-router.sh [--core]' >&2; exit 1;; esac
apk del gl-ui-udp-discovery-relay
[ "${1:-}" != --core ] || apk del udp-discovery-relay
