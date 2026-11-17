/* SDK4 view using built-in Vue/GL components and the authenticated RPC client. */
module.exports = (function () {
    'use strict';
    var words = {
        description: ['在已互通的 IPv4 子网之间中继 UDP 发现报文，保留设备源地址。文件传输由设备直接完成。', 'Relay UDP discovery between routed IPv4 subnets while preserving device addresses. Devices transfer files directly.'],
        dashboard: ['概览', 'Overview'], rules: ['中继规则', 'Relay rules'], logs: ['日志', 'Logs'],
        running: ['运行中', 'Running'], stopped: ['已停止', 'Stopped'], unknown: ['状态未知', 'Unknown status'],
        boot: ['开机启动', 'Start at boot'], enabled: ['启用中继', 'Enable relay'],
        start: ['启动', 'Start'], stop: ['停止', 'Stop'], restart: ['重启', 'Restart'],
        save: ['保存', 'Save'], apply: ['保存并应用', 'Save and apply'], reset: ['恢复默认配置', 'Restore defaults'],
        refresh: ['刷新', 'Refresh'], loading: ['正在加载…', 'Loading…'], empty: ['暂无记录', 'No entries'],
        addLocal: ['添加 LocalSend', 'Add LocalSend'], addCustom: ['添加自定义 UDP', 'Add custom UDP'],
        name: ['名称', 'Name'], protocol: ['协议', 'Protocol'], port: ['UDP 端口', 'UDP port'],
        multicast: ['组播或广播地址', 'Multicast or broadcast address'], direction: ['转发方向', 'Direction'],
        bidirectional: ['双向', 'Bidirectional'], oneway: ['单向', 'One way'], network: ['参与网络', 'Participating networks'],
        input: ['输入网络', 'Input networks'], output: ['输出网络', 'Output networks'],
        log_level: ['日志级别', 'Log level'], error: ['错误', 'Error'], warn: ['警告', 'Warning'], info: ['信息', 'Info'], debug: ['调试', 'Debug'], off: ['关闭', 'Off'],
        remove: ['删除', 'Delete'], ruleEnabled: ['启用规则', 'Enable rule'],
        max_packet_size: ['最大 UDP 载荷（字节）', 'Maximum UDP payload (bytes)'], loop_protection: ['循环过滤', 'Loop filtering'],
        version: ['版本', 'Version'], uptime_seconds: ['运行时间（秒）', 'Uptime (seconds)'], active_rules: ['活跃规则', 'Active rules'],
        received: ['接收报文', 'Received packets'], forwarded: ['转发报文', 'Forwarded packets'], dropped: ['丢弃报文', 'Dropped packets'], loop_filtered: ['循环过滤报文', 'Loop filtered packets'],
        rss_bytes: ['内存（字节）', 'Memory (bytes)'], recent_errors: ['最近错误', 'Recent errors'], interfaces: ['活跃接口', 'Active interfaces'],
        statsPending: ['等待守护进程统计…', 'Waiting for daemon statistics…'],
        auto: ['自动刷新', 'Auto refresh'], errorsOnly: ['仅错误', 'Errors only'], clear: ['清空显示并暂停', 'Clear display and pause'], export: ['导出日志', 'Export logs'],
        logHint: ['读取 logd 最近 200 条记录，最多显示 64 KiB。清空只影响当前显示。', 'Reads the latest 200 logd entries, limited to 64 KiB. Clearing affects this display only.'],
        hint: ['接口来自 OpenWrt 逻辑网络。双向规则至少选择两个不同子网；单向规则分别选择输入和输出网络。广播请使用 255.255.255.255。', 'Interfaces come from OpenWrt logical networks. Select at least two distinct subnets for bidirectional rules, or separate input/output networks for one-way rules. Use 255.255.255.255 for broadcast.'],
        firewall: ['请先允许所选网络到路由器的发现 UDP，以及设备间的 TCP 53317。此插件不会自动修改防火墙。', 'Allow discovery UDP to the router from the selected networks and TCP 53317 between devices. This plugin does not change firewall rules.'],
        saved: ['配置已保存。', 'Configuration saved.'], applied: ['配置已保存并请求重新加载，请查看状态与最近错误。', 'Configuration saved and reload requested. Check status and recent errors.'],
        requested: ['操作已完成，请查看服务状态。', 'Action completed. Check service status.'],
        unsaved: ['有未保存的修改', 'Unsaved changes'], synced: ['配置已同步', 'Configuration synchronized'],
        confirm: ['确定', 'Confirm'], cancel: ['取消', 'Cancel'], confirmTitle: ['确认操作', 'Confirm action'],
        discard: ['丢弃未保存的修改？', 'Discard unsaved changes?'], resetConfirm: ['恢复 LocalSend 默认配置并替换当前规则？保存后点击应用使其生效。', 'Replace current rules with the LocalSend defaults? Apply afterward to activate them.'],
        removeConfirm: ['删除这条规则？保存并应用后生效。', 'Delete this rule? Save and apply to activate the change.'],
        restartConfirm: ['当前存在未保存的修改。使用已保存的配置启动或重启？', 'Start or restart using saved settings while there are unsaved changes?']
    };
    var css = '.udr-native{min-width:0}.udr-actions{display:flex;gap:10px;align-items:center;flex-wrap:wrap}.udr-muted{color:var(--text-subtitle);line-height:1.7}.udr-tabs{display:flex;gap:20px;border-bottom:1px solid var(--border);margin:20px 0}.udr-tab{background:transparent;border:0;border-bottom:2px solid transparent;padding:12px 0;color:var(--text-subtitle);font:inherit;cursor:pointer}.udr-tab.active{color:var(--primary);border-color:var(--primary)}.udr-row{display:grid;grid-template-columns:minmax(160px,35%) minmax(0,1fr);gap:16px;align-items:center;margin:16px 0}.udr-native .el-select,.udr-native .el-input{width:100%;max-width:500px}.udr-native .gl-btn{margin:0;height:36px;min-height:36px;padding:0 16px}.udr-rule{border-top:1px solid var(--border);padding:16px 0;margin-top:16px}.udr-metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px;margin:20px 0}.udr-metric{background:var(--bg-body);padding:16px;border:1px solid var(--border);border-radius:6px}.udr-metric strong{display:block;font-size:21px;margin-top:10px}.udr-error{color:var(--error,#c44);white-space:pre-wrap;overflow-wrap:anywhere}.udr-notice{color:var(--success,#20a87b)}.udr-log{background:var(--bg-body);border:1px solid var(--border);padding:14px;max-height:440px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font:12px/1.7 monospace}.udr-footer{border-top:1px solid var(--border);padding-top:20px;margin-top:24px;justify-content:space-between}.udr-tab:focus-visible{outline:2px solid var(--primary)}@media(max-width:600px){.udr-row{grid-template-columns:1fr;gap:8px}}';
    return {
        name: 'udp-discovery-relay',
        data: function () { return {settings: {}, revision: '', original: '', interfaces: [], loaded: false, status: {}, statusLoaded: false, statusPending: false, busy: false, error: '', notice: '', active: 'dashboard', logText: '', logAuto: true, logPending: false, errorsOnly: false, timer: null, disposed: false, confirmVisible: false, confirmMessage: '', confirmResolve: null, unloadHandler: null}; },
        computed: {
            dirty: function () { return this.loaded && JSON.stringify(this.settings) !== this.original; },
            visibleLogs: function () { return this.errorsOnly ? this.logText.split('\n').filter(function (l) { return /error|failed|rejected/i.test(l); }).join('\n') : this.logText; }
        },
        created: function () { this.fetchConfig(); this.refreshStatus(); },
        mounted: function () {
            var self = this;
            this.unloadHandler = function (e) { if (self.dirty) { e.preventDefault(); e.returnValue = ''; } };
            window.addEventListener('beforeunload', this.unloadHandler);
            this.timer = setInterval(function () { if (document.hidden || self.busy) return; self.refreshStatus(); if (self.active === 'logs' && self.logAuto) self.loadLogs(); }, 5000);
        },
        beforeDestroy: function () { this.disposed = true; clearInterval(this.timer); window.removeEventListener('beforeunload', this.unloadHandler); this.confirmAnswer(false); },
        beforeRouteLeave: function (to, from, next) { if (!this.dirty) next(true); else this.ask(this.t('discard')).then(next); },
        methods: {
            t: function (key) { var lang = (this.$store && this.$store.getters.lang) || (this.$i18n && this.$i18n.locale) || 'en'; return words[key] ? words[key][/^zh/.test(lang) ? 0 : 1] : key; },
            rpc: async function (method, args) { var r = await window.$rpcRequest('call', ['sid', 'udp-discovery-relay', method, args || {}], {timeout: 30000, isCancel: false}); if (!r || r.err_code) throw new Error(r && r.err_msg || this.t('error')); return r; },
            run: async function (fn) { if (this.busy || this.disposed) return; this.busy = true; this.error = ''; this.notice = ''; try { await fn(); } catch (e) { if (!this.disposed) this.error = e.message || String(e); } finally { this.busy = false; } },
            loadConfig: async function () { var d = await this.rpc('get_config'); if (this.disposed) return; if (!Array.isArray(d.settings.rules)) d.settings.rules = []; d.settings.rules.forEach(function (r) { ['network','input','output'].forEach(function (k) { if (!Array.isArray(r[k])) r[k] = []; }); }); this.settings = d.settings; this.revision = d.revision; this.interfaces = Array.isArray(d.interfaces) ? d.interfaces : []; this.original = JSON.stringify(this.settings); this.loaded = true; },
            fetchConfig: function () { var self = this; return this.run(function () { return self.loadConfig(); }); },
            refreshStatus: async function () { if (this.statusPending || this.disposed) return; this.statusPending = true; try { var d = await this.rpc('get_status'); if (!this.disposed) { this.status = d; this.statusLoaded = true; } } catch (e) { this.statusLoaded = false; this.status = {}; this.error = e.message || String(e); } finally { this.statusPending = false; } },
            refresh: async function () { if (this.dirty && !await this.ask(this.t('discard'))) return; await this.fetchConfig(); await this.refreshStatus(); },
            save: function (apply) { var self = this; return this.run(async function () { var r = await self.rpc('set_config', {settings: self.settings, revision: self.revision, apply: apply}); self.revision = r.revision; self.original = JSON.stringify(self.settings); self.notice = self.t(apply && r.applied ? 'applied' : 'saved'); if (r.warning) self.error = r.warning; await self.refreshStatus(); }); },
            reset: async function () { if (!await this.ask(this.t('resetConfirm'))) return; var self = this; return this.run(async function () { await self.rpc('reset', {revision: self.revision}); await self.loadConfig(); self.notice = self.t('saved'); }); },
            action: async function (action) { if (this.busy) return; if ((action === 'start' || action === 'restart') && this.dirty && !await this.ask(this.t('restartConfirm'))) return; var self = this; return this.run(async function () { await self.rpc('service', {action: action}); self.notice = self.t('requested'); await self.refreshStatus(); }); },
            add: function (local) { if (this.settings.rules.length >= 32) return; this.settings.rules.push({id: 'r' + Date.now().toString(36) + this.settings.rules.length, name: local ? 'LocalSend' : 'UDP', enabled: true, protocol: 'udp', port: local ? 53317 : 12345, multicast: local ? '224.0.0.167' : '255.255.255.255', direction: 'bidirectional', log_level: 'info', network: [], input: [], output: []}); this.active = 'rules'; },
            remove: async function (index) { if (await this.ask(this.t('removeConfirm'))) this.settings.rules.splice(index, 1); },
            loadLogs: async function () { if (this.logPending || this.disposed) return; this.logPending = true; try { var r = await this.rpc('get_logs'); if (!this.disposed) this.logText = r.text; } catch (e) { this.error = e.message || String(e); } finally { this.logPending = false; } },
            exportLogs: function () { var url = URL.createObjectURL(new Blob([this.visibleLogs], {type:'text/plain;charset=utf-8'})); var a = document.createElement('a'); a.href = url; a.download = 'udp-discovery-relay.log'; a.click(); setTimeout(function () { URL.revokeObjectURL(url); }, 1000); },
            ask: function (message) { if (this.confirmResolve) this.confirmResolve(false); this.confirmMessage = message; this.confirmVisible = true; var self = this; return new Promise(function (resolve) { self.confirmResolve = resolve; }); },
            confirmAnswer: function (answer) { var resolve = this.confirmResolve; this.confirmResolve = null; this.confirmVisible = false; if (resolve) resolve(answer); }
        },
        render: function (h) {
            var self = this;
            function t(k) { return self.t(k); }
            function button(k, action, primary, disabled) { return h('gl-button', {attrs:{type:primary ? 'primary' : 'default',disabled:self.busy || !!disabled},on:{click:action}}, [t(k)]); }
            function row(k, control) { return h('div', {class:'udr-row'}, [h('label',[t(k)]),control]); }
            function field(obj, k, kind, options) {
                var attrs = {value:obj[k],disabled:self.busy,'aria-label':t(k)}, control;
                function change(v) { self.$set(obj,k,kind === 'number' ? Number(v) : v); self.notice = ''; }
                if (kind === 'bool') control = h('gl-switch',{attrs:attrs,on:{input:change}});
                else if (kind === 'select' || kind === 'networks') { attrs.multiple = kind === 'networks'; attrs['popper-class'] = 'main-select'; control = h('el-select',{attrs:attrs,on:{input:change}},options.map(function (v) { return h('el-option',{key:v,attrs:{value:v,label:kind === 'networks' ? v : t(v)}}); })); }
                else { attrs.type = kind === 'number' ? 'number' : 'text'; attrs.maxlength = k === 'name' ? 80 : 32; control = h('el-input',{attrs:attrs,on:{input:change}}); }
                return row(k,control);
            }
            var body = [];
            if (!this.loaded) body.push(h('p',[t('loading')]),button('refresh',this.refresh));
            else if (this.active === 'dashboard') {
                body.push(field(this.settings,'enabled','bool'),field(this.settings,'log_level','select',['off','error','warn','info','debug']),field(this.settings,'loop_protection','bool'),field(this.settings,'max_packet_size','number'));
                var m = this.statusLoaded && this.status.metrics_available && this.status.metrics;
                body.push(h('div',{class:'udr-metrics'},['version','uptime_seconds','active_rules','received','forwarded','dropped','loop_filtered','rss_bytes'].map(function (k) { return h('div',{class:'udr-metric',key:k},[h('span',{class:'udr-muted'},[t(k)]),h('strong',[m && m[k] !== undefined ? String(m[k]) : k === 'version' && self.status.version || '—'])]); })));
                if (!m && this.status.running) body.push(h('p',{class:'udr-muted'},[t('statsPending')]));
                body.push(h('h3',[t('recent_errors')]),h('pre',{class:'udr-error'},[m && Array.isArray(m.recent_errors) && m.recent_errors.join('\n') || t('empty')]),h('h3',[t('interfaces')]),h('pre',{class:'udr-log'},[m && Array.isArray(m.interfaces) && m.interfaces.join('\n') || t('empty')]));
            } else if (this.active === 'rules') {
                body.push(h('p',{class:'udr-muted'},[t('hint')]),h('div',{class:'udr-actions'},[button('addLocal',function () { self.add(true); }),button('addCustom',function () { self.add(false); })]));
                if (!this.settings.rules.length) body.push(h('p',[t('empty')]));
                this.settings.rules.forEach(function (r,i) {
                    var controls=[h('div',{class:'udr-actions'},[h('h3',[r.name || r.id]),button('remove',function () { self.remove(i); })]),row('ruleEnabled',h('gl-switch',{attrs:{value:r.enabled,disabled:self.busy,'aria-label':t('ruleEnabled')},on:{input:function (v) { r.enabled = v; }}})),field(r,'name','text'),field(r,'protocol','select',['udp']),field(r,'port','number'),field(r,'multicast','text')];
                    controls.push(row('direction',h('el-select',{attrs:{value:r.direction,disabled:self.busy,'aria-label':t('direction'),'popper-class':'main-select'},on:{input:function (v) { r.direction = v; r.network = []; r.input = []; r.output = []; }}},['bidirectional','oneway'].map(function (v) { return h('el-option',{attrs:{value:v,label:t(v)}}); }))));
                    if (r.direction === 'oneway') controls.push(field(r,'input','networks',self.interfaces),field(r,'output','networks',self.interfaces)); else controls.push(field(r,'network','networks',self.interfaces));
                    controls.push(field(r,'log_level','select',['off','error','warn','info','debug']));body.push(h('section',{class:'udr-rule',key:r.id},controls));
                });
            } else {
                body.push(h('p',{class:'udr-muted'},[t('logHint')]),h('div',{class:'udr-actions'},[button('refresh',this.loadLogs,false,this.logPending),t('auto'),h('gl-switch',{attrs:{value:this.logAuto,'aria-label':t('auto')},on:{input:function (v) { self.logAuto = v; }}}),t('errorsOnly'),h('gl-switch',{attrs:{value:this.errorsOnly,'aria-label':t('errorsOnly')},on:{input:function (v) { self.errorsOnly = v; }}}),button('clear',function () { self.logAuto = false; self.logText = ''; }),button('export',this.exportLogs)]),h('pre',{class:'udr-log',attrs:{role:'log','aria-label':t('logs')}},[this.visibleLogs || t('empty')]));
            }
            return h('div',{class:'udr-native'},[h('style',[css]),h('gl-title',{attrs:{title:'UDP Discovery Relay'}}),
                h('el-dialog',{attrs:{visible:this.confirmVisible,title:t('confirmTitle'),'append-to-body':true,'close-on-click-modal':false},on:{close:function () { self.confirmAnswer(false); }}},[h('p',[this.confirmMessage]),h('div',{slot:'footer'},[button('cancel',function () { self.confirmAnswer(false); }),button('confirm',function () { self.confirmAnswer(true); },true)])]),
                h('gl-card',[h('p',{class:'udr-muted'},[t('description')]),h('p',{class:'udr-muted'},[t('firewall')]),h('div',{class:'udr-actions'},[h('strong',{attrs:{role:'status'}},[t(!this.statusLoaded ? 'unknown' : this.status.running ? 'running' : 'stopped')]),t('boot'),h('gl-switch',{attrs:{value:!!this.status.enabled,disabled:this.busy || !this.statusLoaded || this.statusPending,'aria-label':t('boot')},on:{input:function (v) { self.action(v ? 'enable' : 'disable'); }}}),button('start',function () { self.action('start'); },false,!this.statusLoaded || this.status.running),button('stop',function () { self.action('stop'); },false,!this.statusLoaded || !this.status.running),button('restart',function () { self.action('restart'); },false,!this.statusLoaded || !this.status.running)]),
                    h('div',{class:'udr-tabs',attrs:{role:'tablist'}},['dashboard','rules','logs'].map(function (k) { return h('button',{class:['udr-tab',self.active === k ? 'active' : ''],attrs:{type:'button',role:'tab','aria-selected':String(self.active === k)},on:{click:function () { self.active = k; if (k === 'logs') self.loadLogs(); }}},[t(k)]); })),
                    this.error ? h('p',{class:'udr-error',attrs:{role:'alert'}},[this.error]) : null,this.notice ? h('p',{class:'udr-notice',attrs:{role:'status'}},[this.notice]) : null,h('div',{attrs:{role:'tabpanel'}},body),
                    h('div',{class:'udr-footer udr-actions'},[h('span',{class:'udr-muted'},[t(this.dirty ? 'unsaved' : 'synced')]),h('div',{class:'udr-actions'},[button('refresh',this.refresh),button('reset',this.reset,false,!this.loaded),button('save',function () { self.save(false); },false,!this.loaded || !this.dirty),button('apply',function () { self.save(true); },true,!this.loaded)])])
                ])]);
        }
    };
})();
