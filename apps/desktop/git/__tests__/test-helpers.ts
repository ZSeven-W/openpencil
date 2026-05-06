// apps/desktop/git/__tests__/test-helpers.ts
//
// Shared 用于桌面 git 层测试的测试实用程序。 Each 测试创建
// 一个新的临时目录，运行其操作，并通过返回的进行清理
// 处理器。 This 保持测试隔离和并行安全。

import { mkdtemp, rm, writeFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/**
 * Create OS
 * 临时路径下的新临时目录。 Returns 路径和递归删除它的处理程序。 Always 将呼叫与 `try { ... } finally {
 * await dispose(); }` 配对。
 */
export async function mkTempDir(prefix = 'op-git-test-'): Promise<{
  dir: string;
  dispose: () => Promise<void>;
}> {
  const dir = await mkdtemp(join(tmpdir(), prefix));
  return {
    dir,
    dispose: async () => {
      await rm(dir, { recursive: true, force: true });
    },
  };
}

/**
 * Write 将
 * `.op` 存根文件（一个很小的 PenDocument JSON）放入目录中。 Returns 绝对文件路径。
 */
export async function writeOpFile(
  dir: string,
  name: string,
  content: object = { version: '1.0.0', children: [] },
): Promise<string> {
  const path = join(dir, name);
  await writeFile(path, JSON.stringify(content), 'utf-8');
  return path;
}

/**
 * Create 临时根目录
 * 下的嵌套目录结构。 Useful 用于设置“父 git 存储库内的文件”场景。
 */
export async function mkSubdir(root: string, ...segments: string[]): Promise<string> {
  const dir = join(root, ...segments);
  await mkdir(dir, { recursive: true });
  return dir;
}
