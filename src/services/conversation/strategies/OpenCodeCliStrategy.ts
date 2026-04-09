import { BaseAgentStrategy } from './BaseAgentStrategy'

export class OpenCodeCliStrategy extends BaseAgentStrategy {
  readonly name = 'OpenCode CLI'
  protected readonly runtimeKey = 'opencode-cli' as const
}
