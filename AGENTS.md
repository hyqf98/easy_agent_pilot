# 仓库协作规范

## 项目概览
本项目是 `Tauri 2 + Vue 3 + TypeScript + Rust` 的桌面应用。

- 前端开发服务使用 Vite，固定端口为 `1430`
- Vite HMR 端口为 `1431`
- Tauri 开发态 `devUrl` 为 `http://localhost:1430`
- Tauri 调试态启用了 `tauri-plugin-mcp-bridge`
- Tauri MCP Bridge 基准端口为 `9423`

开发、联调、自动化测试时，默认按“前端 1430 + Tauri 2 宿主 + MCP Bridge 9423”理解当前运行环境，不要自行改成其他端口，除非任务明确要求。

## 目录与模块说明

### 前端模块
`src/` 为前端主目录，按业务域拆分，而不是按纯技术层堆叠。

- `src/views/`
  路由级页面入口。当前包含主页工作台、设置页、MCP 测试页、Mini Panel 页面。
- `src/components/layout/`
  主工作区骨架。负责顶部栏、侧边导航、项目区、会话区、消息区、统一面板、文件编辑器切换。
- `src/components/settings/`
  设置中心。当前菜单包括：通用设置、智能体设置、Agent 配置、Marketplace、Provider Switch、会话管理、主题、LSP、数据管理、日志管理。
- `src/components/plan/`
  计划模式核心模块。负责计划列表、计划新建/编辑、任务拆分、拆分预览、任务看板、任务详情、计划进度、执行日志、继续拆分等流程。
- `src/components/memory/`
  记忆模式模块。负责记忆库、原始记忆池、AI 合并、批量删除、Markdown 记忆维护。
- `src/components/message/`
  会话消息渲染模块。负责消息气泡、Markdown 渲染、Thinking 展示、工具调用展示、执行时间线、结构化结果渲染。
- `src/components/marketplace/`
  市场模块。负责 MCP、Skill、Plugin 的列表、详情、安装、启停、更新入口。
- `src/components/skill-config/`
  Agent 维度的 MCP / Skill / Plugin 配置中心，包含配置列表、编辑器、文件工作区、详情侧栏等。
- `src/components/file-tree/`
  项目文件树与文件操作模块，负责重命名、移动、删除、上下文菜单等。
- `src/components/agent/`
  智能体配置与模型管理模块，负责 Agent 表单、模型编辑、Claude 配置扫描等。
- `src/components/project/`
  项目创建与项目入口相关组件。
- `src/components/common/`
  通用 UI 组件库，如按钮、输入框、弹窗、选择器、图标、骨架屏、进度条、提示组件。
- `src/modules/file-editor/`
  文件编辑子系统。基于 Monaco，负责编辑器工作区、语言策略、文件编辑服务、LSP 接入。
- `src/stores/`
  Pinia 状态管理层。覆盖项目、会话、消息、计划、任务、任务执行、设置、主题、窗口状态、记忆、Marketplace、Agent 配置等状态。
- `src/services/conversation/`
  会话执行服务层。统一封装 Claude/Codex 的 CLI 与 SDK 执行策略、消息构建、执行器、文件追踪。
- `src/services/plan/`
  计划编排服务层。负责任务拆分编排、动态表单、执行进度管理、计划提示词。
- `src/services/memory/`
  记忆合并服务层，负责原始记忆压缩、项目记忆提示词、记忆库合成逻辑。
- `src/services/compression/`
  会话压缩能力，负责长上下文压缩与压缩结果组织。
- `src/composables/`
  组合式逻辑封装，如消息编辑、会话视图、快捷键、异步操作、对话框、外部点击处理等。
- `src/router/`
  路由定义与页面标题更新逻辑。
- `src/locales/`
  中英文文案资源。
- `src/types/`
  统一类型定义，如计划、任务执行、记忆、时间线、文件追踪等。
- `src/utils/`
  工具层，负责日志、校验、MCP 配置、会话工具输入、计划执行文本、结构化内容转换等。
- `src/styles/`
  全局样式、变量与动画定义。

### 当前页面与业务入口说明
- 主会话
  位于主页工作台，核心由 `MainLayout`、`PanelContainer`、`SessionTabs`、`MessageArea` 组成，用于项目上下文下的多会话对话、消息展示和文件编辑切换。
- 菜单设置
  入口在顶部 `AppHeader` 设置按钮，进入 `SettingsView`。设置导航由 `SettingsNav` 和 `settingsTabs.ts` 管理。
