/* AniKit CEP spike —— 宿主脚本（供 evalScript 调用）
 * 验证点：选中读取 / 复制粘贴到新文档 / 缩放对齐舞台中央（仅缩略图画面，fla 保原坐标）/
 *         PNG 与逐帧序列导出（movieclip 转 graphic 逐帧同步）/ fla 另存。
 * 返回值恒为字符串（evalScript 契约）：'OK:...' / 'ERROR:...'。
 */

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
  // jsx 无 JSON.stringify（ES3），手写最简序列化
  var parts = [];
  for (var k in o) {
    var v = o[k];
    if (v === null) parts.push('"' + k + '":null');
    else if (typeof v === 'string') parts.push('"' + k + '":"' + v.replace(/"/g, '\\"') + '"');
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

/* 导出主流程：name 空串 = 取元件名；outDir = 平台路径（正斜杠）；dynamic = 是否导 PNG 序列
 * 口径（PRD 07-03）：缩放居中只为缩略图画面，导出后还原坐标再保存 fla（保原坐标）。 */
function exportSelection(name, outDir, dynamic) {
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

  var W = doc.width, H = doc.height;          // 舞台尺寸跟随当前文档
  var frameCount = item.timeline.frameCount;  // 元件时间轴帧数
  var MAX_FRAMES = 240;                        // spike 固定 10s@24fps 档
  var frames = dynamic ? Math.min(frameCount, MAX_FRAMES) : 1;

  // ① 复制元件
  doc.selectNone();
  doc.selection = [el];
  doc.clipCopy();
  doc.selectNone();

  // ② 新建文档（舞台同尺寸）+ 原位粘贴
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

  // ⑥ 还原坐标与缩放（fla 保原坐标）+ 恢复 symbolType/loop
  p.scaleX = origSX; p.scaleY = origSY;
  p.x = origX; p.y = origY;
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
