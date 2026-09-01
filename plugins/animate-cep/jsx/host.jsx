/* AniKit CEP 宿主脚本（供 evalScript 调用）
 * 选中读取 / 复制粘贴到新文档 / 缩放对齐舞台中央（仅缩略图画面；fla 坐标按对齐档位：保持坐标 =
 * 导出后还原，合适舞台 = 居中态直存）/ PNG 与逐帧序列导出（movieclip 转 graphic 逐帧同步）/
 * fla 另存 / 中文命名原生输入框 / 仅缩略图直导（批次 2）/ 多选批量分别导出（批次 3）。
 * 返回值恒为字符串（evalScript 契约）：'OK:...' / 'ERROR:...' / 'CANCEL'。
 */

/* 中文命名输入框（CEF 面板 HTML input 打不了中文 = CEF IME 顽疾）：
 * JSFL 原生 prompt() 对话框（Animate 内置、走系统 IME）——ScriptUI 在 Animate 的
 * ExtendScript 引擎不存在（Window is not defined，2026-09-01 实机报错钉死；
 * 用户旧项目同款 prompt() 回退路径）。确认返回文本、取消返回 'CANCEL'
 *（2026-09-01 定案，面板「中」辅助钮调用，回填 HTML 命名框） */
function promptForName(defaultName) {
  try {
    var r = prompt('输入素材名称：', defaultName || '');
    return r === null ? 'CANCEL' : r;
  } catch (e) {
    return 'ERROR:' + e;
  }
}

function getSelectedInfo() {
  var doc = fl.getDocumentDOM();
  if (!doc) return 'ERROR:没有打开的文档';
  var sel = doc.selection;
  if (!sel || sel.length === 0) return 'ERROR:请先选中舞台上的元件';
  var el = sel[0];
  var info = {
    elementType: el.elementType,
    instanceName: el.name || '',
    libraryItem: el.libraryItem ? el.libraryItem.name : null,
    frameCount: el.libraryItem ? el.libraryItem.timeline.frameCount : 0,
    width: el.width, height: el.height, x: el.x, y: el.y,
    docW: doc.width, docH: doc.height, fps: doc.frameRate
  };
  return 'OK:' + toJson(info);
}

function toJson(o) {
  // jsx 无 JSON.stringify（ES3），手写最简序列化（字符串转义反斜杠与引号——文档路径含反斜杠，批次 2 起需要）
  var parts = [];
  for (var k in o) {
    var v = o[k];
    if (v === null) parts.push('"' + k + '":null');
    else if (typeof v === 'string') parts.push('"' + k + '":"' + v.replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '"');
    else parts.push('"' + k + '":' + v);
  }
  return '{' + parts.join(',') + '}';
}

function pad4(n) {
  return n < 10 ? '000' + n : n < 100 ? '00' + n : n < 1000 ? '0' + n : String(n);
}

function ensureDir(uri) {
  if (!FLfile.exists(uri)) FLfile.createFolder(uri);
}

/* 清空目录内 png（防同名重导时上次残留帧混入——元件导出与仅缩略图共用，2026-09-02） */
function cleanSeqDir(uri) {
  if (!FLfile.exists(uri)) return;
  var files = FLfile.listFolder(uri, 'files');
  if (!files) return;
  for (var i = 0; i < files.length; i++) {
    if (/\.png$/i.test(files[i])) FLfile.remove(uri + '/' + files[i]);
  }
}

/* —— 批次 2：仅缩略图（不导新 fla，PRD 07-03 手动兜底条） —— */

/* 当前文档信息：path = 平台路径（未保存 = 空串）；frameCount / fps / currentFrame（1-based） */
function getDocInfo() {
  var doc = fl.getDocumentDOM();
  if (!doc) return 'ERROR:没有打开的文档';
  var path = '';
  try { if (doc.pathURI) path = FLfile.uriToPlatformPath(doc.pathURI); } catch (e) {}
  var tl = doc.getTimeline();
  return 'OK:' + toJson({ path: path, frameCount: tl.frameCount, fps: doc.frameRate, currentFrame: tl.currentFrame + 1 });
}

