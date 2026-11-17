# 测试指南

## 本地检查

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
node --test openwrt/gl-ui-udp-discovery-relay/scripts/test-ui.cjs
lua5.1 tests/rpc.lua openwrt/gl-ui-udp-discovery-relay
lua5.1 tests/config-adapter.lua /tmp/udr-empty-config.json
cargo run --locked -- --config /tmp/udr-empty-config.json --check
python3 scripts/check-version.py
```

Rust 单元测试覆盖配置、IPv4/UDP 校验和与边界、重复包缓存、接口子网计算、统计错误上限及禁用的 IPv6 模块。Linux 才编译的平台模块需在 Linux 检查；其他主机可执行可移植模块测试。

JavaScript 测试覆盖配置与 revision、规则操作、固定 SDK4 请求、错误处理、统计缺失、日志和多语言。Lua 测试执行真实适配器代码，但模拟 UCI/SDK4 边界；它们不能替代目标系统的登录权限和服务生命周期验收。

## Linux 网络集成

需要 Python 3、OpenSSL、iproute2、nftables 和创建网络命名空间的 root 权限：

```sh
cargo build --release --locked
sudo python3 tests/netns.py target/release/udp-discovery-relay
```

测试在独立命名空间中创建两个 IPv4 子网、veth 和 Linux bridge，使用实际二进制与真实 socket，不改变主机的路由或防火墙。覆盖：

- 双向 multicast/broadcast、源 IP/端口与载荷保留。
- HTTP/HTTPS 注册、证书指纹比较、实际上传字节与哈希。
- 活跃 masquerade 下的 UDP 来源、重复报文与多客户端。
- 包大小限制、单向规则、逻辑网络解析和单播过滤。
- SIGHUP 校验、全局禁用、SIGTERM、停止与重启。
- 接口 down/up、地址变化、删除重建和 bridge 出入路径。
- CAP_NET_RAW、no_new_privs、真实计数与资源采样。

客户端是协议测试夹具。正式 LocalSend 客户端、实际交换网络、系统防火墙及 Wi-Fi 策略仍需按 [验收指南](acceptance.md) 验证。

## APK 事务

在具备匹配 SDK 的 Linux 主机上：

```sh
sudo env UDR_APK_ARCH=aarch64_cortex-a53 scripts/build-apk.sh /path/to/sdk
scripts/test-apk.sh /path/to/sdk
```

打包前需要先构建对应二进制。事务测试检查 Core/UI 文件所有权，使用独立临时根目录和测试依赖能力，验证安装、单独卸载 UI、升级保留用户配置和移除 Core。普通用户使用 usermode 数据库，不执行固件服务脚本；实际 procd/nginx 行为需要部署验收。

## 结果与性能

网络测试将结果写入 `.test-output/integration.json`、`performance.json` 和 `relay.log`。GitHub Actions 上传当次结果为 `test-results`，构建产物为 `openwrt-arm64-apks`。以具体提交对应的记录为准，不在源码中保存特定设备或旧版本的结果快照。

性能记录包含二进制字节数、`/proc` 报告的 RSS 和十秒空闲 CPU 采样。CPU 使用进程 ticks 计算，有内核计时粒度限制。记录测试架构、构建选项、规则数量和流量条件，再比较结果；短时空闲读数不代表峰值负载。

实际环境可结合服务统计、`/proc/<pid>/status`、CPU 采样与抓包检查；单个转发计数不能确认客户端注册、返回路由或文件传输成功。
