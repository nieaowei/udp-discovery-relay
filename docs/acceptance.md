# 安装与验收

## 平台与兼容性

Core 运行于 Linux，以 JSON 配置独立使用；OpenWrt 集成提供 UCI/procd 适配，GL.iNet SDK4 UI 作为可选软件包提供。Core 不依赖特定路由器型号或 UI。

当前 CI 输出 ARM64 Cortex-A53 / musl 参考产物。安装前核对 CPU、OpenWrt 软件包架构、SDK 来源和系统依赖。`/etc/apk/arch` 决定 APK 可接受的架构；`apk --print-arch` 只表示 apk-tools 自身的编译架构。其他平台需要对应构建及验证。

## 普通 Linux

编译、复制示例配置，并将 `devices` 调整为实际以太网类接口：

```sh
cargo build --release --locked
cp examples/relay.json relay.json
./target/release/udp-discovery-relay --config relay.json --check
sudo ./target/release/udp-discovery-relay --config relay.json --status /run/udr-status.json
```

配置文件校验不需要网络权限。运行时需要创建 packet socket；进程完成初始化后保留 CAP_NET_RAW 并设置 no_new_privs。状态文件目录应预先存在，且避免非授权用户写入。使用发行版的服务管理器启动时，应传入相同的配置和状态路径。

## OpenWrt APK

保留匹配系统的固件软件源，将所选版本的 APK、校验和与 `build-provenance.txt` 复制到 `/tmp/udr/`。下例版本为 `0.0.1-r1`：

```sh
cd /tmp/udr
sha256sum -c udp-discovery-relay-0.0.1-r1.apk.sha256
cat build-provenance.txt
apk add --simulate --allow-untrusted ./udp-discovery-relay-0.0.1-r1.apk
apk add --allow-untrusted ./udp-discovery-relay-0.0.1-r1.apk
/usr/bin/udp-discovery-relay --uci --check
```

这里的 `--allow-untrusted` 用于已校验的本地未签名产物；通过可信签名 feed 安装时遵循该 feed 的签名验证流程。遇到架构或依赖冲突时使用匹配的构建，不强制覆盖系统依赖。

Core 依赖 Lua、lua-cjson、libuci-lua 和 ubus。可选 UI 由 [独立仓库](https://github.com/nieaowei/gl-ui-udp-discovery-relay) 构建和发布，还需要兼容的 gl-sdk4-ui-core、gl-oui-rpc、luci-lib-nixio。下载该仓库的 UI APK 后可单独安装：

```sh
sha256sum -c gl-ui-udp-discovery-relay-0.0.1-r1.apk.sha256
apk add --simulate --allow-untrusted ./gl-ui-udp-discovery-relay-0.0.1-r1.apk
apk add --allow-untrusted ./gl-ui-udp-discovery-relay-0.0.1-r1.apk
```

默认逻辑网络为 `wan` 与 `lan`。按实际网络编辑 `/etc/config/udp_discovery_relay`，并参考 [防火墙指南](firewall.md) 检查 INPUT、FORWARD、返回路由与必要的 TCP SNAT 豁免。

```sh
/etc/init.d/udp-discovery-relay enable
/etc/init.d/udp-discovery-relay restart
/etc/init.d/udp-discovery-relay status
cat /var/run/udp-discovery-relay/status.json
logread -e udp-discovery-relay -l 50
```

UI 入口为应用程序中的 UDP Discovery Relay。配置保存与重新加载分别执行；已停止的服务需要显式启动。全局中继开关控制报文处理，开机开关控制进程自启动。升级保留用户 UCI 配置；出现 `.apk-new` 时应比较差异。

## 客户端与报文验证

选择参与子网中的两个客户端 A、B，先确认它们能通过真实 IPv4 地址互相访问。由配置的 Linux 接口或 OpenWrt 逻辑网络确认实际出入设备：

```sh
ubus call network.interface.lan status
ubus call network.interface.wan status
# 将下面的接口名替换为实际值。
tcpdump -ni INTERFACE -vv 'udp port 53317 or tcp port 53317'
```

1. 两端重开发现，确认对方可见；两个接口上观察中继 UDP 的源 IP、源端口与载荷保持一致。
2. 分别执行 A→B、B→A 的实际文件传输，比较接收文件 SHA-256。显示名称或增加转发计数不能替代传输验收。
3. 确认 TCP 注册的接收端看到真实客户端地址；有 SNAT 时按实际拓扑添加精确豁免。HTTPS 注册路径可从客户端日志核对，不关闭证书校验或添加 TLS 代理。
4. 同时启动多个客户端，检查报文量稳定、循环过滤有效。重复报文缓存会合并同一来源在短时间内的相同载荷。
5. 停止中继并重新发起客户端发现，确认新的跨子网 UDP 发现停止；已有连接、缓存、手动地址或 HTTP 探测仍可能继续工作。
6. 分别测试规则开关、全局开关、reload、restart；不合法配置应被拒绝，旧运行配置保持有效。
7. 通过本地或其他可恢复的管理通道测试接口 down/up、重新创建及地址续租，检查约五秒的检测周期后恢复。随后检查系统重启后的服务和配置恢复。
8. 检查既有 mDNS、DHCP、IPv6 等服务及其配置保持正常，尤其是与发现中继同时运行的其他服务。

## 可选 UI 验证

在实际 SDK4 环境中检查菜单、主题、语言、规则编辑、服务启停、开机设置、保存/应用、恢复默认和日志控制。统计应来自 Core；缺失时显示未知值。两个管理会话同时编辑时，旧 revision 保存应被拒绝。

在退出登录或使用无效 sid 时调用模块，固件必须拒绝请求。先将 `ROUTER_URL` 设置为实际管理地址，再运行只读验证：

```sh
curl -i -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"call","params":["invalid-session","udp-discovery-relay","get_status",{}]}' \
  "$ROUTER_URL/rpc"
```

拒绝可以是 SDK4 JSON 错误，不一定返回 HTTP 401。认证检查必须由服务端分派层执行。配置写入及系统操作还需要在已认证会话中验证参数校验和实际结果。

## 维护与卸载

```sh
uci show udp_discovery_relay
/etc/init.d/udp-discovery-relay reload
/etc/init.d/udp-discovery-relay stop
/etc/init.d/udp-discovery-relay disable
apk del gl-ui-udp-discovery-relay
apk del udp-discovery-relay
```

UI 可独立卸载，Core 继续运行；仅在不再需要中继时移除 Core。手工创建的防火墙规则由管理员按 firewall.md 单独撤销，软件包不修改其他插件的配置。

logd 管理系统日志缓冲；UI 每次读取最多 200 条、64 KiB。Core 状态在 tmpfs 中覆盖更新，最近错误最多保留 16 条。
