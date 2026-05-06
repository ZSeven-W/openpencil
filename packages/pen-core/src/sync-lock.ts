/**
 * When 锁定，文档存储
 *
 * → Fabric 同步被跳过（Fabric 是源）。 Uses 是一个 getter 函数，而不是裸露的 `let` 导出，以便跨模块读取始终解析当前值
 * - 即使捆绑程序不保留
 * `let` 变量的 ES 模块实时绑定。
 */
let _locked = false;

export function isFabricSyncLocked(): boolean {
  return _locked;
}

export function setFabricSyncLock(v: boolean) {
  _locked = v;
}
