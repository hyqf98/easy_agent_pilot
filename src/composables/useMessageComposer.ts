import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAgentConfigStore } from '@/stores/agentConfig'
import { useAgentStore } from '@/stores/agent'
import { useMessageStore } from '@/stores/message'
import { useNotificationStore } from '@/stores/notification'
import { useProjectStore } from '@/stores/project'
import { useSessionExecutionStore } from '@/stores/sessionExecution'
import { useSessionStore } from '@/stores/session'
import { useSettingsStore } from '@/stores/settings'
import { useTokenStore, type CompressionStrategy, type TokenLevel } from '@/stores/token'
import { compressionService } from '@/services/compression'
import { conversationService } from '@/services/conversation'
import { resolveSessionAgent, resolveSessionAgentId } from '@/utils/sessionAgent'

interface TextSegment {
  type: 'text' | 'file'
  content: string
  fullPath?: string
}

export function useMessageComposer() {
  const { t } = useI18n()
  const messageStore = useMessageStore()
  const sessionStore = useSessionStore()
  const settingsStore = useSettingsStore()
  const notificationStore = useNotificationStore()
  const projectStore = useProjectStore()
  const agentStore = useAgentStore()
  const agentConfigStore = useAgentConfigStore()
  const sessionExecutionStore = useSessionExecutionStore()
  const tokenStore = useTokenStore()

  const showCompressionDialog = ref(false)
  const isCompressing = ref(false)
  const isAgentDropdownOpen = ref(false)
  const agentDropdownRef = ref<HTMLElement | null>(null)
  const isModelDropdownOpen = ref(false)
  const modelDropdownRef = ref<HTMLElement | null>(null)
  const selectedModelId = ref<string>('')
  const textareaRef = ref<HTMLTextAreaElement | null>(null)
  const renderLayerRef = ref<HTMLDivElement | null>(null)
  const showFileMention = ref(false)
  const fileMentionPosition = ref({ x: 0, y: 0, width: 0, height: 0 })
  const mentionStart = ref(-1)
  const mentionSearchText = ref('')

  const agentOptions = computed(() =>
    agentStore.agents.map(agent => ({
      label: agent.name,
      value: agent.id,
      modelId: agent.modelId,
      provider: agent.provider,
      type: agent.type,
      isCustom: agent.customModelEnabled || false
    }))
  )

  const currentSessionId = computed(() => sessionStore.currentSessionId)

  const currentAgentId = computed(() => {
    return resolveSessionAgentId(sessionStore.currentSession, agentStore.agents)
  })

  const currentAgent = computed(() => {
    return resolveSessionAgent(sessionStore.currentSession, agentStore.agents)
  })

  const currentProject = computed(() => {
    const sessionId = sessionStore.currentSessionId
    if (!sessionId) return null
    return projectStore.projects.find(project => project.id === sessionStore.currentSession?.projectId) || null
  })

  const currentAgentName = computed(() => {
    if (!currentAgent.value) {
      return t('settings.agentConfig.selectAgent')
    }
    return currentAgent.value.name
  })

  const modelOptions = computed(() => {
    const agentId = currentAgentId.value
    if (!agentId) return []

    return agentConfigStore.getModelsConfigs(agentId)
      .filter(config => config.enabled)
      .map(config => ({
        value: config.modelId,
        label: config.displayName,
        isDefault: config.isDefault
      }))
  })

  const presetModelOptions = computed(() => modelOptions.value)

  const inputText = computed({
    get: () => currentSessionId.value ? sessionExecutionStore.getInputText(currentSessionId.value) : '',
    set: (value) => {
      if (currentSessionId.value) {
        sessionExecutionStore.setInputText(currentSessionId.value, value)
      }
    }
  })

  const isSending = computed(() =>
    currentSessionId.value ? sessionExecutionStore.getIsSending(currentSessionId.value) : false
  )

  const parsedInputText = computed<TextSegment[]>(() => {
    const text = inputText.value
    if (!text) return []

    const segments: TextSegment[] = []
    const filePattern = /@([a-zA-Z0-9_\-\u4e00-\u9fa5./\\]+)(?=\s|$)/g
    let lastIndex = 0
    let match

    while ((match = filePattern.exec(text)) !== null) {
      if (match.index > lastIndex) {
        const content = text.slice(lastIndex, match.index)
        if (content) {
          segments.push({
            type: 'text',
            content
          })
        }
      }

      const fullPath = match[1]
      const fileName = fullPath.split(/[/\\]/).pop() || fullPath

      segments.push({
        type: 'file',
        content: fileName,
        fullPath
      })

      lastIndex = match.index + match[0].length
    }

    if (lastIndex < text.length) {
      const content = text.slice(lastIndex)
      if (content) {
        segments.push({
          type: 'text',
          content
        })
      }
    }

    return segments
  })

  const tokenUsage = computed(() => {
    if (!currentSessionId.value) {
      return { used: 0, limit: 0, percentage: 0, level: 'safe' as TokenLevel }
    }
    return tokenStore.getTokenUsage(currentSessionId.value)
  })

  const messageCount = computed(() => {
    if (!currentSessionId.value) return 0
    return messageStore.messagesBySession(currentSessionId.value).length
  })

  const shouldShowCompressButton = computed(() => {
    return tokenUsage.value.percentage >= 50 && messageCount.value > 0
  })

  const inputPlaceholder = computed(() => {
    const shortcut = settingsStore.settings.sendOnEnter
      ? t('message.shortcutEnter')
      : t('message.shortcutModifierEnter')
    return t('message.inputPlaceholder', { shortcut })
  })

  watch(currentAgentId, async (agentId) => {
    if (agentId) {
      await agentConfigStore.loadModelsConfigs(agentId)
    }
  }, { immediate: true })

  watch(currentAgent, async (agent) => {
    if (agent && currentAgentId.value) {
      await agentConfigStore.loadModelsConfigs(currentAgentId.value)
      const configs = agentConfigStore.getModelsConfigs(currentAgentId.value)
      const defaultModel = configs.find(config => config.isDefault && config.enabled)
      selectedModelId.value = defaultModel?.modelId || ''
    } else {
      selectedModelId.value = ''
    }
  }, { immediate: true })

  watch(() => sessionStore.currentSessionId, async (sessionId) => {
    if (sessionId) {
      await nextTick()
      textareaRef.value?.focus()
    }
  }, { immediate: true })

  const handleClickOutside = (event: MouseEvent) => {
    if (agentDropdownRef.value && !agentDropdownRef.value.contains(event.target as Node)) {
      isAgentDropdownOpen.value = false
    }
    if (modelDropdownRef.value && !modelDropdownRef.value.contains(event.target as Node)) {
      isModelDropdownOpen.value = false
    }
  }

  onMounted(async () => {
    try {
      await agentStore.loadAgents()
      if (currentAgentId.value) {
        await agentConfigStore.loadModelsConfigs(currentAgentId.value)
      }
    } catch (error) {
      console.error('Failed to load agents:', error)
    }
    document.addEventListener('click', handleClickOutside)
  })

  onUnmounted(() => {
    document.removeEventListener('click', handleClickOutside)
  })

  const syncScroll = () => {
    if (textareaRef.value && renderLayerRef.value) {
      renderLayerRef.value.scrollTop = textareaRef.value.scrollTop
    }
  }

  const toggleAgentDropdown = () => {
    isAgentDropdownOpen.value = !isAgentDropdownOpen.value
    if (isAgentDropdownOpen.value) {
      isModelDropdownOpen.value = false
    }
  }

  const selectAgent = async (agentId: string) => {
    const sessionId = sessionStore.currentSessionId
    if (!sessionId) {
      isAgentDropdownOpen.value = false
      return
    }

    try {
      const agent = agentStore.agents.find(item => item.id === agentId)
      await sessionStore.updateSession(sessionId, { agentType: agentId })
      selectedModelId.value = agent?.modelId || ''
      isAgentDropdownOpen.value = false
    } catch (error) {
      console.error('Failed to update session agent:', error)
    }
  }

  const toggleModelDropdown = () => {
    isModelDropdownOpen.value = !isModelDropdownOpen.value
    if (isModelDropdownOpen.value) {
      isAgentDropdownOpen.value = false
    }
  }

  const selectModel = async (modelId: string) => {
    if (!currentAgent.value || !currentAgentId.value) return

    selectedModelId.value = modelId
    isModelDropdownOpen.value = false

    try {
      const configs = agentConfigStore.getModelsConfigs(currentAgentId.value)
      const selectedConfig = configs.find(config => config.modelId === modelId)
      if (selectedConfig) {
        await agentConfigStore.updateModelConfig(selectedConfig.id, currentAgentId.value, {
          isDefault: true
        })
      }
    } catch (error) {
      console.error('Failed to update agent model:', error)
    }
  }

  const getModelLabel = (modelId: string) => {
    const model = modelOptions.value.find(item => item.value === modelId)
    return model ? model.label : modelId || '使用默认模型'
  }

  const handleOpenCompress = () => {
    showCompressionDialog.value = true
  }

  const handleConfirmCompress = async (strategy: CompressionStrategy) => {
    if (!currentSessionId.value) return

    const session = sessionStore.currentSession
    const agentId = resolveSessionAgentId(session, agentStore.agents) || currentAgent.value?.id

    if (!agentId) {
      notificationStore.smartError('压缩会话', new Error('无法获取智能体信息'))
      showCompressionDialog.value = false
      return
    }

    showCompressionDialog.value = false
    isCompressing.value = true

    try {
      const result = await compressionService.compressSession(
        currentSessionId.value,
        agentId,
        { strategy }
      )

      if (result.success) {
        notificationStore.success(t('compression.success'))
      } else {
        notificationStore.error(t('compression.failed'), result.error)
      }
    } catch (error) {
      notificationStore.smartError('压缩失败', error instanceof Error ? error : new Error(String(error)))
    } finally {
      isCompressing.value = false
      showCompressionDialog.value = false
    }
  }

  const handleCancelCompress = () => {
    showCompressionDialog.value = false
  }

  const closeFileMention = () => {
    showFileMention.value = false
    fileMentionPosition.value = { x: 0, y: 0, width: 0, height: 0 }
    mentionStart.value = -1
    mentionSearchText.value = ''
  }

  const openFileMention = (x: number, y: number, query: string, start: number) => {
    if (!sessionStore.currentSessionId || !currentProject.value) {
      return
    }

    showFileMention.value = true
    fileMentionPosition.value = { x, y, width: 280, height: 0 }
    mentionStart.value = start
    mentionSearchText.value = query
  }

  const handleFileSelect = (_path: string, relativePath: string, mentionStartPos: number) => {
    closeFileMention()

    const textarea = textareaRef.value
    const cursorPos = textarea ? textarea.selectionStart : inputText.value.length
    const beforeAt = inputText.value.slice(0, mentionStartPos)
    const afterSearch = inputText.value.slice(cursorPos)
    const insertText = `@${relativePath} `

    inputText.value = beforeAt + insertText + afterSearch

    nextTick(() => {
      if (textarea) {
        textarea.focus()
        const newPosition = beforeAt.length + insertText.length
        textarea.setSelectionRange(newPosition, newPosition)
      }
    })
  }

  const getCaretCoordinates = (textarea: HTMLTextAreaElement, position: number) => {
    const text = textarea.value.substring(0, position)
    const lines = text.split('\n')
    const currentLine = lines.length - 1
    const currentCol = lines[lines.length - 1].length

    const style = window.getComputedStyle(textarea)
    const lineHeight = parseFloat(style.lineHeight)
    const paddingTop = parseFloat(style.paddingTop)
    const paddingLeft = parseFloat(style.paddingLeft)
    const fontSize = parseFloat(style.fontSize)
    const fontFamily = style.fontFamily

    const lineHeightActual = lineHeight || fontSize * 1.5
    const y = paddingTop + currentLine * lineHeightActual

    const canvas = document.createElement('canvas')
    const context = canvas.getContext('2d')
    if (context) {
      context.font = `${fontSize}px ${fontFamily}`
      const textWidth = context.measureText(lines[lines.length - 1]).width
      canvas.remove()
      return { x: paddingLeft + textWidth, y }
    }
    canvas.remove()
    return { x: paddingLeft + currentCol * (fontSize * 0.6), y }
  }

  const handleInput = (event: Event) => {
    const target = event.target as HTMLTextAreaElement
    const value = target.value
    const cursorPosition = target.selectionStart || 0

    if (showFileMention.value && mentionStart.value >= 0) {
      if (value[mentionStart.value] !== '@') {
        closeFileMention()
      } else if (cursorPosition < mentionStart.value || cursorPosition > mentionStart.value + 100) {
        closeFileMention()
      } else {
        mentionSearchText.value = value.slice(mentionStart.value + 1, cursorPosition)
      }
      inputText.value = value
      return
    }

    if (value.length > 0 && cursorPosition > 0 && value[cursorPosition - 1] === '@') {
      const charBefore = cursorPosition > 1 ? value[cursorPosition - 2] : ' '
      if (charBefore === ' ' || charBefore === '\n' || charBefore === '\r' || cursorPosition === 1) {
        const rect = target.getBoundingClientRect()
        const caretPos = getCaretCoordinates(target, cursorPosition - 1)
        const x = rect.left + caretPos.x
        const y = rect.top + caretPos.y + 20
        openFileMention(x, y, '', cursorPosition - 1)
      }
    }

    inputText.value = value
  }

  const sendWithCurrentAgent = async (userInput: string): Promise<boolean> => {
    const sessionId = sessionStore.currentSessionId
    if (!userInput.trim() || !sessionId || isSending.value) return false

    if (!currentAgent.value) {
      notificationStore.smartError('发送消息', new Error('请先选择一个智能体'))
      return false
    }

    const availability = conversationService.isAgentAvailable(currentAgent.value)
    if (!availability.available) {
      notificationStore.smartError('发送消息', new Error(availability.reason || '智能体不可用'))
      return false
    }

    try {
      const projectId = sessionStore.currentSession?.projectId
      await conversationService.sendMessage(sessionId, userInput, currentAgent.value.id, projectId)
      return true
    } catch (error) {
      console.error('Failed to send message:', error)
      notificationStore.smartError('发送消息', error instanceof Error ? error : new Error(String(error)))
      sessionExecutionStore.endSending(sessionId)
      return false
    }
  }

  const runDemoStream = async (sessionId: string) => {
    sessionExecutionStore.startSending(sessionId)

    try {
      await messageStore.addMessage({
        sessionId,
        role: 'user',
        content: '/demo - 模拟流式输出演示',
        status: 'completed'
      })

      const aiMessage = await messageStore.addMessage({
        sessionId,
        role: 'assistant',
        content: '',
        status: 'streaming'
      })

      sessionExecutionStore.setCurrentStreamingMessageId(sessionId, aiMessage.id)

      const streamResponse = generateStreamResponse('演示')
      let currentIndex = 0
      const chunkSize = 3
      const streamInterval = 30

      const timerId = setInterval(async () => {
        if (currentIndex < streamResponse.length) {
          const nextIndex = Math.min(currentIndex + chunkSize, streamResponse.length)
          const newContent = streamResponse.slice(0, nextIndex)

          await messageStore.updateMessage(aiMessage.id, {
            content: newContent
          })

          currentIndex = nextIndex
        } else {
          clearInterval(timerId)
          sessionExecutionStore.setStreamTimerId(sessionId, null)
          sessionExecutionStore.setCurrentStreamingMessageId(sessionId, null)
          await messageStore.updateMessage(aiMessage.id, {
            status: 'completed'
          })

          sessionStore.updateLastMessage(sessionId, streamResponse.slice(0, 50))
          sessionExecutionStore.endSending(sessionId)
        }
      }, streamInterval)

      sessionExecutionStore.setStreamTimerId(sessionId, timerId)
    } catch {
      sessionExecutionStore.endSending(sessionId)
    }
  }

  const simulateError = (errorType: string) => {
    switch (errorType.toLowerCase()) {
      case 'cli':
      case 'cli_path':
        notificationStore.cliPathError('/usr/local/bin/claude', new Error('CLI 路径不存在: /usr/local/bin/claude'))
        break
      case 'auth':
      case 'api_key':
        notificationStore.apiAuthError(new Error('401 Unauthorized: Invalid API key'))
        break
      case 'timeout':
        notificationStore.timeoutError('连接 API 服务')
        break
      case 'mcp':
        notificationStore.mcpConnectionError('filesystem-mcp', new Error('MCP 服务器初始化失败'))
        break
      case 'network':
        notificationStore.networkError('获取数据', '无法连接到服务器: Connection refused')
        break
      case 'database':
        notificationStore.databaseError('保存消息', new Error('SQLITE_BUSY: database is locked'))
        break
      default:
        notificationStore.smartError('操作', new Error(`模拟错误: ${errorType}`))
    }
  }

  const showHelpMessage = async () => {
    if (!sessionStore.currentSessionId) return

    const helpContent = `## 可用命令

### 演示命令
\`\`\`
/demo - 运行模拟流式输出演示
\`\`\`

### 错误模拟命令（用于测试）

您可以使用以下命令测试不同的错误场景：

\`\`\`
/error cli     - 模拟 CLI 路径无效错误
/error auth    - 模拟 API 认证失败错误
/error timeout - 模拟网络超时错误
/error mcp     - 模拟 MCP 连接失败错误
/error network - 模拟网络连接错误
/error database - 模拟数据库错误
\`\`\`

### 错误类型说明

| 错误类型 | 触发条件 | 用户友好提示 |
|---------|---------|------------|
| CLI 路径无效 | CLI 工具不存在 | 请检查路径是否正确 |
| API 认证失败 | API 密钥无效或过期 | 请检查您的配置 |
| 网络超时 | 请求超过 10 秒无响应 | 请检查网络连接 |
| MCP 连接失败 | MCP 服务器无法启动 | 请检查服务状态 |`

    await messageStore.addMessage({
      sessionId: sessionStore.currentSessionId,
      role: 'assistant',
      content: helpContent,
      status: 'completed'
    })

    sessionStore.updateLastMessage(sessionStore.currentSessionId, '可用命令帮助')
  }

  const generateStreamResponse = (userMessage: string): string => {
    const responses = [
      `感谢您的提问！关于"${userMessage.slice(0, 20)}"，我来为您详细解答。\n\n这是一个流式输出的演示。在实际应用中，AI 的响应会逐字符或逐块地显示，让您能够实时看到内容的生成过程。\n\n## 主要特点\n\n1. **实时显示** - 内容逐步呈现，无需等待完整响应\n2. **流畅体验** - 用户可以开始阅读而不必等待\n3. **状态反馈** - 通过光标动画指示正在生成\n\n如果您有任何其他问题，请随时提问！`,
      `您好！这是一个流式输出的测试响应。\n\n### 流式输出演示\n\n流式输出允许您在 AI 生成响应的同时就开始阅读，而不是等待整个响应完成。\n\n\`\`\`javascript\n// 示例代码\nconst message = "Hello, World!";\nconsole.log(message);\n\`\`\`\n\n这种技术特别适用于长响应，提升了用户体验。`,
      `我理解您的问题是关于"${userMessage.slice(0, 30)}"。\n\n让我为您提供一个详细的回答：\n\n流式输出是一种优化技术，它将完整的响应拆分成小块逐步发送。这样做有几个好处：\n\n- **降低感知延迟** - 用户可以更快地看到内容\n- **更好的交互性** - 用户可以提前决定是否继续阅读\n- **资源效率** - 可以在生成过程中取消请求\n\n希望这个解释对您有帮助！`
    ]

    const index = userMessage.length % responses.length
    return responses[index]
  }

  const handleSend = async () => {
    const sessionId = sessionStore.currentSessionId
    if (!inputText.value.trim() || !sessionId || isSending.value) return

    const rawInput = inputText.value
    const userInput = rawInput.trim()

    if (userInput.startsWith('/error ')) {
      inputText.value = ''
      simulateError(userInput.slice(7).trim())
      return
    }

    if (userInput === '/help') {
      inputText.value = ''
      await showHelpMessage()
      return
    }

    if (userInput === '/demo') {
      inputText.value = ''
      await runDemoStream(sessionId)
      return
    }

    const success = await sendWithCurrentAgent(userInput)
    if (success) {
      inputText.value = ''
    } else {
      inputText.value = rawInput
    }
  }

  const handleMessageFormSubmit = async (formId: string, values: Record<string, unknown>) => {
    if (!currentSessionId.value || !currentAgent.value || isSending.value) {
      return
    }

    const payload = JSON.stringify({
      type: 'form_response',
      formId,
      values
    }, null, 2)

    await sendWithCurrentAgent(payload)
  }

  const handleKeyDown = (event: KeyboardEvent) => {
    if (showFileMention.value) {
      return
    }

    if (event.key === 'Enter') {
      const sendOnEnter = settingsStore.settings.sendOnEnter

      if (sendOnEnter && !event.shiftKey) {
        event.preventDefault()
        void handleSend()
      } else if (!sendOnEnter && (event.metaKey || event.ctrlKey)) {
        event.preventDefault()
        void handleSend()
      }
    }
  }

  return {
    agentDropdownRef,
    agentOptions,
    closeFileMention,
    currentAgent,
    currentAgentId,
    currentAgentName,
    currentProject,
    currentSessionId,
    fileMentionPosition,
    getModelLabel,
    handleCancelCompress,
    handleConfirmCompress,
    handleFileSelect,
    handleInput,
    handleKeyDown,
    handleMessageFormSubmit,
    handleOpenCompress,
    handleSend,
    inputPlaceholder,
    inputText,
    isAgentDropdownOpen,
    isCompressing,
    isModelDropdownOpen,
    isSending,
    mentionSearchText,
    mentionStart,
    modelDropdownRef,
    parsedInputText,
    presetModelOptions,
    renderLayerRef,
    selectedModelId,
    selectAgent,
    selectModel,
    shouldShowCompressButton,
    showCompressionDialog,
    showFileMention,
    syncScroll,
    textareaRef,
    toggleAgentDropdown,
    toggleModelDropdown,
    tokenUsage,
    messageCount
  }
}
