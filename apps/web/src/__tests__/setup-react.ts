/**
 * Vitest 安装文件，
 *
 * 用于修复 Bun monorepos 中的“React 的多个副本”错误。 Problem：Vite 的 ESM 模块运行程序创建一个单独的
 * React 实例（通过其
 * 转换管道），该实例的 ReactSharedInternals 与 React-dom 使用的本机 CJS React 不同。 When
 * React-dom 在 CJS React 的 ReactSharedInternals.h 上渲染并设置钩子调度程序，pen-react
 * 中的钩子（使用 vite 转换的 React）看到一个空调度程序并抛出“Invalid 钩子调用”。 Root 原因：`import 'react'` 通过
 *
 * vite 的管道返回一个带有自己的 `__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANN
 * OT_UPGRADE` 对象的模块，与本机 CJS `require('react')` 返回的模块分开（react-dom 通过其自己的
 * require
 * 链在内部使用）。 Fix：In setupFiles（与每个测试文件在相同的 vitest 工作范围中运行），在 vite 转换的 React
 *
 * 的内部安装代理，以便所有 reads/writes 委托给本机 CJS React 的内部。 After 这个，当 react-dom 设置
 * h（钩子调度程序）时，pen-react 钩子立即看到它。
 *
 *
 */
import { createRequire } from 'node:module';

// Import 通过 vite 的转换管道进行“反应”——与 pen-react hooks 使用的实例相同
import * as viteTranformedReact from 'react';

// Load 通过本机 CJS require 进行反应 — 与 React-dom 内部使用的实例相同。 Resolve
// 通过节点自己的模块查找动态地进行，因此该文件不绑定到任何单个开发人员的计算机（之前的硬编码绝对路径在该用户家之外的每次结帐时破坏了整个 apps/web
// 测试套件）。
const require = createRequire(import.meta.url);
const cjsReact = require('react') as Record<string, any>;

const viteInternals = (viteTranformedReact as any)
  .__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE as Record<string, any>;
const cjsInternals =
  cjsReact.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE as Record<string, any>;

if (viteInternals && cjsInternals && viteInternals !== cjsInternals) {
  // Make vite 转换的 React 的内部结构将所有 reads/writes 委托给 CJS 内部结构。 This 桥接两个 React 实例，因此
  // React-dom 的调度程序对钩子可见。
  for (const key of Object.keys(cjsInternals)) {
    Object.defineProperty(viteInternals, key, {
      get: () => (cjsInternals as any)[key],
      set: (v) => {
        (cjsInternals as any)[key] = v;
      },
      configurable: true,
      enumerable: true,
    });
  }
}
