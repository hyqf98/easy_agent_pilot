import { computed, reactive } from 'vue'
import { defineStore } from 'pinia'
import type { FileEditTrace } from '@/types/fileTrace'

interface SessionTraceUiState {
  selectedMessageId: string | null
  selectedTraceId: string | null
  paneWidth: number
  isPaneVisible: boolean
  isPaneCollapsed: boolean
  isMobileDrawerOpen: boolean
  isAutoFollow: boolean
  manuallyHidden: boolean
  unseenCount: number
}

const DEFAULT_WIDTH = 640
const MIN_WIDTH = 460
const MAX_WIDTH = 1080

function createDefaultState(): SessionTraceUiState {
  return {
    selectedMessageId: null,
    selectedTraceId: null,
    paneWidth: DEFAULT_WIDTH,
    isPaneVisible: false,
    isPaneCollapsed: false,
    isMobileDrawerOpen: false,
    isAutoFollow: true,
    manuallyHidden: false,
    unseenCount: 0
  }
}

export const useAiEditTraceStore = defineStore('aiEditTrace', () => {
  const sessionStates = reactive<Record<string, SessionTraceUiState>>({})

  const ensureState = (sessionId: string): SessionTraceUiState => {
    if (!sessionStates[sessionId]) {
      sessionStates[sessionId] = createDefaultState()
    }

    return sessionStates[sessionId]
  }

  const getSessionState = computed(() => {
    return (sessionId: string) => ensureState(sessionId)
  })

  function selectTrace(
    sessionId: string,
    payload: {
      messageId: string
      traceId: string
      openPane?: boolean
      openMobileDrawer?: boolean
      userInitiated?: boolean
    }
  ) {
    const state = ensureState(sessionId)
    state.selectedMessageId = payload.messageId
    state.selectedTraceId = payload.traceId
    state.unseenCount = 0

    if (payload.openPane) {
      state.isPaneVisible = true
      state.isPaneCollapsed = false
      state.manuallyHidden = false
    }

    if (payload.openMobileDrawer) {
      state.isMobileDrawerOpen = true
      state.manuallyHidden = false
    }

    if (payload.userInitiated) {
      state.manuallyHidden = false
    }
  }

  function handleIncomingTrace(
    sessionId: string,
    payload: {
      messageId: string
      traceId: string
      shouldAutoOpen: boolean
      isDesktop: boolean
    }
  ) {
    const state = ensureState(sessionId)

    if (state.isAutoFollow || !state.selectedTraceId) {
      state.selectedMessageId = payload.messageId
      state.selectedTraceId = payload.traceId
    }

    if (state.manuallyHidden || !payload.shouldAutoOpen) {
      state.unseenCount += 1
      return
    }

    if (payload.isDesktop) {
      state.isPaneVisible = true
      state.isPaneCollapsed = false
    } else {
      state.unseenCount += 1
    }
  }

  function showPane(sessionId: string) {
    const state = ensureState(sessionId)
    state.isPaneVisible = true
    state.isPaneCollapsed = false
    state.manuallyHidden = false
    state.unseenCount = 0
  }

  function hidePane(sessionId: string) {
    const state = ensureState(sessionId)
    state.isPaneVisible = false
    state.isPaneCollapsed = true
    state.manuallyHidden = true
  }

  function openMobileDrawer(sessionId: string) {
    const state = ensureState(sessionId)
    state.isMobileDrawerOpen = true
    state.unseenCount = 0
    state.manuallyHidden = false
  }

  function closeMobileDrawer(sessionId: string) {
    const state = ensureState(sessionId)
    state.isMobileDrawerOpen = false
  }

  function toggleAutoFollow(sessionId: string) {
    const state = ensureState(sessionId)
    state.isAutoFollow = !state.isAutoFollow
  }

  function setPaneWidth(sessionId: string, nextWidth: number) {
    const state = ensureState(sessionId)
    state.paneWidth = Math.max(MIN_WIDTH, Math.min(nextWidth, MAX_WIDTH))
  }

  function resetSession(sessionId: string) {
    delete sessionStates[sessionId]
  }

  function resetSessions(sessionIds: string[]) {
    sessionIds.forEach((sessionId) => {
      delete sessionStates[sessionId]
    })
  }

  function findSelectedTrace(sessionId: string, traces: Array<FileEditTrace & { messageId: string }>) {
    const state = ensureState(sessionId)

    return traces.find(trace =>
      trace.id === state.selectedTraceId && trace.messageId === state.selectedMessageId
    ) ?? traces[traces.length - 1] ?? null
  }

  return {
    getSessionState,
    selectTrace,
    handleIncomingTrace,
    showPane,
    hidePane,
    openMobileDrawer,
    closeMobileDrawer,
    toggleAutoFollow,
    setPaneWidth,
    resetSession,
    resetSessions,
    findSelectedTrace
  }
})
