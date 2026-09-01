/**
 * CEP 宿主脚本（JSX）
 * 供 evalScript 调用的函数
 */

function hasSelection() {
  var doc = fl.getDocumentDOM();
  if (!doc) return 'false';
  var sel = doc.selection;
  return (sel && sel.length > 0) ? 'true' : 'false';
}

function getSelectedElementName() {
  var doc = fl.getDocumentDOM();
  if (!doc) return '';
  var sel = doc.selection;
  if (!sel || sel.length === 0) return '';
  var el = sel[0];
  if (el.name) return el.name;
  if (el.libraryItem && el.libraryItem.name) {
    var parts = el.libraryItem.name.split('/');
    return parts[parts.length - 1];
  }
  return '未命名';
}

function promptForName(defaultName) {
  if (typeof Window !== 'undefined') {
    try {
      var w = new Window('dialog', '输入素材名称');
      w.add('statictext', undefined, '请输入素材名称:');
      var input = w.add('edittext', undefined, defaultName || '');
      input.characters = 30;
      input.active = true;
      var btnGroup = w.add('group');
      var okBtn = btnGroup.add('button', undefined, '确认');
      var cancelBtn = btnGroup.add('button', undefined, '取消');
      var result = null;
      okBtn.onClick = function() { result = input.text; w.close(); };
      cancelBtn.onClick = function() { result = null; w.close(); };
      w.show();
      return result;
    } catch (e) {}
  }
  return prompt('请输入素材名称:', defaultName || '');
}

