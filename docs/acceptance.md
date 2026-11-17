# MT3600BE 安装与真机验收

## 已知目标与产物边界

用户提供的设备信息：GL-MT3600BE，固件 4.11.0，OpenWrt `25.12-SNAPSHOT r32295+925-c25265953b`，Linux `6.12.94`，`mediatek/filogic`，`DISTRIB_ARCH=aarch64_cortex-a53`，`apk --print-arch=aarch64`。

本次本机构建使用已有、只读挂载的 OpenWrt **25.12.6 r33311-981c978eff** 工具链，GCC 14.3.0，Rust stable 1.100.0。Core 为 Cortex-A53、静态 musl ARM64 可执行文件，没有动态 ELF interpreter；不导入该 SDK 的共享库，不安装其他版本的库、不更换固件软件源、不包含内核模块。APK 使用同一 SDK 的官方 apk-tools 3.0.5 打包，Core 包架构为 `/etc/apk/arch` 接受的 `aarch64_cortex-a53`（`apk --print-arch=aarch64` 仅表示工具自身编译架构），UI 为 `noarch`。这减少用户态 ABI 耦合，但**不是厂商 SDK 与目标固件完全匹配的证明**。

已完成真实路由器安装、原生菜单/状态/规则/日志加载、无效 sid 拒绝、procd 运行与 reload，以及真实 LAN→WAN 源地址抓包验证。见 [安装记录](router-install-20261117.md)。开机恢复、正式 LocalSend 客户端传输、物理 DHCP 续租和 AirPlay/HomeKit 共存仍待验收。不要把协议测试夹具当成官方 LocalSend 客户端。

## 安装

下面的示例针对当前源码 0.0.1-r1。历史真机安装记录为开发版 0.1.0-r2，保留于安装记录中；变更源码版本不会自动更新路由器已安装的软件。

从 `dist/` 复制两个 APK、各自的 `.sha256` 和 `build-provenance.txt` 到路由器 `/tmp/udr/`。保留原有固件软件源，先确认 `lua`、`lua-cjson`、`libuci-lua`、`ubus`、`gl-sdk4-ui-core`、`gl-oui-rpc`、`luci-lib-nixio` 可由当前固件满足。

```sh
cd /tmp/udr
sha256sum -c udp-discovery-relay-0.0.1-r1.apk.sha256
sha256sum -c gl-ui-udp-discovery-relay-0.0.1-r1.apk.sha256
cat build-provenance.txt
apk add --simulate --allow-untrusted ./udp-discovery-relay-0.0.1-r1.apk ./gl-ui-udp-discovery-relay-0.0.1-r1.apk
# 审核模拟安装结果后实际安装。本次包未签名，allow-untrusted 仅针对这两个已校验本地文件。
apk add --allow-untrusted ./udp-discovery-relay-0.0.1-r1.apk ./gl-ui-udp-discovery-relay-0.0.1-r1.apk
/usr/bin/udp-discovery-relay --uci --check
/etc/init.d/udp-discovery-relay status
cat /var/run/udp-discovery-relay/status.json
```

若遇到架构或依赖冲突，停止安装并使用匹配的厂商 SDK/软件源重新构建；不要强制覆盖系统依赖。Core 可单独安装。卸载 UI 只删除菜单、页面与 RPC；Core 保持运行。APK 的 `/etc` 配置保护和 OpenWrt conffiles 元数据保留用户配置；升级时出现 `.apk-new` 应人工比较。

默认配置启用 LocalSend、逻辑网络 `wan` 与 `lan`。如网络名不同，先编辑 `/etc/config/udp_discovery_relay`。按 [防火墙说明](firewall.md) 审核精确的 INPUT/FORWARD 与必要 TCP SNAT 豁免，再执行：

```sh
/etc/init.d/udp-discovery-relay enable
/etc/init.d/udp-discovery-relay restart
logread -e udp-discovery-relay -l 50
```

原生管理界面：应用程序 → UDP Discovery Relay。硬刷新浏览器；页面没有外部 CDN、iframe 或 LuCI 页面依赖。

## 抓包与真实客户端

准备主路由侧 A=`192.168.1.X`，副路由侧 B=`192.168.8.Y`。先确认双方使用默认 LocalSend 组播地址和 53317 端口，允许操作系统防火墙中的 LocalSend 通信。Linux/macOS/Windows 终端、手机的 LocalSend 都应使用正式版本。开启 HTTPS，避免为了测试关闭证书校验。

在路由器两个 SSH 窗口抓包（先使用 `ubus call network.interface.wan status` 与 `lan status` 确认设备名）：

