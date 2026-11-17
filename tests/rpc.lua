-- Execute actual production RPC with SDK4/OpenWrt boundaries mocked.
local json=require 'cjson.safe'
local ui=arg[1] or '../gl-ui-udp-discovery-relay'
local core='openwrt/udp-discovery-relay/files/usr/lib/lua/udp-discovery-relay/config.lua'
local path='/etc/config/udp_discovery_relay'
local files={[path]='initial'}
local dirs,commands={},{}
local fail_sync,fail_validation,concurrent=false,false,false
local function revision(raw) local n=0;for i=1,#raw do n=(n*31+raw:byte(i))%2147483647 end;return string.format('%032x',n) end
local function iterator(t) local i=0;return function() i=i+1;return t[i] end end
local fs={
 lstat=function(p) return files[p] and {type='reg',size=#files[p]} end,
 stat=function(p) return files[p] and {type='reg',size=#files[p]} end,
 readfile=function(p) if p=='/proc/sys/kernel/random/uuid' then return '12345678-1234-1234-1234-123456789012\n',37 end;return files[p],files[p] and #files[p] end,
 dir=function() return iterator({'S95udp-discovery-relay'}) end,
 mkdir=function(p) if dirs[p] then return nil end;dirs[p]=true;return true end,
 rmdir=function(p) dirs[p]=nil;return true end,
 unlink=function(p) files[p]=nil;return true end,
 rename=function(a,b) files[b]=files[a];files[a]=nil;return true end
}
package.loaded['nixio.fs']=fs
package.loaded.nixio={open_flags=function() return 0 end,open=function(p)
 if files[p] then return nil end;files[p]=''
 return {write=function(_,s) local chunk=s:sub(1,9);files[p]=files[p]..chunk;return #chunk end,sync=function() return not fail_sync end,close=function() end}
end}
package.loaded['oui.rpc']={ERROR_CODE_INVALID_PARAMS=-32602}
package.loaded['oui.ubus']={call=function() return {['udp-discovery-relay']={instances={main={running=true,pid=42}}}} end}
package.loaded.uci={cursor=function() return {foreach=function(_,name,kind,cb) if name=='network' then cb({['.name']='lan'});cb({['.name']='wan'}) end end} end}
package.loaded['udp-discovery-relay.config']=dofile(core)
ngx={md5=revision,pipe={spawn=function(argv)
 commands[#commands+1]=argv
 if argv[5]=='/usr/bin/udp-discovery-relay' then
  assert(argv[6]=='--config' and argv[8]=='--check','fixed validator argv')
  assert(files[argv[7]],'candidate exists')
  if concurrent then files[path]='external change' end
 end
 return {stdout_read_all=function() return (argv[5]=='/sbin/logread' and string.rep('error\n',20000) or '')..'\nUDR_EXIT:'..(fail_validation and '1' or '0')..'\n' end}
end}}
local api=dofile(ui..'/files/usr/lib/oui-httpd/rpc/udp-discovery-relay')
local c={enabled=true,log_level='info',loop_protection=true,max_packet_size=1400,rules={{id='localsend',name='LocalSend',enabled=true,protocol='udp',port=53317,multicast='224.0.0.167',direction='bidirectional',log_level='info',network={'lan','wan'},input={},output={}}}}
local function save(rev) return api.set_config({settings=c,revision=rev or revision(files[path]),apply=true}) end
local function rejected_unchanged(fn) local before=files[path];local r=fn();assert(r.err_code,r.err_msg);assert(files[path]==before);assert(not next(dirs),'lock released') end
local r=save();assert(r.success and r.applied);assert(files[path]:find("list network 'lan'",1,true));assert(commands[#commands][6]=='reload')
rejected_unchanged(function() return save(string.rep('0',32)) end)
fail_validation=true;rejected_unchanged(save);fail_validation=false
fail_sync=true;rejected_unchanged(save);fail_sync=false
c.rules[1].network={'lan;reboot','wan'};rejected_unchanged(save);c.rules[1].network={'lan','wan'}
concurrent=true;r=save();assert(r.err_code and files[path]=='external change');concurrent=false
local count=#commands;assert(api.service({action='stop;reboot'})==-32602);assert(#commands==count)
assert(api.service({action='stop'}).success);assert(commands[#commands][5]=='/etc/init.d/udp-discovery-relay')
files['/var/run/udp-discovery-relay/status.json']='{"pid":42,"received":99}'
r=api.get_status();assert(r.running and r.enabled and r.metrics.received==99)
files['/var/run/udp-discovery-relay/status.json']='{"pid":43,"received":100}'
assert(not api.get_status().metrics_available,'stale PID rejected')
assert(#api.get_logs().text==65536)
assert(not next(dirs))
print('PASS RPC: real adapter writes, validation, revisions, concurrency, partial IO, injection, pid metrics, bounded logs')
