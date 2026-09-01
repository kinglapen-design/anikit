/* AniKit CEP spike —— 面板逻辑（2a：对接正式 WS 服务）
 * 链路：端口文件发现 → hello/welcome 握手（拿 cepOutDir 与库就绪态）→ jsx 导出产物到
 * 库 temp/cep-out → exported 通知 → importDone/importError 回执。
 */
var cs = new CSInterface();
var ws = null;
var cepOut = null;   // welcome 下发的库 temp/cep-out 路径（导出产物落点）
var libReady = false;

function log(msg) {
  var el = document.getElementById('log');
  el.textContent += msg + '\n';
  el.scrollTop = el.scrollHeight;
}

function setConn(ok, text) {
  document.getElementById('dot').className = ok ? 'dot on' : 'dot';
  document.getElementById('conn').textContent = text;
}

function readPort() {
  try {
    var fs = window.require('fs');
    var path = window.require('path');
    var p = path.join(process.env.APPDATA, 'AniKit', 'plugin-server.port');
    return parseInt(fs.readFileSync(p, 'utf8').trim(), 10);
  } catch (e) {
    return null;
  }
}

function connect() {
  var port = readPort();
  if (!port) {
    setConn(false, '无端口文件（AniKit 未启动？）');
    return;
  }
  try { if (ws) ws.close(); } catch (e) {}
  ws = new WebSocket('ws://127.0.0.1:' + port);
  ws.onopen = function () {
    ws.send(JSON.stringify({ type: 'hello', app: 'animate', version: '2024' }));
  };
  ws.onmessage = function (ev) {
    log('[ws] ' + ev.data);
    try {
      var msg = JSON.parse(ev.data);
      if (msg.type === 'welcome') {
        libReady = !!msg.libReady;
        cepOut = msg.cepOutDir || null;
        setConn(true, libReady ? '已连接（库就绪）' : '已连接（未开库）');
      } else if (msg.type === 'importDone') {
        log('[ok] 入库完成：' + msg.flaAssetId + (msg.overLimit ? '（超 2M 保留产出）' : ''));
      } else if (msg.type === 'importError') {
        log('[err] 入库失败：' + msg.message);
      }
    } catch (e) {}
  };
  ws.onclose = function () { setConn(false, '连接断开'); libReady = false; };
  ws.onerror = function () { setConn(false, '连接失败（端口 ' + port + '）'); libReady = false; };
}

function evalJsx(script, cb) {
  cs.evalScript(script, function (result) { cb(result); });
}

function doExport(dynamic) {
  if (!ws || ws.readyState !== 1 || !libReady || !cepOut) {
    log('[ui] AniKit 未就绪（未连接或未开库）');
    return;
  }
  var name = document.getElementById('name').value.trim();
  var dir = cepOut.replace(/\\/g, '/');
  log('[jsx] 导出到 ' + dir + ' …');
  var script = 'exportSelection(' + JSON.stringify(name) + ',' + JSON.stringify(dir) + ',' + dynamic + ')';
  evalJsx(script, function (result) {
    log('[jsx] ' + result);
    if (result.indexOf('OK:') !== 0) return;
    var m = result.match(/fla:"(.+)\.fla",frames:(\d+),fps:([\d.]+)/);
    if (!m) { log('[ui] 回执解析失败'); return; }
    var stem = m[1];
    ws.send(JSON.stringify({
      type: 'exported',
      payload: {
        name: stem,
        folderId: null, // 树选择器随 2b，本阶段恒未分类
        dynamic: dynamic,
        flaPath: dir + '/' + stem + '.fla',
        seqDir: dynamic ? dir + '/' + stem + '_seq' : null,
        pngPath: dynamic ? null : dir + '/' + stem + '.png',
        fps: parseFloat(m[3]) || 24
      }
    }));
  });
}

document.getElementById('btn-info').onclick = function () {
  evalJsx('getSelectedInfo()', function (r) { log('[jsx] ' + r); });
};
document.getElementById('btn-open').onclick = function () {
  if (!cepOut) { log('[ui] 未连接'); return; }
  window.require('child_process').exec('explorer.exe "' + cepOut.replace(/\//g, '\\') + '"');
};
document.getElementById('btn-static').onclick = function () { doExport(false); };
document.getElementById('btn-anim').onclick = function () { doExport(true); };

connect();
log('spike 面板已加载（2a 对接正式服务）');
