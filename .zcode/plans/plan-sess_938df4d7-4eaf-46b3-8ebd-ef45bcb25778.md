# 实施计划：空白页修复 + 居中欢迎屏 + 时间问候 + 发送动画 + Git 分支显示

## 背景结论（探索已确认）
- **空白页根因**：`MessageArea.vue:92` 用 `<template v-if="sessionStore.currentSessionId">` 包裹全部内容，**无 `v-else`**。所有 tab 关闭 → `currentSessionId=null` → 只剩空 `<div>`。
- **欢迎屏脚手架已存在但未接线**：locale key `messageArea.welcome.*`（zh-CN.ts:195-204）、`useMessageArea.ts:616-620` 的 `hasProjects`/`handleImportProject` 已 return 但 `.vue` 未消费。
- **居中/动画**：`.message-area__conversation` 是 flex 列，`MessageList flex:1` + `ConversationComposer flex-shrink:0` 常驻底部。`MessageList.vue:159-191` 有空状态卡片但永远在顶部。
- **Git 分支：后端+store 完全就绪，UI 从未渲染**。`get_project_git_branch`（git.rs，lib.rs:162 注册）+ `projectStore.currentBranch`（project.ts:117，`loadCurrentBranch` 543-556，切项目自动刷新 559-561）—— 但全项目**零组件**读取 `currentBranch`。
- **发送阻塞**：`handleSend`（useComposerSender.ts:566）`if (!sessionId) return` → 无会话时发送无效。同文件 `createSessionAndSend`（377-416）可创建并发送。

## 统一设计：单一欢迎模式
`isWelcomeMode = !currentSessionId || 当前会话消息数 === 0`
- 无 tab / 新建空会话 → 都进欢迎模式（居中问候 + 居中输入框 + 分支 chip）
- 发出第一条消息后 → 消息到达 → `isWelcomeMode=false` → 过渡到聊天模式（输入框到底部）

---

## 任务 1+2+3：欢迎屏 + 居中 + 动画

### 1. `src/locales/zh-CN.ts` & `src/locales/en-US.ts`
扩展 `messageArea.welcome`，新增 6 时段问候（你已批准的文案）：
```
greetings: { lateNight(00-04), morning(05-10), noon(11-13), afternoon(14-17), evening(18-22), night(23) }
```
zh-CN 用批准文案（下午 = "下午好呀，让我来帮你写代码，你赶紧去摸鱼"）；en-US 做对应调皮翻译。同时新增 `startTip`（输入框下小提示，如"按 Enter 发送，Shift+Enter 换行"）。

### 2. `src/components/layout/messageArea/MessageArea/useMessageArea.ts`
- 新增 `currentMessageCount` computed（`messageStore.messagesBySession(currentSessionId).length`）。
- 新增 `isWelcomeMode` computed：`!sessionStore.currentSessionId || currentMessageCount.value === 0`。
- 新增 `greeting` computed：按 `new Date().getHours()` 选时段文案（6 分段：00-04/05-10/11-13/14-17/18-22/23）。
- return 新增 `isWelcomeMode`、`greeting`、`currentMessageCount`（`hasProjects`/`handleImportProject` 已存在）。

### 3. `src/components/layout/messageArea/MessageArea/MessageArea.vue`
重构模板为两态切换（保持单 composer 实例的简单方案——欢迎态与聊天态各渲染一个 composer，因切换发生在发送瞬间、输入框已被 `clearComposerDraft` 清空，remount 无副作用）：
```
<div class="message-area">
  <Transition name="ea-welcome-switch">
    <!-- 欢迎模式 -->
    <div v-if="isWelcomeMode" class="message-area__welcome-stage">
      <div class="message-area__welcome-card">
        <p class="message-area__greeting-kicker">{{ greeting.emoji }} {{ greeting.period }}</p>
        <h2 class="message-area__greeting-text">{{ greeting.text }}</h2>
        <!-- 无项目时显示导入 CTA（复用 hasProjects/handleImportProject） -->
      </div>
      <ConversationComposer
        :session-id="sessionStore.currentSessionId"
        panel-type="main"
        @focus="handleComposerFocus"
      />
    </div>
    <!-- 聊天模式（现有 workspace，原样保留） -->
    <div v-else class="message-area__workspace" ...> ... </div>
  </Transition>
</div>
```
> 注意：聊天模式分支保留原有 `<template v-if="currentSessionId">` 内的全部逻辑（trace/plan panes、MessageList、composer）。欢迎态不挂 trace/plan（空会话无意义）。

