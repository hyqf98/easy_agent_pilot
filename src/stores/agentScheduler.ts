import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { AgentRole, AgentRoleConfig } from '@/types/plan'
import { AGENT_ROLES as ROLES, getAgentRoleConfig } from '@/types/plan'
import { useTaskStore } from './task'

export const useAgentSchedulerStore = defineStore('agentScheduler', () => {
  const activeRole = ref<AgentRole | null>(null)
  const roleHistory = ref<Array<{ role: AgentRole; timestamp: string; taskId?: string }>>([])

  const currentRoleConfig = computed<AgentRoleConfig | null>(() => {
    if (!activeRole.value) return null
    return getAgentRoleConfig(activeRole.value) ?? null
  })

  const availableRoles = computed<AgentRoleConfig[]>(() => {
    return ROLES
  })

  // Actions

  function setActiveRole(role: AgentRole, taskId?: string) {
    activeRole.value = role
    roleHistory.value.push({
      role,
      timestamp: new Date().toISOString(),
      taskId
    })

    // 保留最近 50 条历史
    if (roleHistory.value.length > 50) {
      roleHistory.value = roleHistory.value.slice(-50)
    }
  }

  // 清除活动角色
  function clearActiveRole() {
    activeRole.value = null
  }

  // 获取下一个推荐任务
  function getNextRecommendedTask(planId: string): string | null {
    const taskStore = useTaskStore()
    const readyTasks = taskStore.getReadyTasks(planId)

    if (readyTasks.length === 0) return null

    // 返回优先级最高且顺序最靠前的任务
    const priorityOrder: Record<string, number> = {
      high: 0,
      medium: 1,
      low: 2
    }

    readyTasks.sort((a, b) => {
      const priorityDiff = priorityOrder[a.priority] - priorityOrder[b.priority]
      if (priorityDiff !== 0) return priorityDiff
      return a.order - b.order
    })

    return readyTasks[0].id
  }

  return {
    activeRole,
    roleHistory,
    currentRoleConfig,
    availableRoles,
    setActiveRole,
    clearActiveRole,
    getNextRecommendedTask
  }
})
