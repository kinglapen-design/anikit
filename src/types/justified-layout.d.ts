declare module "justified-layout" {
  interface Box {
    aspectRatio: number;
    top: number;
    width: number;
    height: number;
    left: number;
  }
  interface Result {
    containerHeight: number;
    boxes: Box[];
  }
  interface Options {
    containerWidth?: number;
    containerPadding?:
      | number
      | { top?: number; right?: number; bottom?: number; left?: number };
    boxSpacing?: number;
    targetRowHeight?: number;
    targetRowHeightTolerance?: number;
  }
  export default function justifiedLayout(
    input: Array<{ width: number; height: number }>,
    options?: Options
  ): Result;
}
