/* AniKit CEP 面板（2b 正式版）——控件逻辑与 WS 通信
 * 链路：端口文件发现 → hello/welcome（cepOutDir / settings / libReady）→ 静/动 + 命名 +
 * 目标文件夹（getTree 树选择器）→ jsx 导出 → exported 通知 → importDone/Error。
 * 批次 2（2026-09-02）：导出模式分段——新文件（选中元件导新 fla，上链路）/ 仅缩略图
 *（当前文档补缩略图不导 fla：getDocInfo → exportThumbFrames → thumbOnly → thumbDone
 *  bound 命中绑定 / beside 池外落原文件旁边 / thumbError）。
 * 断线续传（2026-09-02 用户实机报修复）：导出单条 pending 落 localStorage——断连/未发送/进程退出
 * 后重连开库自动重发；未开库时 5s 轮询 hello 自动跟上；「产物不存在」回执 = 重复重发撞已清场，温和提示。
 */
var cs = new CSInterface();
var ws = null;
var cepOut = null;
var libReady = false;
var settings = { maxSecs: 10, maxBytes: 2097152, maxEdge: 1024 };

var dynamic = false;          // 静/动开关（默认静态）
var keepOrigin = true;        // 对齐方式：true = 保持坐标（fla 保原坐标，发送至 AN 原位粘贴联动）；false = 合适舞台（居中态直存，拆素材）
var mode = 'file';            // 导出模式：file = 新文件（选中元件导新 fla）；thumb = 仅缩略图（当前文档补缩略图）
var folderId = null;          // 目标文件夹（null = 未分类）
var folderName = '未分类';
var nameDirty = false;        // 用户改过命名框后不再自动填充元件名
var exporting = false;

var $ = function (id) { return document.getElementById(id); };

function log(msg, cls) {
  var el = $('status');
  var line = document.createElement('div');
  if (cls) line.className = cls;
  line.textContent = msg;
  el.appendChild(line);
  el.scrollTop = el.scrollHeight;
}

function setConn(state, text) {
  $('connDot').className = 'conn-dot' + (state === 'on' ? ' on' : '');
  $('connText').className = 'conn-text' + (state === 'on' ? ' on' : '');
  $('connText').textContent = text;
  refreshExportBtn();
}

function refreshExportBtn() {
  $('btnExport').disabled = !(ws && ws.readyState === 1 && libReady) || exporting;
}

/* —— WS（自动重连：断开 / 失败 / 无端口文件 2s 重试；端口文件每次重读，
      AniKit 重启换端口也能跟上，2026-09-01 用户实机报） —— */
var retryTimer = 0;
function scheduleRetry() {
  if (retryTimer) return;
  retryTimer = setTimeout(function () {
    retryTimer = 0;
    connect();
  }, 2000);
}

function readPort() {
  try {
    var fs = window.require('fs');
    var path = window.require('path');
    return parseInt(fs.readFileSync(path.join(process.env.APPDATA, 'AniKit', 'plugin-server.port'), 'utf8').trim(), 10);
  } catch (e) { return null; }
}

