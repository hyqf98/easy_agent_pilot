import { defineStore } from 'pinia'
import { ref, watch, computed } from 'vue'

// 左侧面板类型 - 统一面板
export type LeftPanelType = 'unified' | null

// 项目 Tab 类型
export type ProjectTabType = 'sessions' | 'files'

// 智能体筛选（使用智能体 ID，'all' 表示全部）
export type AgentFilter = string | 'all'

// 会话排序方式
export type SessionSortBy = 'updatedAt' | 'createdAt'

// 面板状态
export interface PanelState {
  width: number
}

const LEFT_PANEL_STATE_KEY = 'ea-left-panel-state'

// 默认配置
const DEFAULT_PANEL_WIDTH = 260

// 面板宽度限制
export const PANEL_LIMITS = {
  navRail: {
    width: 48 // 导航栏宽度
  },
  panel: {
    minWidth: 200,
    maxWidth: 400,
    defaultWidth: 260
  }
}

// 布局常量
export const RESIZER_WIDTH = 4 // 拖拽器宽度
export const MESSAGE_AREA_MIN_WIDTH = 400 // 消息区域最小宽度
export const HEADER_HEIGHT = 56
// 项目 Tab 状态存储 key
const PROJECT_TAB_STATES_KEY = 'ea-project-tab-states'
// 智能体类型筛选存储 key
const AGENT_TYPE_FILTER_KEY = 'ea-agent-type-filter'
// 会话排序存储 key
const SESSION_SORT_BY_KEY = 'ea-session-sort-by'

function loadFromStorage<T>(key: string, defaultValue: T): T {
  try {
    const saved = localStorage.getItem(key)
    if (saved) return JSON.parse(saved)
  } catch {
    // ignore
  }
  return defaultValue
}

function saveToStorage(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // ignore
  }
}

// 从 localStorage 读取状态
function loadPanelState(): { activePanel: LeftPanelType; panelWidth: number } {
  return loadFromStorage(LEFT_PANEL_STATE_KEY, { activePanel: 'unified' as LeftPanelType, panelWidth: DEFAULT_PANEL_WIDTH })
}

// 从 localStorage 读取项目 Tab 状态
function loadProjectTabStates(): Map<string, ProjectTabType> {
  const parsed = loadFromStorage<Record<string, ProjectTabType> | null>(PROJECT_TAB_STATES_KEY, null)
  return parsed ? new Map(Object.entries(parsed)) : new Map()
}

// 保存项目 Tab 状态到 localStorage
function saveProjectTabStates(states: Map<string, ProjectTabType>) {
  saveToStorage(PROJECT_TAB_STATES_KEY, Object.fromEntries(states))
}

// 从 localStorage 读取智能体筛选
function loadAgentFilter(): AgentFilter {
  try {
    const saved = localStorage.getItem(AGENT_TYPE_FILTER_KEY)
    if (saved) return saved as AgentFilter
  } catch {
    // ignore
  }
  return 'all'
}

// 保存智能体筛选到 localStorage
function saveAgentFilter(filter: AgentFilter) {
  try {
    localStorage.setItem(AGENT_TYPE_FILTER_KEY, filter)
  } catch {
    // ignore
  }
}

// 从 localStorage 读取会话排序
function loadSessionSortBy(): SessionSortBy {
  const saved = loadFromStorage<string | null>(SESSION_SORT_BY_KEY, null)
  return saved === 'createdAt' || saved === 'updatedAt' ? saved : 'updatedAt'
}

// 保存会话排序到 localStorage
function saveSessionSortBy(sortBy: SessionSortBy) {
  saveToStorage(SESSION_SORT_BY_KEY, sortBy)
}

// 保存状态到 localStorage
function savePanelState(state: { activePanel: LeftPanelType; panelWidth: number }) {
  saveToStorage(LEFT_PANEL_STATE_KEY, state)
}

export const useLayoutStore = defineStore('layout', () => {
  // State
  const savedState = loadPanelState()
  const activePanel = ref<LeftPanelType>(savedState.activePanel)
  const panelWidth = ref(savedState.panelWidth)

  // 项目 Tab 状态：每个项目的当前 Tab（会话/文件）
  const projectTabStates = ref<Map<string, ProjectTabType>>(loadProjectTabStates())

  // 智能体筛选状态（使用智能体 ID，'all' 表示全部）
  const agentFilter = ref<AgentFilter>(loadAgentFilter())

  // 会话排序状态
  const sessionSortBy = ref<SessionSortBy>(loadSessionSortBy())

  // Computed
  const isPanelOpen = computed(() => activePanel.value !== null)

  // 获取项目的当前 Tab
  function getProjectTab(projectId: string): ProjectTabType {
    return projectTabStates.value.get(projectId) || 'sessions'
  }

  // 设置项目的当前 Tab
  function setProjectTab(projectId: string, tab: ProjectTabType) {
    projectTabStates.value.set(projectId, tab)
    saveProjectTabStates(projectTabStates.value)
  }

  // 设置智能体筛选
  function setAgentFilter(filter: AgentFilter) {
    agentFilter.value = filter
    saveAgentFilter(filter)
  }

  // 设置会话排序
  function setSessionSortBy(sortBy: SessionSortBy) {
    sessionSortBy.value = sortBy
    saveSessionSortBy(sortBy)
  }

  // 当前面板宽度（考虑是否展开）
  const currentPanelWidth = computed(() => {
    if (!isPanelOpen.value) {
      return PANEL_LIMITS.navRail.width
    }
    return PANEL_LIMITS.navRail.width + panelWidth.value + RESIZER_WIDTH
  })

  // Actions
  /**
   * 切换面板
   * - 如果点击当前已激活的面板，则关闭面板
   * - 如果点击其他面板，则切换到该面板
   */
  function togglePanel(panel: LeftPanelType) {
    if (activePanel.value === panel) {
      // 点击当前激活的面板，关闭它
      activePanel.value = null
    } else {
      // 切换到新面板
      activePanel.value = panel
    }
  }

  /**
   * 设置面板宽度
   */
  function setPanelWidth(width: number) {
    const limitedWidth = Math.max(PANEL_LIMITS.panel.minWidth, Math.min(PANEL_LIMITS.panel.maxWidth, width))
    panelWidth.value = limitedWidth
  }

  /**
   * 关闭面板
   */
  function closePanel() {
    activePanel.value = null
  }

  /**
   * 响应式调整面板宽度
   */
  function handleResize() {
    const windowWidth = window.innerWidth
    const availableForPanel = windowWidth - MESSAGE_AREA_MIN_WIDTH - PANEL_LIMITS.navRail.width - RESIZER_WIDTH

    // 如果空间不足，缩小面板宽度
    if (isPanelOpen.value && availableForPanel < panelWidth.value) {
      panelWidth.value = Math.max(PANEL_LIMITS.panel.minWidth, availableForPanel)
    }
  }

  // 监听状态变化并持久化（防抖 300ms）
  let saveTimer: ReturnType<typeof setTimeout> | null = null
  watch([activePanel, panelWidth], () => {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => {
      savePanelState({
        activePanel: activePanel.value,
        panelWidth: panelWidth.value
      })
    }, 300)
  })

  return {
    // State
    activePanel,
    panelWidth,
    projectTabStates,
    agentFilter,
    sessionSortBy,
    // Computed
    isPanelOpen,
    currentPanelWidth,
    // Actions
    togglePanel,
    setPanelWidth,
    closePanel,
    handleResize,
    // 项目 Tab 管理
    getProjectTab,
    setProjectTab,
    // 智能体筛选
    setAgentFilter,
    // 会话排序
    setSessionSortBy
  }
})
