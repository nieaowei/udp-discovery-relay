#!/bin/sh
# Explicit opt-in for the inspected MT3600BE topology. Never called by packages.
set -eu
[ "$(id -u)" = 0 ] || exit 1
[ -z "$(uci changes firewall)" ] || { echo 'Pending firewall edits exist; preserve/review them before running this script.' >&2; exit 1; }
for section in udr_input_wan udr_no_snat; do
    if uci -q get "firewall.$section" >/dev/null; then
        echo "Refusing to overwrite existing firewall.$section" >&2
        exit 1
    fi
done
include=/etc/udp-discovery-relay-no-snat.nft
[ ! -e "$include" ] || { echo 'NAT include already exists; inspect it first.' >&2; exit 1; }
backup=$(mktemp -d /root/udr-firewall-backup.XXXXXX)
chmod 700 "$backup"
cp -p /etc/config/firewall "$backup/firewall"
cat > "$include" <<'NFT'
ip saddr 192.168.8.0/24 ip daddr 192.168.1.0/24 tcp dport 53317 counter return comment "UDR direct LocalSend"
NFT
uci batch <<'UCI'
set firewall.udr_input_wan=rule
set firewall.udr_input_wan.name='UDP relay discovery from upstream LAN'
set firewall.udr_input_wan.src='wan'
set firewall.udr_input_wan.src_ip='192.168.1.0/24'
set firewall.udr_input_wan.dest_ip='224.0.0.167'
set firewall.udr_input_wan.proto='udp'
set firewall.udr_input_wan.dest_port='53317'
set firewall.udr_input_wan.family='ipv4'
set firewall.udr_input_wan.target='ACCEPT'
set firewall.udr_no_snat=include
set firewall.udr_no_snat.type='nftables'
set firewall.udr_no_snat.path='/etc/udp-discovery-relay-no-snat.nft'
set firewall.udr_no_snat.position='chain-pre'
set firewall.udr_no_snat.chain='srcnat'
UCI
if ! fw4 check; then
    uci revert firewall
    rm -f "$include"
    echo "Validation failed; no firewall reload. Backup: $backup" >&2
    exit 1
fi
uci commit firewall
if ! /etc/init.d/firewall reload; then
    cp -p "$backup/firewall" /etc/config/firewall
    rm -f "$include"
    /etc/init.d/firewall reload
    echo "Reload failed; original configuration restored from $backup" >&2
    exit 1
fi
printf 'Applied scoped LocalSend rules. Backup: %s\n' "$backup"