function connect() {
  var port = readPort();
  if (!port) { setConn('off', 'AniKit 未启动'); scheduleRetry(); return; }
  try { if (ws) ws.close(); } catch (e) {}
  ws = new WebSocket('ws://127.0.0.1:' + port);
  ws.onopen = function () {
    ws.send(JSON.stringify({ type: 'hello', app: 'animate', version: '2024' }));
  };
  ws.onmessage = function (ev) {
    var msg;
    try { msg = JSON.parse(ev.data); } catch (e) { return; }
    if (msg.type === 'welcome') {
      libReady = !!msg.libReady;
      cepOut = msg.cepOutDir || null;
      if (msg.settings) settings = msg.settings;
      $('params').textContent = '上限 ' + settings.maxSecs + 's · ' + Math.round(settings.maxBytes / 1048576) + 'M · ' + settings.maxEdge + 'px';
      setConn('on', libReady ? '已连接' : '已连接（AniKit 未开库）');
      if (libReady) {
        ws.send(JSON.stringify({ type: 'getTree' }));
        // 断线续传：有未完成导出 → 自动重发（产物在 cep-out；已被处理/清理则 importError 报清）。
        // 重发等回执期间禁新导出（exporting=true）——防新导出覆盖 pending 后旧回执误清
        if (pending && !batch && !exporting) {
          exporting = true;
          refreshExportBtn();
          log('重发未完成的导出（' + (pending.payload.name || '缩略图') + '）…');
          try {
            ws.send(JSON.stringify({ type: pending.type, payload: pending.payload }));
          } catch (e) {
            exporting = false;
            refreshExportBtn();
          }
        }
      } else {
        // 未开库：5s 轮询重发 hello（开库后面板自动跟上，2026-09-02 用户场景；单链不叠加）
        setTimeout(function () {
          if (ws && ws.readyState === 1 && !libReady) ws.send(JSON.stringify({ type: 'hello', app: 'animate', version: '2024' }));
        }, 5000);
      }
    } else if (msg.type === 'tree') {
      renderTree(msg.data);
    } else if (msg.type === 'settings') {
      // APP 设置页保存后实时推送（8.1 二轮 2b）
      if (msg.settings) {
        settings = msg.settings;
        $('params').textContent = '上限 ' + settings.maxSecs + 's · ' + Math.round(settings.maxBytes / 1048576) + 'M · ' + settings.maxEdge + 'px';
      }
    } else if (msg.type === 'importProgress') {
      log('合成 WebP 中…');
    } else if (msg.type === 'importDone') {
      if (batch) { batch.ok++; batch.done++; batchCheckDone(); return; } // 批量：计数静默，完成汇总
      clearPending();
      exporting = false;
      refreshExportBtn();
      log('✓ 已入库：' + msg.flaAssetId + (msg.overLimit ? '（超 2M 保留产出）' : ''), 'ok');
    } else if (msg.type === 'importError') {
      if (batch) { batch.fail++; batch.done++; log('✗ 入库失败：' + msg.message, 'err'); batchCheckDone(); return; }
      clearPending(); // 服务端明确拒绝（含产物已不存在），不重发防死循环
      exporting = false;
      refreshExportBtn();
      // 「产物不存在」= 重发撞见已入库清场（退出 APP 时机在 commit 后回执前）——非错误，温和提示
      if (/不存在/.test(msg.message)) log('· ' + msg.message + '（产物已入库或已清理，无需重发）');
      else log('✗ 入库失败：' + msg.message, 'err');
    } else if (msg.type === 'thumbDone') {
      // 批次 2 仅缩略图回执：bound = 已绑定库内资产；beside = 池外落原文件旁边
      clearPending();
      exporting = false;
      refreshExportBtn();
      if (msg.mode === 'bound') {
        log('✓ 已绑定缩略图到「' + msg.name + '」' + (msg.overLimit ? '（超上限保留产出）' : ''), 'ok');
      } else {
        log('✓ 不在库内，已导出到原文件旁边：' + msg.outPath + '（自行导入）', 'ok');
      }
    } else if (msg.type === 'thumbError') {
      clearPending();
      exporting = false;
      refreshExportBtn();
      if (/不存在/.test(msg.message)) log('· ' + msg.message + '（产物已处理或已清理，无需重发）');
      else log('✗ 缩略图导出失败：' + msg.message, 'err');
    }
  };
  ws.onclose = function () { onWsDead('连接断开'); };
  ws.onerror = function () { onWsDead('连接失败'); };
}

/* 连接断开/失败：复位导出卡死态并明确提示（单选/仅缩略图 = 产物保留待重发；批量 = 中止报重导）并重连 */
function onWsDead(text) {
  libReady = false;
  setConn('off', text);
  if (exporting) {
    exporting = false;
    if (batch) {
      batch = null;
      log('✗ 连接中断，批量导出中止（已入库项有效，剩余项请重新导出）', 'err');
    } else if (pending) {
      log('✗ 连接中断，导出未完成——产物已保留，重连开库后自动重发', 'err');
    } else {
      log('✗ ' + text + '，导出中止', 'err');
    }
    refreshExportBtn();
  }
  scheduleRetry();
}

/* —— jsx 桥 —— */
function evalJsx(script, cb) { cs.evalScript(script, function (r) { cb(r); }); }

