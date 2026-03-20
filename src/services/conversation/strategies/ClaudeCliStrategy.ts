import type { AgentConfig } from '@/stores/agent'
import type {
  ConversationContext,
  ExecutionRequest
} from './types'
import { BaseAgentStrategy } from './BaseAgentStrategy'
import { appendClaudeMcpAllowedTools } from '@/utils/mcpServerConfig'

/**
 * Claude CLI 策略
 * 通过调用本地 claude 命令行工具执行对话
 */
export class ClaudeCliStrategy extends BaseAgentStrategy {
  readonly name = 'Claude CLI'

  supports(agent: AgentConfig): boolean {
    return agent.type === 'cli' && agent.provider === 'claude'
  }

  protected getEventName(sessionId: string): string {
    return `claude-stream-${sessionId}`
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
    const allowedTools = appendClaudeMcpAllowedTools(
      ['Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash', 'WebFetch', 'WebSearch'],
      mcpServers
    )

    return {
      sessionId,
      agentType: 'cli',
      provider: 'claude',
      cliPath: agent.cliPath || 'claude',
      modelId: agent.modelId && agent.modelId.trim() && agent.modelId !== 'default'
        ? agent.modelId
        : undefined,
      messages: this.toMessageInputs(messages),
      workingDirectory,
      allowedTools,
      mcpServers,
      cliOutputFormat: cliOutputFormat ?? (responseMode === 'json_once' ? 'json' : 'stream-json'),
      jsonSchema,
      extraCliArgs,
      executionMode: executionMode ?? 'chat',
      responseMode: responseMode ?? 'stream_text'
    }
  }
}
