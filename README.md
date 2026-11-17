# UDP Discovery Relay

通用 Linux IPv4 UDP 组播与广播发现中继，使用 Rust 实现。保留发送端 IP、UDP 源端口和应用载荷，支持 LocalSend 及自定义 UDP 发现规则。服务注册与文件传输由客户端直接完成。

当前版本：**0.0.1**。Core 可独立运行；OpenWrt 软件包和 GL.iNet SDK4 原生管理插件是独立的集成组件。

## 功能与运行要求

- 多条规则、多接口、双向中继；OpenWrt 逻辑网络还支持独立输入和输出的单向规则。
- `IP_PKTINFO` 识别入接口，`AF_PACKET` 在出接口发送，保留发现报文的真实来源。
- 有限重复包缓存、载荷与 MTU 检查、日志级别、真实报文计数和内存统计。
- SIGTERM 退出、SIGHUP 重载、接口状态变化后自动恢复。
- 单线程阻塞事件循环，无大型异步运行时、数据库或额外后台服务。

运行中继需要 Linux、以太网类接口（包括 bridge/VLAN）以及互不重叠的直连 IPv4 子网。IPv6 尚未启用。DNS、DHCP、SSDP 和 mDNS 的相关端口受到保护，避免与已有发现服务重叠。

Core 不绑定路由器品牌或型号。普通 Linux 可通过 JSON 配置运行；OpenWrt 可使用 UCI/procd。其他 CPU 架构需要相应构建和验证，不能直接复用不匹配的二进制或 APK。

## 构建与独立运行

使用稳定版 Rust：

```sh
cargo build --release --locked
cp examples/relay.json relay.json
```

将 `relay.json` 中的 `devices` 改为实际 Linux 接口名，然后在 Linux 上运行：

```sh
sudo target/release/udp-discovery-relay --config relay.json --status /run/udr-status.json
```

状态文件父目录需预先存在。`--config relay.json --check` 仅校验配置，不创建网络 socket；macOS 也可执行配置校验和可移植单元测试。

默认 LocalSend 预设使用 `224.0.0.167:53317`。广播规则使用 `255.255.255.255`，程序会按出接口转换为对应广播地址。详细字段与校验限制见 [配置定义](src/config.rs)。

## OpenWrt 集成

`openwrt/udp-discovery-relay` 提供 UCI 配置、procd 服务及标准软件包配方。默认使用逻辑网络 `wan`、`lan`，通过 netifd 的 `l3_device` 动态解析 Linux 接口；部署时应调整为实际网络名称。

```sh
/usr/bin/udp-discovery-relay --uci --check
/etc/init.d/udp-discovery-relay enable
/etc/init.d/udp-discovery-relay start
```

配置位于 `/etc/config/udp_discovery_relay`。单向规则使用 `direction=oneway` 和不重叠的 `input`、`output` 逻辑网络列表；双向规则使用 `network` 列表。普通 Linux 独立配置中的 `devices` 用于双向规则。

标准 feed 构建使用 OpenWrt 官方 `rust/host` 和 `rust-package.mk`，要求 SDK 已配置与其版本匹配、包含 `lang/rust` 的 packages feed：

```sh
scripts/prepare-feed.sh /path/to/sdk
cd /path/to/sdk
./scripts/feeds install udp-discovery-relay
make defconfig
make package/udp-discovery-relay/compile V=s
```

当前 CI 与静态构建脚本提供 **ARM64 Cortex-A53 / musl 参考产物**，固定使用 OpenWrt 25.12.6 Mediatek Filogic SDK 并验证下载校验和。这是参考构建目标，Core 的源码不依赖该硬件平台。使用其他目标时，应按匹配的 SDK、工具链和软件包架构重新构建。

```sh
rustup target add aarch64-unknown-linux-musl
UDR_SDK_LABEL='your SDK revision' scripts/build-static.sh /path/to/sdk
sudo env UDR_APK_ARCH=aarch64_cortex-a53 scripts/build-apk.sh /path/to/sdk
scripts/test-apk.sh /path/to/sdk
```

APK 打包必须记录 root 文件所有权。CI 仅对打包步骤使用 sudo；安装事务测试在独立临时根目录使用 usermode 数据库。普通用户可尝试 fakeroot，脚本会拒绝不兼容 SDK 包装器生成的错误文件所有权。

静态产物使用 SDK 的 musl libc/crt 与 Rust 对应的编译器支持归档，没有动态 ELF interpreter。构建来源写入 `build-provenance.txt`；这不替代目标系统的架构、依赖和兼容性检查。APK v3 由 SDK 的 `apk mkpkg` 生成，附校验和。

## GL.iNet SDK4 原生管理插件

[gl-ui-udp-discovery-relay](https://github.com/nieaowei/gl-ui-udp-discovery-relay) 提供中英文、主题适配、响应式规则编辑、真实统计、服务管理和有限日志。页面使用 SDK4 的 Vue/GL 组件与已认证 RPC 分派，配置写入经过 Core 校验、并发 revision 检查和原子替换。

UI 的源码、OpenWrt 包配方、测试与 APK 发布均由独立 UI 仓库维护。Core 仓库不包含 UI 快照，构建、版本检查和安装脚本只处理 Core。可从 UI 仓库单独构建或安装管理插件；卸载 UI 不会停止 Core。

## 网络配置与验证

UDP 发现、TCP 注册和文件传输需要分别验证。参与子网必须双向可达；LocalSend 注册与回连还需要正确的真实源地址。静态路由解决可达性，必要的免 NAT 规则保留客户端来源。软件包不会自动修改防火墙或路由。

- [安装与验收](docs/acceptance.md)：Linux/OpenWrt 部署、客户端传输、恢复和 UI 验证。
- [防火墙与免 NAT](docs/firewall.md)：示例拓扑中的 INPUT、FORWARD、SNAT 配置与撤销。
- [协议与实现](docs/protocol.md)：LocalSend 注册、TLS 指纹、源地址保留和权限边界。
- [测试指南](docs/testing.md)：本地检查、网络集成、APK 事务和性能测量。

Core 的 GitHub Actions 执行格式、Clippy、单元/UCI、网络命名空间、ARM64 编译与 Core APK 事务检查，并上传当次的测试记录和安装包。UI 检查与打包在独立仓库运行。`v0.0.1` 形式的版本 tag 才触发 Release；分支提交只测试与构建。

GPL-3.0-only；见 [LICENSE](LICENSE)。
