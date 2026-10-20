# 实现计划：侧边栏紧凑化 + 会话来源标识

## 任务 1：侧边栏 UI 紧凑化

### 1.1 合并展开箭头到项目图标（鼠标悬停切换）

**文件：** `src/components/layout/UnifiedPanelProjectEntry/UnifiedPanelProjectEntry.vue`

- 删除 `.project-item__arrow` 整个 div（当前第 56-69 行）
- 将展开/收起视觉指示合并到 `.project-item__icon` 上：
  - 默认状态：显示 `folder` 图标
  - `.project-item:hover` 时：folder 图标淡出，显示 `chevron-right`（收起态）或 `chevron-down`（展开态）
  - 用 CSS 控制两个图标的叠加 + opacity 切换（folder 在底层，chevron 在上层 hover 时 opacity:1）
- 保留 loading spinner 逻辑：当 `isProjectRowLoading` 时，图标区域显示 spinner

**文件：** `src/components/layout/UnifiedPanelProjectEntry/styles.css`
- 删除 `.project-item__arrow` 相关样式（第 50-62 行、第 549-553 行）
- 重写 `.project-item__icon` 为相对定位容器，内含两个绝对定位的图标层
- hover 时通过 `.project-item:hover .project-item__icon-folder { opacity: 0 }` + `.project-item:hover .project-item__icon-chevron { opacity: 1 }` 实现切换
- 展开态 chevron 用 `rotate(90deg)` 或直接用 `chevron-down`

### 1.2 图标左对齐

**文件：** `src/components/layout/UnifiedPanelProjectEntry/styles.css`
- 箭头移除后，项目图标自然左移到行首。调整 `.project-item { gap }` 从 `7px` → `6px`
- 会话列表 `.project-content { margin-left }` 从 `var(--spacing-2)` (8px) → `0`，让会话内容与项目图标对齐
- `.session-item { padding-left }` 从 `6px` → `8px`，使会话名称与项目图标中心对齐

### 1.3 整体紧凑化（减少留白）

**文件：** `src/components/layout/UnifiedPanel/styles.css`
- `.unified-panel__section-header { padding: 11px 10px 7px }` → `padding: 8px 8px 5px`
- `.unified-panel__content { padding: 0 8px 10px }` → `padding: 0 6px 6px`
- `.project-list { gap: 3px }` → `gap: 2px`

**文件：** `src/components/layout/UnifiedPanelProjectEntry/styles.css`
- `.project-item { padding: 7px 8px }` → `padding: 5px 8px`
- `.project-content { margin-bottom: 3px }` → `margin-bottom: 1px`

**文件：** `src/components/layout/UnifiedPanelSessionList/styles.css`
- `.session-list { padding: 1px 0 4px }` → `padding: 1px 0 2px`
- `.session-item { padding: 3px 6px }` → `padding: 2px 6px`

### 1.4 减小侧边栏宽度

**文件：** `src/components/layout/MainLayout/MainLayout.vue`
- `:sidebar-width="280"` → `:sidebar-width="248"`

**文件：** `src/components/layout/WorkspaceShell/useWorkspaceShell.ts`
- 默认宽度 `sidebarWidth: 280` → `248`（与 MainLayout 保持一致）

---

## 任务 2：会话来源标识（完整推断）

### 2.1 后端：扩展 list_sessions 查询，计算 source 字段

**文件：** `src-tauri/sql/session.html`

修改 `list_sessions` 的 SELECT，用 `CASE WHEN ... EXISTS(...)` 子查询推断来源：

```sql
select s.*, ...,
  case
    when exists(select 1 from unattended_threads ut where ut.session_id = s.id) then 'unattended'
    when exists(select 1 from agent_cli_usage_records u where u.session_id = s.id and u.execution_mode = 'task_split') then 'plan_split'
    when exists(select 1 from task_runtime_bindings trb where trb.external_session_id = s.cli_session_id) then 'task'
    when exists(select 1 from solo_runtime_bindings srb where srb.external_session_id = s.cli_session_id) then 'solo'
    when exists(select 1 from agent_cli_usage_records u where u.session_id = s.id and u.execution_mode = 'solo_execution') then 'solo'
    when exists(select 1 from agent_cli_usage_records u where u.session_id = s.id and u.execution_mode = 'task_execution') then 'task'
    when exists(select 1 from agent_cli_usage_records u where u.session_id = s.id and u.execution_mode = 'chat') then 'chat'
    when s.name like '无人值守%' then 'unattended'
    when s.cli_session_id is not null then 'cli'
    else 'chat'
  end as source
from sessions s where s.project_id = #{project_id} order by ...
```

同样修改 `get_session_by_id` 查询。

### 2.2 后端：扩展 Row 模型 + DTO

**文件：** `src-tauri/src/models/mod.rs`
- `SessionRow` 新增字段 `pub source: Option<String>`

**文件：** `src-tauri/src/commands/session.rs`
- `Session` DTO 新增 `pub source: String`（带 `#[serde(rename_all = "camelCase")]` 已在结构体上）
- `session_row_to_session()` 映射新增 `source: row.source.unwrap_or_else(|| "chat".to_string())`

### 2.3 前端：扩展类型 + Store

**文件：** `src/stores/session.ts`
- `Session` interface 新增 `source: SessionSource`
- 新增类型 `type SessionSource = 'chat' | 'unattended' | 'plan_split' | 'task' | 'solo' | 'cli'`
- `RustSession` 新增 `source?: string`
- `transformSession()` 映射 `source: s.source ?? 'chat'`

### 2.4 前端：会话列表添加来源徽标

**文件：** `src/components/layout/UnifiedPanelSessionList/UnifiedPanelSessionList.vue`
- 在 `.session-item__name` 后方或 `.session-item__lead` 位置，当 `session.source !== 'chat'` 时渲染一个来源徽标

**文件：** `src/components/layout/UnifiedPanelSessionList/useUnifiedPanelSessionList.ts`
- 新增 `getSessionSourceLabel(source)` / `getSessionSourceClass(source)` 工具函数
- 来源→标签映射：`unattended → 无人值守`, `plan_split → 计划`, `task → 任务`, `solo → SOLO`, `cli → CLI`

**文件：** `src/components/layout/UnifiedPanelSessionList/styles.css`
- 新增 `.session-item__source-badge` 样式：小号圆角标签，低饱和背景色，字号 10px
- 每种来源用不同的中性色调（不引入亮色），与现有 workspace 设计体系一致

### 2.5 国际化

**文件：** `src/locales/zh-CN.ts` + `src/locales/en-US.ts`
- 新增 `session.source.chat / unattended / plan_split / task / solo / cli` 翻译键

---

## 验收

- `pnpm typecheck` + `pnpm lint` 通过
- `cd src-tauri && cargo clippy` 通过
- 手动验证：
  1. 项目图标 hover 切换 folder↔chevron，点击展开/收起正常
  2. 项目图标与会话列表左对齐
  3. 侧边栏整体更紧凑，宽度 248px
  4. 不同来源的会话显示对应徽标（主会话无徽标，无人值守/计划/任务/SOLO 有徽标）
