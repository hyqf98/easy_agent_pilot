<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { usePomodoroStore } from "../../stores/pomodoro";

const {
  state,
  activeTasks,
  completedTasks,
  totalTasks,
  completionRate,
  hydrateStore,
  addTask,
  toggleTask,
  removeTask,
  startTimer,
  pauseTimer,
  resetTimer,
  tickTimer,
} = usePomodoroStore();

const newTaskTitle = ref("");
let intervalId: ReturnType<typeof setInterval> | null = null;

const timerLabel = computed(() => {
  const minutes = Math.floor(state.remainingSeconds / 60);
  const seconds = state.remainingSeconds % 60;
  return `${`${minutes}`.padStart(2, "0")}:${`${seconds}`.padStart(2, "0")}`;
});

const focusStatus = computed(() => {
  if (state.isRunning) {
    return "专注进行中";
  }

  if (state.remainingSeconds !== state.durationSeconds) {
    return "已暂停";
  }

  return "准备开始";
});

const nextTaskLabel = computed(() => activeTasks.value[0]?.title ?? "选一件最重要的事开始");

const completionSummary = computed(() => {
  if (!totalTasks.value) {
    return "今日还没有任务，先写下一件要完成的事。";
  }

  return `今日任务完成率 ${completionRate.value}%`;
});

const lastSessionLabel = computed(() => {
  if (!state.lastCompletedAt) {
    return "上一轮番茄钟尚未完成";
  }

  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
  }).format(state.lastCompletedAt);
});

function handleAddTask() {
  addTask(newTaskTitle.value);
  newTaskTitle.value = "";
}

function toggleTimer() {
  if (state.isRunning) {
    pauseTimer();
    stopTimerLoop();
    return;
  }

  startTimer();
  startTimerLoop();
}

function handleReset() {
  resetTimer();
  stopTimerLoop();
}

/**
 * 页面层维持一个轻量轮询，驱动 UI 与本地恢复逻辑保持同步。
 */
function startTimerLoop() {
  if (intervalId) {
    return;
  }

  intervalId = window.setInterval(() => {
    tickTimer();
    if (!state.isRunning) {
      stopTimerLoop();
    }
  }, 1000);
}

function stopTimerLoop() {
  if (!intervalId) {
    return;
  }

  window.clearInterval(intervalId);
  intervalId = null;
}

onMounted(() => {
  hydrateStore();

  if (state.isRunning) {
    startTimerLoop();
  }
});

onUnmounted(() => {
  stopTimerLoop();
});
</script>