- 计划拆分
  从计划列表进入，核心弹窗为 `TaskSplitDialog`。该流程负责 AI 拆分、表单补充、拆分预览、二次拆分、确认生成任务。
- 计划执行
  由 `TaskBoard`、`KanbanColumn`、`TaskExecutionLog`、`PlanProgressDetail` 组成。支持待办执行、一键执行、暂停、恢复、失败重试、日志查看。
- 记忆管理
  由 `MemoryModePanel` 负责，支持记忆库维护、原始记忆筛选、批量删除、AI 合并入库。

### 后端模块
`src-tauri/src/` 为 Tauri 2 Rust 后端。

- `src-tauri/src/lib.rs`
  Tauri 应用入口。负责插件注册、命令注册、数据库初始化、日志初始化、计划调度恢复、MCP Bridge 启动。
- `src-tauri/src/database/`
  数据库初始化与表结构准备逻辑，当前持久化核心为本地 SQLite。
- `src-tauri/src/logging.rs`
  运行时日志初始化与日志写入。
- `src-tauri/src/scheduler/`
  计划调度模块，负责定时计划恢复、定时触发与后台调度。
- `src-tauri/src/commands/agent.rs`
  智能体管理命令。
- `src-tauri/src/commands/agent_config.rs`
  Agent 关联的 MCP、Skill、Plugin、Model 配置命令。
- `src-tauri/src/commands/app_state.rs`
  应用状态读写命令。
- `src-tauri/src/commands/cli.rs`
  CLI 工具检测、路径管理、迁移等命令。
- `src-tauri/src/commands/cli_config.rs`
  CLI 配置文件读写与同步命令。
- `src-tauri/src/commands/cli_installer.rs`
  CLI 安装、升级、取消安装等命令。
- `src-tauri/src/commands/conversation/`
  会话执行后端能力，负责 Claude/Codex CLI/SDK 执行、流式输出、中断、执行策略与统一执行器。
- `src-tauri/src/commands/data.rs`
  数据导出、导入、清空、统计命令。
- `src-tauri/src/commands/file_editor.rs`
  项目文件读取、写入、语言识别命令。
- `src-tauri/src/commands/install.rs`
  安装会话记录、回滚、安装状态维护命令。
- `src-tauri/src/commands/lsp.rs`
  LSP 服务下载、移除、激活等命令。
- `src-tauri/src/commands/marketplace.rs`
  市场源管理命令。
- `src-tauri/src/commands/mcp.rs`
  MCP Server 管理、测试、工具调用命令。
- `src-tauri/src/commands/mcp_market.rs`
  MCP 市场拉取、安装、启停、卸载、更新命令。
- `src-tauri/src/commands/mcpmarket_source.rs`
  MCP 市场源选项与来源处理命令。
- `src-tauri/src/commands/memory.rs`
  记忆库、原始记忆、记忆合并命令。
- `src-tauri/src/commands/message.rs`
  消息 CRUD、清空、图片上传命令。
- `src-tauri/src/commands/mini_panel.rs`
  Mini Panel 显示、隐藏、目录、快捷键等命令。
- `src-tauri/src/commands/plan.rs`
  计划 CRUD、计划状态、计划调度命令。
- `src-tauri/src/commands/plan_split.rs`
  计划拆分会话、拆分日志、开始/继续/停止拆分、提交表单命令。
- `src-tauri/src/commands/plugins_market.rs`
  插件市场拉取、安装、启停、卸载命令。
- `src-tauri/src/commands/project.rs`
  项目 CRUD、目录校验、文件列举、文件移动删除重命名命令。
- `src-tauri/src/commands/project_access.rs`
  最近项目访问记录命令。
- `src-tauri/src/commands/provider_profile.rs`
  Provider Profile 管理与切换命令。
- `src-tauri/src/commands/runtime_log.rs`
  运行日志摘要、文件列表、内容读取、清理命令。
- `src-tauri/src/commands/scan.rs`
  CLI 配置、MCP、会话扫描命令。
- `src-tauri/src/commands/session.rs`
  会话 CRUD、置顶命令。
- `src-tauri/src/commands/settings.rs`
  应用设置读写命令。
- `src-tauri/src/commands/skill_plugin.rs`
  Skill / Plugin 文件读写与脚手架命令。
- `src-tauri/src/commands/skills_market.rs`
  Skill 市场拉取、安装、启停、卸载、更新命令。
