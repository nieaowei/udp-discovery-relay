-- UCI adapter shared by the independent daemon and SDK4 RPC. Never runs a shell.
local json = require 'cjson.safe'
local M = {}
local function boolean(v, default)
    if v == nil then return default end
    if v == '1' then return true elseif v == '0' then return false end
    error('Expected UCI boolean 0 or 1')
end
local function array(v)
    if v == nil then return {} end
    if type(v) == 'string' then return {v} end
    return v
end
function M.read(cursor)
    local global = cursor:get_all('udp_discovery_relay', 'main')
    if not global or global['.type'] ~= 'global' then error('Missing global main section') end
    local c = {enabled=boolean(global.enabled,true),log_level=global.log_level or 'info',loop_protection=boolean(global.loop_protection,true),max_packet_size=tonumber(global.max_packet_size or '1400'),rules={}}
    if not c.max_packet_size then error('Invalid packet size') end
    cursor:foreach('udp_discovery_relay','rule',function(r)
        local port=tonumber(r.port)
        if not port then error('Invalid rule port') end
        c.rules[#c.rules+1]={id=r['.name'],name=r.name or r['.name'],enabled=boolean(r.enabled,true),protocol=r.protocol or 'udp',port=port,multicast=r.multicast or '',direction=r.direction or 'bidirectional',log_level=r.log_level or 'info',network=array(r.network),input=array(r.input),output=array(r.output)}
    end)
    return c
end
function M.encode(config)
    local raw,err=json.encode(config)
    if not raw then return nil,err end
    -- lua-cjson 2.1 encodes empty Lua arrays as objects. These keys are arrays.
    for _,key in ipairs({'rules','network','input','output','devices'}) do raw=raw:gsub('"'..key..'":{}','"'..key..'":[]') end
    return raw
end
local function quote(v)
    if type(v) ~= 'string' or v:find('[%z\1-\31\127]') then error('Invalid UCI string') end
    return "'"..v:gsub("'", "'\\''").."'"
end
function M.serialize(c)
    local lines={"config global 'main'"}
    local function option(k,v) lines[#lines+1]='\toption '..k..' '..quote(tostring(v)) end
    option('enabled',c.enabled and '1' or '0');option('log_level',c.log_level);option('loop_protection',c.loop_protection and '1' or '0');option('max_packet_size',c.max_packet_size)
    for _,r in ipairs(c.rules) do
        if type(r.id)~='string' or not r.id:match('^[%w_]+$') then error('Invalid rule id') end
        lines[#lines+1]='\nconfig rule '..quote(r.id)
        option('enabled',r.enabled and '1' or '0')
        for _,k in ipairs({'name','protocol','port','multicast','direction','log_level'}) do option(k,r[k]) end
        for _,k in ipairs({'network','input','output'}) do for _,v in ipairs(r[k] or {}) do lines[#lines+1]='\tlist '..k..' '..quote(v) end end
    end
    return table.concat(lines,'\n')..'\n'
end
return M