/* —— 未完成导出的断线续传（2026-09-02 用户实机报：导出时退出 APP → 卡「等待入库」无提示无重试）——
 * 单选/仅缩略图 = 单条 pending（localStorage cepPending 持久化；产物在 cep-out 待重发，面板重开也不丢）；
 * 发消息前必查 readyState（WS 在 CLOSED 态 send **静默丢弃**是坑）；收到回执（成功/失败）清 pending；
 * welcome 且 libReady 时自动重发。批量不适用（部分入库状态不明，中断提示重导剩余项）。 —— */
var pending = null; // { type: 'exported'|'thumbOnly', payload }

function setPending(type, payload) {
  pending = { type: type, payload: payload };
  try { localStorage.setItem('cepPending', JSON.stringify(pending)); } catch (e) {}
}
function clearPending() {
  pending = null;
  try { localStorage.removeItem('cepPending'); } catch (e) {}
}
(function loadPending() {
  try {
    var s = localStorage.getItem('cepPending');
    if (s) pending = JSON.parse(s);
  } catch (e) {}
})();

/* 发导出消息（带 pending 登记与就绪检查）；返回 false = 未发送（已提示） */
function sendPayload(type, payload) {
  if (!ws || ws.readyState !== 1) {
    setPending(type, payload);
    exporting = false;
    refreshExportBtn();
    log('✗ 连接已断开，导出未发送——产物已保留，重连开库后自动重发', 'err');
    return false;
  }
  setPending(type, payload);
  try {
    ws.send(JSON.stringify({ type: type, payload: payload }));
  } catch (e) {
    exporting = false;
    refreshExportBtn();
    log('✗ 发送失败（' + e + '）——产物已保留，重连开库后自动重发', 'err');
    return false;
  }
  return true;
}

/* —— 静/动分段（2026-09-02 复验改回：复选框不直观，有标题分组后不乱——用户定） —— */
$('segStatic').onclick = function () { dynamic = false; $('segStatic').className = 'seg-item on'; $('segAnim').className = 'seg-item'; refreshRangeRow(); };
$('segAnim').onclick = function () { dynamic = true; $('segAnim').className = 'seg-item on'; $('segStatic').className = 'seg-item'; refreshRangeRow(); };

/* —— 对齐分段（2026-09-02 用户定）：保持坐标 = 导出后还原（fla 保原坐标，发送至 AN 原位粘贴联动）；
 * 合适舞台 = 缩放居中态直存（拥挤 fla 拆素材场景，拆出即居中）。localStorage（cepAlign）记忆。 —— */
function setAlign(keep, save) {
  keepOrigin = keep;
  $('segKeep').className = 'seg-item' + (keep ? ' on' : '');
  $('segFit').className = 'seg-item' + (keep ? '' : ' on');
  if (save !== false) { try { localStorage.setItem('cepAlign', keep ? 'keep' : 'fit'); } catch (e) {} }
}
$('segKeep').onclick = function () { setAlign(true); };
$('segFit').onclick = function () { setAlign(false); };
(function () { // 恢复上次选择
  var saved = null;
  try { saved = localStorage.getItem('cepAlign'); } catch (e) {}
  if (saved === 'fit') setAlign(false, false);
})();

/* —— 导出模式分段（批次 2，2026-09-02）：仅缩略图模式 = 不导新 fla，
 * 舞台尺寸（比例/像素）/ 对齐 / 入库组（命名 / 目标文件夹）隐藏（不裁舞台不改名、绑定目标按文档路径匹配），
 * 模式说明（modeDesc）显示。localStorage（cepMode）记忆。 —— */
function refreshRangeRow() {
  $('rangeRow').style.display = (mode === 'thumb' && dynamic) ? '' : 'none';
}

function setMode(m, save) {
  mode = m;
  var isThumb = m === 'thumb';
  $('segFile').className = 'seg-item' + (isThumb ? '' : ' on');
  $('segThumb').className = 'seg-item' + (isThumb ? ' on' : '');
  $('segRatio').style.display = isThumb ? 'none' : '';
  $('pixelRow').style.display = isThumb ? 'none' : '';
  $('segAlign').style.display = isThumb ? 'none' : ''; // 对齐档位只管新 fla（仅缩略图不动文档）
  $('groupInbox').style.display = isThumb ? 'none' : '';
  $('modeDesc').style.display = isThumb ? '' : 'none';
  $('btnExport').textContent = isThumb ? '导出缩略图' : '导出';
  refreshRangeRow();
  if (save !== false) { try { localStorage.setItem('cepMode', m); } catch (e) {} }
}
$('segFile').onclick = function () { setMode('file'); };
$('segThumb').onclick = function () { setMode('thumb'); };
(function () { // 恢复上次模式
  var saved = null;
  try { saved = localStorage.getItem('cepMode'); } catch (e) {}
  if (saved === 'thumb') setMode('thumb', false);
})();

