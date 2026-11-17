const {test} = require('node:test');
const assert = require('node:assert/strict');
const view = require('../files/www/views/gl-sdk4-ui-udp-discovery-relay.common.js');
function instance() {
  const v = view.data();
  Object.entries(view.methods).forEach(([k, fn]) => v[k] = fn.bind(v));
  v.$store = {getters:{lang:'en'}}; v.$set = (obj,k,val) => { obj[k]=val; };
  Object.entries(view.computed).forEach(([k,get]) => Object.defineProperty(v,k,{get:()=>get.call(v)}));
  return v;
}
function h(tag, data, children) { if(Array.isArray(data)) {children=data;data={};} return {tag,data:data||{},children:children||[]}; }
function flatten(node) { return [node].concat(node && node.children ? node.children.flatMap(flatten) : []); }
test('loads real settings, edits rules, saves revision and applies via fixed RPC',async()=>{
  const v=instance(), calls=[];
  v.rpc=async(method,args)=>{calls.push({method,args}); if(method==='get_config') return {settings:{enabled:true,log_level:'info',loop_protection:true,max_packet_size:1400,rules:[]},interfaces:['wan','lan'],revision:'a'.repeat(32)}; if(method==='set_config') return {revision:'b'.repeat(32),applied:true}; return {running:true,enabled:true};};
  await v.loadConfig();assert.equal(v.dirty,false);v.add(true);assert.equal(v.settings.rules[0].port,53317);assert.deepEqual(v.settings.rules[0].network,[]);
  v.settings.rules[0].network=['wan','lan'];assert.equal(v.dirty,true);await v.save(true);assert.equal(v.dirty,false);assert.equal(v.revision,'b'.repeat(32));assert(calls.some(c=>c.method==='set_config'&&c.args.apply));
  const nodes=flatten(view.render.call(v,h));assert(nodes.some(n=>n&&n.tag==='gl-card'));assert(!nodes.some(n=>n&&n.tag==='iframe'));
});
test('RPC rejection preserves unsaved edits and exposes actual error',async()=>{
 const v=instance();v.loaded=true;v.settings={rules:[]};v.original='{}';v.rpc=async()=>{throw new Error('Invalid multicast')};await v.save(false);assert.equal(v.error,'Invalid multicast');assert(v.dirty);
});
test('metrics are absent until reported by daemon, status failures remove stale data',async()=>{
 const v=instance();v.loaded=true;v.settings={rules:[]};const tree=flatten(view.render.call(v,h));assert(tree.filter(n=>n==='—').length>=8);v.status={running:true,metrics:{received:123}};v.rpc=async()=>{throw new Error('procd unavailable')};await v.refreshStatus();assert.equal(v.statusLoaded,false);assert.deepEqual(v.status,{});
});
test('logs filter actual records, failed operations release busy flag, languages render',async()=>{
 const v=instance();v.logText='ready\nerror: broken\nreload rejected\n';v.errorsOnly=true;assert.equal(v.visibleLogs,'error: broken\nreload rejected');
 v.$store.getters.lang='zh-cn';assert.equal(v.t('save'),'保存');v.$store.getters.lang='en';assert.equal(v.t('save'),'Save');v.rpc=async()=>{throw new Error('permission denied')};await v.action('start');assert.equal(v.busy,false);assert.equal(v.error,'permission denied');
});
test('route leave and delete require confirmation, cancellation keeps rules',async()=>{
 const v=instance();v.settings={rules:[{id:'one'}]};v.ask=async()=>false;await v.remove(0);assert.equal(v.settings.rules.length,1);v.ask=async()=>true;await v.remove(0);assert.equal(v.settings.rules.length,0);
});
test('authenticated SDK4 call uses sid and fixed module',async()=>{
 const v=instance();let call;global.window={$rpcRequest:async(...args)=>{call=args;return {success:true}}};await v.rpc('service',{action:'stop'});assert.deepEqual(call[1],['sid','udp-discovery-relay','service',{action:'stop'}]);delete global.window;
});
