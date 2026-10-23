/*
 * AniKit 贴图直链 —— Obsidian 插件（零构建、零依赖，纯 CommonJS）
 *
 * 行为：粘贴 / 拖入图片时接管默认事件，图片不存 vault 附件夹，
 * 而是拷进 AniKit 自动导入收件箱（随后被 AniKit 移走入库），
 * 并当场在光标处插入可渲染可点击的 markdown 链接。
 *
 * 安装：整个目录拷到 vault 的 .obsidian/plugins/anikit-paste/ 下，设置里启用即可。
 */
const { Plugin, PluginSettingTab, Setting, Notice } = require("obsidian");
const fs = require("fs");
const os = require("os");
const path = require("path");

// 支持入库的图片扩展名白名单（小写，不带点）
const IMAGE_EXTS = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"];

// MIME → 扩展名兜底：剪贴板截图的 File 名未必可靠（常统一叫 image.png），名字看不出来时以 MIME 为准
const MIME_EXT = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/gif": "gif",
  "image/webp": "webp",
  "image/bmp": "bmp",
  "image/svg+xml": "svg",
};

// mcp-server.port 缺失 / 解析失败时的回退端口
const DEFAULT_PORT = 17701;

const DEFAULT_SETTINGS = {
  // 收件箱目录覆盖；留空 = 自动推导（%APPDATA%\AniKit\config.json → spaces[0].path\自动导入）
  inboxDir: "",
  // 链接形态：dual = 双形态（http 渲染 + anikit:// 点击拉起）；http = 仅 http 图片（Typora 等环境用）
  linkForm: "dual",
};

// AniKit 数据目录（%APPDATA%\AniKit）；APPDATA 缺失时按 Windows 常规路径兜底
function anikitDataDir() {
  const appData = process.env.APPDATA || path.join(os.homedir(), "AppData", "Roaming");
  return path.join(appData, "AniKit");
}

// 自动推导收件箱：config.json → spaces[0].path（最近使用的储存空间根）+ 自动导入
function autoInboxDir() {
  const cfgPath = path.join(anikitDataDir(), "config.json");
  const cfg = JSON.parse(fs.readFileSync(cfgPath, "utf8"));
  const space = cfg.spaces && cfg.spaces[0] && cfg.spaces[0].path;
  if (!space) throw new Error("config.json 中没有 spaces[0].path");
  return path.join(space, "自动导入");
}

// 读 %APPDATA%\AniKit\mcp-server.port（纯数字文本）；缺失 / 解析失败回退 DEFAULT_PORT
function readPort() {
  try {
    const txt = fs.readFileSync(path.join(anikitDataDir(), "mcp-server.port"), "utf8").trim();
    const n = parseInt(txt, 10);
    if (Number.isInteger(n) && n > 0 && n <= 65535) return n;
  } catch (e) {
    // 静默回退
  }
  return DEFAULT_PORT;
}

// 本地时间戳：yyyyMMdd-HHmmss-fff
function timestamp(d) {
  const p = (n, w) => String(n).padStart(w, "0");
  return (
    `${d.getFullYear()}${p(d.getMonth() + 1, 2)}${p(d.getDate(), 2)}` +
    `-${p(d.getHours(), 2)}${p(d.getMinutes(), 2)}${p(d.getSeconds(), 2)}` +
    `-${p(d.getMilliseconds(), 3)}`
  );
}

// 唯一新名：贴图-yyyyMMdd-HHmmss-fff.{原扩展名小写}；同秒撞名追加 -2、-3……
function uniqueDest(inbox, ext) {
  const base = `贴图-${timestamp(new Date())}`;
  let dest = path.join(inbox, `${base}.${ext}`);
  for (let i = 2; fs.existsSync(dest); i++) {
    dest = path.join(inbox, `${base}-${i}.${ext}`);
  }
  return dest;
}

// 判定 File 是否为白名单图片，命中返回小写扩展名，否则返回空串
function imageExt(file) {
  const name = file.name || "";
  if (name.includes(".")) {
    const ext = name.split(".").pop().toLowerCase();
    if (IMAGE_EXTS.includes(ext)) return ext;
  }
  // 剪贴板截图兜底：文件名不可靠时看 MIME
  return MIME_EXT[file.type] || "";
}

// 拼链接。注意：链接写入时资产尚未入库——by-name 由 AniKit 侧解析最新一笔，唯一名保确定性
function buildLink(fileName, port, form) {
  const enc = encodeURIComponent(fileName);
  const http = `http://127.0.0.1:${port}/item/by-name/${enc}`;
  if (form === "http") return `![${fileName}](${http})`;
  // 双形态：内层 http 渲染图片（AniKit 媒体服务），外层 anikit:// 点击拉起 AniKit 定位
  return `[![${fileName}](${http})](anikit://item/by-name/${enc})`;
}

