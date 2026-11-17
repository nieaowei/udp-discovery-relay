# Source preservation and LocalSend

Primary references:

- [Official LocalSend protocol v2.2](https://github.com/localsend/protocol)
- [Official discovery implementation](https://github.com/localsend/localsend/blob/main/packages/core/src/discovery/mod.rs), especially `answer_announcement` and `register_peer`.
- [Official multicast implementation](https://github.com/localsend/localsend/tree/main/packages/core/src/multicast)
- Linux [ip(7)](https://man7.org/linux/man-pages/man7/ip.7.html), [raw(7)](https://man7.org/linux/man-pages/man7/raw.7.html), [packet(7)](https://man7.org/linux/man-pages/man7/packet.7.html).

An announcement advertises alias, version, fingerprint, listening port, HTTP/HTTPS and the announcement flag. The receiver learns the host address from the packet's source. A response to an announcement first attempts the HTTP registration endpoint; multicast with the response flag is a fallback. A response must not trigger an endless response exchange. The relay leaves this application behavior to clients and copies every UDP payload unchanged.

For encrypted discovery, the certificate identity matters. The official discovery code uses the announced certificate fingerprint when contacting a peer and derives peer identity from TLS in encrypted mode. A router-address UDP forwarder sends registration to the router, which cannot present the device's identity. Adding an address field to arbitrary JSON does not fix the official client.

## Receive and transmit choices

The daemon receives UDP through Linux sockets with `IP_PKTINFO`, obtaining the actual destination and ingress ifindex. It joins only configured IPv4 multicast groups on the resolved devices. It accepts only matching multicast/broadcast traffic from an on-link, non-router source. Unicast traffic is not relayed. A rule's UDP listening port and the sender's original UDP source port are distinct and both preserved correctly.

`IP_MULTICAST_IF` selects an interface, but does not make a normal UDP socket preserve a foreign source. `IP_HDRINCL` on an IPv4 raw socket permits an explicit source and complete header. However, the IPv4 POSTROUTING hook can subsequently apply WAN masquerade.

The relay transmits complete IP/UDP packets through `AF_PACKET/SOCK_DGRAM`, using the selected ifindex and the deterministic Ethernet multicast or broadcast destination. The kernel builds the Ethernet header. This avoids IPv4 OUTPUT/POSTROUTING SNAT of the discovery packet. No ARP forgery or neighbor-table change is required. IPv4 and UDP checksums are constructed in Rust, including the UDP pseudoheader and zero-checksum mapping. Packet TTL is 1 and DF is set. Egress MTU and configured payload limits are checked before transmission.

This is limited to Ethernet-like devices (including bridges and VLAN interfaces). Tunnel and point-to-point devices, overlapping subnets, down devices and missing IPv4 addresses produce explicit errors. It is a deliberate local-link discovery reflector, not a multicast routing daemon. Discovery send traffic bypasses IPv4 OUTPUT filtering; configure narrow relay rules accordingly. Bridge/netdev filtering, hardware or Wi-Fi multicast policy can still prevent delivery.

## Return paths and unsupported topology

Clients still exchange HTTP(S) directly. Both networks need routes to each other, host firewalls permitting LocalSend and precise router FORWARD permission. A TCP connection that can reach a peer through NAT is not proof of transparent bidirectional registration: the receiving peer may learn the NAT address as the registering device. Exempt only inter-subnet LocalSend TCP from SNAT when necessary; see [firewall.md](firewall.md).

The daemon detects missing/down/overlapping local interfaces and unsupported L2 devices. It cannot infer remote host firewall rules, upstream routes or whether a remote certificate belongs to a particular installed app. `scripts/diagnose.sh` reports the available local network, routing, firewall and daemon evidence; packet capture and actual-client acceptance are required to establish those external facts. Do not interpret a positive forwarded counter as successful file transfer.

## Loops, resources and privilege

Only sources in the ingress subnet are eligible; a relayed source arriving on another subnet is rejected. Local router addresses are rejected. A keyed hash cache also coalesces identical sender/destination/port/payload packets for 750 ms. It contains at most 4096 entries, cannot grow with payload size, and does not refresh an entry on duplicates. Disabling the cache does not disable subnet/source eligibility checks. One reflector per connected set of networks is recommended. Identical legitimate retransmissions within the cache window are intentionally coalesced.

A single blocking `poll` event loop processes bounded batches of 64 packets, handles signals within approximately one second, and checks interface state every five seconds. No async runtime or worker pool is needed. Buffers are reused. Payloads are untrusted and are never parsed as LocalSend JSON by the daemon. The error ring contains at most 16 entries; repeated error logging is rate limited, while counters continue to reflect drops. Status snapshots replace one bounded JSON file in tmpfs every five seconds.

Startup uses root for socket creation and access to root-owned OpenWrt configuration. After setup, the daemon retains only `CAP_NET_RAW`, locks root capability re-acquisition, and sets `no_new_privs`. It does not retain `CAP_NET_ADMIN`. UID 0 is retained for UCI/netifd access; this is not full UID isolation. `--check` creates no network sockets and needs no network capabilities. The UI never supplies executable names or shell source. IPv6 is an explicitly disabled, separate module; no IPv6 memberships or sysctls are changed.
