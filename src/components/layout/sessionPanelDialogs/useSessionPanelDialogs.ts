import { useI18n } from 'vue-i18n'
import type { Session } from '@/stores/session'

export interface SessionPanelDialogsProps {
  sessionCreateModalVisible: boolean
  newSessionName: string
  isNewSessionFormValid: boolean
  showDeleteConfirm: boolean
  deletingSession: Session | null
  deletingSessionCount: number
  isDeletingSessions: boolean
  showClearMessagesConfirm: boolean
  isClearingMessages: boolean
  showErrorModal: boolean
  errorSession: Session | null
  showSummaryModal: boolean
  summarySession: Session | null
}

export type SessionPanelDialogsEmits = {
  (e: 'closeCreate'): void
  (e: 'submitCreate'): void
  (e: 'updateNewSessionName', value: string): void
  (e: 'closeDelete'): void
  (e: 'confirmDelete'): void
  (e: 'closeClearMessages'): void
  (e: 'confirmClearMessages'): void
  (e: 'closeError'): void
  (e: 'retryError'): void
  (e: 'closeSummary'): void
  (e: 'rerunSummary'): void
}

/**
 * 统一暴露会话弹窗文案，保持入口组件仅负责模板装配。
 */
export function useSessionPanelDialogs() {
  const { t } = useI18n()

  return { t }
}