/* 仅缩略图导出：不动文档内容（不复制元件、不缩放对齐——用户已自行调好画面，PRD 手动兜底口径），
 * 直接导当前文档画面。dynamic=false → 当前播放头帧单帧 PNG；
 * true → 主时间轴 [startFrame, endFrame]（1-based 含端点，越界 clamp）逐帧 PNG 序列，播放头导出后还原。
 * 注意：主时间轴上的 movieclip 实例不随播放头逐帧推进（Animate 天性），嵌套动画请用元件导出（新文件模式）。 */
function exportThumbFrames(outDir, name, dynamic, startFrame, endFrame) {
  var doc = fl.getDocumentDOM();
  if (!doc) return 'ERROR:没有打开的文档';
  var outURI = FLfile.platformPathToURI(outDir);
  if (!outURI) return 'ERROR:路径无效';
  ensureDir(outURI);
  var base = outURI + '/' + name;
  var tl = doc.getTimeline();
  var total = tl.frameCount;
  var origFrame = tl.currentFrame;
  var frames = 1;
  try {
    if (!dynamic) {
      doc.exportPNG(base + '.png', true, true); // 当前播放头帧
    } else {
      var s = Math.max(1, Math.floor(startFrame));
      var e = Math.min(total, Math.floor(endFrame));
      if (e < s) return 'ERROR:帧范围无效（' + startFrame + '-' + endFrame + '，共 ' + total + ' 帧）';
      frames = e - s + 1;
      var seqBase = outURI + '/' + name + '_seq';
      ensureDir(seqBase);
      cleanSeqDir(seqBase);
      for (var i = 0; i < frames; i++) {
        tl.currentFrame = s - 1 + i;
        doc.exportPNG(seqBase + '/f_' + pad4(i + 1) + '.png', true, true);
      }
      tl.currentFrame = origFrame; // 还原播放头
    }
  } catch (e) {
    tl.currentFrame = origFrame;
    return 'ERROR:PNG 导出失败 ' + e;
  }
  return 'OK:{fla:"' + name + '",frames:' + frames + ',fps:' + doc.frameRate + '}';
}

/* 导出主流程：name 空串 = 取元件名；outDir = 平台路径（正斜杠）；dynamic = 是否导 PNG 序列；
 * maxFrames = 动态帧数上限（APP 设置下发 = maxSecs × fps；≤0 兜底 240）；
 * ratioW/ratioH = 舞台比例（宽:高；任一 ≤0 = 跟随当前文档原尺寸）。
 * 比例基准（2026-09-02 用户定）：高度跟随当前文档，宽 = 高 × ratioW/ratioH（四舍五入取整）。
 * keepOrigin = 对齐方式（2026-09-02 用户定）：false = **合适舞台**——缩放居中态直接保存
 *（拥挤 fla 拆素材场景，拆出来元件即在舞台中央）；缺省/true = **保持坐标**——导出后还原
 * 坐标再保存（fla 保原坐标，与快速发送原位粘贴联动，PRD 07-03/04）。
 * sizeW/sizeH = 舞台像素直定（2026-09-02 用户定「直接传像素建文档」；均 >0 生效，优先于比例档）。
 * 口径（PRD 07-03）：缩放居中只为缩略图画面，导出后还原坐标再保存 fla（保原坐标）。 */
