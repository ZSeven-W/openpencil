import { useSyncExternalStore, useCallback, useRef } from 'react';
import type { DesignEngine, DesignEngineEvents } from '@zseven-w/pen-engine';

/**
 * Generic 钩子通过
 *
 * useSyncExternalStore 订阅引擎事件。当状态未更改时，getSnapshot MUST 返回稳定的引用。 The
 * 引擎保证不可变的引用 -
 *
 * 请参阅 pen-engine 中的 Immutability 合约。 The 引擎的 `on()` 方法返回一个取消订阅函数，这正是
 * useSyncExternalStore 的 `subscribe` 回调所期望的。 We 通过将最新的 getSnapshot
 *
 * 存储在引用中，保持捕捉函
 * 数引用在渲染中稳定。当调用者将内联箭头函数作为 getSnapshot 传递时，This 可防止无限循环。
 *
 */
export function useEngineSubscribe<K extends keyof DesignEngineEvents, T>(
  engine: DesignEngine,
  event: K,
  getSnapshot: (engine: DesignEngine) => T,
): T {
  const snapshotRef = useRef(getSnapshot);
  snapshotRef.current = getSnapshot;

  const subscribe = useCallback((cb: () => void) => engine.on(event, cb as any), [engine, event]);
  const snap = useCallback(() => snapshotRef.current(engine), [engine]);
  return useSyncExternalStore(subscribe, snap);
}
