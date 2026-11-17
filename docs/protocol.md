# 源地址保留与 LocalSend

主要参考资料：

- [LocalSend 官方协议 v2.2](https://github.com/localsend/protocol)
- [官方设备发现实现](https://github.com/localsend/localsend/blob/main/packages/core/src/discovery/mod.rs)，重点参阅 `answer_announcement` 和 `register_peer`。
- [官方组播实现](https://github.com/localsend/localsend/tree/main/packages/core/src/multicast)
- Linux 手册：[ip(7)](https://man7.org/linux/man-pages/man7/ip.7.html)、[raw(7)](https://man7.org/linux/man-pages/man7/raw.7.html)、[packet(7)](https://man7.org/linux/man-pages/man7/packet.7.html)。

LocalSend 公告报文包含别名、版本、指纹、监听端口、HTTP/HTTPS 协议类型及公告标志。接收端从报文的源地址识别对端主机。收到公告后，客户端优先通过 HTTP 注册接口响应，必要时回退到带响应标志的组播报文；响应不能继续触发无限的相互响应。中继程序将这些应用层行为交给客户端处理，保持 UDP 载荷不变。

使用加密通信时，证书身份同样重要。官方设备发现实现会在连接对端时使用公告中的证书指纹，并通过 TLS 确认对端身份。如果普通 UDP 转发将源地址改成路由器地址，客户端就会向路由器注册，而路由器无法提供原设备的证书身份。向 JSON 中额外添加地址字段不能解决官方客户端的这一问题。

## 收包与发包方案

守护进程通过 Linux UDP socket 接收报文，使用 `IP_PKTINFO` 获取实际目标地址和入接口索引。它只在解析出的接口上加入已配置的 IPv4 组播组，并且只接收来源位于入接口子网、源地址不是路由器本机的匹配组播或广播报文。单播报文不参与中继。规则的 UDP 监听端口与发送端的原始 UDP 源端口分别处理，转发时保留原始源端口。

`IP_MULTICAST_IF` 可以选择出接口，但不能让普通 UDP socket 保留其他主机的源地址。IPv4 raw socket 的 `IP_HDRINCL` 允许显式构造源地址和完整 IP 首部，不过随后经过 IPv4 POSTROUTING 钩子时，报文仍可能被 WAN 地址伪装规则改写。

中继使用 `AF_PACKET/SOCK_DGRAM` 发送完整 IP/UDP 报文，指定出接口索引以及由目标地址确定的以太网组播或广播地址，由内核生成以太网首部。这能避开发现报文在 IPv4 OUTPUT/POSTROUTING 路径上的 SNAT，无需伪造 ARP 或修改邻居表。Rust 代码计算 IPv4、UDP 校验和，包括 UDP 伪首部及校验和为零时的映射处理。报文 TTL 设为 1，并设置 DF 标志；发送前检查出接口 MTU 和配置的载荷大小限制。

此方案适用于以太网类接口，包括 bridge 和 VLAN。隧道、点对点接口、重叠子网、停用接口和缺失 IPv4 地址等情况会产生明确错误。程序用于反射本地链路的发现报文，不承担组播路由功能。发现报文的发送会绕过 IPv4 OUTPUT 过滤，因此应严格限制中继规则范围；bridge/netdev 过滤、硬件行为或 Wi-Fi 组播策略仍可能阻止报文到达。

## 返回路径与拓扑限制

客户端之间仍直接进行 HTTP(S) 通信。参与网络必须双向可达，客户端防火墙需要允许 LocalSend，路由器也需要相应的 FORWARD 放行规则。通过 NAT 建立一个 TCP 连接，并不能证明双向注册能正确工作：接收端可能把 NAT 地址记作注册设备的地址。必要时，仅对跨子网的 LocalSend TCP 流量设置 SNAT 豁免，具体见 [防火墙指南](firewall.md)。

守护进程可以检测本地接口缺失、停用、子网重叠和不支持的二层接口类型，但无法推断远端主机的防火墙、上游路由，或远端证书是否属于某个实际安装的应用。`scripts/diagnose.sh` 用于读取本地网络、路由、防火墙及守护进程状态；外部条件仍需通过抓包和真实客户端验收确认。转发计数增加不能替代实际文件传输验证。

## 循环防护、资源与权限

只有源地址属于入接口子网的报文才符合中继条件。来自其他子网的中继副本，以及源地址属于路由器本机的报文，都会被拒绝。此外，带随机密钥的哈希缓存会在 750 毫秒内合并发送端、目标、端口和载荷均相同的报文。缓存最多包含 4096 项，不存储完整载荷，因此其内存占用不会随载荷大小增长；收到重复包时也不会刷新该项的有效期。关闭缓存不会关闭子网和来源检查。建议每组互通网络只部署一个中继实例；缓存窗口内的合法相同重传也会被合并。

单线程通过阻塞 `poll` 事件循环工作，每批最多处理 64 个报文，约一秒内响应信号，每五秒检查接口状态。程序复用缓冲区，不需要异步运行时或工作线程池。所有外部载荷均视为不可信数据，守护进程不会将其解析为 LocalSend JSON。最近错误列表最多保留 16 项；重复错误日志受到速率限制，丢弃计数仍会持续更新。状态快照每五秒覆盖写入 tmpfs 中的一个有大小上限的 JSON 文件。

启动时使用 root 创建 socket，并访问由 root 拥有的 OpenWrt 配置。初始化完成后，进程只保留 `CAP_NET_RAW`，锁定重新获取 root capabilities 的能力，并设置 `no_new_privs`，不保留 `CAP_NET_ADMIN`。为访问 UCI/netifd，进程仍使用 UID 0，因此这不等于完整的 UID 隔离。`--check` 不创建网络 socket，也不需要网络 capabilities。UI 不向后端提供可执行程序名称或 Shell 源码。IPv6 作为独立模块明确禁用，不加入 IPv6 组播组，也不修改 IPv6 sysctl。