function exportSelection(name, outDir, dynamic, maxFrames, ratioW, ratioH, keepOrigin, sizeW, sizeH) {
  var doc = fl.getDocumentDOM();
  if (!doc) return 'ERROR:没有打开的文档';
  var sel = doc.selection;
  if (!sel || sel.length === 0) return 'ERROR:请先选中舞台上的元件';
  if (sel.length > 1) return 'ERROR:spike 只支持单选';
  var el = sel[0];
  if (el.elementType !== 'instance') return 'ERROR:选中的不是元件实例（' + el.elementType + '）';
  var item = el.libraryItem;
  if (!item) return 'ERROR:元件实例无库引用';

  if (!name) {
    var parts = item.name.split('/');
    name = parts[parts.length - 1];
  }

  var outURI = FLfile.platformPathToURI(outDir);
  if (!outURI) return 'ERROR:路径无效';
  ensureDir(outURI);
  var base = outURI + '/' + name;
  var flaURI = base + '.fla';
  var pngURI = base + '.png';

  var W = doc.width, H = doc.height;          // 舞台尺寸：默认跟随当前文档
  if (sizeW > 0 && sizeH > 0) {
    W = Math.max(1, Math.round(sizeW));       // 像素档：直定尺寸（优先）
    H = Math.max(1, Math.round(sizeH));
  } else if (ratioW > 0 && ratioH > 0) {
    W = Math.max(1, Math.round(H * ratioW / ratioH)); // 比例档：高随文档、宽 = 高 × 比例
  }
  var frameCount = item.timeline.frameCount;  // 元件时间轴帧数
  var MAX_FRAMES = (maxFrames && maxFrames > 0) ? Math.floor(maxFrames) : 240; // APP 设置下发，兜底 10s@24fps
  var frames = dynamic ? Math.min(frameCount, MAX_FRAMES) : 1;

  // ① 复制元件
  doc.selectNone();
  doc.selection = [el];
  doc.clipCopy();
  doc.selectNone();

  // ② 新建文档（舞台 = 高随文档、宽按比例）+ 原位粘贴
  var origDoc = doc;
  fl.createDocument();
  var nd = fl.getDocumentDOM();
  nd.width = W; nd.height = H;
  nd.clipPaste(true); // true = 原位粘贴
  nd.selectAll();
  if (!nd.selection || nd.selection.length === 0) {
    fl.closeDocument(nd, false);
    return 'ERROR:粘贴失败';
  }
  var p = nd.selection[0];

  // ③ 动态导出前置：movieclip 不随主时间轴播放头逐帧推进，转 graphic 才逐帧同步（旧项目结论）
  var origSymbolType = p.symbolType, origLoop = p.loop;
  if (dynamic && frames > 1 && p.symbolType !== undefined) {
    p.symbolType = 'graphic';
    p.loop = 'loop';
  }

  // ④ 记录原坐标，等比缩放至舞台短边 90% 并居中（left/top = 包围盒左上舞台坐标）
  var origX = p.x, origY = p.y, origSX = p.scaleX, origSY = p.scaleY;
  var s = Math.min((W * 0.9) / p.width, (H * 0.9) / p.height);
  p.scaleX = origSX * s;
  p.scaleY = origSY * s;
  p.x += (W - p.width) / 2 - p.left;
  p.y += (H - p.height) / 2 - p.top;

  // ⑤ 导出 PNG（第三参 true = 当前播放头帧）
  try {
    if (!dynamic || frames <= 1) {
      nd.exportPNG(pngURI, true, true);
    } else {
      var tl = nd.getTimeline();
      tl.insertFrames(frames - 1); // 当前帧(0)处追加，主时间轴扩展到 frames 帧
      var seqBase = outURI + '/' + name + '_seq';
      ensureDir(seqBase);
      cleanSeqDir(seqBase); // 同名重导防上次残留帧混入
      for (var i = 0; i < frames; i++) {
        tl.currentFrame = i;
        nd.exportPNG(seqBase + '/f_' + pad4(i + 1) + '.png', true, true);
      }
    }
  } catch (e) {
    fl.closeDocument(nd, false);
    fl.setActiveWindow(origDoc);
    return 'ERROR:PNG 导出失败 ' + e;
  }

  // ⑥ 坐标与缩放还原（仅 keepOrigin 保持坐标档）；合适舞台档 = 缩放居中态直接保存不还原。
  // symbolType/loop 恢复恒做（元件语义与坐标无关——动态导出 movieclip→graphic 是导帧用临时态）
  if (keepOrigin !== false) {
    p.scaleX = origSX; p.scaleY = origSY;
    p.x = origX; p.y = origY;
  }
  if (dynamic && frames > 1 && origSymbolType !== undefined) {
    p.symbolType = origSymbolType;
    p.loop = origLoop;
  }

  // ⑦ 保存 fla → 关闭不保存后续变化 → 焦点切回原文档
  try {
    fl.saveDocument(nd, flaURI);
  } catch (e) {
    fl.closeDocument(nd, false);
    fl.setActiveWindow(origDoc);
    return 'ERROR:FLA 保存失败 ' + e;
  }
  fl.closeDocument(nd, false);
  fl.setActiveWindow(origDoc);

  return 'OK:{fla:"' + name + '.fla",frames:' + frames + ',fps:' + doc.frameRate + '}';
}

