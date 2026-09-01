/**
 * 核心导出脚本
 * ECMAScript 3 only. No let/const/arrow functions/template literals.
 */

function hasSelection() {
  var doc = fl.getDocumentDOM();
  if (!doc) return 'false';
  var selection = doc.selection;
  return (selection && selection.length > 0) ? 'true' : 'false';
}

function getSelectedElementName() {
  var doc = fl.getDocumentDOM();
  if (!doc) return '';
  var selection = doc.selection;
  if (!selection || selection.length === 0) return '';
  var element = selection[0];
  if (element.name) return element.name;
  if (element.libraryItem && element.libraryItem.name) {
    var parts = element.libraryItem.name.split('/');
    return parts[parts.length - 1];
  }
  return '未命名';
}

function exportToLibrary(jsonArgs) {
  var args;
  try {
    args = JSON.parse(jsonArgs);
  } catch (e) {
    return 'ERROR: 参数解析失败';
  }

  var doc = fl.getDocumentDOM();
  if (!doc) return 'ERROR: 没有打开的文档';

  var selection = doc.selection;
  if (!selection || selection.length === 0) {
    return 'ERROR: 请选中舞台上的元件';
  }

  var element = selection[0];

  // 1. 获取原始信息
  var origX = element.x;
  var origY = element.y;
  var origScaleX = element.scaleX;
  var origScaleY = element.scaleY;
  var origRotation = element.rotation || 0;
  var origWidth = element.width;
  var origHeight = element.height;

  // 2. 复制元件
  doc.clipCopy();

  // 3. 新建临时文档
  var originalDoc = doc;
  fl.createDocument();
  var tempDoc = fl.getDocumentDOM();

  // 4. 原位粘贴
  tempDoc.clipPaste(true);
  var pasted = tempDoc.selection[0];
  if (!pasted) {
    fl.closeDocument(tempDoc, false);
    return 'ERROR: 粘贴失败';
  }

  // 记录原始锚点位置
  var anchorX = pasted.x;
  var anchorY = pasted.y;

  // 5. 适配舞台可视区（缩放至不超出舞台）
  var stageW = tempDoc.width;
  var stageH = tempDoc.height;
  var margin = 20;
  var maxW = stageW - margin * 2;
  var maxH = stageH - margin * 2;

  var scale = 1;
  if (origWidth > maxW || origHeight > maxH) {
    var scaleW = maxW / origWidth;
    var scaleH = maxH / origHeight;
    scale = scaleW < scaleH ? scaleW : scaleH;
  }

  pasted.scaleX = origScaleX * scale;
  pasted.scaleY = origScaleY * scale;
  pasted.x = stageW / 2;
  pasted.y = stageH / 2;

  // 6. 获取导出目录（从宿主应用传入，或通过其他方式获取）
  // 这里使用固定的素材库路径，实际应用中应由 Electron 应用传入
  var libPath = '';
  try {
    var envFile = FLfile.platformPathToURI(fl.configURI) + 'animate-asset-manager-path.txt';
    if (FLfile.exists(envFile)) {
      libPath = FLfile.read(envFile).replace(/\\/g, '/');
    }
  } catch (e) {}

  if (!libPath) {
    fl.closeDocument(tempDoc, false);
    return 'ERROR: 未配置素材库路径';
  }

  var baseName = args.name;
  var outDir = libPath;
  if (outDir.charAt(outDir.length - 1) !== '/') {
    outDir += '/';
  }

  var results = [];

  // 6. 导出 PNG 缩略图
  if (args.exportPng) {
    var pngPath = outDir + baseName + '.png';
    var pngUri = FLfile.platformPathToURI(pngPath);
    try {
      tempDoc.exportPNG(pngUri, true, true);
      results.push('PNG');
    } catch (e) {
      results.push('PNG失败:' + e);
    }
  }

  // 7. 导出 SWF 预览
  if (args.exportSwf) {
    var swfPath = outDir + baseName + '.swf';
    var swfUri = FLfile.platformPathToURI(swfPath);
    try {
      tempDoc.exportSWF(swfUri, true);
      results.push('SWF');
    } catch (e) {
      results.push('SWF失败:' + e);
    }
  }

  // 8. 撤销至原位粘贴状态（恢复原始坐标和缩放）
  tempDoc.undo();

  // 9. 保存 .fla
  var flaPath = outDir + baseName + '.fla';
  var flaUri = FLfile.platformPathToURI(flaPath);
  try {
    tempDoc.saveAs(flaUri);
    results.push('FLA');
  } catch (e) {
    results.push('FLA失败:' + e);
  }

  // 10. 写入元数据
  var metaPath = outDir + baseName + '.meta.json';
  var metaUri = FLfile.platformPathToURI(metaPath);
  var meta = {
    x: origX,
    y: origY,
    scaleX: origScaleX,
    scaleY: origScaleY,
    rotation: origRotation,
    width: origWidth,
    height: origHeight,
    anchorX: anchorX,
    anchorY: anchorY
  };
  try {
    FLfile.write(metaUri, JSON.stringify(meta, null, 2));
    results.push('META');
  } catch (e) {
    results.push('META失败:' + e);
  }

  // 11. 关闭临时文档（不保存 Animate 原生格式变更）
  fl.closeDocument(tempDoc, false);

  // 恢复原始文档为活动文档
  fl.setActiveWindow(originalDoc);

  return results.join(', ');
}
