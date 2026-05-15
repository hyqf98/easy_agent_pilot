# Easy Agent Pilot

<p align="center">
  <img src="./images/首页.png" alt="Easy Agent Pilot" width="100%" />
</p>

<p align="center">
  <strong>面向本地开发环境的 AI Agent 桌面工作台</strong><br />
  聚合 Claude CLI / Codex CLI / OpenCode CLI，统一管理项目、会话、计划与执行
</p>

<p align="center">
  <a href="https://github.com/hyqf98/easy_agent_pilot"><img src="./images/badge-github.svg" alt="GitHub" /></a>
  <img src="./images/badge-tauri.svg" alt="Tauri 2" />
  <img src="./images/badge-vue.svg" alt="Vue 3" />
  <img src="./images/badge-rust.svg" alt="Rust" />
  <img src="./images/badge-sqlite.svg" alt="SQLite" />
  <img src="./images/badge-license.svg" alt="MIT License" />
</p>

---

## Agent 配置

多模型提供商配置，支持 Provider Profile 快速切换，适配不同环境。

<table>
  <tr>
    <td><img src="./images/模型管理页面.png" alt="模型管理" /></td>
    <td><img src="./images/快速实现配置切换.png" alt="配置切换" /></td>
  </tr>
  <tr>
    <td align="center"><strong>模型管理</strong></td>
    <td align="center"><strong>配置切换</strong></td>
  </tr>
</table>

## Agent CLI 配置

管理多个 CLI 工具，支持会话预览和切换。

<table>
  <tr>
    <td><img src="./images/CLI管理页面.png" alt="CLI管理" /></td>
    <td><img src="./images/CLI原始会话管理.png" alt="CLI原始会话管理" /></td>
  </tr>
  <tr>
    <td align="center"><strong>CLI 管理</strong></td>
    <td align="center"><strong>会话管理</strong></td>
  </tr>
  <tr>
    <td colspan="2"><img src="./images/CLI原始会话预览.png" alt="CLI原始会话预览" /></td>
  </tr>
  <tr>
    <td colspan="2" align="center"><strong>会话预览</strong></td>
  </tr>
</table>

## 项目管理

项目文件树浏览，支持项目级和全局级文件引用。

<table>
  <tr>
    <td><img src="./images/项目管理页面.png" alt="项目管理" /></td>
    <td><img src="./images/项目文件管理.png" alt="项目文件管理" /></td>
  </tr>
  <tr>
    <td align="center"><strong>项目管理</strong></td>
    <td align="center"><strong>文件管理</strong></td>
  </tr>
</table>

## 扩展配置

统一管理 MCP、Skills、Plugins，支持 Skills 命令扫描和配置切换。

<table>
  <tr>
    <td><img src="./images/技能MCP管理.png" alt="技能MCP管理" /></td>
    <td><img src="./images/Skills命令扫描.png" alt="Skills命令扫描" /></td>
  </tr>
  <tr>
    <td align="center"><strong>技能 MCP 管理</strong></td>
    <td align="center"><strong>Skills 命令扫描</strong></td>
  </tr>
</table>

## 智能体专家

多 Agent 专家管理，定义角色提示词、默认模型和适用任务，不同场景使用不同专家。

<p align="center">
  <img src="./images/多Agent专家管理.png" alt="多Agent专家管理" width="100%" />
</p>

## 主会话

项目级多会话协作，支持结构化消息渲染、思考区、工具调用时间线、记忆引用和图片粘贴。

<p align="center">
  <img src="./images/主会话对话.png" alt="主会话对话" width="100%" />
</p>

## 记忆系统

支持原始记忆池、AI 压缩记忆库，可在主会话中引用记忆，沉淀项目知识。

<p align="center">
  <img src="./images/记忆库管理.png" alt="记忆库管理" width="100%" />
</p>

## 计划拆分

从目标拆分到任务执行的完整闭环，支持动态表单、可拖拽看板、执行日志和结果回写。

<table>
  <tr>
    <td><img src="./images/计划管理.png" alt="计划管理" /></td>
    <td><img src="./images/计划任务管理.png" alt="计划任务管理" /></td>
  </tr>
  <tr>
    <td align="center"><strong>计划管理</strong></td>
    <td align="center"><strong>任务管理</strong></td>
  </tr>
</table>

