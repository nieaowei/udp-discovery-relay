#!/usr/bin/env python3
"""Root-only wire integration test. Isolated namespaces; never changes host rules.
Protocol peers implement v2.2 register/prepare-upload/upload and certificate pinning.
They are test fixtures, not official LocalSend clients.
"""
import ctypes
import hashlib
import http.client
import http.server
import json
import multiprocessing as mp
import os
from pathlib import Path
import signal
import socket
import ssl
import subprocess as sp
import sys
import tempfile
import threading
import time

GROUP = '224.0.0.167'
PORT = 53317
PREFIX = 'udr-' + str(os.getpid())

def command(*args):
    return sp.run(args, check=True, stdout=sp.PIPE, stderr=sp.PIPE, text=True).stdout

def enter(ns):
    fd = os.open('/var/run/netns/' + ns, os.O_RDONLY)
    try:
        if ctypes.CDLL(None, use_errno=True).setns(fd, 0):
            raise OSError(ctypes.get_errno(), 'setns')
    finally:
        os.close(fd)

def peer(ns, ip, conn, cert, key):
    enter(ns)
    cert_der = ssl.PEM_cert_to_DER_cert(Path(cert).read_text())
    fingerprint = hashlib.sha256(cert_der).hexdigest()
    events = []
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass
        def do_POST(self):
            body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
            events.append((self.path, self.client_address[0], hashlib.sha256(body).hexdigest()))
            if self.path == '/api/localsend/v2/register':
                result = {'alias': ip, 'version': '2.2', 'fingerprint': fingerprint}
            elif self.path == '/api/localsend/v2/prepare-upload':
                assert json.loads(body)['files']['fixture']['size'] > 0
                result = {'sessionId': 'test-session', 'files': {'fixture': 'test-token'}}
            elif self.path == '/api/localsend/v2/upload?sessionId=test-session&fileId=fixture&token=test-token':
                result = {}
            else:
                self.send_error(404)
                return
            raw = json.dumps(result).encode()
            self.send_response(200)
            self.send_header('Content-Length', str(len(raw)))
            self.end_headers()
            self.wfile.write(raw)
    server = http.server.ThreadingHTTPServer((ip, PORT), Handler)
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(cert, key)
    server.socket = context.wrap_socket(server.socket, server_side=True)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    plain = http.server.ThreadingHTTPServer((ip, PORT+2), Handler)
    threading.Thread(target=plain.serve_forever, daemon=True).start()
    udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    udp.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    udp.setsockopt(socket.SOL_SOCKET, socket.SO_BROADCAST, 1)
    udp.bind(('', PORT))
    udp.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, socket.inet_aton(GROUP) + socket.inet_aton(ip))
    udp.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_IF, socket.inet_aton(ip))
    udp.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_LOOP, 0)
    conn.send({'ready': ip})
    while True:
        request = conn.recv()
        try:
            op = request['op']
            if op == 'stop':
                break
            if op == 'send':
                data = request.get('data')
                if data is None:
                    data = json.dumps({'alias': ip, 'version': '2.2', 'fingerprint': fingerprint,
                                       'port': PORT+2 if request.get('protocol')=='http' else PORT, 'protocol': request.get('protocol','https'), 'announce': request.get('announce', True),
                                       'nonce': request.get('nonce', '')}).encode()
                if request.get('source'):
                    extra = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                    extra.bind((request['source'], 0))
                    extra.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_IF, socket.inet_aton(request['source']))
                    extra.sendto(data, (GROUP, PORT));extra.close()
                else:
                    udp.sendto(data, (request.get('dst', GROUP), request.get('port', PORT)))
                result = True
            elif op == 'drain':
                udp.settimeout(0.05)
                try:
                    while True:
                        udp.recvfrom(65535)
                except socket.timeout:
                    pass
                result = True
            elif op == 'receive':
                udp.settimeout(request.get('timeout', 2))
                try:
                    data, addr = udp.recvfrom(65535)
                    result = {'data': data, 'addr': addr}
                except socket.timeout:
                    result = None
            elif op == 'transfer':
                announcement = json.loads(request['data'])
                host = request['source']
                if announcement['protocol']=='https':
                    c = http.client.HTTPSConnection(host, announcement['port'], timeout=4, context=ssl._create_unverified_context())
                    c.connect()
                    actual = hashlib.sha256(c.sock.getpeercert(binary_form=True)).hexdigest()
                    assert actual == announcement['fingerprint'], (actual, announcement['fingerprint'])
                else:
                    c = http.client.HTTPConnection(host, announcement['port'], timeout=4)
                def post(path, data):
                    c.request('POST', '/api/localsend/v2/' + path, data,
                              {'Content-Type': 'application/json', 'Connection': 'keep-alive'})
                    r = c.getresponse()
                    raw = r.read()
                    assert r.status == 200, (r.status, raw)
                    return json.loads(raw)
                post('register', json.dumps({'alias': ip, 'version': '2.2', 'fingerprint': fingerprint, 'port': PORT, 'protocol': 'https'}))
                data = b'LocalSend end-to-end test\n' * 4096
                post('prepare-upload', json.dumps({'info': {'alias': ip}, 'files': {'fixture': {'id': 'fixture', 'size': len(data)}}}))
                post('upload?sessionId=test-session&fileId=fixture&token=test-token', data)
                c.close()
                result = hashlib.sha256(data).hexdigest()
            elif op == 'events':
                result = list(events)
            else:
                raise ValueError(op)
            conn.send({'result': result})
        except Exception as exc:
            conn.send({'error': repr(exc)})
    server.shutdown()
    plain.shutdown()