/* —— 舞台尺寸（批次 1 比例档；2026-09-02 自定义改像素双框，用户定「比例算起来麻烦，直接传像素建文档」）——
 * 生效源互斥三态：seg 比例档（高度跟随文档按比例算宽，「文档」= 原尺寸）/ 像素框（两框都填直定尺寸，优先）；
 * seg 点击 = 清像素框；像素输入 = seg 去选中；像素全空 = 回「文档」。
 * 记忆：cepRatio（seg 档）+ cepSize（像素，恢复时优先；旧 custom: 比例值废弃回「文档」）。 —— */
var ratioW = 0, ratioH = 0; // 比例档（0 = 跟随文档）
var sizeW = 0, sizeH = 0;   // 像素直定舞台（>0 优先于比例档）

function parseRatioText(t) {
  var m = t.match(/^\s*(\d+(?:\.\d+)?)\s*[:：xX×\/]\s*(\d+(?:\.\d+)?)\s*$/);
  return m ? [parseFloat(m[1]), parseFloat(m[2])] : null;
}

function markRatioSeg(btn) {
  var items = $('segRatio').getElementsByClassName('seg-item');
  for (var i = 0; i < items.length; i++) items[i].className = 'seg-item';
  if (btn) btn.className = 'seg-item on';
}

function setRatio(w, h, btn, save) {
  ratioW = w; ratioH = h;
  markRatioSeg(btn);
  if (save === false) return;
  try {
    localStorage.setItem('cepRatio', btn ? (btn.getAttribute('data-ratio') || 'auto') : 'auto');
  } catch (e) {}
}

function clearSizeInputs() {
  sizeW = sizeH = 0;
  $('sizeW').value = '';
  $('sizeH').value = '';
  $('sizeW').classList.remove('err');
  $('sizeH').classList.remove('err');
  try { localStorage.removeItem('cepSize'); } catch (e) {}
}

function onSizeInput() {
  var w = $('sizeW').value.trim();
  var h = $('sizeH').value.trim();
  if (!w && !h) { // 全空 = 回「文档」档
    clearSizeInputs();
    markRatioSeg($('segRatio').getElementsByClassName('seg-item')[0]);
    setRatio(0, 0, $('segRatio').getElementsByClassName('seg-item')[0]);
    return;
  }
  var wOk = /^\d+$/.test(w) && parseInt(w, 10) > 0;
  var hOk = /^\d+$/.test(h) && parseInt(h, 10) > 0;
  $('sizeW').classList.toggle('err', !wOk); // 半填/非法标红（两框成对必填才生效）
  $('sizeH').classList.toggle('err', !hOk);
  if (wOk && hOk) {
    sizeW = parseInt(w, 10);
    sizeH = parseInt(h, 10);
    ratioW = 0; ratioH = 0;
    markRatioSeg(null); // seg 去选中 = 像素自定义生效
    try {
      localStorage.setItem('cepSize', sizeW + 'x' + sizeH);
      localStorage.setItem('cepRatio', 'auto');
    } catch (e) {}
  }
}

