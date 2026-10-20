# AGENTS.md — Easy Agent Pilot 开发指南

> 本文件是面向 AI 协作（与人类开发者）的项目级开发约定入口。详细风格规范见 `docs/`，本文件聚焦**强制约定、快速上手与模块导航**。

## 0. 开发执行规范（强制）

> **⚠️ 所有开发工作必须严格按照 `.agents/skills/tauri-harness/` 技能规范执行。**

`tauri-harness` 是本项目的**开发驾驭规范**，基于 ETCLOVG 七层分类法，涵盖从环境验证到提交验收的完整开发生命周期。开发前**必须先阅读** `.agents/skills/tauri-harness/SKILLS.md`，并按其「阶段化读取路由表」加载对应阶段的 reference 文件。

### 核心执行要求

| 要求 | 说明 | 参考文件 |
|------|------|---------|
| **先读 SKILLS.md** | 每次开发会话开始时，先完整阅读 `.agents/skills/tauri-harness/SKILLS.md` | `SKILLS.md` |
| **按阶段路由加载** | 根据「阶段化读取路由表」只加载当前阶段的必读 reference，**严禁一次性加载所有 references** | `SKILLS.md` §0 |
| **遵守全局红线** | 所有阶段适用的红线（禁止裸 invoke、文件职责分离、测试独立文件等） | `SKILLS.md` §2 |
| **标准开发流程** | 环境验证 → `pnpm tauri dev` → 修改后 `pnpm typecheck + cargo clippy` → 提交前测试 → `pnpm tauri build` | `SKILLS.md` §3 |
| **八道验收门禁** | 每次提交前必须通过八道门禁（编译 → Lint → 类型 → 测试 → 架构 → 测试文件分离 → 权限审计 → 构建验证） | `references/acceptance-criteria.md` |
| **上下文恢复** | 会话中断恢复时检查 `.tauri-harness/CONTEXT.md`，按其中进度继续 | `references/context-management.md` |

### 阶段化读取路由（摘要）

| 当前任务 | 必读 references |
|----------|----------------|
| 项目初始化 / 脚手架 | `references/project-setup.md` |
| 前端开发（`.vue`/`.ts`/`.css`） | `references/frontend-standards.md` |
| 后端开发（`src-tauri/**/*.rs`） | `references/backend-standards.md`、`references/observability.md` |
| IPC 契约设计（新增 command/event） | `references/ipc-contract.md` |
| 编写测试 | `references/testing-standards.md` |
| 提交前 / CI 验收 | `references/acceptance-criteria.md` |
| 会话中断恢复 | `references/context-management.md` |

### 代码模板

开发新功能时，使用 tauri-harness 提供的标准模板：
- 前端功能模块：`templates/feature-module/`（`.vue` + `.css` + composable + types + index）
- 后端服务模块：`templates/rust-service/`（mod + service + repository + factory + models）

## 1. 项目简介

Easy Agent Pilot 是一个本地化 AI Agent 桌面工作台（Tauri 2 + Vue 3），聚合 Claude / Codex / OpenCode 等 CLI，提供会话、计划任务、无人值守、技能/插件配置、文件编辑与审查等能力。

- **技术栈**：前端 Vue 3 (`<script setup>` + TS) + Vite 6 + Pinia + Vue Router + Vue I18n + Naive UI + Tailwind CSS；后端 Rust + Tauri 2 + SQLite。
- **包管理器**：`pnpm`（lockfile `pnpm-lock.yaml`）。

## 2. 仓库布局

