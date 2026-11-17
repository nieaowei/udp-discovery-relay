# Test results — 2026-11-17

后续更新：已完成真实设备安装及原生 UI / LAN→WAN 抓包检查，详见 [真机安装记录](router-install-20261117.md)。下文保留安装前的隔离测试记录；r2 已修正 APK 架构为 `/etc/apk/arch` 接受的 `aarch64_cortex-a53`，并重新通过 APK 事务测试。

## Environment and artifact

Host: macOS ARM64. Linux execution: privileged, isolated Docker container on ARM64, Debian bookworm userspace, network namespaces/veth, Linux bridge and nftables. No real router was accessed or changed.

Final executable: Rust stable 1.100.0, AArch64 Cortex-A53, static musl. Linked with the read-only local OpenWrt 25.12.6 r33311-981c978eff GCC 14.3.0 toolchain; Rust's corresponding compiler-support `libunwind.a` is static. `readelf` verified AArch64 and absence of a dynamic interpreter. Official SDK apk-tools 3.0.5 generated and verified both real APKv3 packages. This SDK is a reference build environment, **not** the GL.iNet vendor SDK for the supplied snapshot.

| Final static binary measurement | Observed |
|---|---:|
| ELF file size | 528,520 bytes (516.13 KiB) |
| VmRSS sample | 630,784 bytes (616 KiB) |
| Idle CPU over 10 seconds | 0.0% (process ticks) |
| Configured active test rules | 2 |

RSS/CPU are Linux container measurements, not estimates and not MT3600BE benchmarks. CPU sampling has kernel tick resolution; 0.0% does not mean the process never wakes. It uses blocking poll with a one-second maximum timeout and five-second maintenance.

## Executed successfully

- `cargo fmt --check`.
- `cargo clippy --all-targets --locked -- -D warnings` on Linux and macOS.
- `cargo test --locked`: six Linux module tests (config, checksum/wire bounds, bounded dedup cache, interface/subnet arithmetic, metrics error ring, disabled IPv6 module); five portable tests on macOS earlier in development.
- Six JavaScript UI/controller tests: real settings/revision flow, add/delete rules, fixed authenticated SDK4 request shape, actual error propagation, missing/stale metrics, bilingual labels, logs and confirmation cancellation.
- Production Lua syntax checks for RPC, validator and core UCI adapter.
- UCI adapter suite: strict booleans/integers, lists, quote escaping and empty-array JSON; its empty config passed the real Rust `--check` command.
- Production RPC adapter suite against mocked SDK/OpenWrt boundaries: successful write/apply, partial writes, sync failure, Rust validator failure, old revisions, concurrent external changes, lock cleanup, injected network/action rejection, fixed command argv, stale metrics PID rejection and bounded logs. This is **not** live SDK authentication validation.
- Shell and workflow YAML syntax; version agreement (`0.1.0`, `v0.1.0`).
- Official SDK GCC static musl build, APKv3 generation and `apk verify` for both packages.
- Real APK manager transactions in a disposable root with explicitly simulated firmware dependency capabilities: install both, remove UI while preserving Core, upgrade Core with a changed packaged default while preserving user-modified UCI (replacement became `.apk-new`), remove Core. Hooks were deliberately not executed in this fake firmware root; actual procd/nginx lifecycle still needs hardware acceptance.

## Fifteen final namespace checks

The **final static release executable** was used, not a mocked relay. Each test creates its own three namespaces; host routes/firewall are unchanged.

1. Effective capabilities contain only CAP_NET_RAW and `no_new_privs=1`.
2. Bidirectional multicast preserves source IP and UDP source port, performs HTTPS certificate fingerprint comparison, posts to register/prepare-upload/upload, and checks actual uploaded byte hashes.
3. Bidirectional unencrypted HTTP registration/upload succeeds at the advertised alternate TCP port.
4. Duplicate burst is coalesced without endless forwarding.
5. Different source devices with identical payloads both arrive.
6. UDP source remains unchanged with active multicast masquerade rules.
7. Oversized UDP is dropped; routed unicast is delivered once and never reflected.
8. Broadcast is forwarded with its original source endpoint to the egress broadcast domain.
9. Invalid SIGHUP preserves the previous configuration; disabling globally stops discovery and reports zero active rules; reenabling restores discovery.
10. Logical network names are resolved through the actual Rust ubus adapter against a fixture executable; one-way rules block reverse forwarding.
11. Interface down/up and address changes recover.
12. Deleting and recreating the veth with a new ifindex recovers.
13. SIGTERM shuts down cleanly; stopped relay no longer discovers; restart restores it.
14. Linux `br-lan` bridge receive/transmit preserves the source in both directions.
15. Daemon counters/RSS and idle CPU are measured from the running process.

The peers are Python **protocol fixtures**, not official LocalSend app builds. They use real sockets, HTTP/TLS, independent endpoint certificates and actual uploaded bytes. No proxy is involved. Positive fixture results do not replace official-client testing.

Raw final evidence: [integration.json](test-results/integration.json), [performance.json](test-results/performance.json), [relay.log](test-results/relay.log). Recent errors in the snapshot are expected historical entries from deliberate interface fault injection; active rules recovered to two. Earlier test development caught a musl `msghdr` field-width mismatch, the need for compiler-support libunwind when using the SDK linker, and a bridge test that transmitted before the documented five-second recovery interval. These were corrected before this final successful run.

## Not executed

- Real GL-MT3600BE firmware 4.11.0 installation, vendor SDK build or APK source/signature integration.
- Native GL runtime rendering, actual RPC login/authorization rejection and procd/nginx lifecycle on that firmware.
- Official LocalSend app discovery and file transfers on physical devices.
- Physical WAN DHCP renewal, router reboot and Avahi/AirPlay/HomeKit coexistence.
- GitHub Actions on a remote repository, release/tag publication or package signing. Workflows are supplied locally.

[acceptance.md](acceptance.md) supplies exact installation, packet capture, permission, transfer, recovery and coexistence checks. [firewall.md](firewall.md) supplies narrow, reversible INPUT/FORWARD/TCP NAT examples. No unexecuted hardware test is represented as passed.

## Layout preview

Chinese dark/mobile layout was reviewed in a local Vue fixture at 390px viewport (375px content width, no horizontal overflow). It approximates the unavailable GL components, clearly identifies its fixture data, and is excluded from APK installation. English labels and controller behavior were checked in the JavaScript suite. See [preview image](images/mobile-dark-preview.jpg); this is not a router screenshot.