(function initRatio() {
  var items = $('segRatio').getElementsByClassName('seg-item');
  for (var i = 0; i < items.length; i++) {
    (function (btn) {
      btn.onclick = function () {
        var v = btn.getAttribute('data-ratio'); // '' = 文档（跟随原尺寸）
        var p = v ? parseRatioText(v) : null;
        clearSizeInputs();
        setRatio(p ? p[0] : 0, p ? p[1] : 0, btn);
      };
    })(items[i]);
  }
  $('sizeW').addEventListener('input', onSizeInput);
  $('sizeH').addEventListener('input', onSizeInput);
  // 恢复上次选择：像素优先（旧 cepRatio "custom:宽:高" 比例自定义值废弃 → 回「文档」）
  var savedSize = null;
  try { savedSize = localStorage.getItem('cepSize'); } catch (e) {}
  if (savedSize) {
    var sm = savedSize.match(/^(\d+)x(\d+)$/);
    if (sm) {
      sizeW = parseInt(sm[1], 10);
      sizeH = parseInt(sm[2], 10);
      $('sizeW').value = sm[1];
      $('sizeH').value = sm[2];
      markRatioSeg(null);
      return;
    }
  }
  var saved = null;
  try { saved = localStorage.getItem('cepRatio'); } catch (e) {}
  if (!saved || saved === 'auto' || saved.indexOf('custom:') === 0) return;
  for (var j = 0; j < items.length; j++) {
    if (items[j].getAttribute('data-ratio') === saved) {
      var ps = parseRatioText(saved);
      if (ps) setRatio(ps[0], ps[1], items[j], false);
      return;
    }
  }
})();

/* —— 命名框：轮询填充元件名（用户未编辑过时；仅缩略图模式跳过） —— */
$('name').addEventListener('input', function () { nameDirty = $('name').value.trim().length > 0; });
setInterval(function () {
  if (mode === 'thumb' || nameDirty || document.activeElement === $('name')) return;
  evalJsx('getSelectedInfo()', function (r) {
    if (r.indexOf('OK:') !== 0) return;
    var m = r.match(/"libraryItem":"([^"]+)"/);
    if (!m) return;
    var stem = m[1].split('/').pop();
    if (stem && $('name').value !== stem) $('name').value = stem;
  });
}, 1500);

/* —— 中文辅助钮：jsx ScriptUI 原生输入框（系统 IME），回填命名框 —— */
$('btnZh').onclick = function () {
  evalJsx('promptForName(' + JSON.stringify($('name').value) + ')', function (r) {
    if (!r || r === 'CANCEL') return;
    if (r.indexOf('ERROR:') === 0) {
      log('✗ 中文输入框打开失败：' + r.slice(6), 'err');
      return;
    }
    $('name').value = r;
    nameDirty = true;
  });
};

/* —— 目标文件夹树选择器（tab 页切换，2026-09-01 用户定：不占遮罩） —— */
var treeData = null;
function showTree(on) {
  if (on && !libReady) return;
  if (on && ws && ws.readyState === 1) ws.send(JSON.stringify({ type: 'getTree' })); // 每次取最新
  document.body.classList.toggle('show-tree', on);
}
$('btnFolder').onclick = function () { showTree(true); };
$('treeBack').onclick = function () { showTree(false); };
$('treeCancel').onclick = function () { showTree(false); };

function renderTree(data) {
  treeData = data;
  var body = $('treeBody');
  body.innerHTML = '';
  if (!data || !data.nodes) {
    var empty = document.createElement('div');
    empty.className = 'tree-item';
    empty.textContent = '（无文件夹）';
    body.appendChild(empty);
    return;
  }
  addTreeItem(body, null, '未分类', 0, false);
  (data.nodes || []).forEach(function (n) { renderNode(body, n, 0); });
}

function renderNode(body, node, depth) {
  addTreeItem(body, node.id, node.name, depth, node.kind === 'archive');
  (node.children || []).forEach(function (c) { renderNode(body, c, depth + 1); });
}

function addTreeItem(body, id, name, depth, isArchive) {
  var el = document.createElement('div');
  el.className = 'tree-item' + (isArchive ? ' archive' : '') + (id === folderId ? ' on' : '');
  el.style.paddingLeft = (8 + depth * 14) + 'px';
  el.textContent = name;
  el.onclick = function () {
    folderId = id;
    folderName = name;
    $('folderValue').textContent = name;
    showTree(false);
  };
  body.appendChild(el);
}

/* —— 导出 —— */
$('btnExport').onclick = function () {
  if (exporting) return;
  if (mode === 'thumb') { exportThumb(); return; }
  var dir = (cepOut || '').replace(/\\/g, '/');
  // 多选探测（批次 3）：beginBatch 拿选区数，>1 走批量分别导出，=1 走原单选流程
  evalJsx('beginBatch()', function (r) {
    if (r.indexOf('OK:') !== 0) { log('✗ ' + r.replace('ERROR:', ''), 'err'); return; }
    var info;
    try { info = JSON.parse(r.slice(3)); } catch (e) { log('✗ 选区信息解析失败', 'err'); return; }
    if (info.count <= 1) {
      evalJsx('endBatch()', function () {});
      exportSingle(dir);
    } else {
      exportBatch(dir, info.count);
    }
  });
};

