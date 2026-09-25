# CDP 探针共享桩工厂（2026-09-25 T0 验证网修复抽出）：生成注入前端的 window.__TAURI_INTERNALS__ 伪造桩 JS 字符串，
# 供 Page.addScriptToEvaluateOnNewDocument({"source": STUB}) 原位替换各 cdp-* 探针的旧内联 STUB 段。
#
# 用法：
#   from _probe_stub import make_stub
#   STUB = make_stub({"get_current_library": {"name": "测试库", "path": "E:/test-lib"}})
#
# - overrides：命令名 → 桩值。str = 原样 JS 表达式（可引用 head/handlers 内常量）；其余类型
#   （dict / list / bool / int / None）= JSON 编码注入。在 DEFAULT_STUBS 上同名覆盖、异名追加。
# - head：注入在 __TAURI_INTERNALS__ 之前的裸 JS（桩数据常量 / window.__calls 记录器等）。
# - handlers：注入在 invoke 内 R 表查表之前的裸 JS（自定义命令拦截分支，4 空格缩进随现行）。
# - convert_file_src：convertFileSrc 桩体（默认 asset://localhost/ 形式；需真读盘缩略图的探针
#   换 encodeURI("/@fs/" + p) 形式，见 cdp-sidebar-anim-probe.py）。
#
# 验证纪律（开发规范 §8，踩坑 029/041/064/081）：改完代码 ≠ 页面跑新码——探针连接 dev 实例后
# 先强制 reload 一次再开始断言，断言离奇失效先查「页面是不是旧码」再查业务。
import json

# 启动默认桩表（前端启动链必调命令；形状抄各现行探针 2026-09-25 静态盘点：
# get_ui_settings {} / list_folder_tree {blocks,nodes} / list_inbox_snapshot [] /
# get_undo_state 带 undoLabel/redoLabel 四键——cdp-context-menu-probe.py 等现行形状）
DEFAULT_STUBS: dict[str, object] = {
    "get_window_fx_mode": '"solid"',
    "get_ui_settings": "{}",
    "get_current_library": "null",
    "list_placements": "[]",
    "list_folder_tree": '{ blocks: [], nodes: [] }',
    "list_recent_assets": "[]",
    "list_recycle": "[]",
    "list_archives": "[]",
    "list_relations": "[]",
    "list_inbox_snapshot": "[]",
    "get_undo_state": '{ canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" }',
}

DEFAULT_CONVERT_FILE_SRC = '(p) => "asset://localhost/" + encodeURIComponent(p)'


def _js(value: object) -> str:
    # str = 调用方给的原样 JS 表达式；其余 = JSON（合法 JS 字面量子集）
    return value if isinstance(value, str) else json.dumps(value, ensure_ascii=False)


def make_stub(
    overrides: dict[str, object] | None = None,
    *,
    head: str = "",
    handlers: str = "",
    convert_file_src: str = DEFAULT_CONVERT_FILE_SRC,
) -> str:
    """默认桩表 + overrides 覆盖/追加，产出完整 STUB JS 字符串（末尾带换行）。"""
    table = {**DEFAULT_STUBS, **(overrides or {})}
    rows = "\n".join(f"      {cmd}: {_js(v)}," for cmd, v in table.items())
    head_js = head.strip("\n")
    handlers_js = handlers.strip("\n")
    lines: list[str] = []
    if head_js:
        lines.append(head_js)
    lines.append("window.__cbId = 0; window.__cbs = {};")
    lines.append("window.__TAURI_INTERNALS__ = {")
    lines.append('  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },')
    lines.append(f"  convertFileSrc: {convert_file_src},")
    lines.append("  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },")
    lines.append("  invoke: (cmd, args) => {")
    if handlers_js:
        lines.append(handlers_js)
    lines.append("    const R = {")
    lines.append(rows)
    lines.append("    };")
    lines.append("    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);")
    lines.append("  },")
    lines.append("};")
    lines.append("// tauri event 插件内部（event.js unlisten 直调，缺了 listen 页面清理即崩，2026-09-25 T0 实跑实证）")
    lines.append("window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };")
    return "\n".join(lines) + "\n"
