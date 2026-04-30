import { describe, expect, it } from 'vitest'
import { parseSoloCoordinatorDecision } from './prompts'

describe('parseSoloCoordinatorDecision', () => {
  it('parses structured_output wrapper payloads', () => {
    const decision = parseSoloCoordinatorDecision(JSON.stringify({
      type: 'result',
      structured_output: {
        type: 'dispatch_step',
        stepRef: 'fix-solo-parser',
        depth: 1,
        title: '修复 SOLO 解析',
        description: '兼容协调层包装结果。',
        executionPrompt: '修改解析逻辑并补测试。',
        doneWhen: ['解析 structured_output', '补覆盖测试']
      }
    }))

    expect(decision.type).toBe('dispatch_step')
    if (decision.type !== 'dispatch_step') {
      throw new Error('expected dispatch_step')
    }
    expect(decision.step.stepRef).toBe('fix-solo-parser')
    expect(decision.step.doneWhen).toEqual(['解析 structured_output', '补覆盖测试'])
  })

  it('parses nested result wrapper payloads', () => {
    const decision = parseSoloCoordinatorDecision(JSON.stringify({
      result: {
        type: 'block_run',
        reason: '需要用户提供部署环境',
        question: '请补充目标部署环境'
      }
    }))

    expect(decision).toEqual({
      type: 'block_run',
      reason: '需要用户提供部署环境',
      question: '请补充目标部署环境'
    })
  })
})
