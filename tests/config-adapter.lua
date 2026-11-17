local model=dofile('openwrt/udp-discovery-relay/files/usr/lib/lua/udp-discovery-relay/config.lua')
local global={['.type']='global',enabled='1',max_packet_size='1400'}
local rules={{['.name']='localsend',name="Bob's LocalSend",port='53317',multicast='224.0.0.167',network={'wan','lan'}}}
local cursor={get_all=function() return global end,foreach=function(_,p,t,fn) for _,r in ipairs(rules) do fn(r) end end}
local c=model.read(cursor)
assert(c.enabled and c.loop_protection and c.max_packet_size==1400)
assert(c.rules[1].port==53317 and c.rules[1].network[2]=='lan')
local raw=model.serialize(c)
assert(raw:find("Bob'\\''s LocalSend",1,true),'single quotes escaped')
global.enabled='yes';assert(not pcall(model.read,cursor));global.enabled='1'
rules[1].port='bad';assert(not pcall(model.read,cursor));rules={}
local encoded=model.encode(model.read(cursor));assert(encoded:find('"rules":[]',1,true))
local file=assert(io.open(arg[1] or '/tmp/udr-empty-config.json','w'));file:write(encoded);file:close()
print('PASS UCI adapter: booleans, numeric fields, network lists, quote escaping, empty JSON arrays')