/* 单选导出（原流程）：命名框内容为准 */
function exportSingle(dir) {
  var name = $('name').value.trim();
  exporting = true;
  refreshExportBtn();
  log(dynamic ? '导出动态 WebP…' : '导出静态图…');
  evalJsx('getSelectedInfo()', function (info) {
    var fps = 24;
    var fm = info.match(/"fps":([\d.]+)/);
    if (fm) fps = parseFloat(fm[1]) || 24;
    var maxFrames = dynamic ? Math.round(settings.maxSecs * fps) : 0;
    evalJsx(
      'exportSelection(' + JSON.stringify(name) + ',' + JSON.stringify(dir) + ',' + dynamic + ',' + maxFrames + ',' + ratioW + ',' + ratioH + ',' + keepOrigin + ',' + sizeW + ',' + sizeH + ')',
      onExported(name, dir)
    );
  });
}

/* —— 批量分别导出（批次 3，PRD 07-03）：面板逐单项驱动 exportBatchItem（进度实时、jsx 快照选区），
 * 每项产物走既有 exported 单条消息入库。命名（2026-09-02 用户点题）：命名框批量期作**前缀**——
 * 空 = 各元件名（现状）；非空 = 「前缀-序号」（总数 ≥10 补零对齐排序，jsx 侧拼）。
 * 完成条件：jsx 循环结束 且 已发 WS 全部收到 importDone/Error 回执。 —— */
var batch = null; // { total, sent, done, ok, fail, jsxDone }；非 null = 批量进行中

function exportBatch(dir, total) {
  exporting = true;
  refreshExportBtn();
  var prefix = $('name').value.trim(); // 空 = 各元件名；非空 = 前缀-序号
  batch = { total: total, sent: 0, done: 0, ok: 0, fail: 0, jsxDone: false };
  log('批量导出 ' + total + ' 项（' + (prefix ? '命名 = ' + prefix + '-序号' : '各以元件名命名') + '）…');
  evalJsx('getDocInfo()', function (r0) {
    var fps = 24;
    if (r0.indexOf('OK:') === 0) { try { fps = JSON.parse(r0.slice(3)).fps || 24; } catch (e) {} }
    var maxFrames = dynamic ? Math.round(settings.maxSecs * fps) : 0;
    var i = 0;
    (function next() {
      if (i >= total) {
        batch.jsxDone = true;
        evalJsx('endBatch()', function () {});
        batchCheckDone();
        return;
      }
      var idx = i++;
      evalJsx(
        'exportBatchItem(' + idx + ',' + JSON.stringify(dir) + ',' + dynamic + ',' + maxFrames + ',' + ratioW + ',' + ratioH + ',' + keepOrigin + ',' + sizeW + ',' + sizeH + ',' + JSON.stringify(prefix) + ')',
        function (r2) { onBatchItem(idx, r2, dir); next(); }
      );
    })();
  });
}

function onBatchItem(idx, result, dir) {
  if (result.indexOf('OK:') !== 0) {
    batch.fail++;
    log('✗ (' + (idx + 1) + '/' + batch.total + ') ' + result.replace('ERROR:', ''), 'err');
    return;
  }
  var m = result.match(/fla:"(.+)\.fla",frames:(\d+),fps:([\d.]+)/);
  if (!m) {
    batch.fail++;
    log('✗ (' + (idx + 1) + '/' + batch.total + ') 回执解析失败', 'err');
    return;
  }
  var stem = m[1];
  if (!ws || ws.readyState !== 1) { // CLOSED 态 send 静默丢弃——批量中断（onWsDead 随后复位）
    batch.fail++;
    log('✗ (' + (idx + 1) + '/' + batch.total + ') ' + stem + ' 连接已断开，未发送', 'err');
    return;
  }
  batch.sent++;
  log('(' + (idx + 1) + '/' + batch.total + ') ' + stem + ' 已导出，等待入库…');
  ws.send(JSON.stringify({
    type: 'exported',
    payload: {
      name: stem,
      folderId: folderId,
      dynamic: dynamic,
      flaPath: dir + '/' + stem + '.fla',
      seqDir: dynamic ? dir + '/' + stem + '_seq' : null,
      pngPath: dynamic ? null : dir + '/' + stem + '.png',
      fps: parseFloat(m[3]) || 24
    }
  }));
}