<table>
  <tr>
    <td><img src="./images/任务拆分.png" alt="任务拆分" /></td>
    <td><img src="./images/动态表单时收集需求.png" alt="动态表单" /></td>
  </tr>
  <tr>
    <td align="center"><strong>任务拆分</strong></td>
    <td align="center"><strong>动态表单</strong></td>
  </tr>
  <tr>
    <td colspan="2"><img src="./images/可拖拽式任务执行.png" alt="可拖拽式任务执行" /></td>
  </tr>
  <tr>
    <td colspan="2" align="center"><strong>可拖拽式任务执行看板</strong></td>
  </tr>
</table>

## SOLO 单兵执行

全程自主规划模式，由协调 AI 自动派发任务、推进步骤、回写结果，支持暂停/继续/重试，适合复杂交付流。

<table>
  <tr>
    <td><img src="./images/SOLO模式执行.png" alt="SOLO执行" /></td>
    <td><img src="./images/SOLO模式可选专家团队.png" alt="SOLO专家团队" /></td>
  </tr>
  <tr>
    <td align="center"><strong>SOLO 执行时间线</strong></td>
    <td align="center"><strong>可选专家团队</strong></td>
  </tr>
</table>

## Token 统计

实时监控 Token 使用情况，帮助了解模型消耗和成本。

<p align="center">
  <img src="./images/token统计.png" alt="Token统计" width="100%" />
</p>

## 文件预览

项目文件树浏览，支持 Markdown、PDF、DOC 文件预览，项目级和全局级文件引用。

<table>
  <tr>
    <td><img src="./images/md文件预览.png" alt="Markdown预览" /></td>
    <td><img src="./images/pdf文件预览.png" alt="PDF预览" /></td>
  </tr>
  <tr>
    <td align="center"><strong>Markdown 预览</strong></td>
    <td align="center"><strong>PDF 预览</strong></td>
  </tr>
  <tr>
    <td colspan="2"><img src="./images/doc文件预览.png" alt="DOC预览" /></td>
  </tr>
  <tr>
    <td colspan="2" align="center"><strong>DOC 文件预览</strong></td>
  </tr>
</table>

<table>
  <tr>
    <td><img src="./images/图片引用.png" alt="图片引用" /></td>
    <td><img src="./images/项目级全局级文件引用.png" alt="文件引用" /></td>
  </tr>
  <tr>
    <td align="center"><strong>图片引用</strong></td>
    <td align="center"><strong>文件引用</strong></td>
  </tr>
</table>

## 无人值守

支持远程操作软件，可用于远程运维、远程排障等场景。

<p align="center">
  <img src="./images/WeChatClaw对接.png" alt="WeChatClaw对接" width="100%" />
</p>

## 支持的 CLI 工具

| 工具 | 说明 | 安装方式 |
|------|------|----------|
| Claude Code | Anthropic 官方 CLI 编码助手 | `npm install -g @anthropic-ai/claude-code` |
| Codex | OpenAI 官方 CLI 编码助手 | `npm install -g @openai/codex` |
| OpenCode | 开源终端 AI 编码工具 | `npm install -g opencode` |

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/) 8+
- [Rust](https://www.rust-lang.org/) 1.70+
- 至少安装一个支持的 CLI 工具

### 安装与运行

```bash
# 安装依赖
pnpm install

# 启动开发环境
pnpm tauri dev

# 前端构建
pnpm build

# 代码检查
pnpm lint

# Rust 后端检查
cargo check --manifest-path src-tauri/Cargo.toml
```

### 构建桌面包

```bash
pnpm build:mac-arm    # macOS (Apple Silicon)
pnpm build:windows    # Windows
pnpm build:linux      # Linux
```

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | Vue 3, TypeScript, Pinia, Vite |
| 桌面 | Tauri 2 |
| 后端 | Rust |
| 存储 | SQLite |
| 编辑器 | Monaco Editor, xterm.js |

## 仓库结构

```text
src/                    前端界面、状态管理、业务服务
├── components/         Vue 组件
├── stores/             Pinia 状态管理
├── composables/        组合式函数
└── services/           业务服务层

src-tauri/              Tauri 2 Rust 后端
├── src/commands/       Tauri 命令层
└── src/db/             数据库操作

images/                 README 截图资源
```

## 许可证

MIT License
