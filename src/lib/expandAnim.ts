// 树容器 / 区块展开收起动画（2026-08-30 用户实机定）：150ms 通用档 + 缓入缓出 token 曲线。
// JS 高度补间（scrollHeight 实测）——v-if 保持挂载语义（稳态不挂整棵折叠子树），
// 仅展开/收起瞬间由 Transition JS 钩子（:css="false"）驱动高度 0↔实测值；
// 树节点子层（FolderTree .ft-children）与文件夹区块折叠（LibraryView .block-tree）共用。
const DURATION = 150; // --n-dur-150 通用档
const EASING = "cubic-bezier(.32,.08,.1,1)"; // --n-ease-in-out（JS 补间用不了 CSS 变量，字面量同步）

/** Transition @enter：高度 0 → 实测内容高，完成后清 inline 样式归还布局 */
export function expandEnter(el: Element, done: () => void): void {
  const target = (el as HTMLElement);
  const h = target.scrollHeight;
  target.style.overflow = "hidden";
  target.style.height = "0px";
  void target.offsetHeight; // reflow：先绘制 0 高，再起过渡（同帧写两值不动画）
  target.style.transition = `height ${DURATION}ms ${EASING}`;
  target.style.height = `${h}px`;
  settle(target, done);
}

/** Transition @leave：实测内容高 → 0，完成后由 Transition 卸载元素 */
export function expandLeave(el: Element, done: () => void): void {
  const target = el as HTMLElement;
  target.style.overflow = "hidden";
  target.style.height = `${target.scrollHeight}px`;
  void target.offsetHeight;
  target.style.transition = `height ${DURATION}ms ${EASING}`;
  target.style.height = "0px";
  settle(target, done);
}

/** 收尾：transitionend 清 inline 样式 + done；丢失兜底（display 切换等场景 transitionend 不发） */
function settle(el: HTMLElement, done: () => void): void {
  let finished = false;
  const finish = (): void => {
    if (finished) return;
    finished = true;
    el.style.transition = "";
    el.style.height = "";
    el.style.overflow = "";
    done();
  };
  el.addEventListener(
    "transitionend",
    (e) => {
      if (e.propertyName === "height") finish();
    },
    { once: true },
  );
  setTimeout(finish, DURATION + 40);
}
