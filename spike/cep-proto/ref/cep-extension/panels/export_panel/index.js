var fs = require('fs');
var path = require('path');
var csInterface = new CSInterface();

var exportBtn = document.getElementById('exportBtn');
var statusDiv = document.getElementById('status');

function setStatus(text, type) {
  statusDiv.textContent = text;
  statusDiv.className = 'status status-' + (type || 'info');
}

// 不再使用轮询检查选择状态，改为点击导出按钮时一次性检查
// 避免频繁 evalScript 干扰 Animate 其他面板输入
function checkSelection(onDone) {
  csInterface.evalScript('hasSelection()', function(result) {
    onDone(result === 'true');
  });
}

function escapeForScript(str) {
  return str.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

function getLibraryPath() {
  try {
    var extPath = csInterface.getSystemPath('extension');
    var pathFile = extPath + '/libpath.txt';
    if (fs.existsSync(pathFile)) {
      return fs.readFileSync(pathFile, 'utf-8').trim();
    }
  } catch (e) {
    console.log('读取素材库路径失败:', e);
  }
  return '';
}

function getFfmpegPath(libPath) {
  try {
    // 1. 优先素材库根目录 .ffmpeg-path
    var ffmpegPathFile = path.join(libPath, '.ffmpeg-path');
    if (fs.existsSync(ffmpegPathFile)) {
      var p = fs.readFileSync(ffmpegPathFile, 'utf-8').trim();
      if (p && fs.existsSync(p)) return p;
    }
  } catch (e) {}

  try {
    // 2. 固定位置
    var appData = process.env.APPDATA || path.join(process.env.USERPROFILE, 'AppData', 'Roaming');
    var fixedPath = path.join(appData, 'animate-asset-manager', 'tools', 'ffmpeg.exe');
    if (fs.existsSync(fixedPath)) return fixedPath;
  } catch (e) {}

  try {
    // 3. 扩展目录下的 ffmpeg.exe
    var extPath = csInterface.getSystemPath('extension');
    var extFfmpeg = path.join(extPath, 'ffmpeg.exe');
    if (fs.existsSync(extFfmpeg)) return extFfmpeg;
  } catch (e) {}

  return '';
}

function getGifConfig() {
  try {
    var extPath = csInterface.getSystemPath('extension');
    var configPath = path.join(extPath, 'gif-config.json');
    if (fs.existsSync(configPath)) {
      return JSON.parse(fs.readFileSync(configPath, 'utf-8'));
    }
  } catch (e) {
    console.log('读取 GIF 配置失败:', e);
  }
  return { maxWidth: 320, fps: 15, maxFrames: 60, maxFileSizeMB: 2 };
}

function parseNeedGif(resultStr) {
  var parts = resultStr.split('|');
  var params = {};
  for (var i = 1; i < parts.length; i++) {
    var idx = parts[i].indexOf('=');
    if (idx > 0) {
      var key = parts[i].slice(0, idx);
      var value = parts[i].slice(idx + 1);
      params[key] = value;
    }
  }
  return params;
}

function cleanFrames(framesDir, frameCount) {
  try {
    for (var j = 0; j < frameCount; j++) {
      var delNum = j + 1;
      var delStr = delNum < 10 ? '000' + delNum : delNum < 100 ? '00' + delNum : delNum < 1000 ? '0' + delNum : String(delNum);
      var delFile = path.join(framesDir, 'frame_' + delStr + '.png');
      if (fs.existsSync(delFile)) {
        fs.unlinkSync(delFile);
      }
    }
    fs.rmdirSync(framesDir);
  } catch (e) {
    console.log('清理临时帧失败:', e);
  }
}

function synthesizeGif(params, onDone) {
  var libPath = getLibraryPath();
  var ffmpegPath = getFfmpegPath(libPath);

  if (!ffmpegPath) {
    onDone('GIF失败: 找不到 FFmpeg，请在设置中重新安装导出面板');
    return;
  }

  var framesDir = params.framesDir || '';
  var outputPath = params.output || '';
  var frameCount = parseInt(params.frameCount || '1', 10);
  var config = getGifConfig();
  var fps = parseInt(params.fps || String(config.fps), 10);
  var maxWidth = parseInt(params.maxWidth || String(config.maxWidth), 10);

  var framePattern = path.join(framesDir, 'frame_%04d.png');
  var vf = 'fps=' + fps + ',scale=' + maxWidth + ':-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse';

  var spawn = require('child_process').spawn;
  var args = [
    '-y',
    '-i', framePattern,
    '-vf', vf,
    '-loop', '0',
    outputPath
  ];

  console.log('FFmpeg cmd:', ffmpegPath, args.join(' '));

  var proc = spawn(ffmpegPath, args, { windowsHide: true });
  var stderrBuf = '';

  proc.stderr.on('data', function(data) {
    stderrBuf += data.toString();
  });

  proc.on('error', function(err) {
    console.error('FFmpeg spawn error:', err);
    cleanFrames(framesDir, frameCount);
    onDone('GIF失败: ' + err.message);
  });

  proc.on('close', function(code) {
    console.log('FFmpeg exit code:', code);
    if (code === 0 && fs.existsSync(outputPath)) {
      cleanFrames(framesDir, frameCount);
      onDone(null, 'GIF');
    } else {
      // 写调试日志到素材库根目录
      try {
        var debugPath = path.join(libPath, 'ffmpeg-debug.txt');
        var debugContent = 'CMD:\n' + ffmpegPath + ' ' + args.join(' ') + '\n\nSTDERR:\n' + stderrBuf + '\n\nEXIT CODE:\n' + code;
        fs.writeFileSync(debugPath, debugContent, 'utf-8');
      } catch (e) {}
      cleanFrames(framesDir, frameCount);
      onDone('GIF失败: FFmpeg 退出码 ' + code + '，详见 ffmpeg-debug.txt');
    }
  });
}

// 保存和恢复面板大小（切回桌面后防止重置为 manifest 默认值）
function savePanelSize() {
  try {
    localStorage.setItem('aam_panel_w', String(window.innerWidth));
    localStorage.setItem('aam_panel_h', String(window.innerHeight));
    console.log('[CEP] Saved panel size:', window.innerWidth, 'x', window.innerHeight);
  } catch (e) {
    console.log('[CEP] Save size failed:', e);
  }
}

function restorePanelSize() {
  try {
    var w = localStorage.getItem('aam_panel_w');
    var h = localStorage.getItem('aam_panel_h');
    console.log('[CEP] Restore size from localStorage:', w, 'x', h);
    if (w && h) {
      var iw = parseInt(w, 10);
      var ih = parseInt(h, 10);
      csInterface.resizeContent(iw, ih);
      console.log('[CEP] Called resizeContent:', iw, 'x', ih);
    } else {
      console.log('[CEP] No saved size found');
    }
  } catch (e) {
    console.log('[CEP] Restore size failed:', e);
  }
}

window.addEventListener('resize', function() {
  clearTimeout(window._resizeTimer);
  window._resizeTimer = setTimeout(savePanelSize, 300);
});

// 初始状态：按钮可用，提示用户先选中元件
exportBtn.disabled = false;
setStatus('请选中舞台上的元件，然后点击导出', 'info');

// 脚本在 body 末尾加载，DOM 已就绪，直接恢复面板尺寸
restorePanelSize();

exportBtn.addEventListener('click', function() {
  var libPath = getLibraryPath();
  if (!libPath) {
    setStatus('未配置素材库路径，请先在主程序设置面板中安装 CEP 扩展', 'error');
    return;
  }

  // 点击时才检查选择状态，彻底消除轮询干扰
  checkSelection(function(hasSelection) {
    if (!hasSelection) {
      setStatus('请选中舞台上的元件后再导出', 'error');
      return;
    }

    var thumbFormat = document.querySelector('input[name="thumbFormat"]:checked').value;

    setStatus('正在导出...', 'info');
    exportBtn.disabled = true;

    var safePath = escapeForScript(libPath);
    var configJson = escapeForScript(JSON.stringify(getGifConfig()));
    var script = 'exportToLibrary("' + safePath + '", "' + thumbFormat + '", "' + configJson + '")';

    csInterface.evalScript(script, function(result) {
      exportBtn.disabled = false;

      var resStr = String(result || '');

      if (resStr === 'CANCEL') {
        setStatus('已取消', 'info');
        return;
      }

      if (!resStr || resStr === 'undefined' || resStr === 'null') {
        setStatus('导出失败：未返回结果', 'error');
        return;
      }

      if (resStr === 'EvalScript error.' || resStr.indexOf('ERROR') === 0) {
        setStatus('导出失败：' + resStr, 'error');
        return;
      }

      // 处理需要 FFmpeg 合成 GIF 的情况
      if (resStr.indexOf('NEED_GIF') === 0) {
        var gifParams = parseNeedGif(resStr);
        setStatus('正在合成 GIF...', 'info');
        synthesizeGif(gifParams, function(err, successMsg) {
          if (err) {
            setStatus(err, 'error');
          } else {
            setStatus('导出成功：FLA, META, ' + successMsg, 'success');
          }
        });
        return;
      }

      setStatus('导出成功：' + resStr, 'success');
    });
  });
});

// 轮询 Electron 发来的导入到舞台请求（低频，不影响面板输入）
setInterval(function() {
  try {
    var os = require('os');
    var requestFile = path.join(os.tmpdir(), 'animate-asset-manager-import.json');
    if (!fs.existsSync(requestFile)) return;

    var content = fs.readFileSync(requestFile, 'utf-8');
    var request = JSON.parse(content);
    fs.unlinkSync(requestFile);

    var safePath = escapeForScript(request.path);
    csInterface.evalScript('importToStage("' + safePath + '")', function(result) {
      var resStr = String(result || '');
      if (resStr.indexOf('ERROR') === 0) {
        console.log('导入到舞台失败:', resStr);
      } else {
        console.log('导入到舞台成功');
      }
    });
  } catch (e) {
    console.log('轮询导入请求失败:', e);
  }
}, 2000);

// 调试日志（写到素材库根目录的 cep-debug.txt）
function writeDebugLog(msg) {
  try {
    var libPath = getLibraryPath();
    if (!libPath) return;
    var debugPath = path.join(libPath, 'cep-debug.txt');
    var timestamp = new Date().toISOString();
    var line = '[' + timestamp + '] ' + msg + '\n';
    fs.appendFileSync(debugPath, line, 'utf-8');
  } catch (e) {}
}

// 批量缩略图任务队列轮询
setInterval(function() {
  try {
    var os = require('os');
    var tasksFile = path.join(os.tmpdir(), 'animate-asset-manager-tasks.json');
    if (!fs.existsSync(tasksFile)) return;

    var content = fs.readFileSync(tasksFile, 'utf-8');
    var data = JSON.parse(content);

    if (data.cancelled) return;
    if (!data.tasks || data.tasks.length === 0) return;

    // 检查是否有任务正在处理中
    if (data.currentIndex >= 0 && data.tasks[data.currentIndex].status === 'processing') {
      // 超时检查：60秒
      var startedAt = data.tasks[data.currentIndex].startedAt || 0;
      if (startedAt && (Date.now() - startedAt > 60000)) {
        writeDebugLog('任务超时: ' + data.tasks[data.currentIndex].flaPath);
        data.tasks[data.currentIndex].status = 'error';
        data.tasks[data.currentIndex].error = 'ERROR: 处理超时（超过60秒）';
        fs.writeFileSync(tasksFile, JSON.stringify(data, null, 2), 'utf-8');
      }
      return;
    }

    // 如果最后一个任务已完成或出错，全部结束
    if (data.currentIndex >= data.tasks.length - 1) {
      var lastTask = data.tasks[data.currentIndex];
      if (lastTask && lastTask.status !== 'pending') {
        writeDebugLog('全部任务已完成/出错');
        return;
      }
    }

    // 找到下一个待处理任务
    var nextIndex = data.currentIndex + 1;
    var task = data.tasks[nextIndex];
    if (!task || task.status !== 'pending') return;

    // 标记为处理中
    data.currentIndex = nextIndex;
    task.status = 'processing';
    task.startedAt = Date.now();
    fs.writeFileSync(tasksFile, JSON.stringify(data, null, 2), 'utf-8');

    writeDebugLog('开始处理: ' + task.flaPath + ' 格式:' + task.format);

    var safeFlaPath = escapeForScript(task.flaPath);
    var safeOutput = escapeForScript(task.outputPath);
    var script = 'exportThumbnail("' + safeFlaPath + '", "' + safeOutput + '", "' + task.format + '")';

    csInterface.evalScript(script, function(result) {
      try {
        var resStr = String(result || '');
        writeDebugLog('JSFL返回: ' + resStr.slice(0, 200));

        var content2 = fs.readFileSync(tasksFile, 'utf-8');
        var data2 = JSON.parse(content2);
        var currentTask = data2.tasks[data2.currentIndex];
        if (!currentTask) return;

        if (resStr.indexOf('NEED_GIF') === 0) {
          var gifParams = parseNeedGif(resStr);
          writeDebugLog('需要合成GIF: ' + gifParams.output);
          synthesizeGif(gifParams, function(err, successMsg) {
            try {
              var content3 = fs.readFileSync(tasksFile, 'utf-8');
              var data3 = JSON.parse(content3);
              var t = data3.tasks[data3.currentIndex];
              if (t) {
                if (err) {
                  t.status = 'error';
                  t.error = err;
                  writeDebugLog('GIF合成失败: ' + err);
                } else {
                  t.status = 'done';
                  writeDebugLog('GIF合成成功');
                }
                fs.writeFileSync(tasksFile, JSON.stringify(data3, null, 2), 'utf-8');
              }
            } catch (e) {
              writeDebugLog('更新GIF任务状态失败: ' + e.message);
            }
          });
        } else if (resStr.indexOf('ERROR') === 0 || resStr === 'EvalScript error.') {
          currentTask.status = 'error';
          currentTask.error = resStr;
          fs.writeFileSync(tasksFile, JSON.stringify(data2, null, 2), 'utf-8');
          writeDebugLog('任务出错: ' + resStr);
        } else {
          currentTask.status = 'done';
          fs.writeFileSync(tasksFile, JSON.stringify(data2, null, 2), 'utf-8');
          writeDebugLog('任务完成: ' + resStr);
        }
      } catch (e) {
        writeDebugLog('回调处理失败: ' + e.message);
      }
    });
  } catch (e) {
    writeDebugLog('轮询异常: ' + e.message);
  }
}, 3000);
