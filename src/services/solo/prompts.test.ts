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

  it('infers dispatch_step when coordinator omits top-level type', () => {
    const decision = parseSoloCoordinatorDecision(JSON.stringify({
      stepRef: 'refresh-file-tree',
      depth: 1,
      title: '修复文件树刷新',
      description: '让新增、删除和重命名后列表实时刷新。',
      executionPrompt: '修复文件树缓存与刷新逻辑并验证。',
      doneWhen: ['操作后列表立即刷新', '手动刷新拿到最新数据']
    }))

    expect(decision.type).toBe('dispatch_step')
    if (decision.type !== 'dispatch_step') {
      throw new Error('expected dispatch_step')
    }
    expect(decision.step.stepRef).toBe('refresh-file-tree')
    expect(decision.step.doneWhen).toEqual(['操作后列表立即刷新', '手动刷新拿到最新数据'])
  })

  it('parses action and payload wrapped dispatch payloads', () => {
    const decision = parseSoloCoordinatorDecision(JSON.stringify({
      action: 'dispatch',
      payload: {
        step_ref: 'fix-minimax-structured-output',
        parent_step_ref: 'solo-root',
        depth: '2',
        title: '兼容 Minimax 结构化结果',
        description: '兼容 action/payload 和 snake_case 字段。',
        selected_expert_id: 'expert-minimax',
        execution_prompt: '补齐解析兼容和测试。',
        done_when: ['兼容 action/payload', '兼容 snake_case']
      }
    }))

    expect(decision.type).toBe('dispatch_step')
    if (decision.type !== 'dispatch_step') {
      throw new Error('expected dispatch_step')
    }
    expect(decision.step.stepRef).toBe('fix-minimax-structured-output')
    expect(decision.step.parentStepRef).toBe('solo-root')
    expect(decision.step.selectedExpertId).toBe('expert-minimax')
    expect(decision.step.doneWhen).toEqual(['兼容 action/payload', '兼容 snake_case'])
  })

  it('parses block_run with forms fallback schema', () => {
    const decision = parseSoloCoordinatorDecision(JSON.stringify({
      action: 'blocked',
      reason: '需要用户补充运行环境',
      question: '请确认运行环境',
      forms: [{
        formId: 'env',
        title: '运行环境',
        fields: [
          {
            name: 'runtime',
            label: '运行时',
            type: 'text'
          }
        ]
      }]
    }))

    expect(decision.type).toBe('block_run')
    if (decision.type !== 'block_run') {
      throw new Error('expected block_run')
    }
    expect(decision.reason).toBe('需要用户补充运行环境')
    expect(decision.formSchema?.formId).toBe('env')
  })
})
