# Web 部署到腾讯云

Web 端通过 GitHub Actions 构建 Docker 镜像并部署到腾讯云 CVM。流水线会构建
`full` Docker target，推送到 GitHub Container Registry，然后把 Compose/Caddy
配置复制到服务器并重启服务。

## 服务器前置条件

- 已安装 Docker Engine 和 Docker Compose plugin。
- 腾讯云安全组已开放 SSH、TCP 80、TCP 443、UDP 443。
- 域名 A 记录已解析到 CVM 公网 IP。
- 部署目录为 `/opt/openpencil`；如果不存在，流水线会自动创建。

## GitHub 配置

Repository variables：

| 名称 | 含义 | 示例 |
| --- | --- | --- |
| `APP_DOMAIN` | 解析到腾讯云 CVM 的公网域名。Caddy 用它申请 HTTPS 证书。 | `openpencil.example.com` |
| `SUPABASE_URL` | Supabase 项目地址。前端镜像构建期和 Nitro 服务端接口都会使用。 | `https://xxxx.supabase.co` |
| `TENCENT_PORT` | 可选，CVM 的 SSH 端口；未设置时默认 `22`。 | `22` |

Repository secrets：

| 名称 | 含义 | 备注 |
| --- | --- | --- |
| `SUPABASE_ANON_KEY` | Supabase anon API key，浏览器和服务端运行期都会使用。 | anon key 本身可用于前端，但仍建议放 Secret，避免误打到日志或提交中。 |
| `TENCENT_HOST` | 腾讯云 CVM 公网 IP 或可解析主机名。 | `ssh`、`scp`、`ssh-keyscan` 都会使用。 |
| `TENCENT_USER` | CVM 上的 SSH 用户。 | 必须有执行 Docker 命令的权限。 |
| `TENCENT_SSH_KEY` | `TENCENT_USER` 对应的 SSH 私钥。 | 填完整私钥内容，包括 begin/end 行。 |
| `GHCR_USERNAME` | CVM 拉取 GHCR 镜像时使用的 GitHub 用户名。 | 通常就是你的 GitHub 用户名。 |
| `GHCR_READ_TOKEN` | CVM 执行 `docker login ghcr.io` 时使用的 GitHub PAT。 | 需要 `read:packages` 权限。 |
| `ANTHROPIC_API_KEY` | 可选，服务端 AI 功能使用的 Anthropic key。 | 可不填。 |
| `OPENAI_API_KEY` | 可选，服务端 AI 功能使用的 OpenAI key。 | 可不填。 |
| `GEMINI_API_KEY` | 可选，服务端 AI 功能使用的 Gemini key。 | 可不填。 |
| `GOOGLE_API_KEY` | 可选，Gemini 兼容流程可能使用的 Google API key。 | 可不填。 |

服务器上的 `.env` 文件由流水线每次部署时自动生成。具体运行时变量模板见
`deploy/.env.example`，实际文件会写入 `/opt/openpencil/.env`。

## 流水线行为

- 触发方式：推送到 `prod` 分支，或手动执行 `workflow_dispatch`。
- 镜像标签：每次生成 `ghcr.io/<owner>/<repo>:sha-<short-sha>`；推送到 `prod`
  时同时更新 `ghcr.io/<owner>/<repo>:prod`。
- 运行方式：Docker Compose 启动 `openpencil`，Caddy 负责反向代理和 HTTPS。
- HTTPS：Caddy 会根据 `APP_DOMAIN` 自动申请和续期证书。

## 服务器手动检查

部署后可以 SSH 到服务器执行：

```sh
cd /opt/openpencil
docker compose --env-file .env -f docker-compose.prod.yml ps
docker compose --env-file .env -f docker-compose.prod.yml logs --tail=100
```