class AnikitPasteSettingTab extends PluginSettingTab {
  constructor(app, plugin) {
    super(app, plugin);
    this.plugin = plugin;
  }

  display() {
    const { containerEl } = this;
    containerEl.empty();

    new Setting(containerEl)
      .setName("收件箱目录覆盖")
      .setDesc(
        "图片拷入的 AniKit 自动导入收件箱。留空 = 自动推导：" +
          "%APPDATA%\\AniKit\\config.json → spaces[0].path（最近使用的储存空间根）下的「自动导入」目录。"
      )
      .addText((text) =>
        text
          .setPlaceholder("留空 = 自动（储存空间根\\自动导入）")
          .setValue(this.plugin.settings.inboxDir)
          .onChange(async (value) => {
            this.plugin.settings.inboxDir = value.trim();
            await this.plugin.saveSettings();
          })
      );

    new Setting(containerEl)
      .setName("链接形态")
      .setDesc(
        "双形态 = 内层 http 图片渲染 + 外层 anikit:// 点击拉起 AniKit 定位；" +
          "仅 http 图片 = 纯图片链接（Typora 等只认 http 的环境用）。"
      )
      .addDropdown((drop) =>
        drop
          .addOption("dual", "双形态（默认）")
          .addOption("http", "仅 http 图片")
          .setValue(this.plugin.settings.linkForm)
          .onChange(async (value) => {
            this.plugin.settings.linkForm = value;
            await this.plugin.saveSettings();
          })
      );
  }
}

module.exports = class AnikitPastePlugin extends Plugin {
  async onload() {
    await this.loadSettings();

    this.registerEvent(
      this.app.workspace.on("editor-paste", (evt, editor) => this.onPasteOrDrop(evt, editor))
    );
    this.registerEvent(
      this.app.workspace.on("editor-drop", (evt, editor) => this.onPasteOrDrop(evt, editor))
    );

    this.addSettingTab(new AnikitPasteSettingTab(this.app, this));
  }

  async loadSettings() {
    this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
  }

  async saveSettings() {
    await this.saveData(this.settings);
  }

  // editor-paste 的 evt 是 ClipboardEvent（clipboardData），editor-drop 是 DragEvent（dataTransfer）
  onPasteOrDrop(evt, editor) {
    if (!editor) return;
    const dt = evt.clipboardData || evt.dataTransfer;
    if (!dt || !dt.files || dt.files.length === 0) return;

    const files = Array.from(dt.files);
    // 无白名单图片则完全不干预默认行为（维持 Obsidian 原生的附件落盘 / 文本粘贴）
    if (!files.some((f) => imageExt(f))) return;

    // preventDefault 必须在事件回调内同步调用，随后的拷贝与插入异步进行
    evt.preventDefault();

    // 拖放时尽量把光标移到落点，让链接插在落点处；拿不到坐标就用当前光标
    if (evt.dataTransfer && typeof evt.clientX === "number") {
      try {
        const cm = editor.cm;
        if (cm && typeof cm.posAtCoords === "function") {
          const pos = cm.posAtCoords({ x: evt.clientX, y: evt.clientY });
          if (pos != null) editor.setCursor(editor.offsetToPos(pos));
        }
      } catch (e) {
        // 忽略，退回当前光标
      }
    }

    this.importFiles(files, editor);
  }

  async importFiles(files, editor) {
    // 收件箱目录：设置覆盖优先，留空走自动推导
    let inbox = this.settings.inboxDir.trim();
    if (!inbox) {
      try {
        inbox = autoInboxDir();
      } catch (e) {
        new Notice(
          "AniKit 贴图直链：读不到 AniKit 配置（%APPDATA%\\AniKit\\config.json），" +
            "请在插件设置里手动指定收件箱目录"
        );
        return;
      }
    }
    try {
      fs.mkdirSync(inbox, { recursive: true });
    } catch (e) {
      new Notice(`AniKit 贴图直链：收件箱目录无法创建——${e.message}`);
      return;
    }

    const port = readPort();
    const links = [];
    for (const f of files) {
      const ext = imageExt(f);
      if (!ext) continue;
      try {
        // 统一走 arrayBuffer：粘贴的剪贴板截图 File 没有磁盘路径，不能 fs.copyFile
        const buf = Buffer.from(await f.arrayBuffer());
        const dest = uniqueDest(inbox, ext);
        fs.writeFileSync(dest, buf);
        // 链接里的文件名 = 拷入时的唯一新名
        links.push(buildLink(path.basename(dest), port, this.settings.linkForm));
      } catch (e) {
        new Notice(`AniKit 贴图直链：「${f.name || "未命名图片"}」拷入收件箱失败——${e.message}`);
      }
    }

    // 全失败就不动编辑器；部分成功只插成功的——绝不插半截链接
    if (links.length === 0) return;
    // 多图各一行，末尾换行让后续输入另起一行
    editor.replaceSelection(links.join("\n") + "\n");
  }
};
