import type { AgentConfig } from '@/stores/agent'
import type {
  ConversationContext,
  ExecutionRequest
} from './types'
import { BaseAgentStrategy } from './BaseAgentStrategy'

/**
 * Codex CLI 策略
 * 通过调用本地 codex 命令行工具执行对话
 */
export class CodexCliStrategy extends BaseAgentStrategy {
  readonly name = 'Codex CLI'

  supports(agent: AgentConfig): boolean {
    return agent.type === 'cli' && agent.provider === 'codex'
  }

  protected getEventName(sessionId: string): string {
    return `codex-stream-${sessionId}`
  }

  protected getAbortCommand(): 'abort_cli_execution' {
    return 'abort_cli_execution'
  }

  protected buildRequest(context: ConversationContext): ExecutionRequest {
    const {
      sessionId,
      agent,
      workingDirectory,
      mcpServers,
      cliOutputFormat,
      jsonSchema,
      extraCliArgs,
      executionMode,
      responseMode,
      messages
    } = context

    return {
      sessionId,
      agentType: 'cli',
      provider: 'codex',
      cliPath: agent.cliPath || 'codex',
      modelId: agent.modelId && agent.modelId.trim() && agent.modelId !== 'default'
        ? agent.modelId
        : undefined,
      messages: this.toMessageInputs(messages),
      workingDirectory,
      allowedTools: ['Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash'],
      mcpServers,
      cliOutputFormat: cliOutputFormat ?? (responseMode === 'json_once' ? 'json' : 'stream-json'),
      jsonSchema,
      extraCliArgs,
      executionMode: executionMode ?? 'chat',
      responseMode: responseMode ?? 'stream_text'
    }
  }
}