```
easy_agent_pilot/
├── .agents/skills/tauri-harness/  # ⚠️ 开发驾驭规范（强制遵守）
│   ├── SKILLS.md                  # 唯一聚合入口：路由 + 全局红线 + 恢复流程
│   ├── references/                # 各阶段详细标准（按需加载）
│   └── templates/                 # 前后端代码模板
├── src/                  # 前端（Vue 3 + TS）
│   ├── components/       # UI 组件，按功能模块组织（见各模块 AGENTS.md）
│   ├── modules/          # 功能子系统：fileEditor / officeViewer / documentEditor
│   ├── stores/           # Pinia 状态（组合式 store）
│   ├── services/         # Tauri IPC 封装 / 业务服务
│   ├── composables/      # 可复用组合式函数 useXxx.ts
│   ├── views/            # 路由级页面
│   ├── types/  utils/  constants/  locales/  router/  styles/
│   └── App/  main.ts  i18n.ts
├── src-tauri/            # 后端（Rust + Tauri 2 + SQLite）
│   ├── src/lib.rs        # 应用入口：插件注册、命令注册、setup
│   └── src/commands/     # ~45 个命令模块（+ conversation/ 子系统）
├── docs/                 # 详细风格指南：frontend-style.md、backend-style.md、modules.md
└── scripts/              # 构建/测试脚本
```

## 3. 常用命令

```bash
pnpm install            # 安装依赖
pnpm tauri dev          # 全栈开发（前端 + Rust，桌面端）
pnpm dev                # 仅前端开发（vite，端口 1430）
pnpm build              # 类型检查 + 前端打包（vue-tsc && vite build）
pnpm typecheck          # 仅类型检查（vue-tsc --noEmit）
pnpm lint               # ESLint 检查并自动修复（eslint src --fix）
pnpm test               # 单元测试（vitest run）
```

> 迁移/重构后务必依次运行 `pnpm typecheck` → `pnpm lint` → `pnpm build`，确保零 error。
> 完整八道验收门禁见 `.agents/skills/tauri-harness/references/acceptance-criteria.md`。

## 4. 前端开发规范（强制）

> 细节见 `docs/frontend-style.md` 和 `.agents/skills/tauri-harness/references/frontend-standards.md`。以下为**必须遵守**的核心约定。

### 4.1 组件强制三段拆分（本次迁移确立的统一约定）

**所有组件必须拆分为 `*.vue`（模板 + 极薄 script 胶水）/ `*.ts`（全部逻辑）/ `*.css`（全部样式），每个组件独占一个文件夹：**

```
src/components/<feature>/<ComponentName>/
├── <ComponentName>.vue     # 仅 <template> + 极薄 <script setup lang="ts">
├── use<ComponentName>.ts   # 全部逻辑：Props/Emits 类型 + composable 工厂
└── styles.css              # 全部 CSS（固定文件名 styles.css）
```

**`.vue` 胶水规则（禁止写业务逻辑与内联 CSS）：**

```vue
<script setup lang="ts">
// 仅允许：
// 1) 导入 composable + 类型
import { useXxx, type XxxProps, type XxxEmits } from './useXxx'
// 2) 编译宏（defineProps / defineEmits / withDefaults / defineModel / defineExpose）
const props = withDefaults(defineProps<XxxProps>(), { /* defaults */ })
const emit = defineEmits<XxxEmits>()
// 3) 解构调用 composable
const { /* 模板用到的全部成员，含子组件 */ } = useXxx(props, emit)
</script>

<template>
  <!-- 单根元素，BEM 命名 -->
</template>

<style scoped src="./styles.css"></style>
```

**`useXxx.ts` 规则：**
- 导出 `interface XxxProps` / `interface XxxEmits`（组件有时）；导出 `useXxx(props?, emit?)`。
- 内部实例化 store、`useI18n()`、`ref/computed/watch/生命周期`、所有 handler；**模板用到的子组件需 import 并 return**；普通工具函数也 return。
- 无 props/emits 时签名 `useXxx()`，store 与模板 ref 在内部创建并 return。

**样式规则：**
- `styles.css` 为纯类选择器；scoped 与否由 `.vue` 的 `<style>` 标签决定——原 `<style scoped>` → `<style scoped src="./styles.css">`；原非 scoped → `<style src="./styles.css">`。
- teleport 到 `<body>` 需全局样式的模态框，用第二块非 scoped：`<style src="./modalStyles.css"></style>` + `<style scoped src="./styles.css"></style>`。
- `:deep() / :global()` 原样保留，由 Vue 编译器处理。