/* —— 批次 3（2026-09-02）：多选批量分别导出（PRD 07-03「多选：将所选项分别导出 fla」）——
 * 面板逐单项驱动（每项 = 一次 exportSelection 单选流程，进度实时回传可中断）；
 * jsx 侧快照多选列表（全局引用跨 evalScript 有效），逐项设单选后复用 exportSelection。
 * 命名 = 各元件库内名（批量不听命名框）；非元件实例项占位跳过；快照期元素失效（文档切换/删除）报错跳过。 —— */
var __batchSel = null;

/* 开始批量：记录当前舞台多选快照。返回 count 与 names（| 分隔，非元件实例 = 空串占位） */
function beginBatch() {
  var doc = fl.getDocumentDOM();
  if (!doc) return 'ERROR:没有打开的文档';
  var sel = doc.selection;
  if (!sel || sel.length === 0) return 'ERROR:请先在舞台上选中元件（可多选）';
  __batchSel = [];
  var names = [];
  for (var i = 0; i < sel.length; i++) {
    var el = sel[i];
    if (el.elementType === 'instance' && el.libraryItem) {
      __batchSel.push(el);
      names.push(el.libraryItem.name.split('/').pop());
    } else {
      __batchSel.push(null); // 占位：导出时报跳过
      names.push('');
    }
  }
  return 'OK:' + toJson({ count: sel.length, names: names.join('|') });
}

/* 批量导第 index 项（0-based）：设单选 → 复用 exportSelection。
 * namePrefix（2026-09-02 用户点题批量命名）：空 = 元件名；非空 = 「前缀-序号」（总数 ≥10 补零对齐排序） */
function exportBatchItem(index, outDir, dynamic, maxFrames, ratioW, ratioH, keepOrigin, sizeW, sizeH, namePrefix) {
  if (!__batchSel) return 'ERROR:批量会话未开始（beginBatch 未调）';
  if (index < 0 || index >= __batchSel.length) return 'ERROR:批量序号越界（' + index + '）';
  var el = __batchSel[index];
  if (!el) return 'ERROR:第 ' + (index + 1) + ' 项不是元件实例，跳过';
  var doc = fl.getDocumentDOM();
  if (!doc) return 'ERROR:没有打开的文档';
  try { var t = el.elementType; t = t + ''; } catch (e) {
    return 'ERROR:第 ' + (index + 1) + ' 项已失效（选区/文档已变）';
  }
  doc.selectNone();
  doc.selection = [el];
  var name = '';
  if (namePrefix) {
    var s = String(index + 1);
    var digits = String(__batchSel.length).length;
    while (s.length < digits) s = '0' + s;
    name = namePrefix + '-' + s;
  }
  return exportSelection(name, outDir, dynamic, maxFrames, ratioW, ratioH, keepOrigin, sizeW, sizeH);
}

/* 结束批量：清快照 */
function endBatch() {
  __batchSel = null;
  return 'OK';
}
