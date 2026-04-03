import { computed, reactive } from "vue";

const STORAGE_KEY = "solo-tauri2-demo:pomodoro";
const DEFAULT_DURATION_SECONDS = 25 * 60;

export interface Task {
  id: string;
  title: string;
  completed: boolean;
  createdAt: number;
  completedAt?: number;
}

interface PomodoroStorageShape {
  dayKey: string;
  tasks: Task[];
  completedPomodoros: number;
  durationSeconds: number;
  remainingSeconds: number;
  isRunning: boolean;
  lastStartedAt: number | null;
  lastCompletedAt: number | null;
}

export interface PomodoroState extends PomodoroStorageShape {}

const state = reactive<PomodoroState>(createDefaultState());
let hasHydrated = false;

const activeTasks = computed(() => state.tasks.filter((task) => !task.completed));
const completedTasks = computed(() => state.tasks.filter((task) => task.completed));
const totalTasks = computed(() => state.tasks.length);
const completionRate = computed(() => {
  if (!state.tasks.length) {
    return 0;
  }

  return Math.round((completedTasks.value.length / state.tasks.length) * 100);
});

function createDefaultState(dayKey = createDayKey()): PomodoroState {
  return {
    dayKey,
    tasks: [],
    completedPomodoros: 0,
    durationSeconds: DEFAULT_DURATION_SECONDS,
    remainingSeconds: DEFAULT_DURATION_SECONDS,
    isRunning: false,
    lastStartedAt: null,
    lastCompletedAt: null,
  };
}

function createDayKey(timestamp = Date.now()): string {
  const date = new Date(timestamp);
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");

  return `${year}-${month}-${day}`;
}

function safeStorage(): Storage | null {
  if (typeof window === "undefined") {
    return null;
  }

  return window.localStorage;
}

function persistState() {
  const storage = safeStorage();
  if (!storage) {
    return;
  }

  const snapshot: PomodoroStorageShape = {
    dayKey: state.dayKey,
    tasks: state.tasks,
    completedPomodoros: state.completedPomodoros,
    durationSeconds: state.durationSeconds,
    remainingSeconds: state.remainingSeconds,
    isRunning: state.isRunning,
    lastStartedAt: state.lastStartedAt,
    lastCompletedAt: state.lastCompletedAt,
  };

  storage.setItem(STORAGE_KEY, JSON.stringify(snapshot));
}

function assignState(nextState: PomodoroState) {
  state.dayKey = nextState.dayKey;
  state.tasks = nextState.tasks;
  state.completedPomodoros = nextState.completedPomodoros;
  state.durationSeconds = nextState.durationSeconds;
  state.remainingSeconds = nextState.remainingSeconds;
  state.isRunning = nextState.isRunning;
  state.lastStartedAt = nextState.lastStartedAt;
  state.lastCompletedAt = nextState.lastCompletedAt;
}

function normalizeStoredState(value: unknown): PomodoroState {
  if (!value || typeof value !== "object") {
    return createDefaultState();
  }

  const record = value as Partial<PomodoroStorageShape>;
  const todayKey = createDayKey();

  if (record.dayKey !== todayKey) {
    return createDefaultState(todayKey);
  }

  const tasks = Array.isArray(record.tasks)
    ? record.tasks
        .filter((task): task is Task => Boolean(task && typeof task === "object"))
        .map((task) => ({
          id: typeof task.id === "string" ? task.id : crypto.randomUUID(),
          title: typeof task.title === "string" ? task.title : "",
          completed: Boolean(task.completed),
          createdAt: typeof task.createdAt === "number" ? task.createdAt : Date.now(),
          completedAt: typeof task.completedAt === "number" ? task.completedAt : undefined,
        }))
        .filter((task) => task.title.trim().length > 0)
    : [];

  const durationSeconds =
    typeof record.durationSeconds === "number" && record.durationSeconds > 0
      ? Math.floor(record.durationSeconds)
      : DEFAULT_DURATION_SECONDS;
  const remainingSeconds =
    typeof record.remainingSeconds === "number" && record.remainingSeconds >= 0
      ? Math.min(Math.floor(record.remainingSeconds), durationSeconds)
      : durationSeconds;

  return {
    dayKey: todayKey,
    tasks,
    completedPomodoros:
      typeof record.completedPomodoros === "number" && record.completedPomodoros >= 0
        ? Math.floor(record.completedPomodoros)
        : 0,
    durationSeconds,
    remainingSeconds,
    isRunning: Boolean(record.isRunning),
    lastStartedAt: typeof record.lastStartedAt === "number" ? record.lastStartedAt : null,
    lastCompletedAt: typeof record.lastCompletedAt === "number" ? record.lastCompletedAt : null,
  };
}

