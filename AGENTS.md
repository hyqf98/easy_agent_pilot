# 仓库协作规范

## 项目概览

本项目是 `Tauri 2 + Vue 3 + TypeScript + Rust` 的桌面应用。

- 前端开发服务使用 Vite，固定端口为 `1430`
- Vite HMR 端口为 `1431`
- Tauri 开发态 `devUrl` 为 `http://localhost:1430`
- Tauri 调试态启用了 `tauri-plugin-mcp-bridge`
- Tauri MCP Bridge 基准端口为 `9423`

开发、联调、自动化测试时，默认按"前端 1430 + Tauri 2 宿主 + MCP Bridge 9423"理解当前运行环境，不要自行改成其他端口，除非任务明确要求。

---

## 文档索引

以下文档从本文件拆分出去，各自聚焦一个领域，修改时只需编辑对应文件：

| 文档 | 内容 | 路径 |
|------|------|------|
| 模块与目录说明 | 前后端模块职责、目录结构、业务入口、sidecar 索引 | [docs/modules.md](docs/modules.md) |
| 前端开发风格指南 | Vue 组件结构、Pinia Store、Composable、样式管理、Tauri IPC、注释标准 | [docs/frontend-style.md](docs/frontend-style.md) |
| 后端开发风格指南 | Rust 命令结构、数据库访问、多线程、错误处理、设计模式、注释标准 | [docs/backend-style.md](docs/backend-style.md) |

---

## 开发约束

### 通用原则

- 所有变更优先遵守现有业务结构与命名风格，不要顺手做无关重构。
- 任何新增功能先判断应放入现有业务域，优先复用现有 store、service、command、component。
- 前后端都要优先写可读代码，避免炫技式抽象。

### 代码风格要求

- 禁止出现过深 `if` 嵌套。优先使用卫语句、提前返回、状态映射、策略分发。
- 同类分支逻辑超过 3 个时，优先考虑 `enum`、常量映射、策略模式、工厂模式，而不是继续堆 `if/else if`。
- 跨平台差异、Provider 差异、Agent 差异、执行策略差异，优先使用设计模式封装，禁止把差异逻辑散落在页面或命令入口里。
- 共享字符串、状态值、事件名、命令名、路由名、表单字段名，优先抽成常量或枚举，禁止魔法字符串满天飞。
- 公共数据结构必须先定义类型，再落地实现，禁止大量 `any`、隐式对象、无约束返回值。
- 复杂流程要拆为"小函数 + 主流程编排"，不要让单个函数承担过多职责。
- Vue 组件保持单一职责。页面组件负责装配，复杂业务逻辑优先下沉到 store / composable / service。
- Rust 命令层只负责输入输出编排，数据库访问、执行策略、状态判断优先下沉到可复用函数。

### 设计建议

- Provider、CLI、SDK、不同 Agent 的执行流程优先使用策略模式。
- 市场安装、回滚、升级、同步等流程优先使用命令模式或服务对象封装。
- 会话状态、计划状态、执行状态优先使用枚举和显式状态流转，禁止散乱布尔值堆叠。
- 表单、任务拆分、执行时间线等动态流程优先采用配置驱动和映射表，而不是硬编码分支。

---

## 开发与提交流程

- 修改前先确认变更属于前端、后端还是跨端联动。
- 涉及 UI 的功能开发，必须同步检查会话页、计划页、设置页是否受影响。
- 涉及计划拆分或任务执行的改动，必须考虑创建、继续、停止、重试、日志、结果回写链路。
- 涉及设置项或 Provider 配置的改动，必须考虑持久化、初始化加载、切换回显。
- 提交信息建议使用简短 Conventional Commit 风格，如 `fix: 修复计划继续拆分状态异常`、`feat: 新增日志筛选能力`。

---

## 测试要求

### 基础要求

- 所有变更至少满足"编译通过"。
- 前端改动后至少执行 `pnpm build`。
- 后端或跨端改动后至少执行 `cargo check --manifest-path src-tauri/Cargo.toml`。
- 涉及 Vue 组件、store、composable 的改动，补充执行 `pnpm lint`。

### Tauri MCP 验证要求

- 所有页面级改动都必须使用 Tauri MCP 做页面访问验证，至少确认对应页面可以正常打开、主要区域正常渲染、无明显报错。
- 涉及设置菜单、主会话、计划拆分、计划执行、记忆管理、Marketplace 的改动，必须使用 Tauri MCP 实际进入对应页面或弹窗验证。
- 如果是功能开发，不接受只看代码不走流程，必须使用 Tauri MCP 自动化工具完成全流程验证。
- 涉及专家切换、运行时切换、动态表单、上下文策略提示、压缩提示、token 进度的改动，必须验证 UI 展示、实际执行、日志 / 状态回写三者一致。

