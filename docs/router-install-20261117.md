# MT3600BE installation — 2026-11-17

Installed on the user's actual GL-MT3600BE at 192.168.8.1, GL.iNet 4.11.0 CN, OpenWrt 25.12-SNAPSHOT r32295+925-c25265953b / Linux 6.12.94.

- Core and UI installed as 0.1.0-r2, using the existing firmware dependencies.
- `apk --print-arch` reports the apk-tools compile architecture `aarch64`; actual package acceptance is `/etc/apk/arch = aarch64_cortex-a53`. The initial r1 architecture simulation was rejected before installation; r2 corrects package metadata and is accepted without forcing architecture or changing system feeds.
- Core running under procd and enabled at boot. LocalSend bidirectional rule active on logical wan/lan, resolved to eth0 192.168.1.211 / br-lan 192.168.8.1.
- Real hardware VmRSS 503,808 bytes at initial inspection, effective/permitted capabilities only CAP_NET_RAW, NoNewPrivs=1; no recent errors.
- UCI validation, procd status, init reload and nginx configuration check succeeded.
- Native Applications menu, overview, rule editor and log page loaded successfully after login in the actual GL.iNet UI. Invalid-session RPC request was rejected with Access denied (-32000).
- Real diagnostic multicast from the Mac (192.168.8.161:63670) captured on eth0 as 192.168.8.161:63670 to 224.0.0.167:53317, TTL 1, UDP checksum correct. This verifies real LAN-to-WAN source-preserving discovery transport, including with current WAN masquerade enabled. It does not verify official LocalSend app file transfers.
- SHA256 of /etc/config/firewall, /etc/config/network and /etc/avahi/avahi-daemon.conf matched the preinstall baseline. Avahi remained running.

## Pending opt-in firewall changes

Current WAN INPUT drops UDP 53317. Existing FORWARD rules already allow the two private networks, so no new general forwarding rule is needed. Current srcnat_wan masquerades IPv4 TCP, which can change the registering device's IP.

Prepared, **not automatically applied**: `scripts/enable-router-firewall.sh`.

1. INPUT ACCEPT only from WAN zone / 192.168.1.0/24 to multicast 224.0.0.167, UDP destination port 53317, IPv4.
2. SNAT exemption only from 192.168.8.0/24 to 192.168.1.0/24, TCP destination port 53317. General WAN NAT remains enabled.

The script rejects preexisting conflicting sections or pending edits, backs up the firewall configuration, validates with fw4 before reloading, and restores on reload failure. It does not edit Avahi, IPv6 or general WAN exposure. Firewall changes require explicit user opt-in because the goal objective mandates automatic modification be off by default.

Official LocalSend client transfers, router reboot persistence and physical DHCP-renewal acceptance remain unexecuted.

## User decision

The user explicitly selected “只保留安装，暂不改防火墙”. No firewall rules or NAT exceptions were applied. The agent applied no firewall changes. During the earlier checks, firewall, network and Avahi hashes matched the preinstall baseline and the original Avahi PID remained running. At the final check, the firewall file had been separately updated at 11:19 and the live WAN chain contained new GL- TCP/UDP 53317 accepts; these were preserved. Neither firewall.udr_input_wan nor firewall.udr_no_snat nor /etc/udp-discovery-relay-no-snat.nft existed, confirming the prepared opt-in plan was not applied. No attribution is made for the separate changes. Final daemon counters confirmed the single diagnostic packet was received and forwarded with zero drops/errors. Official-client registration/file transfer and the effect of existing TCP SNAT still require verification.
