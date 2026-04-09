import type { AgentConfig } from '@/stores/agent'
import type { AgentStrategy, ConversationContext, StreamEvent } from './strategies/types'
import { ClaudeCliStrategy } from './strategies/ClaudeCliStrategy'
import { CodexCliStrategy } from './strategies/CodexCliStrategy'
import { OpenCodeCliStrategy } from './strategies/OpenCodeCliStrategy'
import { ClaudeSdkStrategy } from './strategies/ClaudeSdkStrategy'
import { CodexSdkStrategy } from './strategies/CodexSdkStrategy'

/**
 * 智能体执行器
 * 负责管理策略注册和执行
 */
export class AgentExecutor {
  private strategies: AgentStrategy[] = []
  private activeStrategies: Map<string, AgentStrategy> = new Map()

  constructor() {
    // 注册默认策略
    this.registerStrategy(new ClaudeCliStrategy())
    this.registerStrategy(new CodexCliStrategy())
    this.registerStrategy(new OpenCodeCliStrategy())
    this.registerStrategy(new ClaudeSdkStrategy())
    this.registerStrategy(new CodexSdkStrategy())
  }

  /**
   * 注册策略
   */
  registerStrategy(strategy: AgentStrategy): void {
    this.strategies.push(strategy)
  }

  /**
   * 获取支持的策略
   */
  getSupportedStrategy(agent: AgentConfig): AgentStrategy | null {
    return this.strategies.find(strategy => strategy.supports(agent)) || null
  }

  /**
   * 检查是否支持该智能体
   */
  isSupported(agent: AgentConfig): boolean {
    return this.strategies.some(strategy => strategy.supports(agent))
  }

  /**
   * 执行对话
   */
  async execute(
    context: ConversationContext,
    onEvent: (event: StreamEvent) => void
  ): Promise<void> {
    const { agent, sessionId } = context

    // 查找支持的策略
    const strategy = this.getSupportedStrategy(agent)
    if (!strategy) {
      onEvent({
        type: 'error',
        error: `不支持的智能体类型: ${agent.type} (${agent.provider || 'unknown'})`
      })
      return
    }

    // 将策略注册到 activeStrategies
    this.activeStrategies.set(sessionId, strategy)

    try {
      await strategy.execute(context, onEvent)
    } finally {
      // 从 Map 中移除
      this.activeStrategies.delete(sessionId)
    }
  }

  /**
   * 中断指定会话的执行
   */
  abort(sessionId: string): void {
    const strategy = this.activeStrategies.get(sessionId)
    if (strategy) {
      strategy.abort()
      this.activeStrategies.delete(sessionId)
    }
  }

  /**
   * 获取所有已注册的策略名称
   */
  getRegisteredStrategies(): string[] {
    return this.strategies.map(s => s.name)
  }
}

// 创建全局单例
export const agentExecutor = new AgentExecutor()
