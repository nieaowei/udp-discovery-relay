# UDP Discovery Relay

Rust IPv4 UDP 组播/广播发现中继，附独立的 GL.iNet SDK4 原生管理插件。首要支持 LocalSend，保留发送设备的 IP、UDP 源端口和应用载荷，设备直接建立 HTTP(S) 连接与传文件。

**当前源码版本为 0.0.1（OpenWrt 包版本 0.0.1-r1）。** 早期开发版 0.1.0-r2 已部署到真实 MT3600BE / GL.iNet 4.11.0，验证原生页面与源地址保留；用户随后通过配置跨子网免 NAT 确认 LocalSend 传输成功。新版本的 APK 由构建脚本或 CI 生成，实际构建环境与适用边界见 [安装与验收](docs/acceptance.md)。

[真机安装记录与待启用的防火墙项](docs/router-install-20261117.md)。

## 组件

- `src/`：独立 Rust Core；Linux `IP_PKTINFO` 接收、`AF_PACKET` 源地址保留发送、校验和、界面恢复、SIGHUP/SIGTERM、有限循环过滤、真实统计。无异步 runtime。
- `openwrt/udp-discovery-relay/`：UCI/procd、Lua UCI 适配器、标准 OpenWrt Rust 包 Makefile。
- `openwrt/gl-ui-udp-discovery-relay/`：独立 UI 包的可构建快照，包括页面、menu.d、SDK4 RPC 和 UI 测试。主实现同时交付于相邻独立仓库 `gl-ui-udp-discovery-relay`，通过 `scripts/sync-ui.sh` 同步快照。
- `tests/netns.py`：真实 Linux namespace/veth、UDP、TLS 指纹、注册与上传测试；不是官方 LocalSend 客户端。
- `.github/workflows/ci.yml`：fmt、Clippy、单元/UI/RPC测试、namespace、固定校验和 SDK、ARM64 musl、两个 APK、包事务测试、语义化版本和 Release 校验和。

IPv6 是独立、显式禁用模块。SSDP、mDNS、DNS、DHCP 端口受到保护，避免与 Avahi 等服务重叠。本版本支持以太网/bridge/VLAN 设备上的直接连接、互不重叠 IPv4 子网。远端路由、客户端防火墙与 TCP NAT 要按文档验证。

## 快速构建

稳定 Rust；不在路由器本地安装工具链。

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

macOS 可以执行配置校验与可移植单元测试，运行中继需要 Linux。Linux 集成测试需要 root、iproute2、nftables、Python 3、OpenSSL；CI 使用隔离 namespace，不改主机路由和防火墙：

```sh
sudo apt-get install iproute2 nftables lua5.1 lua-cjson libcap2-bin
sudo python3 tests/netns.py target/release/udp-discovery-relay
node --test openwrt/gl-ui-udp-discovery-relay/scripts/test-ui.cjs
lua5.1 tests/rpc.lua openwrt/gl-ui-udp-discovery-relay
```

Core 不依赖 UI。普通 Linux 使用 JSON：

```sh
cp tests/config.json relay.json
# 按实际 Linux 设备修改 devices；状态文件父目录需预先存在。
sudo target/release/udp-discovery-relay --config relay.json --status /run/udr-status.json
```

OpenWrt 默认使用 `/etc/config/udp_discovery_relay`，逻辑网络由 `ubus network.interface.<name> status` 的真实 `l3_device` 动态解析；每 5 秒恢复网络变动，不能解析的规则会报告错误并重试。`--uci --check` 校验 UCI，`--config FILE --check` 校验 JSON。规则字段与默认值可见 `tests/config.json`、`src/config.rs` 与默认 UCI 文件。单向规则使用不重叠 `input`/`output` 逻辑网络列表及 `direction=oneway`；双向使用 `network`，普通 Linux 双向使用 `devices`。

## APK 与 SDK

### 静态 Core + 官方 SDK 打包

使用匹配设备的 SDK 优先；当前参考 CI SDK 固定为官方 25.12.6 Filogic，不混用动态库或固件源。先核验 SDK 下载 SHA-256，然后在 Linux 执行：

```sh
rustup target add aarch64-unknown-linux-musl
UDR_SDK_LABEL='your audited SDK revision' scripts/build-static.sh /path/to/sdk
UDR_APK_ARCH=aarch64_cortex-a53 scripts/build-apk.sh /path/to/sdk
scripts/test-apk.sh /path/to/sdk
```

脚本使用 SDK GCC、musl libc/crt 和 SDK 的真实 `apk mkpkg`，输出 APK v3，不是重命名二进制。Rust 匹配版本的静态 `libunwind` 仅作为编译器支持归档；ELF 不依赖动态 loader。未签名 APK 附 SHA-256、GPL 许可证与构建来源记录。正式分发应通过你信任的签名 feed。APK 安装验证、依赖范围和路由器命令见 [acceptance.md](docs/acceptance.md)。

### 标准 OpenWrt feed 构建

SDK 必须已安装与其版本匹配的 packages feed，含 `lang/rust`。`scripts/prepare-feed.sh` 生成锁定的本地源码归档并把 Core 配方放到该 feed；拒绝覆盖已有包。接着：

```sh
scripts/prepare-feed.sh /path/to/sdk
cd /path/to/sdk
./scripts/feeds install udp-discovery-relay
make defconfig
make package/udp-discovery-relay/compile V=s
```

Core 的 Makefile 使用 OpenWrt 官方 `rust/host` / `rust-package.mk` 机制。此路径默认按 SDK 策略链接，因此要求匹配固件 ABI，不能将它与静态产物的兼容性边界混为一谈。UI 的 Makefile 可单独复制到 SDK `package/gl-ui-udp-discovery-relay` 后构建；厂商 SDK4 依赖必须在固件中真实存在。UI 卸载不会停止 Core。

## 协议、安全与验收

- [协议研究与关键技术选择](docs/protocol.md)：注册目标、TLS 指纹、IP_HDRINCL 与 NAT 的问题、AF_PACKET 方案和权限限制。
- [精确防火墙建议与撤销](docs/firewall.md)：INPUT 与 FORWARD 的区别；无自动修改，不触碰 Avahi/HomeKit。
- [MT3600BE 安装、抓包、真机验收](docs/acceptance.md)：全部硬件验收步骤和未验证边界。
- [实际测试报告](docs/test-results.md)：执行环境、资源实测、通过项目与未执行项目。

UI 使用参考 `gl-ui-avahi-daemon` 的真实扩展机制：`/usr/share/oui/menu.d/`、`/www/views/gl-sdk4-ui-*.common.js`、`/usr/lib/oui-httpd/rpc/`、`gl-validator.d`、`window.$rpcRequest('call', ['sid', ...])`，以及 `oui.ubus` 的 procd 查询。未猜测新私有 API。权限由已有 SDK4 会话分派层执行；必须按验收文档确认当前固件会拒绝未认证调用。写入前由同一个 Rust 校验器检查，带 revision 并发保护、排他锁与原子替换。前端只能选择固定服务操作，没有任意命令或路径接口。

UI 提供中英文、主题变量、响应式规则编辑、真实统计、有限日志、服务和开机管理。开发预览会明确标记 fixture，并只近似替代 GL 组件用于布局复核；预览代码不进入安装包。

版本采用 `MAJOR.MINOR.PATCH`，Release tag `v0.0.1` 必须同时匹配 Cargo、Core/UI Makefile 与 UI package.json。推送版本 tag 才触发发布；普通分支提交只运行测试与构建，不创建 GitHub Release。

GPL-3.0-only；见 [LICENSE](LICENSE)。
