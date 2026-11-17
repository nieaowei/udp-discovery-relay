#!/usr/bin/env python3
from pathlib import Path
import re
import sys
version = re.search(r'^version = "([^"]+)"', Path('Cargo.toml').read_text(), re.M).group(1)
assert re.fullmatch(r'\d+\.\d+\.\d+', version), version
package = Path('openwrt/udp-discovery-relay/Makefile').read_text()
assert f'PKG_VERSION:={version}\n' in package
assert re.search(r'^PKG_RELEASE:=\d+$', package, re.M), 'Missing package release'
assert Path('openwrt/udp-discovery-relay/files/usr/share/udp-discovery-relay/version').read_text().strip() == version
if len(sys.argv) > 1 and sys.argv[1].startswith('v'):
    assert sys.argv[1] == 'v' + version, 'Release tag must match Cargo and package versions'
print('Core versions consistent:', version)
