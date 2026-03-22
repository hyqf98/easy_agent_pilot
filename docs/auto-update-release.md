# 自动更新发布说明

本文档用于维护 `EasyAgentPilot` 的 Tauri 自动更新发布流程，适用于当前仓库的 GitHub Actions + GitHub Release 方案。

## 当前方案

- 应用内更新源：`https://github.com/hyqf98/easy-agent-pilot/releases/latest/download/latest.json`
- 发布工作流：[`../.github/workflows/release.yml`](../.github/workflows/release.yml)
- CI 构建工作流：[`../.github/workflows/ci.yml`](../.github/workflows/ci.yml)
- GitHub 密钥位置：`Settings -> Secrets and variables -> Actions -> Repository secrets`
- 使用同一把 Tauri updater 私钥对所有后续版本签名
- CI 使用 [`../src-tauri/tauri.ci.conf.json`](../src-tauri/tauri.ci.conf.json) 关闭 updater 产物签名，只做构建校验

## 必备前提

发布自动更新前，确认以下事项已经完成：

- 已在 GitHub 仓库中配置 `TAURI_SIGNING_PRIVATE_KEY`
- 当前这把 updater 私钥是加密私钥，需要同时配置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- 本地已安全备份同一把私钥
- 该私钥不要提交到 Git
- 粘贴到 GitHub Secret 的值必须是私钥文件全文，不要带路径、引号或额外空格
- 如果私钥文本末尾带有 `=` 或 `==`，必须一并保留

当前本地建议备份位置：

- 私钥：`../.local/signing/tauri-updater-private.key`
- 公钥：`../.local/signing/tauri-updater-public.key`
- 密码：`../.local/signing/tauri-updater-password.txt`

## 每次发布的标准流程

### 1. 修改版本号

每次发布必须使用新的版本号，不能覆盖已发布的旧版本号。

需要同时更新以下三个文件中的版本：

- [`../package.json`](../package.json)
- [`../src-tauri/tauri.conf.json`](../src-tauri/tauri.conf.json)
- [`../src-tauri/Cargo.toml`](../src-tauri/Cargo.toml)

示例：

- 上一个版本：`1.0.0`
- 下一个版本：`1.0.1`

不要删除 `v1.0.0` 后重新发布 `v1.0.0` 来做升级。已安装的 `1.0.0` 不会把另一个 `1.0.0` 识别成新版本。

### 2. 本地校验

发布前至少执行：

```bash
pnpm build
pnpm lint
pnpm test:update
cargo check --manifest-path src-tauri/Cargo.toml
```

### 3. 提交并创建 Tag

```bash
git add .
git commit -m "feat: release v1.0.1"
git tag v1.0.1
git push origin main
git push origin v1.0.1
```

当前工作流监听 `v*` tag，因此推送 `v1.0.1` 后会自动触发发布。

### 4. 等待 GitHub Actions 构建

GitHub Actions 会基于 tag 运行 [`../.github/workflows/release.yml`](../.github/workflows/release.yml)，构建各平台安装包并创建 draft release。

### 5. 检查 Release 资产

进入 GitHub Releases，确认本次 draft release 中至少包含：

- Windows `-setup.exe` 安装包
- macOS `.dmg` 安装包
- Linux `.deb` 安装包
- Linux `.AppImage` 便携包
- `latest.json`

如果没有 `latest.json`，应用内自动更新不会生效。

### 6. 手动发布 Release

当前工作流配置为 `releaseDraft: true`，因此 GitHub Actions 只会生成草稿。

你需要手动点击：

- `Publish release`

只有发布完成后，`/releases/latest/download/latest.json` 才会对应用内更新可见。

### 7. 在应用内验证更新

使用已安装旧版本的应用执行验证：

1. 打开设置页面
2. 进入“软件更新”
3. 点击“检查更新”
4. 确认能发现新版本
5. 执行下载和安装

## 首次上线自动更新的检查项

首次正式启用自动更新时，额外确认：

- 发布的不是 draft，而是已公开发布的 release
- Release 资产里存在 `latest.json`
- 新发布版本号高于当前已安装版本
- 应用内 updater 公钥与发布签名私钥是一对

## 换电脑后的处理方式

### 只开发，不发版

只需要拉取仓库代码，不需要私钥。

原因：

- 应用检查更新只使用公钥
- 运行和调试不需要 updater 私钥

### 需要接管正式发版

必须继续使用同一把私钥。

可行方式有两种：

- 仅通过 GitHub Actions 发版：确保 GitHub 仓库里的 `Repository secrets` 还在
- 需要本地手工构建正式更新包：把同一份私钥安全复制到新电脑

不要重新生成新的 updater 私钥。否则旧版本用户可能无法验证新版本签名，升级链会中断。

## 故障排查

### 应用内提示无法获取更新信息

优先检查：

- GitHub Release 是否仍是 draft
- 最新 release 是否包含 `latest.json`
- `latest.json` 是否来自当前最新正式 release
- 版本号是否真的变大了

### GitHub Actions 发布失败

优先检查：

- `TAURI_SIGNING_PRIVATE_KEY` 是否仍存在
- 私钥内容是否完整，多行内容是否被截断
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 是否存在且与这把私钥匹配
- 私钥内容末尾的 base64 padding `=` / `==` 是否在复制时丢失

## 密钥管理建议

- 私钥至少保留两份安全备份
- 不要把私钥放入仓库
- 不要通过聊天工具明文长期传递私钥
- 如果仓库迁移或 GitHub Secret 丢失，仍要能找回原私钥

只要已经发布过一个可自动更新的正式版本，就不要随意更换 updater 私钥。