### 4. `src/components/layout/messageArea/MessageArea/styles.css`
- `.message-area__welcome-stage`：`flex:1; display:flex; flex-direction:column; justify-content:center; align-items:center; gap:24px; padding:24px;` —— 让 {问候卡, composer} 垂直居中。
- `.message-area__welcome-card`：居中、最大宽度限制（如 640px）、文本居中。
- `.message-area__greeting-text`：较大字号、次要前景色。
- `Transition name="ea-welcome-switch"`：`opacity + translateY(8px)` 0.3s ease 过渡（CSS 过渡方案，已确认）。
- composer 在欢迎态约束最大宽度（如 `max-width:720px; width:100%`）保持视觉聚焦。

### 5. `src/composables/conversationComposer/useComposerSender.ts`（发送解锁）
调整 `handleSend`（564-585）：把输入解析（rawInput/expandedInput/userInput/displayInput）提到 sessionId 校验之前；将
```ts
if (!sessionId || isUploadingImages.value) return
```
改为
```ts
if (isUploadingImages.value) return
// 无会话（欢迎模式）：走"创建并发送"
if (!sessionId) {
  if (!displayInput && attachmentsPayload.length === 0) return
  if (!projectStore.currentProjectId) { uiStore.openProjectCreateModal(); return }
  dispatchingSessionId.value = null
  await createSessionAndSend(userInput, displayInput)
  return
}
```
这样：欢迎屏键入回车 → `createSessionAndSend` 建会话+开 tab+发消息 → `currentSessionId` 落定 → 消息到达 → `isWelcomeMode` 翻 false → 过渡到聊天模式，输入框滑到底部（即任务 3 动画）。

---

## 任务 4：Git 分支显示（输入框上方）

### 6. `src/components/layout/conversationComposer/useConversationComposer.ts`
- `import { useProjectStore } from '@/stores/project'`
- `const projectStore = useProjectStore()`
- return 新增 `currentBranch: projectStore.currentBranch`。

### 7. `src/components/layout/conversationComposer/ConversationComposer.vue`
在 `.conversation-composer__path-row`（159-173，main 面板）的 path pill 旁追加分支 chip：
```vue
<span
  v-if="currentBranch"
  class="conversation-composer__branch"
  :title="`git: ${currentBranch}`"
>
  <EaIcon name="git-branch" :size="12" />
  <span>{{ currentBranch }}</span>
</span>
```
> 无 git（`currentBranch` 为 null）则 `v-if` 自动不渲染，满足"没 git 就不显示"。mini 面板 status 行（191-201）同步加一份保持一致。`git-branch` 图标需确认已在 EaIcon 图标集，缺失则用 `git-commit-horizontal` 或同名 lucide 图标。

### 8. `src/components/layout/conversationComposer/styles.css`
新增 `.conversation-composer__branch`：小号 chip 样式（次要色背景、圆角、与 path pill 同高同间距、flex 居中）。

---

## 验收门禁（遵循 tauri-harness 八道门禁中的前端相关项）
1. `pnpm typecheck`（vue-tsc 零 error）
2. `pnpm lint`（eslint --fix 零 error）
3. `pnpm build`（vue-tsc + vite build 通过）
> 本次改动不涉及 Rust，无需 cargo clippy。若实施中发现需要后端改动（如 detached-HEAD 返回短 hash），再补 `git.rs` + clippy。

## 端到端测试（Tauri MCP）
1. 后台启动 `pnpm tauri dev`，等待 webview 就绪。
2. `driver_session start`（默认端口 9223）连接应用。
3. 用 `webview_dom_snapshot`/`webview_screenshot` 验证：
   - **任务1**：关闭全部 tab → 不再空白，显示欢迎屏。
   - **任务2**：根据当前时段（2026-07-16 测试时刻）核对问候文案；输入框居中。
   - **任务3**：在居中输入框键入"你好"→ 回车 → 输入框平滑过渡到底部，消息出现在上方，tab 出现。
   - **任务4**：输入框上方 path 行显示 `git-branch icon + feature/2.0.0`（当前分支）。
4. 截图归档，`driver_session stop` 收尾。

## 风险与备注
- **GitNexus 影响分析**：当前环境未挂载 `mcp__gitnexus__*` 工具，无法执行 AGENTS.md 要求的 `impact`/`detect_changes`。本次均为前端 UI 接线 + 一处发送分支调整，影响面小且本地化，改完后用 `pnpm typecheck/lint/build` 兜底。
- **composer remount**：欢迎态→聊天态切换会重建 composer 实例；因切换发生在首次发送后、此时 draft 已清空，无状态丢失。
- **greeting 时段边界**：严格按 00-04/05-10/11-13/14-17/18-22/23 六段，hour 用 24h制 `getHours()`。
