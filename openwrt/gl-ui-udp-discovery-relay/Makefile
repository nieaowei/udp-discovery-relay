include $(TOPDIR)/rules.mk
PKG_NAME:=gl-ui-udp-discovery-relay
PKG_VERSION:=0.0.1
PKG_RELEASE:=1
PKG_LICENSE:=GPL-3.0-only
include $(INCLUDE_DIR)/package.mk

define Package/gl-ui-udp-discovery-relay
  SECTION:=net
  CATEGORY:=Network
  TITLE:=Native GL.iNet UDP Discovery Relay interface
  DEPENDS:=+udp-discovery-relay +gl-sdk4-ui-core +gl-oui-rpc +luci-lib-nixio
  PKGARCH:=all
endef

define Build/Compile
endef

define Package/gl-ui-udp-discovery-relay/install
	$(CP) ./files/* $(1)/
endef

define Package/gl-ui-udp-discovery-relay/postinst
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] || { nginx -t -q && nginx -s reload; }
endef

define Package/gl-ui-udp-discovery-relay/postrm
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] || { nginx -t -q && nginx -s reload; }
endef

$(eval $(call BuildPackage,gl-ui-udp-discovery-relay))