**例外：**
- 纯模板无逻辑组件（script 无实际逻辑，仅 `<template>` + `<style src>`）：建文件夹但无需 `use*.ts`。
- 模块根共享工具（如 `planListShared.ts`、`soloShared.ts`）留在模块根，不随单组件迁移。

### 4.2 其他前端强制约定

- **禁止 Options API**；统一 `<script setup lang="ts">`。
- **命名**：组件 PascalCase（`.vue`）；composable `useXxx.ts`；store/service/type/util camelCase.ts；通用 UI 组件 `Ea` 前缀；文件夹（sidecar）camelCase。
- **导入路径**：用 `@/` 引用 `src/` 下模块；同目录用 `./`；禁止 `../../` 超过两层。
- **Pinia**：必须用组合式 `defineStore('x', () => {...})`。
- **样式令牌**：颜色/间距/字号用 `src/styles/variables.css` 的 `--ea-*` / `--color-*` CSS 变量，禁止硬编码；BEM 命名。
- **IPC**：通过 `invoke`（`@tauri-apps/api/core`）或 `invokeApi`（`src/utils/api.ts`）；前端参数 camelCase。
- **设计模式**：多分支用策略 / 状态机 / 工厂 / 常量映射，避免 `if/else if` 链。

## 5. 后端开发规范（强制）

> 细节见 `docs/backend-style.md` 和 `.agents/skills/tauri-harness/references/backend-standards.md`。核心约定：

- **命令文件布局**：导入 → 数据结构 → 私有辅助（`const XXX_SELECT_SQL`、行映射、转换）→ `#[tauri::command]`；所有命令在 `lib.rs` 注册。
- **序列化**：新增结构体必须 `#[serde(rename_all = "camelCase")]`。
- **错误处理**：命令返回 `Result<T, String>`，用 `.map_err(|e| e.to_string())?` 传播；**命令代码禁止 `unwrap()` / `expect()`**（仅测试允许）。
- **数据库**：每次调用 `open_db_connection()` 新建连接，禁止缓存；写操作走事务；参数化查询用位置占位 `?1, ?2`；SELECT 语句定义为 `const XXX_SELECT_SQL`。
- **同步 vs 异步**：DB CRUD 用同步 `fn`；CLI/SDK 执行用 `async fn`；大文件 I/O 用 `async fn` + `spawn_blocking`。
- **全局状态**：进程级用 `Lazy<Mutex/RwLock>`；跨命令共享且有生命周期依赖用 `manage()` + `State`；锁作用域最小化，持锁时禁止异步/阻塞。
- **平台适配**：`#[cfg(target_os=...)]` 逻辑隔离到独立文件（如 `mini_panel_macos_shortcut.rs`）。

## 6. IPC 与类型约定

- 前端调用：`invoke<T>('command_name', { camelCaseParams })`。
- 结构化错误：`invokeApi` + `classifyError()`（`src/utils/api.ts`）。
- Rust 类型转换：后端 `snake_case` 时定义 `RustX` + `X`(camelCase) + `transformX()`；后端 `#[serde(rename_all="camelCase")]` 时前端可直接用，但仍需定义接口。

## 7. 模块导航

各功能模块在自身目录下提供 `AGENTS.md`，说明职责、目录结构、关键文件、依赖 store/service 与模块约定。主要模块：

- 前端组件：`components/common`、`components/layout`、`components/message`、`components/plan`、`components/settings`、`components/skill-config`、`components/agent`、`components/memory`、`components/fileTree`、`components/project`、`components/solo`、`components/unattended`。
- 功能子系统：`modules/fileEditor`、`modules/officeViewer`、`modules/documentEditor`。
- 基础设施：`stores`、`services`、`composables`、`utils`、`types`、`views`。

完整模块地图与业务入口→组件映射见 `docs/modules.md`。

