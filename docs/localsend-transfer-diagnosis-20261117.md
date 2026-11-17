# Real-client transfer diagnosis — 2026-11-17

Read-only capture and conntrack evidence identified TCP SNAT after successful source-preserving UDP discovery:

- Mac 192.168.8.161 initiated TCP to 192.168.1.186:53317.
- On br-lan, source was 192.168.8.161:56304; on eth0 the same SYN was 192.168.1.211:56304.
- TCP handshake and encrypted application exchanges reached the real peer; conntrack confirmed the source-NAT mapping.
- The upstream peer subsequently attempted 192.168.1.211:53317 and received TCP RST, since the router has no LocalSend TCP server.
- Relayed UDP announcements from 192.168.1.186 were simultaneously observed on br-lan with their correct original source.

This is consistent with a LocalSend registration learning the NAT address rather than the Mac's real IP. It provides a concrete explanation for discovery without successful return file transfer. An unrelated LocalSend IPv6 link-local SYN was also observed on the Mac; it may represent an additional cached address, but its relation to the reported attempt is unconfirmed.

Proposed minimal fix (requires opt-in because the user previously declined firewall modification):

`ip saddr 192.168.8.0/24 ip daddr 192.168.1.0/24 tcp dport 53317 counter return`

Place it before fw4's zone SNAT jumps using a named chain-pre/srcnat include. No changes to INPUT, FORWARD, general WAN NAT, Avahi or IPv6. The reviewed implementation is scripts/enable-localsend-no-snat.sh; it rejects conflicting/pending configuration, backs up, validates with fw4 and restores on apply failure. It has not been applied at the time of preparing this plan.

After opt-in: verify original TCP source and reverse route by capture, restart both LocalSend applications to clear live peer addresses/connections, then retry actual file transfer in both directions. Do not interpret the NAT change alone as a passed transfer test.

User decision: “暂不修改，只提供诊断”. The proposed exception was not applied. The 55-second header-only capture ended automatically; no file contents were read and no application/firewall configuration was changed during diagnosis. The failure mechanism above is supported by real TCP source translation and refused connections to the router; actual HTTP register content was encrypted and was not decrypted, so its relationship is an inference from the connection sequence. Further issues (including any cached IPv6 address) are not ruled out until a corrected-path transfer is tested.
