# 桌面端 CI/CD

OpenPencil 桌面端通过 GitHub Releases 分发。桌面端流水线会在 PR 指向 `prod`、
推送到 `prod`、手动触发，以及推送版本 tag 时构建 macOS、Windows、Linux 安装包。

## 发布流程

1. 同步所有 workspace 版本号：

   ```sh
   bun run bump <version>
   ```

2. 提交版本号变更。
3. 创建并推送版本 tag：

   ```sh
   git tag v<version>
   git push origin v<version>
   ```

4. `Build Electron` workflow 会为 `v*` tag 创建 GitHub Release，并上传桌面端构建产物。

PR 和 `prod` 分支推送只做构建校验并上传短期 workflow artifacts，不创建 GitHub Release。

## 构建产物

Release 构建会上传：

- macOS：`.dmg`、`.zip`、`latest-mac.yml`、`latest-mac-arm64.yml`、blockmaps。
- Windows：NSIS 安装包 `.exe`、portable `.exe`、`latest.yml`、blockmaps。
- Linux：`.AppImage`、`.deb`，以及 electron-builder 生成的更新元数据。

应用包会包含 Electron main/preload 编译产物、Nitro Web 服务输出、MCP server
bundle、CLI bundle，以及 native `@zseven-w/agent-native` 包。

## 当前 unsigned 构建策略

当前 CI 构建有意保持 unsigned。workflow 设置了
`CSC_IDENTITY_AUTO_DISCOVERY=false`，并且 `apps/desktop/electron-builder.yml`
里关闭了 macOS notarization。

unsigned 包会有平台限制：

- macOS Gatekeeper 可能提示或阻止首次启动，需要用户手动允许。
- Windows SmartScreen 可能提示未知发布者，因为没有可信发布者信誉。
- 应用内自动更新先作为预留能力；在完成签名和 notarization 前，不把它视为生产可用能力。

## 后续启用签名发布

准备好签名发布前，需要先配置这些 GitHub Actions secrets：

- `CSC_LINK`
- `CSC_KEY_PASSWORD`
- `WIN_CSC_LINK`
- `WIN_CSC_KEY_PASSWORD`
- `APPLE_ID`
- `APPLE_APP_SPECIFIC_PASSWORD`
- `APPLE_TEAM_ID`

这些密钥准备好后，移除或覆盖 `CSC_IDENTITY_AUTO_DISCOVERY=false`，把
`mac.notarize` 恢复为 `true`，然后跑一次 tag release。正式启用自动更新前，需要确认
macOS notarization 成功，并且 Windows 更新包签名验证能通过。

## 可选包管理器更新

tagged release 后，workflow 可以更新 Homebrew cask 和 Scoop bucket。如果没有配置
`TAP_GITHUB_TOKEN`，这两个任务会自动跳过。