function exportToLibrary(targetPath, thumbFormat, configJson) {
  try {
    var doc = fl.getDocumentDOM();
    if (!doc) return 'ERROR: 没有打开的文档';
    var sel = doc.selection;
    if (!sel || sel.length === 0) return 'ERROR: 请选中舞台上的元件';

    var element = sel[0];

    var defaultName = '';
    if (element.name) defaultName = element.name;
    else if (element.libraryItem && element.libraryItem.name) {
      var parts = element.libraryItem.name.split('/');
      defaultName = parts[parts.length - 1];
    }
    var itemName = promptForName(defaultName);
    if (!itemName) return 'CANCEL';

    var targetURI = (targetPath.indexOf('file://') === 0) ? targetPath : FLfile.platformPathToURI(targetPath);
    if (!targetURI) return 'ERROR: 路径无效';
    if (!FLfile.exists(targetURI)) FLfile.createFolder(targetURI);

    var uncategorizedURI = targetURI + '/未分类';
    if (!FLfile.exists(uncategorizedURI)) FLfile.createFolder(uncategorizedURI);

    var baseURI = uncategorizedURI + '/' + itemName;
    var pngURI = baseURI + '.png';
    var flaURI = baseURI + '.fla';

    var gifConfig = { maxFrames: 60, fps: 15, maxWidth: 320 };
    if (configJson) {
      try { gifConfig = eval('(' + configJson + ')'); } catch (e) {}
    }

    doc.clipCopy();

    var originalDoc = doc;
    fl.createDocument();
    var tempDoc = fl.getDocumentDOM();

    tempDoc.clipPaste(true);
    tempDoc.selectAll();
    if (!tempDoc.selection || tempDoc.selection.length === 0) {
      fl.closeDocument(tempDoc, false);
      return 'ERROR: 粘贴失败';
    }

    // 记录原始 symbolType / loop（GIF 导出前会改为 graphic，导出后恢复）
    var childrenInfo = [];
    for (var k = 0; k < tempDoc.selection.length; k++) {
      var child = tempDoc.selection[k];
      childrenInfo.push({
        symbolType: child.symbolType,
        loop: child.loop
      });
      // 为了导出 GIF 序列帧，提前设为 graphic（在 group 内部也会逐帧同步）
      if (thumbFormat === 'gif' && child.symbolType !== undefined) {
        child.symbolType = 'graphic';
        child.loop = 'loop';
      }
    }

    tempDoc.group();
    var groupItem = tempDoc.selection[0];
    if (!groupItem) {
      fl.closeDocument(tempDoc, false);
      return 'ERROR: 粘贴或成组失败';
    }

    var pasteX = groupItem.x;
    var pasteY = groupItem.y;
    var pasteScaleX = groupItem.scaleX;
    var pasteScaleY = groupItem.scaleY;
    var origWidth = groupItem.width;
    var origHeight = groupItem.height;

    var stageW = tempDoc.width;
    var stageH = tempDoc.height;
    var scaleFactorX = (stageW * 0.9) / origWidth;
    var scaleFactorY = (stageH * 0.9) / origHeight;
    var finalScale = Math.min(scaleFactorX, scaleFactorY);

    groupItem.scaleX = pasteScaleX * finalScale;
    groupItem.scaleY = pasteScaleY * finalScale;
    groupItem.x += (stageW - groupItem.width) / 2 - groupItem.left;
    groupItem.y += (stageH - groupItem.height) / 2 - groupItem.top;

    var results = [];
    var gifNeedSynthesize = false;
    var gifFramesDir = '';
    var gifOutputPath = '';
    var gifFrameCount = 1;

    if (thumbFormat === 'png') {
      try {
        tempDoc.exportPNG(pngURI, true, true);
        results.push('PNG');
      } catch (e) {
        results.push('PNG失败:' + e.toString());
      }
    }

    if (thumbFormat === 'gif') {
      try {
        var frameCount = 1;
        if (element.libraryItem && element.libraryItem.timeline) {
          frameCount = element.libraryItem.timeline.frameCount;
        }

        var tl = tempDoc.getTimeline();
        var maxFrames = gifConfig.maxFrames || 60;
        var framesToExport = Math.min(frameCount, maxFrames);

        if (framesToExport > 1) {
          tl.insertFrames(framesToExport - 1);
        }

        var sep = targetURI.slice(-1) === '/' ? '' : '/';
        var tempDirBase = targetURI + sep + '_gif_frames_' + Date.now();
        FLfile.createFolder(tempDirBase);

        for (var i = 0; i < framesToExport; i++) {
          tl.currentFrame = i;
          var frameNum = i + 1;
          var frameStr = frameNum < 10 ? '000' + frameNum : frameNum < 100 ? '00' + frameNum : frameNum < 1000 ? '0' + frameNum : String(frameNum);
          var frameFile = tempDirBase + '/frame_' + frameStr + '.png';
          tempDoc.exportPNG(frameFile, true, true);
        }

        gifNeedSynthesize = true;
        gifFramesDir = tempDirBase;
        gifOutputPath = baseURI + '.gif';
        gifFrameCount = framesToExport;
      } catch (e) {
        results.push('GIF序列帧导出失败:' + e.toString());
      }
    }

    // 恢复 groupItem 到原始状态，再解散（确保子元素坐标/缩放完全无损）
    groupItem.scaleX = pasteScaleX;
    groupItem.scaleY = pasteScaleY;
    groupItem.x = pasteX;
    groupItem.y = pasteY;
    tempDoc.unGroup();

    // 恢复子元素 symbolType / loop
    tempDoc.selectAll();
    for (var k = 0; k < tempDoc.selection.length && k < childrenInfo.length; k++) {
      var el = tempDoc.selection[k];
      var info = childrenInfo[k];
      if (el.symbolType !== undefined) el.symbolType = info.symbolType;
      if (el.loop !== undefined) el.loop = info.loop;
    }

    tempDoc.selectAll();

    try {
      fl.saveDocument(tempDoc, flaURI);
      results.push('FLA');
    } catch (e) {
      results.push('FLA失败:' + e.toString());
    }

    fl.closeDocument(tempDoc, false);
    fl.setActiveWindow(originalDoc);

    if (gifNeedSynthesize) {
      var localFramesDir = FLfile.uriToPlatformPath(gifFramesDir);
      var localOutput = FLfile.uriToPlatformPath(gifOutputPath);
      var fps = gifConfig.fps || 15;
      var maxWidth = gifConfig.maxWidth || 320;
      return 'NEED_GIF|framesDir=' + localFramesDir + '|output=' + localOutput + '|fps=' + fps + '|maxWidth=' + maxWidth + '|frameCount=' + gifFrameCount;
    }

    return results.join(', ');
  } catch (e) {
    return 'ERROR: ' + e.toString();
  }
}

