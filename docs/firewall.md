# firewall4 / nftables

**The packages never change your firewall, Avahi configuration or routing.** Automatic firewall modification is absent, and therefore off. The examples below use two fictional private subnets: upstream `192.168.1.0/24` in zone `wan`, and downstream `192.168.8.0/24` in zone `lan`. The relay host has an upstream address of `192.168.1.2`. Replace these addresses, zones and ports with the actual network configuration. Verify that the zone names really are `wan` and `lan` using `uci show firewall`; logical networks and firewall zones are different objects.

## INPUT: discovery to the router

Add named sections using `uci batch` (running this block is an intentional firewall change):

```sh
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
set firewall.udr_input_lan=rule
set firewall.udr_input_lan.name='UDP relay discovery from local LAN'
set firewall.udr_input_lan.src='lan'
set firewall.udr_input_lan.src_ip='192.168.8.0/24'
set firewall.udr_input_lan.dest_ip='224.0.0.167'
set firewall.udr_input_lan.proto='udp'
set firewall.udr_input_lan.dest_port='53317'
set firewall.udr_input_lan.family='ipv4'
set firewall.udr_input_lan.target='ACCEPT'
UCI
```

These sections have no `dest` zone, so they govern INPUT, not forwarding. Source subnets are private trusted network scopes, not the public Internet. Narrow them further to device addresses when practical. Broadcast rules require their own destination broadcast address and UDP port. Do not open UDP 5353, DHCP or SSDP for this relay.

## FORWARD: direct LocalSend transfer

Discovery must not be confused with TCP forwarding. Add only if equivalent rules are not already present:

```sh
uci batch <<'UCI'
set firewall.udr_tcp_wan_lan=rule
set firewall.udr_tcp_wan_lan.name='LocalSend upstream to LAN'
set firewall.udr_tcp_wan_lan.src='wan'
set firewall.udr_tcp_wan_lan.dest='lan'
set firewall.udr_tcp_wan_lan.src_ip='192.168.1.0/24'
set firewall.udr_tcp_wan_lan.dest_ip='192.168.8.0/24'
set firewall.udr_tcp_wan_lan.proto='tcp'
set firewall.udr_tcp_wan_lan.dest_port='53317'
set firewall.udr_tcp_wan_lan.family='ipv4'
set firewall.udr_tcp_wan_lan.target='ACCEPT'
set firewall.udr_tcp_lan_wan=rule
set firewall.udr_tcp_lan_wan.name='LocalSend LAN to upstream'
set firewall.udr_tcp_lan_wan.src='lan'
set firewall.udr_tcp_lan_wan.dest='wan'
set firewall.udr_tcp_lan_wan.src_ip='192.168.8.0/24'
set firewall.udr_tcp_lan_wan.dest_ip='192.168.1.0/24'
set firewall.udr_tcp_lan_wan.proto='tcp'
set firewall.udr_tcp_lan_wan.dest_port='53317'
set firewall.udr_tcp_lan_wan.family='ipv4'
set firewall.udr_tcp_lan_wan.target='ACCEPT'
UCI
```

Replies use existing conntrack established/related handling. LocalSend's configurable advertised TCP port may differ from the discovery UDP port; adjust the narrow rules accordingly. The relay never proxies file bytes.

## Check TCP SNAT

The packet-socket discovery transmitter preserves the UDP source even if WAN masquerade is on. Direct TCP registration must also retain the device IP. Capture the source of SYNs and registration requests on both sides. If it becomes the relay host's egress address, add a narrow exemption before fw4's SNAT zone jumps. This does not disable general WAN NAT:

```sh
cat > /etc/udp-discovery-relay-no-snat.nft <<'NFT'
ip saddr 192.168.8.0/24 ip daddr 192.168.1.0/24 tcp dport 53317 counter return comment "UDR direct LocalSend"
NFT
uci batch <<'UCI'
set firewall.udr_no_snat=include
set firewall.udr_no_snat.type='nftables'
set firewall.udr_no_snat.path='/etc/udp-discovery-relay-no-snat.nft'
set firewall.udr_no_snat.position='chain-pre'
set firewall.udr_no_snat.chain='srcnat'
UCI
```

`chain-pre`/`chain` are real [firewall4 include options](https://github.com/openwrt/firewall4/blob/master/root/usr/share/ucode/fw4.uc). Review `fw4 print` and run `fw4 check` before applying. This example assumes fw4 is the only IPv4 NAT manager. Additional NAT chains/VPN policies need separate inspection. Existing conntrack flows retain their previous NAT decision until closed/expired; restart the peer apps before retesting.

The optional `scripts/enable-router-firewall.sh` and `scripts/enable-localsend-no-snat.sh` implement this example topology, require explicit execution and are never called by package installation. Review their fixed subnet and zone values before using them; select only the changes needed by the actual network.

## Apply and undo

Back up `/etc/config/firewall` before editing. Named sections above are replaced on repeated runs, not appended. Inspect `uci changes firewall`, then:

```sh
fw4 check && uci commit firewall && /etc/init.d/firewall reload
```

Remove only this project's named sections to undo (Avahi/HomeKit rules are untouched):

```sh
for section in udr_input_wan udr_input_lan udr_tcp_wan_lan udr_tcp_lan_wan udr_no_snat; do
    uci -q delete "firewall.$section"
done
rm -f /etc/udp-discovery-relay-no-snat.nft
fw4 check && uci commit firewall && /etc/init.d/firewall reload
```

If validation fails before committing, restore the backup or revert only these pending changes. Do not flush the firewall or broadly accept WAN INPUT/FORWARD. In this example, the upstream gateway needs a return route to `192.168.8.0/24` via the relay host at `192.168.1.2`. Install return routes on the routers that forward replies; configure the SNAT exception on the router that would otherwise rewrite the source. Verify both directions before testing client transfers.
