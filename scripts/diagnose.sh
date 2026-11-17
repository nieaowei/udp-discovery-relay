#!/bin/sh
# Read-only local evidence, with no credentials or client payloads.
set -u
printf 'Daemon configuration validation\n'
/usr/bin/udp-discovery-relay --uci --check
printf '\nService\n'
/etc/init.d/udp-discovery-relay status
printf '\nDaemon metrics\n'
cat /var/run/udp-discovery-relay/status.json
printf '\nLogical WAN\n'
ubus call network.interface.wan status
printf '\nLogical LAN\n'
ubus call network.interface.lan status
printf '\nIPv4 routes\n'
ip -4 route show
printf '\nLocal IPv4 firewall and SNAT evidence\n'
nft list table inet fw4
printf '\nRecent bounded relay logs\n'
logread -e udp-discovery-relay -l 50
printf '\nVerify return routes for the configured peer subnets, client firewall permissions, TCP source IP and TLS registration. Local counters alone cannot confirm these.\n'
