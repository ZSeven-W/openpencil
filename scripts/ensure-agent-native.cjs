#!/usr/bin/env node
// Provisions 通过从源代码构建 Zig NAPI 插件二进制文件。
//
// We 始终从主机上的源代码构建，因此生成的 `agent_napi.node`
// 与跑步者的 platform/arch 匹配。 Earlier 修改版也尝试下载
// 从兄弟版本存储库中预构建的，但这条路径很活泼：当
// 当前子模块 SHA 缺少预构建，构建失败
// 进行源代码编译，将二进制文件存放在 `zig-out/napi/...` 处，并且
// electronics-builder（仅发布 `packages/agent-native/napi/`）静默
// 在没有插件的情况下发货 - 每个聊天请求都会在动态中终止
// `@zseven-w/agent-native` 导入。
//
// Build 先决条件：PATH 上 Zig 0.15+。 CI 工作流程通过安装它
// `mlugg/setup-zig`；本地开发人员通过他们的包管理器安装一次。
//
// Set OPENPENCIL_REQUIRE_AGENT_NATIVE=1 构建时安装失败
// 无法运行（电子 CI 使用它来尽早显示缺少的先决条件）。
//
// Set OPENPENCIL_SKIP_AGENT_NATIVE=1 完全不对脚本执行任何操作。 Useful 为
// 工作流程（npm 发布、仅 lint CI）在运行时从不加载插件
// 否则将在每次安装时支付 Zig 构建费用。
//
// Set ZIG_TARGET 交叉编译非主机三元组（例如在 macOS arm64 上
// 使用 `ZIG_TARGET=x86_64-macos` 为 x86_64-macos 构建运行程序）。 Without 它
// 构建遵循主机架构 - 对于本机运行来说很好，当
// 跑步者与您打算运送的工件不匹配。

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const AGENT_DIR = path.join(__dirname, '..', 'packages', 'agent-native');
const NAPI_DIR = path.join(AGENT_DIR, 'napi');
const ZIG_OUT = path.join(AGENT_DIR, 'zig-out', 'napi', 'agent_napi.node');
const BUNDLED = path.join(NAPI_DIR, 'agent_napi.node');
const STRICT = process.env.OPENPENCIL_REQUIRE_AGENT_NATIVE === '1';

function log(msg) {
  console.log(`[agent-native] ${msg}`);
}

function fail(msg) {
  log(msg);
  return STRICT ? 1 : 0;
}

function bundleBinary() {
  fs.mkdirSync(NAPI_DIR, { recursive: true });
  fs.copyFileSync(ZIG_OUT, BUNDLED);
  log(`Bundled binary at ${BUNDLED}.`);
}

function buildFromSource() {
  try {
    execSync('zig version', { stdio: 'ignore' });
  } catch {
    return fail(
      'Zig not installed (need 0.15+). Skipping. Install Zig and re-run `bun run agent:build`.',
    );
  }
  const target = process.env.ZIG_TARGET?.trim();
  const targetFlag = target ? ` -Dtarget=${target}` : '';
  log(`Building NAPI addon (zig build napi -Doptimize=ReleaseFast${targetFlag})…`);
  try {
    execSync(`zig build napi -Doptimize=ReleaseFast${targetFlag}`, {
      cwd: AGENT_DIR,
      stdio: 'inherit',
    });
  } catch (err) {
    return fail(`Zig build failed: ${err.message}`);
  }
  if (!fs.existsSync(ZIG_OUT)) {
    return fail(`Zig build produced no output at ${ZIG_OUT}.`);
  }
  bundleBinary();
  return 0;
}

function main() {
  if (process.env.OPENPENCIL_SKIP_AGENT_NATIVE === '1') {
    log('OPENPENCIL_SKIP_AGENT_NATIVE=1, skipping native binary provisioning.');
    return 0;
  }

  if (!fs.existsSync(path.join(NAPI_DIR, 'package.json'))) {
    return fail('Submodule not initialized; run `git submodule update --init`. Skipping.');
  }

  // Fast path: binary already in place. Make sure both lookup locations are
  // populated so electron-builder (`napi/`) and the runtime loader (which
  // checks `zig-out/` first) both find it without re-running the build.
  if (fs.existsSync(BUNDLED)) {
    log('Binary already present, skipping rebuild.');
    return 0;
  }
  if (fs.existsSync(ZIG_OUT)) {
    log('Binary already built; copying into napi/ for electron-builder.');
    bundleBinary();
    return 0;
  }

  return buildFromSource();
}

process.exit(main());
