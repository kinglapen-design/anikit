// mammoth 自带类型（lib/index.d.ts）的 NodeJsInput 引用 Node 的 Buffer——浏览器侧只用
// ArrayBufferInput，本 shim 只为喂饱类型检查（勿引入 @types/node，会把整条 Node API 放进来）
declare type Buffer = Uint8Array;
