import { onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useProjectStore } from '@/stores/project'
import { useSessionStore } from '@/stores/session'
import { usePlanStore } from '@/stores/plan'
import { useTaskStore } from '@/stores/task'
import { useTaskExecutionStore } from '@/stores/taskExecution'

export function useWindowEvents() {
  const projectStore = useProjectStore()
  const sessionStore = useSessionStore()
  const planStore = usePlanStore()
  const taskStore = useTaskStore()
  const taskExecutionStore = useTaskExecutionStore()

  const unlisteners: UnlistenFn[] = []

  onMounted(async () => {
    // 监听项目更新事件
    const unlistenProject = await listen('project:updated', async () => {
      await projectStore.loadProjects()
    })
    unlisteners.push(unlistenProject)

    // 监听会话锁定事件
    const unlistenSessionLock = await listen<string>('session:locked', (event) => {
      const sessionId = event.payload
      // 如果当前窗口打开了该会话，需要关闭
      if (sessionStore.openSessionIds.includes(sessionId)) {
        sessionStore.closeSession(sessionId)
      }
    })
    unlisteners.push(unlistenSessionLock)

    // 监听项目访问更新事件
    const unlistenAccess = await listen('project-access:updated', async () => {
      await projectStore.getRecentProjectIds()
    })
    unlisteners.push(unlistenAccess)

    // 监听定时计划触发事件
    // 后端会先把计划状态切到 executing / running，并把待执行任务置为 in_progress。
    // 前端这里负责补上真正的执行队列恢复，否则计划只会进入“执行中”状态而不会启动任务执行器。
    const unlistenScheduledTrigger = await listen<string>('plan:scheduled-trigger', async (event) => {
      const planId = event.payload
      console.log('Scheduled plan triggered:', planId)

      try {
        const plan = await planStore.getPlan(planId)
        if (!plan) {
          console.error('Plan not found:', planId)
          return
        }

        await planStore.loadPlans(plan.projectId)
        planStore.setCurrentPlan(planId)
        await taskStore.loadTasks(planId)
        await taskExecutionStore.getPlanExecutionProgress(planId)
        await taskExecutionStore.resumeInProgressExecutionFlow(planId)
      } catch (error) {
        console.error('Failed to handle scheduled plan trigger:', error)
      }
    })
    unlisteners.push(unlistenScheduledTrigger)
  })

  onUnmounted(() => {
    unlisteners.forEach(unlisten => unlisten())
  })
}