function importToStage(itemPath) {
  try {
    var currentDoc = fl.getDocumentDOM();
    if (!currentDoc) return 'ERROR: 没有打开的文档';

    var uri = (itemPath.indexOf('file://') === 0) ? itemPath : FLfile.platformPathToURI(itemPath);
    var flaURI = uri;
    if (uri.indexOf('.fla') === -1) {
      var parts = uri.split('/');
      var folderName = parts[parts.length - 1];
      flaURI = uri + '/' + folderName + '.fla';
    }

    if (!FLfile.exists(flaURI)) return 'ERROR: 找不到源文件 ' + flaURI;

    var sourceDoc = fl.openDocument(flaURI);
    if (!sourceDoc) return 'ERROR: 无法打开素材';

    sourceDoc.selectAll();
    sourceDoc.clipCopy();
    fl.closeDocument(sourceDoc, false);

    fl.setActiveWindow(currentDoc);
    currentDoc.clipPaste(true);

    return 'Success';
  } catch (e) {
    return 'ERROR: ' + e.toString();
  }
}

function exportThumbnail(flaPath, outputPath, format) {
  try {
    var uri = (flaPath.indexOf('file://') === 0) ? flaPath : FLfile.platformPathToURI(flaPath);
    var outUri = (outputPath.indexOf('file://') === 0) ? outputPath : FLfile.platformPathToURI(outputPath);

    var currentDoc = fl.getDocumentDOM();

    var doc = fl.openDocument(uri);
    if (!doc) return 'ERROR: 无法打开文档 ' + flaPath;

    doc.selectAll();
    if (!doc.selection || doc.selection.length === 0) {
      fl.closeDocument(doc, false);
      if (currentDoc) fl.setActiveWindow(currentDoc);
      return 'ERROR: 文档为空';
    }

    doc.clipCopy();

    fl.createDocument();
    var tempDoc = fl.getDocumentDOM();
    tempDoc.clipPaste(true);

    tempDoc.selectAll();
    tempDoc.group();
    var groupItem = tempDoc.selection[0];
    if (!groupItem) {
      fl.closeDocument(tempDoc, false);
      fl.closeDocument(doc, false);
      if (currentDoc) fl.setActiveWindow(currentDoc);
      return 'ERROR: 粘贴或成组失败';
    }

    var stageW = tempDoc.width;
    var stageH = tempDoc.height;
    var scaleFactorX = (stageW * 0.9) / groupItem.width;
    var scaleFactorY = (stageH * 0.9) / groupItem.height;
    var finalScale = Math.min(scaleFactorX, scaleFactorY, 1);

    groupItem.scaleX = finalScale;
    groupItem.scaleY = finalScale;
    groupItem.x += (stageW - groupItem.width) / 2 - groupItem.left;
    groupItem.y += (stageH - groupItem.height) / 2 - groupItem.top;

    if (format === 'png') {
      tempDoc.exportPNG(outUri, true, true);
    } else if (format === 'gif') {
      var tl = tempDoc.getTimeline();
      var frameCount = tl.frameCount;

      var sep = outUri.lastIndexOf('/');
      var outDir = outUri.slice(0, sep);
      var tempDirBase = outDir + '/_thumb_frames_' + Date.now();
      FLfile.createFolder(tempDirBase);

      var maxFrames = 60;
      var framesToExport = Math.min(frameCount, maxFrames);

      for (var i = 0; i < framesToExport; i++) {
        tl.currentFrame = i;
        var frameNum = i + 1;
        var frameStr = frameNum < 10 ? '000' + frameNum : frameNum < 100 ? '00' + frameNum : frameNum < 1000 ? '0' + frameNum : String(frameNum);
        var frameFile = tempDirBase + '/frame_' + frameStr + '.png';
        tempDoc.exportPNG(frameFile, true, true);
      }

      var localFramesDir = FLfile.uriToPlatformPath(tempDirBase);
      var localOutput = FLfile.uriToPlatformPath(outUri);

      fl.closeDocument(tempDoc, false);
      fl.closeDocument(doc, false);
      if (currentDoc) fl.setActiveWindow(currentDoc);

      return 'NEED_GIF|framesDir=' + localFramesDir + '|output=' + localOutput + '|fps=15|maxWidth=320|frameCount=' + framesToExport;
    }

    fl.closeDocument(tempDoc, false);
    fl.closeDocument(doc, false);
    if (currentDoc) fl.setActiveWindow(currentDoc);

    return 'SUCCESS';
  } catch (e) {
    return 'ERROR: ' + e.toString();
  }
}