<template>
  <main class="pomodoro-panel">
    <section class="hero-card">
      <div class="hero-copy">
        <p class="eyebrow">SOLO FOCUS DESK</p>
        <h1>今日任务与番茄钟，留在一张干净的桌面上。</h1>
        <p class="hero-text">
          先列出今天必须完成的事，再用一个 25 分钟周期把注意力收回来。状态会保存在本地，刷新后仍能继续。
        </p>
      </div>

      <div class="hero-stats">
        <article class="stat-chip">
          <span class="stat-value">{{ activeTasks.length }}</span>
          <span class="stat-label">待完成</span>
        </article>
        <article class="stat-chip">
          <span class="stat-value">{{ completedTasks.length }}</span>
          <span class="stat-label">已勾选</span>
        </article>
        <article class="stat-chip accent">
          <span class="stat-value">{{ state.completedPomodoros }}</span>
          <span class="stat-label">番茄钟完成</span>
        </article>
      </div>
    </section>

    <section class="workspace-grid">
      <article class="panel task-panel">
        <div class="panel-heading">
          <div>
            <p class="panel-kicker">TODAY</p>
            <h2>今日任务列表</h2>
          </div>
          <span class="muted">{{ completionSummary }}</span>
        </div>

        <form class="task-form" @submit.prevent="handleAddTask">
          <label class="sr-only" for="task-title">新增任务</label>
          <input
            id="task-title"
            v-model="newTaskTitle"
            class="task-input"
            maxlength="60"
            placeholder="例如：整理迭代说明、回完重点消息"
            type="text"
          />
          <button class="action-button solid" type="submit">新增任务</button>
        </form>

        <ul v-if="state.tasks.length" class="task-list">
          <li
            v-for="task in state.tasks"
            :key="task.id"
            class="task-row"
            :class="{ done: task.completed }"
          >
            <button
              class="task-toggle"
              type="button"
              :aria-pressed="task.completed"
              @click="toggleTask(task.id)"
            >
              <span class="toggle-dot" />
            </button>
            <div class="task-meta">
              <p class="task-title">{{ task.title }}</p>
              <p class="task-caption">
                {{ task.completed ? "已完成" : "待处理" }}
                <span v-if="task.completedAt"> · {{ new Date(task.completedAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) }}</span>
              </p>
            </div>
            <button class="icon-button" type="button" @click="removeTask(task.id)">删除</button>
          </li>
        </ul>

        <div v-else class="empty-card">
          <p>任务列表还是空的。</p>
          <span>写下第一件最值得用 25 分钟完成的事。</span>
        </div>
      </article>

      <article class="panel timer-panel">
        <div class="panel-heading">
          <div>
            <p class="panel-kicker">FOCUS</p>
            <h2>25 分钟倒计时</h2>
          </div>
          <span class="status-pill" :class="{ running: state.isRunning }">{{ focusStatus }}</span>
        </div>

        <div class="timer-core">
          <p class="timer-caption">当前建议聚焦</p>
          <h3 class="timer-task">{{ nextTaskLabel }}</h3>
          <div class="timer-display">{{ timerLabel }}</div>
          <p class="timer-footnote">完成时会自动累计番茄钟次数，并重置为下一轮。</p>
        </div>

        <div class="timer-actions">
          <button class="action-button solid large" type="button" @click="toggleTimer">
            {{ state.isRunning ? "暂停" : "开始专注" }}
          </button>
          <button class="action-button ghost large" type="button" @click="handleReset">重置</button>
        </div>
      </article>

      <article class="panel summary-panel">
        <div class="panel-heading">
          <div>
            <p class="panel-kicker">SUMMARY</p>
            <h2>完成统计</h2>
          </div>
          <span class="muted">本地持久化</span>
        </div>

        <div class="summary-stack">
          <div class="summary-card">
            <span class="summary-label">已完成任务</span>
            <strong>{{ completedTasks.length }}</strong>
            <small>共 {{ totalTasks }} 项</small>
          </div>
          <div class="summary-card">
            <span class="summary-label">番茄钟次数</span>
            <strong>{{ state.completedPomodoros }}</strong>
            <small>上一轮完成于 {{ lastSessionLabel }}</small>
          </div>
          <div class="summary-card tint">
            <span class="summary-label">桌面节奏</span>
            <strong>{{ focusStatus }}</strong>
            <small>{{ state.isRunning ? "离开页面后会自动补算剩余时间" : "随时可从当前进度继续" }}</small>
          </div>
        </div>

        <ul class="completed-list">
          <li v-for="task in completedTasks.slice(0, 4)" :key="task.id">
            {{ task.title }}
          </li>
          <li v-if="!completedTasks.length" class="muted">完成的任务会显示在这里。</li>
        </ul>
      </article>
    </section>
  </main>
</template>