function completePomodoro(completedAt = Date.now()) {
  state.completedPomodoros += 1;
  state.remainingSeconds = state.durationSeconds;
  state.isRunning = false;
  state.lastStartedAt = null;
  state.lastCompletedAt = completedAt;
}

function syncElapsedTime(now = Date.now()) {
  if (!state.isRunning || !state.lastStartedAt) {
    return;
  }

  const elapsedSeconds = Math.max(0, Math.floor((now - state.lastStartedAt) / 1000));
  if (elapsedSeconds === 0) {
    return;
  }

  if (elapsedSeconds >= state.remainingSeconds) {
    completePomodoro(now);
    persistState();
    return;
  }

  state.remainingSeconds -= elapsedSeconds;
  state.lastStartedAt = now;
  persistState();
}

/**
 * 初始化并恢复今日面板状态。
 * 跨天时会清空任务和统计，运行中的计时器会根据离开时间补算剩余秒数。
 */
function hydrateStore() {
  if (hasHydrated) {
    return;
  }

  hasHydrated = true;

  const storage = safeStorage();
  if (!storage) {
    return;
  }

  const rawValue = storage.getItem(STORAGE_KEY);
  if (!rawValue) {
    persistState();
    return;
  }

  try {
    const stored = normalizeStoredState(JSON.parse(rawValue));
    assignState(stored);
    syncElapsedTime();
  } catch (error) {
    console.warn("Failed to restore pomodoro state.", error);
    assignState(createDefaultState());
    persistState();
  }
}

/**
 * 新增一条今日任务，并立即写回本地缓存。
 */
function addTask(title: string) {
  const trimmedTitle = title.trim();
  if (!trimmedTitle) {
    return;
  }

  state.tasks.unshift({
    id: crypto.randomUUID(),
    title: trimmedTitle,
    completed: false,
    createdAt: Date.now(),
  });
  persistState();
}

/**
 * 切换任务完成态，支持将已完成任务恢复为未完成。
 */
function toggleTask(id: string) {
  const task = state.tasks.find((item) => item.id === id);
  if (!task) {
    return;
  }

  task.completed = !task.completed;
  task.completedAt = task.completed ? Date.now() : undefined;
  persistState();
}

/**
 * 删除任务，保证任务列表与统计视图即时同步。
 */
function removeTask(id: string) {
  const nextTasks = state.tasks.filter((task) => task.id !== id);
  if (nextTasks.length === state.tasks.length) {
    return;
  }

  state.tasks = nextTasks;
  persistState();
}

/**
 * 开始或恢复当前 25 分钟专注周期。
 */
function startTimer() {
  if (state.isRunning) {
    return;
  }

  state.isRunning = true;
  state.lastStartedAt = Date.now();
  persistState();
}

/**
 * 暂停计时前先补算已流逝时间，避免刷新或切后台后丢秒。
 */
function pauseTimer() {
  if (!state.isRunning) {
    return;
  }

  syncElapsedTime();
  state.isRunning = false;
  state.lastStartedAt = null;
  persistState();
}

/**
 * 重置到一个新的番茄钟周期，并停止当前计时。
 */
function resetTimer() {
  state.isRunning = false;
  state.remainingSeconds = state.durationSeconds;
  state.lastStartedAt = null;
  persistState();
}

/**
 * 定时驱动入口，允许页面按秒轮询，同时自动处理完结状态。
 */
function tickTimer() {
  syncElapsedTime();
}

export const usePomodoroStore = () => ({
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
});