### 全流程验证要求

- 主会话相关改动：至少验证项目进入、会话切换、消息区展示、必要操作按钮可用。
- 主会话涉及专家链路时：至少验证专家切换、运行时提示、动态表单请求 / 提交、表单提交后的消息状态清理、记忆引用和压缩 / token 展示。
- 设置相关改动：至少验证设置入口、对应菜单页签、表单项读写或展示正常。
- 计划拆分相关改动：至少验证创建计划、启动拆分、停止拆分、继续拆分、表单补充或预览、确认生成任务链路。
- 计划执行相关改动：至少验证任务加载、任务编辑改派专家、开始执行、暂停或恢复、日志查看、状态回写链路。
- 记忆相关改动：至少验证记忆库列表、原始记忆筛选、编辑或合并流程。
- 涉及任务状态拖拽或跨列变更的改动：必须验证列位置、任务状态、计划汇总数字、详情面板状态和执行日志显示保持一致。
- 涉及 Claude / Codex 双运行时的改动：如果改动覆盖运行时选择或上下文构建，至少各验证一条真实链路；若受环境限制无法双跑，必须在结论里明确说明未覆盖原因。

---

## 安全与清理

- 禁止提交本机日志、运行截图、临时测试文件、导出数据、数据库文件、密钥和机器相关配置。
- 修改 `src-tauri/src/commands/`、数据库初始化逻辑、安装回滚逻辑时，必须优先考虑数据安全和可回滚性。
- 除非任务明确要求，否则不要改动端口、持久化目录结构、数据库关键字段和 Tauri 权限配置。

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **easy-agent-pilot** (16347 symbols, 30791 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/easy-agent-pilot/context` | Codebase overview, check index freshness |
| `gitnexus://repo/easy-agent-pilot/clusters` | All functional areas |
| `gitnexus://repo/easy-agent-pilot/processes` | All execution flows |
| `gitnexus://repo/easy-agent-pilot/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |
| Work in the Stores area (993 symbols) | `.claude/skills/generated/stores/SKILL.md` |
| Work in the Commands area (872 symbols) | `.claude/skills/generated/commands/SKILL.md` |
| Work in the Strategies area (189 symbols) | `.claude/skills/generated/strategies/SKILL.md` |
| Work in the Composables area (149 symbols) | `.claude/skills/generated/composables/SKILL.md` |
| Work in the Conversation area (105 symbols) | `.claude/skills/generated/conversation/SKILL.md` |
| Work in the Unattended area (103 symbols) | `.claude/skills/generated/unattended/SKILL.md` |
| Work in the FileTree area (99 symbols) | `.claude/skills/generated/filetree/SKILL.md` |
| Work in the TaskSplitDialog area (70 symbols) | `.claude/skills/generated/tasksplitdialog/SKILL.md` |
| Work in the Plan area (58 symbols) | `.claude/skills/generated/plan/SKILL.md` |
| Work in the Tabs area (51 symbols) | `.claude/skills/generated/tabs/SKILL.md` |
| Work in the Memory area (47 symbols) | `.claude/skills/generated/memory/SKILL.md` |
| Work in the MessageBubble area (33 symbols) | `.claude/skills/generated/messagebubble/SKILL.md` |
| Work in the MemoryAuthoringDialog area (30 symbols) | `.claude/skills/generated/memoryauthoringdialog/SKILL.md` |
| Work in the Message area (30 symbols) | `.claude/skills/generated/message/SKILL.md` |
| Work in the RichMarkdownEditor area (29 symbols) | `.claude/skills/generated/richmarkdowneditor/SKILL.md` |
| Work in the TaskExecutionLog area (29 symbols) | `.claude/skills/generated/taskexecutionlog/SKILL.md` |
| Work in the Usage area (28 symbols) | `.claude/skills/generated/usage/SKILL.md` |
| Work in the MessageList area (27 symbols) | `.claude/skills/generated/messagelist/SKILL.md` |
| Work in the MonacoCodeEditor area (27 symbols) | `.claude/skills/generated/monacocodeeditor/SKILL.md` |
| Work in the Compression area (25 symbols) | `.claude/skills/generated/compression/SKILL.md` |

<!-- gitnexus:end -->
