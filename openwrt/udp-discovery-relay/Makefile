include $(TOPDIR)/rules.mk
PKG_NAME:=udp-discovery-relay
PKG_VERSION:=0.0.1
PKG_RELEASE:=1
PKG_LICENSE:=GPL-3.0-only
PKG_BUILD_DEPENDS:=rust/host
PKG_BUILD_PARALLEL:=1
# scripts/prepare-feed.sh creates this local source archive. No moving git HEAD.
PKG_SOURCE:=$(PKG_NAME)-$(PKG_VERSION).tar.gz
PKG_HASH:=skip
include $(INCLUDE_DIR)/package.mk
include ../../lang/rust/rust-package.mk

define Package/udp-discovery-relay
  SECTION:=net
  CATEGORY:=Network
  TITLE:=Source-preserving UDP discovery relay
  DEPENDS:=+libc +lua +lua-cjson +libuci-lua +ubus
endef

define Package/udp-discovery-relay/conffiles
/etc/config/udp_discovery_relay
endef

define Package/udp-discovery-relay/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_INSTALL_DIR)/bin/udp-discovery-relay $(1)/usr/bin/
	$(CP) ./files/* $(1)/
endef

# OpenWrt default APK lifecycle enables on first install, preserves boot state
# on upgrade, and stops/disables init scripts on removal.

$(eval $(call BuildPackage,udp-discovery-relay))