- `src-tauri/src/commands/task.rs`
  任务 CRUD、排序、重试、批量更新、拆分会话存储命令。
- `src-tauri/src/commands/task_execution.rs`
  任务执行日志、执行结果、计划执行进度命令。
- `src-tauri/src/commands/window.rs`
  多窗口与会话锁定相关命令。

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
- 复杂流程要拆为“小函数 + 主流程编排”，不要让单个函数承担过多职责。
- Vue 组件保持单一职责。页面组件负责装配，复杂业务逻辑优先下沉到 store / composable / service。
- Rust 命令层只负责输入输出编排，数据库访问、执行策略、状态判断优先下沉到可复用函数。

### 设计建议
- Provider、CLI、SDK、不同 Agent 的执行流程优先使用策略模式。
- 市场安装、回滚、升级、同步等流程优先使用命令模式或服务对象封装。
- 会话状态、计划状态、执行状态优先使用枚举和显式状态流转，禁止散乱布尔值堆叠。
- 表单、任务拆分、执行时间线等动态流程优先采用配置驱动和映射表，而不是硬编码分支。

## 注释规范

### 前端注释要求
- Vue 组件、Pinia store、composable、service 中的公共方法、异步编排、复杂状态流转必须写标准注释。
- 组件注释要说明“用途、关键依赖、主要事件或状态”。
- store action 注释要说明“输入、输出、副作用、依赖的 Tauri 命令”。
- service / composable 注释要说明“封装目的、适用范围、关键边界条件”。
- 对明显简单的赋值或模板渲染不要写废话注释。
- 推荐使用 JSDoc 风格，例如 `/** 重新加载当前计划任务，并保持当前选中态 */`。

### 后端注释要求
- `pub` 级别结构体、枚举、函数、Tauri command 必须使用 Rust 文档注释 `///`。
- 命令注释至少说明“用途、主要参数、返回值、关键副作用”。
- 涉及数据库写入、文件写入、进程调用、网络请求、调度恢复时，必须说明边界与风险。
- 私有复杂逻辑使用简洁行注释 `//`，解释为什么这样做，不要解释表面代码。

## 开发与提交流程
- 修改前先确认变更属于前端、后端还是跨端联动。
- 涉及 UI 的功能开发，必须同步检查会话页、计划页、设置页是否受影响。
- 涉及计划拆分或任务执行的改动，必须考虑创建、继续、停止、重试、日志、结果回写链路。
- 涉及设置项或 Provider 配置的改动，必须考虑持久化、初始化加载、切换回显。
- 提交信息建议使用简短 Conventional Commit 风格，如 `fix: 修复计划继续拆分状态异常`、`feat: 新增日志筛选能力`。

## 测试要求

### 基础要求
- 所有变更至少满足“编译通过”。
- 前端改动后至少执行 `pnpm build`。
- 后端或跨端改动后至少执行 `cargo check --manifest-path src-tauri/Cargo.toml`。
- 涉及 Vue 组件、store、composable 的改动，补充执行 `pnpm lint`。

### Tauri MCP 验证要求
- 所有页面级改动都必须使用 Tauri MCP 做页面访问验证，至少确认对应页面可以正常打开、主要区域正常渲染、无明显报错。
- 涉及设置菜单、主会话、计划拆分、计划执行、记忆管理、Marketplace 的改动，必须使用 Tauri MCP 实际进入对应页面或弹窗验证。
- 如果是功能开发，不接受只看代码不走流程，必须使用 Tauri MCP 自动化工具完成全流程验证。

### 全流程验证要求
- 主会话相关改动：至少验证项目进入、会话切换、消息区展示、必要操作按钮可用。
- 设置相关改动：至少验证设置入口、对应菜单页签、表单项读写或展示正常。
- 计划拆分相关改动：至少验证创建计划、启动拆分、表单补充或预览、确认生成任务链路。
- 计划执行相关改动：至少验证任务加载、开始执行、暂停或恢复、日志查看、状态回写链路。
- 记忆相关改动：至少验证记忆库列表、原始记忆筛选、编辑或合并流程。

## 安全与清理
- 禁止提交本机日志、运行截图、临时测试文件、导出数据、数据库文件、密钥和机器相关配置。
- 修改 `src-tauri/src/commands/`、数据库初始化逻辑、安装回滚逻辑时，必须优先考虑数据安全和可回滚性。
- 除非任务明确要求，否则不要改动端口、持久化目录结构、数据库关键字段和 Tauri 权限配置。
