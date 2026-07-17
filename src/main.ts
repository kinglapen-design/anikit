// AniKit 前端入口（阶段 0 占位页 + 任务 2 拖出验证，阶段 1 搭建三栏主窗口）
import { invoke } from "@tauri-apps/api/core";

const app = document.querySelector<HTMLDivElement>("#app")!;

app.innerHTML = `
  <main style="
    height: 100vh; margin: 0; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 12px;
    background: #1e1e1e; color: #d4d4d4;
    font-family: 'Microsoft YaHei', system-ui, sans-serif;
  ">
    <h1 style="margin: 0; font-size: 28px;">AniKit</h1>
    <p style="margin: 0; opacity: 0.7;">阶段 0 · 技术验证 —— 脚手架就绪</p>
    <button id="drag-btn" style="
      margin-top: 16px; padding: 10px 24px; font-size: 15px; cursor: grab;
      background: #3a7bfd; color: #fff; border: none; border-radius: 6px;
    ">按住我拖到文件夹里（拖出测试）</button>
    <p id="drag-status" style="margin: 0; opacity: 0.6; font-size: 13px;"></p>
  </main>
`;
document.body.style.margin = "0";

const status = document.querySelector<HTMLParagraphElement>("#drag-status")!;
const btn = document.querySelector<HTMLButtonElement>("#drag-btn")!;

// 拖出必须在鼠标按下时发起
btn.addEventListener("mousedown", async () => {
  status.textContent = "拖拽中……";
  try {
    await invoke("drag_files", {
      paths: ["E:\\01_Projects\\03_Software-dev\\AniKit\\tools\\test-assets\\drag-test.txt"],
    });
    status.textContent = "拖拽结束：文件出现在目标位置即任务 2 通过。";
  } catch (e) {
    status.textContent = `拖出失败：${e}`;
  }
});
