# firewall4 / nftables 配置

**软件包不会修改防火墙、Avahi 配置或路由，也不会自动应用防火墙规则。** 下文使用两个虚构的私网子网举例：上游 `192.168.1.0/24` 属于 `wan` 区域，下游 `192.168.8.0/24` 属于 `lan` 区域，中继主机的上游地址为 `192.168.1.2`。使用前应按实际网络替换地址、区域和端口，并通过 `uci show firewall` 确认区域名称。OpenWrt 逻辑网络与防火墙区域是不同的对象。

## INPUT：接收发往路由器的发现报文

使用 `uci batch` 添加具名配置节；执行此代码块会有意修改防火墙配置：

```sh
uci batch <<'UCI'
set firewall.udr_input_wan=rule
set firewall.udr_input_wan.name='UDP relay discovery from upstream LAN'
set firewall.udr_input_wan.src='wan'
set firewall.udr_input_wan.src_ip='192.168.1.0/24'
set firewall.udr_input_wan.dest_ip='224.0.0.167'
set firewall.udr_input_wan.proto='udp'
set firewall.udr_input_wan.dest_port='53317'
set firewall.udr_input_wan.family='ipv4'
set firewall.udr_input_wan.target='ACCEPT'
set firewall.udr_input_lan=rule
set firewall.udr_input_lan.name='UDP relay discovery from local LAN'
set firewall.udr_input_lan.src='lan'
set firewall.udr_input_lan.src_ip='192.168.8.0/24'
set firewall.udr_input_lan.dest_ip='224.0.0.167'
set firewall.udr_input_lan.proto='udp'
set firewall.udr_input_lan.dest_port='53317'
set firewall.udr_input_lan.family='ipv4'
set firewall.udr_input_lan.target='ACCEPT'
UCI
```

这些规则没有设置 `dest` 区域，因此作用于 INPUT，而不是跨接口转发。来源范围仅限受信任的私网子网；条件允许时，可进一步缩小到具体设备地址。广播规则需要单独指定对应的广播目标地址和 UDP 端口。本中继不需要开放 UDP 5353、DHCP 或 SSDP。

## FORWARD：允许 LocalSend 直接传输

UDP 发现和 TCP 转发需要分别处理。仅在现有规则尚未提供等效放行时添加：

```sh
uci batch <<'UCI'
set firewall.udr_tcp_wan_lan=rule
set firewall.udr_tcp_wan_lan.name='LocalSend upstream to LAN'
set firewall.udr_tcp_wan_lan.src='wan'
set firewall.udr_tcp_wan_lan.dest='lan'
set firewall.udr_tcp_wan_lan.src_ip='192.168.1.0/24'
set firewall.udr_tcp_wan_lan.dest_ip='192.168.8.0/24'
set firewall.udr_tcp_wan_lan.proto='tcp'
set firewall.udr_tcp_wan_lan.dest_port='53317'
set firewall.udr_tcp_wan_lan.family='ipv4'
set firewall.udr_tcp_wan_lan.target='ACCEPT'
set firewall.udr_tcp_lan_wan=rule
set firewall.udr_tcp_lan_wan.name='LocalSend LAN to upstream'
set firewall.udr_tcp_lan_wan.src='lan'
set firewall.udr_tcp_lan_wan.dest='wan'
set firewall.udr_tcp_lan_wan.src_ip='192.168.8.0/24'
set firewall.udr_tcp_lan_wan.dest_ip='192.168.1.0/24'
set firewall.udr_tcp_lan_wan.proto='tcp'
set firewall.udr_tcp_lan_wan.dest_port='53317'
set firewall.udr_tcp_lan_wan.family='ipv4'
set firewall.udr_tcp_lan_wan.target='ACCEPT'
UCI
```

回复流量由现有 conntrack 的 established/related 规则处理。LocalSend 公告中指定的 TCP 端口可以与发现用的 UDP 端口不同，应相应调整放行范围。中继程序不代理文件传输数据。

## 检查 TCP 源地址转换（SNAT）

即使 WAN 地址伪装已启用，packet socket 发送的 UDP 发现报文仍可保留原始来源；直接进行的 TCP 注册同样需要保留设备 IP。应在两侧抓包，核对 SYN 和注册连接的源地址。如果来源被改成了中继主机的出接口地址，可在 fw4 跳转到各区域 SNAT 链之前添加精确豁免。下面的规则保留其他 WAN 流量的 NAT 行为：

```sh
cat > /etc/udp-discovery-relay-no-snat.nft <<'NFT'
ip saddr 192.168.8.0/24 ip daddr 192.168.1.0/24 tcp dport 53317 counter return comment "UDR direct LocalSend"
NFT
uci batch <<'UCI'
set firewall.udr_no_snat=include
set firewall.udr_no_snat.type='nftables'
set firewall.udr_no_snat.path='/etc/udp-discovery-relay-no-snat.nft'
set firewall.udr_no_snat.position='chain-pre'
set firewall.udr_no_snat.chain='srcnat'
UCI
```

`chain-pre` 和 `chain` 是受支持的 [firewall4 include 配置项](https://github.com/openwrt/firewall4/blob/master/root/usr/share/ucode/fw4.uc)。应用前先查看 `fw4 print`，并运行 `fw4 check`。本示例假设 fw4 是唯一的 IPv4 NAT 管理组件；其他 NAT 链或 VPN 策略需要单独检查。已有 conntrack 连接会保留原来的 NAT 决策，直到连接关闭或过期，因此重新测试前应重开两端应用。

可选脚本 `scripts/enable-router-firewall.sh` 和 `scripts/enable-localsend-no-snat.sh` 实现了此示例拓扑，需要管理员显式执行，安装软件包时不会调用。使用前检查脚本中固定的子网和区域值，仅选择实际网络需要的改动。

## 应用与撤销

编辑前备份 `/etc/config/firewall`。上述具名配置节在重复执行时会被替换，不会不断追加。检查 `uci changes firewall`，然后执行：

```sh
fw4 check && uci commit firewall && /etc/init.d/firewall reload
```

撤销时仅删除本项目添加的具名配置节，保留其他 Avahi/HomeKit 等规则：

```sh
for section in udr_input_wan udr_input_lan udr_tcp_wan_lan udr_tcp_lan_wan udr_no_snat; do
    uci -q delete "firewall.$section"
done
rm -f /etc/udp-discovery-relay-no-snat.nft
fw4 check && uci commit firewall && /etc/init.d/firewall reload
```

如果提交前校验失败，应恢复备份或仅撤回本次待提交的修改，不要清空防火墙或宽泛开放 WAN INPUT/FORWARD。在此示例中，上游网关需要有一条经中继主机 `192.168.1.2` 返回 `192.168.8.0/24` 的路由。返回路由配置在负责转发回复的路由器上，SNAT 豁免则配置在实际改写源地址的路由器上。测试客户端文件传输前，先确认两个方向均可达。