<style scoped>
.pomodoro-panel {
  min-height: 100vh;
  padding: 28px;
  background:
    radial-gradient(circle at top left, rgba(205, 147, 106, 0.22), transparent 30%),
    radial-gradient(circle at bottom right, rgba(38, 63, 73, 0.16), transparent 28%),
    linear-gradient(180deg, #f4ece2 0%, #efe5d6 48%, #eadfce 100%);
  color: var(--ink-strong);
}

.hero-card,
.panel {
  border: 1px solid rgba(35, 28, 24, 0.12);
  border-radius: 28px;
  background: rgba(255, 250, 244, 0.76);
  box-shadow:
    0 24px 50px rgba(69, 51, 38, 0.12),
    inset 0 1px 0 rgba(255, 255, 255, 0.5);
  backdrop-filter: blur(18px);
}

.hero-card {
  display: grid;
  grid-template-columns: minmax(0, 1.8fr) minmax(260px, 1fr);
  gap: 24px;
  padding: 30px;
  margin-bottom: 20px;
}

.eyebrow,
.panel-kicker {
  margin-bottom: 10px;
  color: var(--accent-muted);
  font-size: 0.74rem;
  font-weight: 700;
  letter-spacing: 0.24em;
  text-transform: uppercase;
}

.hero-copy h1,
.panel-heading h2,
.timer-task {
  font-family: "Iowan Old Style", "Palatino Linotype", "Book Antiqua", Georgia, serif;
}

.hero-copy h1 {
  max-width: 12ch;
  font-size: clamp(2.4rem, 4vw, 4.2rem);
  line-height: 0.95;
}

.hero-text {
  max-width: 58ch;
  margin-top: 18px;
  color: var(--ink-soft);
  font-size: 1rem;
}

.hero-stats {
  display: grid;
  gap: 14px;
}

.stat-chip {
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-height: 118px;
  padding: 20px;
  border-radius: 22px;
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid rgba(35, 28, 24, 0.08);
}

.stat-chip.accent {
  background: linear-gradient(135deg, rgba(145, 74, 48, 0.94), rgba(87, 52, 34, 0.98));
  color: #fff8f2;
}

.stat-value {
  font-size: 2.2rem;
  font-weight: 700;
  line-height: 1;
}

.stat-label,
.muted,
.task-caption,
.timer-caption,
.timer-footnote,
.summary-card small,
.completed-list {
  color: var(--ink-soft);
}

.workspace-grid {
  display: grid;
  grid-template-columns: minmax(320px, 1.2fr) minmax(300px, 0.9fr) minmax(280px, 0.9fr);
  gap: 20px;
  align-items: stretch;
}

.panel {
  display: flex;
  flex-direction: column;
  min-height: 520px;
  padding: 24px;
}

.panel-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.panel-heading h2 {
  font-size: 1.55rem;
  line-height: 1;
}

.muted {
  font-size: 0.92rem;
}

.task-form {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 12px;
  margin-bottom: 20px;
}

.task-input {
  min-height: 52px;
  padding: 0 16px;
  border: 1px solid rgba(35, 28, 24, 0.15);
  border-radius: 16px;
  background: rgba(255, 252, 248, 0.84);
  color: var(--ink-strong);
  font: inherit;
  outline: none;
}

.task-input:focus {
  border-color: rgba(150, 79, 52, 0.68);
  box-shadow: 0 0 0 4px rgba(170, 101, 64, 0.12);
}

.action-button,
.icon-button,
.task-toggle {
  border: none;
  cursor: pointer;
  transition:
    transform 140ms ease,
    background-color 140ms ease,
    color 140ms ease,
    box-shadow 140ms ease;
}

.action-button:hover,
.icon-button:hover,
.task-toggle:hover {
  transform: translateY(-1px);
}

.action-button {
  min-height: 52px;
  padding: 0 18px;
  border-radius: 16px;
  font-weight: 700;
  font-size: 0.95rem;
}

.action-button.large {
  width: 100%;
}

.action-button.solid {
  background: linear-gradient(135deg, #9f5f3f, #73412b);
  color: #fffaf5;
  box-shadow: 0 16px 28px rgba(121, 69, 46, 0.18);
}

.action-button.ghost {
  background: rgba(255, 252, 248, 0.9);
  color: var(--ink-strong);
  border: 1px solid rgba(35, 28, 24, 0.12);
}

.task-list,
.completed-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.task-list {
  display: grid;
  gap: 12px;
  overflow: auto;
}

.task-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 14px;
  align-items: center;
  padding: 14px 16px;
  border-radius: 20px;
  background: rgba(255, 252, 248, 0.88);
  border: 1px solid rgba(35, 28, 24, 0.08);
}

.task-row.done {
  background: rgba(236, 225, 213, 0.72);
}

.task-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 999px;
  background: rgba(125, 94, 79, 0.12);
}