function batchCheckDone() {
  if (!batch || !batch.jsxDone || batch.done < batch.sent) return;
  log('批量完成：' + batch.ok + ' 项入库' + (batch.fail ? '，' + batch.fail + ' 项失败（详情见上）' : '') + '，共 ' + batch.total + ' 项', batch.fail ? '' : 'ok');
  batch = null;
  exporting = false;
  refreshExportBtn();
}

/* —— 仅缩略图导出（批次 2）：当前文档画面 → PNG/序列落 cep-out → WS thumbOnly →
 * APP 按文档路径匹配库内 fla：命中 = WebP 入库绑定（已有绑定换绑）；未命中 = 挪到原文件旁边（用户定） —— */
function exportThumb() {
  var dir = (cepOut || '').replace(/\\/g, '/');
  exporting = true;
  refreshExportBtn();
  evalJsx('getDocInfo()', function (r) {
    if (r.indexOf('OK:') !== 0) return fail(r);
    var info;
    try { info = JSON.parse(r.slice(3)); } catch (e) { return fail('ERROR:文档信息解析失败'); }
    if (!info.path) return fail('ERROR:文档未保存，请先保存 fla');
    if (!/\.fla$/i.test(info.path)) return fail('ERROR:当前文档不是已保存的 fla');
    var stem = info.path.replace(/\\/g, '/').split('/').pop().replace(/\.fla$/i, '');
    var fps = info.fps || 24;
    var start = info.currentFrame || 1, end = start;
    if (dynamic) {
      start = parseInt($('rangeStart').value, 10) || 1;
      end = parseInt($('rangeEnd').value, 10) || info.frameCount;
      start = Math.max(1, Math.min(start, info.frameCount));
      end = Math.max(start, Math.min(end, info.frameCount));
      var maxFrames = Math.round(settings.maxSecs * fps);
      if (end - start + 1 > maxFrames) { // 超上限截断（PRD：全帧导出提示卡顿/崩溃风险）
        end = start + maxFrames - 1;
        log('帧数超上限，截断至 ' + start + '-' + end + '（' + maxFrames + ' 帧）');
      }
    }
    log(dynamic ? ('导出 ' + start + '-' + end + ' 帧序列…') : ('导出当前帧（第 ' + start + ' 帧）…'));
    evalJsx(
      'exportThumbFrames(' + JSON.stringify(dir) + ',' + JSON.stringify(stem) + ',' + dynamic + ',' + start + ',' + end + ')',
      function (r2) {
        if (r2.indexOf('OK:') !== 0) return fail(r2);
        log('已导出，等待处理…');
        sendPayload('thumbOnly', {
          docPath: info.path,
          dynamic: dynamic,
          seqDir: dynamic ? dir + '/' + stem + '_seq' : null,
          pngPath: dynamic ? null : dir + '/' + stem + '.png',
          fps: fps
        });
      }
    );
  });
  function fail(r) {
    exporting = false;
    refreshExportBtn();
    log('✗ ' + String(r).replace('ERROR:', ''), 'err');
  }
}

function onExported(name, dir) {
  return function (result) {
      if (result.indexOf('OK:') !== 0) {
        exporting = false;
        refreshExportBtn();
        log('✗ ' + result.replace('ERROR:', ''), 'err');
        return;
      }
      var m = result.match(/fla:"(.+)\.fla",frames:(\d+),fps:([\d.]+)/);
      if (!m) {
        exporting = false;
        refreshExportBtn();
        log('✗ 回执解析失败', 'err');
        return;
      }
      var stem = m[1];
      log('已导出，等待入库…');
      sendPayload('exported', {
        name: stem,
        folderId: folderId,
        dynamic: dynamic,
        flaPath: dir + '/' + stem + '.fla',
        seqDir: dynamic ? dir + '/' + stem + '_seq' : null,
        pngPath: dynamic ? null : dir + '/' + stem + '.png',
        fps: parseFloat(m[3]) || 24
      });
  };
}

connect();
