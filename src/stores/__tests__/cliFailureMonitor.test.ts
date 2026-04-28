import { describe, expect, it } from 'vitest'

import {
  classifyCliFailureFragments,
  createCliFailureFragment
} from '@/utils/cliFailureMonitor'

describe('cliFailureMonitor', () => {
  it('classifies retryable rate-limit payloads', () => {
    const result = classifyCliFailureFragments('OpenCode', [
      createCliFailureFragment(
        'content',
        'API Error: 429 {"error":{"code":"1302","message":"您的账户已达到速率限制，请您控制请求频率"}}'
      )!
    ])

    expect(result?.kind).toBe('retryable')
  })

  it('classifies explicit stderr errors as non-retryable', () => {
    const result = classifyCliFailureFragments('Claude', [
      createCliFailureFragment('stderr', 'fatal error: unsupported model configuration')!
    ])

    expect(result?.kind).toBe('non_retryable')
  })

  it('ignores plain successful content', () => {
    const result = classifyCliFailureFragments('Codex', [
      createCliFailureFragment('content', '任务已执行完成，并生成了最终文件列表。')!
    ])

    expect(result).toBeNull()
  })

  it('ignores benign git stderr warnings when output is otherwise normal', () => {
    const result = classifyCliFailureFragments('OpenCode', [
      createCliFailureFragment('stderr', "fatal: your current branch 'main' does not have any commits yet")!
    ])

    expect(result).toBeNull()
  })

  it('ignores secondary non-content failures after a normal assistant reply', () => {
    const result = classifyCliFailureFragments('Claude', [
      createCliFailureFragment('content', '我已经完成检查，下面是修复方案。')!,
      createCliFailureFragment('tool_result', 'Error: downstream tool execution failed')!,
      createCliFailureFragment('stderr', 'fatal error: external helper exited unexpectedly')!,
      createCliFailureFragment('error', 'Unexpected runtime error')!
    ])

    expect(result).toBeNull()
  })

  it('ignores normal assistant explanations that mention rate limits', () => {
    const result = classifyCliFailureFragments('Codex', [
      createCliFailureFragment('content', '如果出现 429 rate limit，请降低并发后重试；当前任务已经分析完成。')!,
      createCliFailureFragment('stderr', 'fatal error: external helper exited unexpectedly')!
    ])

    expect(result).toBeNull()
  })

  it('classifies broken pipe process exits as retryable', () => {
    const result = classifyCliFailureFragments('Codex', [
      createCliFailureFragment('stderr', 'error: broken pipe (os error 32)')!
    ])

    expect(result?.kind).toBe('retryable')
  })
})
