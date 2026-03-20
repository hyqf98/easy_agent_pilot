import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import type { AgentConfig } from '@/stores/agent'
import type {
  AgentStrategy,
  BackendStreamEvent,
  ConversationContext,
  ExecutionRequest,
  StreamEvent
} from './types'

type AbortCommand = 'abort_cli_execution' | 'abort_sdk_execution'

interface ExecutionEventState {
  sawMeaningfulOutput: boolean
  sawDone: boolean
  sawError: boolean
}

function parseToolPayload(value?: string): Record<string, unknown> | undefined {
  if (!value) return undefined

  try {
    const parsed = JSON.parse(value)
    if (parsed && typeof parsed === 'object') {
      return parsed as Record<string, unknown>
    }
  } catch {
    return { raw: value }
  }

  return undefined
}

export abstract class BaseAgentStrategy implements AgentStrategy {
  abstract readonly name: string

  private abortController: AbortController | null = null
  private unlistenStream: UnlistenFn | null = null
  private currentSessionId: string | null = null

  abstract supports(agent: AgentConfig): boolean

  protected abstract getEventName(sessionId: string): string
  protected abstract getAbortCommand(): AbortCommand
  protected abstract buildRequest(context: ConversationContext): ExecutionRequest

  protected validateContext(
    context: ConversationContext,
    onEvent: (event: StreamEvent) => void
  ): boolean {
    void context
    void onEvent
    return true
  }

  async execute(
    context: ConversationContext,
    onEvent: (event: StreamEvent) => void
  ): Promise<void> {
    if (!this.validateContext(context, onEvent)) {
      return
    }

    this.currentSessionId = context.sessionId
    this.abortController = new AbortController()

    try {
      const eventState: ExecutionEventState = {
        sawMeaningfulOutput: false,
        sawDone: false,
        sawError: false
      }

      this.unlistenStream = await listen<BackendStreamEvent>(
        this.getEventName(context.sessionId),
        (event) => {
          const streamEvent = this.transformEvent(event.payload)
          if (streamEvent) {
            this.recordEventState(eventState, streamEvent)
            onEvent(streamEvent)
          }
        }
      )

      const request = this.buildRequest(context)

      console.info('[AI Execute] start', {
        provider: request.provider,
        mode: request.executionMode,
        responseMode: request.responseMode,
        outputFormat: request.cliOutputFormat,
        sessionId: context.sessionId
      })

      await invoke('execute_agent', { request })

      await this.waitForTerminalEvent(eventState)

      console.info('[AI Execute] done', {
        provider: request.provider,
        mode: request.executionMode,
        sessionId: context.sessionId
      })
    } catch (error) {
      if (this.abortController?.signal.aborted) {
        onEvent({ type: 'done' })
        return
      }

      const eventState = this.getSafeEventState()

      const errorMessage = error instanceof Error ? error.message : String(error)
      console.error('[AI Execute] failed', {
        provider: this.name,
        sessionId: this.currentSessionId,
        error: errorMessage
      })

      if (eventState.sawDone || eventState.sawError) {
        return
      }

      onEvent({
        type: 'error',
        error: errorMessage
      })
    } finally {
      this.cleanup()
    }
  }

  abort(): void {
    this.abortController?.abort()
    this.abortExecution()
    this.cleanup()
  }

  protected toMessageInputs(messages: ConversationContext['messages']): ExecutionRequest['messages'] {
    return messages
      .filter(message => message.role !== 'compression')
      .map(message => ({
        role: message.role as 'system' | 'user' | 'assistant',
        content: message.content,
        attachments: message.attachments
      }))
  }

  protected transformEvent(event: BackendStreamEvent): StreamEvent | null {
    const baseEvent = {
      inputTokens: event.inputTokens,
      outputTokens: event.outputTokens,
      model: event.model
    }

    switch (event.type) {
      case 'content':
        return {
          type: 'content',
          content: event.content,
          ...baseEvent
        }
      case 'thinking':
        return {
          type: 'thinking',
          content: event.content,
          ...baseEvent
        }
      case 'thinking_start':
        return {
          type: 'thinking_start',
          ...baseEvent
        }
      case 'tool_use':
        return {
          type: 'tool_use',
          toolName: event.toolName,
          toolCallId: event.toolCallId,
          toolInput: parseToolPayload(event.toolInput),
          ...baseEvent
        }
      case 'tool_input_delta':
        return {
          type: 'tool_input_delta',
          toolCallId: event.toolCallId,
          toolInput: parseToolPayload(event.toolInput),
          ...baseEvent
        }
      case 'tool_result':
        return {
          type: 'tool_result',
          toolCallId: event.toolCallId,
          toolResult: event.toolResult,
          ...baseEvent
        }
      case 'message_start':
      case 'usage':
        return {
          type: 'usage',
          ...baseEvent
        }
      case 'system':
        return {
          type: 'system',
          content: event.content,
          ...baseEvent
        }
      case 'error':
        return {
          type: 'error',
          error: event.error,
          ...baseEvent
        }
      case 'file_edit':
        return {
          type: 'file_edit',
          fileEdit: event.fileEdit,
          ...baseEvent
        }
      case 'done':
        return {
          type: 'done',
          ...baseEvent
        }
      default:
        if (event.inputTokens !== undefined || event.outputTokens !== undefined || event.model) {
          return {
            type: 'usage',
            ...baseEvent
          }
        }
        return null
    }
  }

  private lastEventState: ExecutionEventState = {
    sawMeaningfulOutput: false,
    sawDone: false,
    sawError: false
  }

  private getSafeEventState(): ExecutionEventState {
    return { ...this.lastEventState }
  }

  private recordEventState(state: ExecutionEventState, event: StreamEvent): void {
    switch (event.type) {
      case 'content':
      case 'thinking':
      case 'system':
      case 'tool_use':
      case 'tool_input_delta':
      case 'tool_result':
      case 'file_edit':
        state.sawMeaningfulOutput = true
        break
      case 'error':
        state.sawError = true
        break
      case 'done':
        state.sawDone = true
        break
      default:
        break
    }

    this.lastEventState = { ...state }
  }

  private async waitForTerminalEvent(state: ExecutionEventState): Promise<void> {
    this.lastEventState = { ...state }

    const deadline = Date.now() + (state.sawMeaningfulOutput ? 5000 : 1500)
    while (Date.now() < deadline) {
      if (state.sawDone || state.sawError) {
        break
      }
      await new Promise(resolve => setTimeout(resolve, 50))
    }
  }

  private abortExecution(): void {
    if (!this.currentSessionId) return

    invoke(this.getAbortCommand(), { sessionId: this.currentSessionId }).catch((error) => {
      console.warn('[AI Execute] abort failed', error)
    })
  }

  private cleanup(): void {
    this.unlistenStream?.()
    this.unlistenStream = null
    this.abortController = null
    this.currentSessionId = null
    this.lastEventState = {
      sawMeaningfulOutput: false,
      sawDone: false,
      sawError: false
    }
  }
}