.toggle-dot {
  width: 12px;
  height: 12px;
  border-radius: 999px;
  background: transparent;
}

.task-row.done .toggle-dot {
  background: linear-gradient(135deg, #9f5f3f, #73412b);
}

.task-title {
  color: var(--ink-strong);
  font-size: 1rem;
  line-height: 1.35;
}

.task-row.done .task-title {
  text-decoration: line-through;
  color: rgba(50, 39, 34, 0.62);
}

.task-caption {
  margin-top: 4px;
  font-size: 0.82rem;
}

.icon-button {
  min-height: 38px;
  padding: 0 12px;
  border-radius: 12px;
  background: rgba(92, 52, 37, 0.08);
  color: var(--accent-muted);
  font-weight: 700;
}

.empty-card {
  display: grid;
  place-items: center;
  flex: 1;
  border-radius: 22px;
  background: repeating-linear-gradient(
    -35deg,
    rgba(255, 250, 244, 0.62),
    rgba(255, 250, 244, 0.62) 14px,
    rgba(243, 232, 221, 0.8) 14px,
    rgba(243, 232, 221, 0.8) 28px
  );
  color: var(--ink-soft);
  text-align: center;
  padding: 20px;
}

.empty-card p {
  font-family: "Iowan Old Style", "Palatino Linotype", Georgia, serif;
  color: var(--ink-strong);
  font-size: 1.4rem;
}

.timer-panel {
  justify-content: space-between;
}

.status-pill {
  padding: 8px 12px;
  border-radius: 999px;
  background: rgba(92, 52, 37, 0.08);
  color: var(--accent-muted);
  font-size: 0.82rem;
  font-weight: 700;
}

.status-pill.running {
  background: rgba(159, 95, 63, 0.12);
}

.timer-core {
  flex: 1;
  display: grid;
  align-content: center;
  justify-items: center;
  text-align: center;
}

.timer-task {
  max-width: 16ch;
  margin: 8px 0 18px;
  font-size: 1.8rem;
  line-height: 1.1;
}

.timer-display {
  font-family: "SFMono-Regular", "Cascadia Code", "Roboto Mono", monospace;
  font-size: clamp(4rem, 9vw, 6rem);
  font-weight: 700;
  line-height: 1;
  letter-spacing: -0.08em;
  color: var(--ink-strong);
}

.timer-actions {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.summary-stack {
  display: grid;
  gap: 12px;
}

.summary-card {
  display: grid;
  gap: 6px;
  padding: 18px;
  border-radius: 20px;
  background: rgba(255, 252, 248, 0.9);
  border: 1px solid rgba(35, 28, 24, 0.08);
}

.summary-card.tint {
  background: linear-gradient(135deg, rgba(255, 239, 226, 0.9), rgba(237, 224, 213, 0.92));
}

.summary-label {
  color: var(--accent-muted);
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.18em;
  text-transform: uppercase;
}

.summary-card strong {
  font-size: 2rem;
  line-height: 1.05;
}

.completed-list {
  display: grid;
  gap: 10px;
  margin-top: 20px;
  font-size: 0.94rem;
}

.completed-list li {
  padding: 12px 14px;
  border-radius: 16px;
  background: rgba(92, 52, 37, 0.06);
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

@media (max-width: 1080px) {
  .hero-card,
  .workspace-grid {
    grid-template-columns: 1fr;
  }

  .panel {
    min-height: auto;
  }
}

@media (max-width: 640px) {
  .pomodoro-panel {
    padding: 16px;
  }

  .hero-card,
  .panel {
    padding: 20px;
    border-radius: 24px;
  }

  .task-form,
  .timer-actions {
    grid-template-columns: 1fr;
  }
}
</style>
