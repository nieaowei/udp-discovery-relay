#!/usr/bin/env python3
import json
from pathlib import Path
import re
import sys
version = re.search(r'^version = "([^"]+)"',Path('Cargo.toml').read_text(),re.M).group(1)
assert re.fullmatch(r'\d+\.\d+\.\d+', version), version
releases=set()
for package in ['udp-discovery-relay','gl-ui-udp-discovery-relay']:
    text = Path('openwrt',package,'Makefile').read_text()
    releases.add(re.search(r'^PKG_RELEASE:=(\d+)$',text,re.M).group(1))
    assert f'PKG_VERSION:={version}\n' in text, package
assert len(releases)==1, 'Core/UI release numbers differ'
assert json.loads(Path('openwrt/gl-ui-udp-discovery-relay/package.json').read_text())['version']==version
assert Path('openwrt/udp-discovery-relay/files/usr/share/udp-discovery-relay/version').read_text().strip()==version
if len(sys.argv)>1 and sys.argv[1].startswith('v'):
    assert sys.argv[1]=='v'+version, 'Release tag must match Cargo and package versions'
print('Versions consistent:',version)
