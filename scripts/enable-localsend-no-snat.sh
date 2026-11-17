#!/bin/sh
# Explicit opt-in: retain TCP LocalSend peer addresses between routed private LANs.
# Leaves existing INPUT/FORWARD, general WAN NAT, Avahi and IPv6 untouched.
set -eu
[ "$(id -u)" = 0 ] || exit 1
[ -z "$(uci changes firewall)" ] || { echo 'Uncommitted firewall edits exist; refusing to overwrite them.' >&2; exit 1; }
include=/etc/udp-discovery-relay-no-snat.nft
if uci -q get firewall.udr_no_snat >/dev/null || [ -e "$include" ]; then
    echo 'Existing udr_no_snat section/include must be reviewed before changing it.' >&2
    exit 1
fi
backup=$(mktemp -d /root/udr-no-snat-backup.XXXXXX)
chmod 700 "$backup"
cp -p /etc/config/firewall "$backup/firewall"
cat > "$include" <<'NFT'
ip saddr 192.168.8.0/24 ip daddr 192.168.1.0/24 tcp dport 53317 counter return comment "UDR direct LocalSend"
NFT
uci batch <<'UCI'
set firewall.udr_no_snat=include
set firewall.udr_no_snat.type='nftables'
set firewall.udr_no_snat.path='/etc/udp-discovery-relay-no-snat.nft'
set firewall.udr_no_snat.position='chain-pre'
set firewall.udr_no_snat.chain='srcnat'
UCI
if ! fw4 check; then
    uci revert firewall
    rm -f "$include"
    echo 'Validation failed; nothing reloaded.' >&2
    exit 1
fi
uci commit firewall
if ! /etc/init.d/firewall reload; then
    cp -p "$backup/firewall" /etc/config/firewall
    rm -f "$include"
    /etc/init.d/firewall reload
    echo "Apply failed; original configuration restored from $backup" >&2
    exit 1
fi
printf 'LocalSend TCP source-preservation exemption applied. Backup: %s\n' "$backup"