def main():
    if os.geteuid() != 0:
        sys.exit('Run as root in Linux (or scripts/test-linux.sh)')
    binary = str(Path(sys.argv[1] if len(sys.argv) > 1 else 'target/release/udp-discovery-relay').resolve())
    namespaces = [PREFIX + x for x in ['r', 'a', 'b']]
    router, left, right = namespaces
    relay = None
    peers = []
    output = Path('.test-output')
    output.mkdir(exist_ok=True)
    for old in ['performance.json','integration.json']:
        (output/old).unlink(missing_ok=True)
    log = (output / 'relay.log').open('w')
    error_log = (output / 'relay-error.log').open('w')
    results = []
    with tempfile.TemporaryDirectory(prefix='udr-test-') as tmp:
        tmp = Path(tmp)
        def record(message):
            print('PASS', message, flush=True)
            results.append(message)
        try:
            for ns in namespaces:
                command('ip', 'netns', 'add', ns)
                command('ip', '-n', ns, 'link', 'set', 'lo', 'up')
            for n, ns, subnet in [('a', left, 1), ('b', right, 8)]:
                command('ip', '-n', router, 'link', 'add', 'r' + n, 'type', 'veth', 'peer', 'name', 'p' + n)
                command('ip', '-n', router, 'link', 'set', 'p' + n, 'netns', ns)
                command('ip', '-n', router, 'addr', 'add', f'192.168.{subnet}.1/24', 'dev', 'r' + n)
                command('ip', '-n', router, 'link', 'set', 'r' + n, 'up')
                command('ip', '-n', ns, 'addr', 'add', f'192.168.{subnet}.2/24', 'dev', 'p' + n)
                command('ip', '-n', ns, 'link', 'set', 'p' + n, 'up')
                command('ip', '-n', ns, 'route', 'add', 'default', 'via', f'192.168.{subnet}.1')
                # Relayed packets intentionally originate in the opposite subnet.
                command('ip', 'netns', 'exec', ns, 'sysctl', '-qw', 'net.ipv4.conf.all.rp_filter=0', f'net.ipv4.conf.p{n}.rp_filter=0')
            command('ip', 'netns', 'exec', router, 'sysctl', '-qw', 'net.ipv4.ip_forward=1')
            config = {'enabled': True, 'max_packet_size': 1400, 'rules': [
                {'id': 'localsend', 'name': 'LocalSend', 'port': PORT, 'multicast': GROUP, 'devices': ['ra', 'rb']},
                {'id': 'broadcast', 'name': 'Broadcast', 'port': PORT + 1, 'multicast': '255.255.255.255', 'devices': ['ra', 'rb']} ]}
            path = tmp / 'config.json'
            status = tmp / 'status.json'
            path.write_text(json.dumps(config))
            def start():
                return sp.Popen(['ip', 'netns', 'exec', router, binary, '--config', str(path), '--status', str(status)], stdout=log, stderr=error_log)
            relay = start()
            time.sleep(0.4)
            assert relay.poll() is None
            process_status=Path(f'/proc/{relay.pid}/status').read_text()
            assert int(next(l.split()[1] for l in process_status.splitlines() if l.startswith('CapEff:')),16)==1<<13
            assert 'NoNewPrivs:\t1' in process_status
            record('only CAP_NET_RAW retained and no_new_privs set')
            conns = []
            for ns, ip in [(left, '192.168.1.2'), (right, '192.168.8.2')]:
                cert, key = tmp / (ns + '.crt'), tmp / (ns + '.key')
                command('openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', str(key), '-out', str(cert), '-days', '1', '-subj', '/CN=' + ns)
                a, b = mp.Pipe()
                p = mp.Process(target=peer, args=(ns, ip, b, str(cert), str(key)))
                p.start()
                peers.append(p)
                assert a.poll(10) and 'ready' in a.recv()
                conns.append(a)
            def call(i, op, **params):
                c = conns[i]
                c.send(dict(op=op, **params))
                assert c.poll(10), (i, op, 'timed out')
                r = c.recv()
                assert 'error' not in r, r
                return r['result']
            def discovery(src, dst, nonce):
                call(dst, 'drain')
                call(src, 'send', nonce=nonce)
                r = call(dst, 'receive')
                expected = ['192.168.1.2', '192.168.8.2'][src]
                assert r and r['addr'] == (expected, PORT), r
                assert json.loads(r['data'])['nonce'] == nonce
                return r
            for src, dst in [(0,1),(1,0)]:
                r = discovery(src,dst,str(src))
                digest = call(dst,'transfer', data=r['data'], source=r['addr'][0])
                events = call(src,'events')
                assert [e[0] for e in events][-3:] == ['/api/localsend/v2/register','/api/localsend/v2/prepare-upload','/api/localsend/v2/upload?sessionId=test-session&fileId=fixture&token=test-token']
                assert events[-1][2] == digest
                assert events[-1][1] == ['192.168.1.2','192.168.8.2'][dst]
            record('bidirectional UDP source IP/port, HTTPS fingerprint pinning, register and real file upload/hash')
            for src,dst in [(0,1),(1,0)]:
                call(dst,'drain');call(src,'send',protocol='http',nonce='plain-'+str(src))
                r=call(dst,'receive');assert r and r['addr'][0]==['192.168.1.2','192.168.8.2'][src]
                digest=call(dst,'transfer',data=r['data'],source=r['addr'][0]);assert call(src,'events')[-1][2]==digest
            record('bidirectional unencrypted HTTP registration and upload on advertised alternate TCP port')
            call(1,'drain')
            for _ in range(20): call(0,'send', data=b'duplicate')
            assert call(1,'receive')['data'] == b'duplicate'
            assert call(1,'receive', timeout=0.4) is None
            record('burst deduplication and no infinite loop')
            for alias in ['192.168.1.3','192.168.1.4']:
                command('ip','-n',left,'addr','add',alias+'/24','dev','pa')
                call(0,'send',data=b'multiple-devices',source=alias)
            seen={call(1,'receive')['addr'][0],call(1,'receive')['addr'][0]}
            assert seen=={'192.168.1.3','192.168.1.4'},seen
            record('distinct devices with identical payloads are not coalesced')
            # A broad WAN-style multicast MASQUERADE would rewrite an IP raw
            # socket. AF_PACKET must preserve the original sender even here.
            command('ip','netns','exec',router,'nft','add','table','ip','udr_nat')
            command('ip','netns','exec',router,'nft','add','chain','ip','udr_nat','postrouting','{ type nat hook postrouting priority srcnat; policy accept; }')
            command('ip','netns','exec',router,'nft','add','rule','ip','udr_nat','postrouting','ip','daddr',GROUP,'udp','dport',str(PORT),'masquerade')
            discovery(0,1,'nat-proof-left');discovery(1,0,'nat-proof-right')
            record('original UDP source survives active multicast masquerade rules')

            call(0,'send',data=b'x'*1401)
            assert call(1,'receive',timeout=0.3) is None
            call(0,'send',data=b'unicast',dst='192.168.8.2')
            # Unicast itself is routed to the receiver; it must not be relayed.
            assert call(1,'receive')['data']==b'unicast'
            assert call(1,'receive',timeout=0.3) is None
            record('oversized datagram rejection and no unicast reflection')
            # Separate broadcast listener in receiver namespace verifies egress rewrite.
            code = "import socket,json; s=socket.socket(socket.AF_INET,socket.SOCK_DGRAM); s.bind(('',53318)); s.settimeout(3); d,a=s.recvfrom(2048); print(json.dumps([d.decode(),a]))"
            recv=sp.Popen(['ip','netns','exec',right,'python3','-c',code],stdout=sp.PIPE,text=True)
            time.sleep(0.2)
            call(0,'send',data=b'broadcast',dst='255.255.255.255',port=53318)
            raw,_=recv.communicate(timeout=5)
            assert recv.returncode==0 and json.loads(raw)==['broadcast',['192.168.1.2',PORT]],raw
            record('broadcast forwarding with original source IP and port')
            path.write_text('{invalid')
            relay.send_signal(signal.SIGHUP)
            time.sleep(0.2)
            discovery(0,1,'invalid-reload-keeps-running')
            config['enabled']=False
            path.write_text(json.dumps(config));relay.send_signal(signal.SIGHUP);time.sleep(0.3)
            call(1,'drain');call(0,'send',nonce='disabled')
            assert call(1,'receive',timeout=0.5) is None
            time.sleep(5.1);assert json.loads(status.read_text())['active_rules']==0
            config['enabled']=True
            path.write_text(json.dumps(config));relay.send_signal(signal.SIGHUP);time.sleep(0.3)
            discovery(0,1,'enabled')
            record('SIGHUP validation, disable stops discovery, reenable recovers')
            info = (output / 'relay.log').read_text()
            errors = (output / 'relay-error.log').read_text()
            assert 'started ' in info and 'configuration reloaded' in info, info
            assert 'started ' not in errors and 'configuration reloaded' not in errors, errors
            assert 'Reload rejected; keeping previous configuration' in errors, errors
            assert 'Reload rejected' not in info, info
            # Resolve logical OpenWrt network names through the real ubus adapter.
            ubus=tmp/'ubus'
            ubus.write_text("""#!/bin/sh
case "$2" in network.interface.alpha) dev=ra;; network.interface.beta) dev=rb;; *) exit 1;; esac
printf '{"up":true,"l3_device":"%s"}' "$dev"
""")
            ubus.chmod(0o755)
            os.environ['PATH']=str(tmp)+':'+os.environ['PATH']
            relay.terminate();assert relay.wait(timeout=3)==0
            assert 'udp-discovery-relay: stopped' in (output / 'relay.log').read_text()
            assert 'udp-discovery-relay: stopped' not in (output / 'relay-error.log').read_text()
            record('successful lifecycle/reload messages use stdout; rejected reloads use stderr')
            one=dict(config['rules'][0]);one.pop('devices');one.update(direction='oneway',input=['alpha'],output=['beta'])
            original_rule=config['rules'][0];config['rules'][0]=one
            path.write_text(json.dumps(config));relay=start();time.sleep(0.4)
            discovery(0,1,'one-way')
            call(0,'drain');call(1,'send',nonce='blocked-direction');assert call(0,'receive',timeout=0.5) is None
            record('logical network resolution and one-way rule enforcement')
            config['rules'][0]=original_rule;path.write_text(json.dumps(config));relay.send_signal(signal.SIGHUP);time.sleep(0.3)
            command('ip','-n',router,'link','set','rb','down');time.sleep(5.3)
            command('ip','-n',router,'link','set','rb','up');time.sleep(5.3)
            discovery(1,0,'link-recovered')
            command('ip','-n',router,'addr','replace','192.168.8.3/24','dev','rb');time.sleep(5.3)
            discovery(0,1,'address-recovered')
            record('interface down/up and IPv4 address renewal recovery')
            conns[1].send({'op':'stop'});peers[1].join(3)
            command('ip','-n',router,'link','del','rb')
            command('ip','-n',router,'link','add','rb','type','veth','peer','name','pb')
            command('ip','-n',router,'link','set','pb','netns',right)
            command('ip','-n',router,'addr','add','192.168.8.1/24','dev','rb')
            command('ip','-n',router,'link','set','rb','up')
            command('ip','-n',right,'addr','add','192.168.8.2/24','dev','pb')
            command('ip','-n',right,'link','set','pb','up')
            command('ip','-n',right,'route','add','default','via','192.168.8.1')
            a,b=mp.Pipe();p=mp.Process(target=peer,args=(right,'192.168.8.2',b,str(tmp/(right+'.crt')),str(tmp/(right+'.key'))))
            p.start();peers[1]=p;conns[1]=a;assert a.poll(10) and 'ready' in a.recv();time.sleep(5.3)
            discovery(0,1,'interface-recreated');discovery(1,0,'interface-recreated-reverse')
            record('deleted/recreated veth with a new ifindex recovers automatically')
            # Restore address, default route's gateway remains on this interface.
            # The recreated device already has the original address.
            relay.terminate();assert relay.wait(timeout=3)==0
            call(1,'drain');call(0,'send',nonce='stopped')
            assert call(1,'receive',timeout=0.5) is None
            relay=start();time.sleep(0.3);discovery(0,1,'restarted')
            record('graceful SIGTERM, no relay while stopped, restart recovery')
            # Exercise the target router's Ethernet-bridge transmit path too.
            command('ip','-n',router,'link','add','br-lan','type','bridge')
            command('ip','-n',router,'addr','del','192.168.8.1/24','dev','rb')
            command('ip','-n',router,'link','set','rb','master','br-lan')
            command('ip','-n',router,'addr','add','192.168.8.1/24','dev','br-lan')
            command('ip','-n',router,'link','set','br-lan','up')
            for rule in config['rules']: rule['devices']=['ra','br-lan']
            path.write_text(json.dumps(config));relay.send_signal(signal.SIGHUP);time.sleep(5.3)
            discovery(0,1,'bridge-forward');discovery(1,0,'bridge-reverse')
            record('Linux br-lan bridge multicast receive/transmit preserves source in both directions')
            time.sleep(5.2)
            stats=json.loads(status.read_text())
            assert stats['active_rules']==2 and stats['forwarded']>0
            def ticks():
                fields=Path(f'/proc/{relay.pid}/stat').read_text().split()
                return int(fields[13])+int(fields[14])
            before=ticks();time.sleep(10);cpu=(ticks()-before)/os.sysconf('SC_CLK_TCK')/10*100
            perf={'binary_bytes':os.path.getsize(binary),'rss_bytes':stats['rss_bytes'],'idle_cpu_percent_10s':cpu,'platform':command('uname','-m').strip(),'statistics':stats}
            (output/'performance.json').write_text(json.dumps(perf,indent=2)+'\n')
            record('daemon statistics and 10-second idle resource measurement')
            for c in conns: c.send({'op':'stop'})
            for p in peers: p.join(3)
        finally:
            if relay and relay.poll() is None:
                relay.terminate();relay.wait(timeout=5)
            for p in peers:
                if p.is_alive(): p.terminate()
                p.join(3)
            for ns in reversed(namespaces):
                sp.run(['ip','netns','del',ns],stdout=sp.DEVNULL,stderr=sp.DEVNULL)
            log.close()
            error_log.close()
    (output/'integration.json').write_text(json.dumps({'passed':results},indent=2)+'\n')
    print(json.dumps(perf,indent=2))

if __name__=='__main__':
    main()