```sh
tcpdump -ni eth0 -vv 'udp port 53317 or tcp port 53317'
tcpdump -ni br-lan -vv 'udp port 53317 or tcp port 53317'
```

1. A/B 全部退出后重开 LocalSend。两侧都应出现对方。br-lan 上由 A 中继来的 UDP 源必须为 A 的真实 IP；eth0 上由 B 中继来的 UDP 源必须为 B 的真实 IP，不能为路由器地址。UDP 源端口也应一致。
2. 分别 A→B 和 B→A 发送实际文件；接收后比较 SHA-256。双方设备详细信息必须显示真实对端地址。仅看到名称不算通过。HTTPS 流量无法直接查看注册路径，可使用 LocalSend 调试日志核对 `/api/localsend/v2/register`；自动化夹具已在真实 TCP/TLS 连接上验证该路径。
3. 同时启动三台以上设备，抓包 60 秒，应为有限的发现/响应报文，不应持续增长；查看 `loop_filtered`、`dropped` 与 RSS。
4. 停止中继，再关闭并重新启动客户端或清除客户端发现缓存；跨子网的**新 UDP 发现**应停止。现有列表、手动指定 IP、已知设备的 HTTP 探测和已有 TCP 传输可能继续正常，这是协议行为。
5. 重启中继并重新发起发现，两向均恢复。分别关闭规则、关闭全局启用、停止服务，验证三种开关的区别。
6. 在保留本地管理通道的条件下进行 WAN DHCP 续租、接口断开与恢复；约 5 秒内重新建立规则。新 IP、设备/ifindex 应出现在统计中。不要在唯一远程 SSH 通道上盲目关闭 WAN。
7. 测试 UI 所有控制项：新增/编辑/删除/启停规则、单向方向、日志级别、保存、应用、恢复默认、服务启停重启与开机开关。无效端口/IP必须返回错误，不能破坏旧配置；两个浏览器同时编辑应拒绝旧 revision 保存。
8. 检查日志刷新、暂停、错误过滤、清空显示、导出；错误/空状态应有文字，统计缺失时显示 `—`，没有生成假计数。
9. 浏览器开发工具中查看 `/rpc` 请求，确认使用已登录 SDK4 会话。用无 Cookie、无有效 sid 的请求调用模块，必须被固件拒绝；前端隐藏按钮不能替代后端权限。测试请求示例：

```sh
curl -i -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"call","params":["invalid-session","udp-discovery-relay","service",{"action":"stop"}]}' \
  http://192.168.8.1/rpc
```

拒绝可能以 SDK4 JSON 错误返回，不一定是 HTTP 401；同时确认服务状态未变化。如固件允许该请求，禁用/卸载 UI 并检查其 RPC 会话分派配置，不能公开部署该模块。

10. 在测试前后比较 `uci show avahi`（如存在）、Avahi 配置与防火墙规则；使用 AirPlay/HomeKit 实际连接并抓包确认 5353 正常。此服务不创建 5353 或 IPv6 socket，不修改 Avahi、DHCP、IPv6、转发 sysctl。
11. 卸载 UI 后 Core 仍可发现和传文件；重新安装 UI。最后重启路由器，检查配置、开机启用状态、procd 进程、两向发现与文件哈希。

## 配置/服务维护

```sh
uci show udp_discovery_relay
/etc/init.d/udp-discovery-relay reload
/etc/init.d/udp-discovery-relay restart
/etc/init.d/udp-discovery-relay disable
/etc/init.d/udp-discovery-relay stop
```

日志由 logd 的系统环形缓冲管理，UI 每次仅取最近 200 条/64 KiB；没有文件日志无限增长。统计文件在 tmpfs，每次覆盖，最多 16 条最近错误。保存不自动启动已停止服务；应用只向运行实例重新加载，停止服务时请显式点击启动。开机启动与全局中继开关分别控制进程自启动和报文中继。

卸载与防火墙撤销：

```sh
apk del gl-ui-udp-discovery-relay
# Core 独立运行，确认不再需要中继后才删除：
apk del udp-discovery-relay
```

按 firewall.md 删除本项目的命名规则即可撤销手工添加的防火墙项；卸载不会修改这些由用户创建的规则，也不会删除其他插件配置。

也可将 `scripts/install-router.sh` 复制到路由器，在审核构建来源和防火墙计划后运行 `sh install-router.sh /tmp/udr`。脚本验证 ARM64 与两个本地 APK 校验和、执行模拟和实际安装，不拉取新软件源、不改防火墙。`scripts/uninstall-router.sh` 默认只移除 UI，带 `--core` 时同时移除 Core。
