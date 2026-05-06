/**
 * Temporary 用于诊断跨提供商空响应的调试标志
 * bug（MiniMax + GLM 5.1 都返回零内容块
 * 子代理提示）。 Toggle 这些将什么部分一分为二
 * 生成技能堆栈正在触发无声拒绝。
 *
 * USAGE
 *   1. Edit 下面是 `false` → `true` 的标志。
 *   2. Reload Web 应用程序（Vite HMR 立即获取更改 —
 * 无需 MCP 重新编译；该文件是浏览器端的）。
 *   3. Trigger 新一代设计。
 *   4. Watch `[sub-agent]` 日志行的开发控制台
 * 报告系统提示大小+包含的技能名称。
 *
 * Each 生成现在在 streamChat 调用之前记录 ONE 行：
 *
 *   [sub-agent] systemPrompt: chars=4231 skills=schema,jsonl,layout,...
 *
 * Rough 决策树：
 *
 *   - SKILLS_MINIMAL_ONLY = 真
 * Loads ONLY `schema` 和 `jsonl-format`。 If THIS 有效，
 * 失败在于其他一些技能。 If 它仍然失败，
 * 失败在于这两项技能本身，用户提示，
 * 或工具模式（我们最近都没有触及）。
 *
 *   - SKILLS_DISABLE_ANTI_SLOP = 真
 * Filters 已解决的技能组中的防倾斜功能，无需
 * 改变其他任何东西。 If 故障停止，防倾斜的
 * Chinese 关键字或 `{{recentHistory}}` 模板是
 * 受牵连。
 *
 *   - SKILLS_DISABLE_LAYOUT = 真
 * Filters `layout` 技能（我扩展了约 45 行
 * 在提交 5bd2c5f 中）。 If 故障停止，最近 ring/row-
 * 添加的宽度太长或包含以下内容
 * 提供商拒绝。
 *
 *   - SKILLS_DISABLE_OVERFLOW = 真
 * Filters `overflow` 技能（优先级 16，始终加载）。
 *
 * REMOVAL
 * This 文件故意很小并且是孤立的。 Once 空-
 * 响应错误已修复，删除它并删除导入
 * `orchestrator-sub-agent.ts`。 No 其他代码引用它。
 */

// COMMITTED DEFAULTS：每个技能标志都是 `false` 所以这个文件是
// 任何构建都无操作。 Toggling 标志是 LOCAL UNCOMMITTED 编辑：
// 开发人员翻转标志，重新加载 Web 应用程序 (Vite HMR)，运行
// 失败提示，观察结果，然后恢复本地
// 在提交任何其他内容之前进行编辑。 Never 推送翻转标志
// 到共享分支 - 它会默默地禁用每个分支的技能
// 拉构建。
//
// LOG_PROMPT_SIZE 是一个例外：它是一个被动的观察者
// 每次子代理调用只发出一根控制台线。 It 可以保持真实
// 在我们积极调试空响应时提交的代码中
// bug，一旦诊断完成就会翻转为 false。
export const SUB_AGENT_DEBUG_FLAGS = {
  /** Strip 除 `schema` 和 `jsonl-format` 之外的所有技能。 */
  SKILLS_MINIMAL_ONLY: false,
  /** Filter `anti-slop` 来自已解决的技能组。 */
  SKILLS_DISABLE_ANTI_SLOP: false,
  /** Filter `layout` 来自已解决的技能组。 */
  SKILLS_DISABLE_LAYOUT: false,
  /** Filter `overflow` 来自已解决的技能组。 */
  SKILLS_DISABLE_OVERFLOW: false,
  /**
   * Log 一句带有系统提示
   * 符大小的单行语句+在每个子代理 streamChat 调用之前包含技能名称。 Passive 仅观察者 —
   * 在主动调试时可以安全地保留 true。
   */
  LOG_PROMPT_SIZE: true,
} as const;
