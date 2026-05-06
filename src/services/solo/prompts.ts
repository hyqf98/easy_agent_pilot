import type {
  SoloCoordinatorBlockResult,
  SoloCoordinatorDecision,
  SoloInputRequest,
  SoloLogEntry,
  SoloRun,
  SoloStep
} from '@/types/solo'
import { buildExpertSystemPrompt } from '@/services/agentTeams/runtime'

const MAX_CONTEXT_STEPS = 12
const MAX_RESUME_LOG_LINES = 40
const MAX_SUMMARY_LENGTH = 220
const SOLO_BUILTIN_COORDINATOR_PROMPT = [
  '你是 SOLO 模式的内置统一调度协调者。',
  '你的职责是把用户给出的需求和目标拆成当前最值得执行的一步，并持续推动直到完成目标。',
  '你要像一个会自己编排团队分工的项目总控，但输出必须克制、稳定、可执行。',
  '协调层只做调度与验收边界定义，执行层只做当前步骤落实，这个边界不能混淆。',
  '优先利用已提供的参与专家目录来决定当前步骤最合适的视角，不要虚构新的专家角色。',
  '如果已经有足够信息，就继续推进；如果必须卡住，只请求继续推进所需的最小输入。',
  '只要上一轮专家已经给出明确下一步，就直接派发实现或验证步骤，不要重复安排调研类步骤。',
  '每一步都要带上清晰范围、可观察完成条件和必要验证动作，让执行专家可以在一次连续回合里闭环。'
].join('\n')

function escapeMultiline(value: string): string {
  return value.trim() || '（空）'
}

function safeJsonParse<T>(raw: string): T | null {
  try {
    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function readString(value: Record<string, unknown>, ...keys: string[]): string | undefined {
  for (const key of keys) {
    if (typeof value[key] === 'string') {
      const normalized = value[key].trim()
      if (normalized) {
        return normalized
      }
    }
  }

  return undefined
}

function readStringArray(value: Record<string, unknown>, ...keys: string[]): string[] {
  for (const key of keys) {
    if (Array.isArray(value[key])) {
      return value[key]
        .map((item) => String(item).trim())
        .filter(Boolean)
    }
  }

  return []
}

function readRecord(value: Record<string, unknown>, ...keys: string[]): Record<string, unknown> | undefined {
  for (const key of keys) {
    const candidate = value[key]
    if (isRecord(candidate)) {
      return candidate
    }
  }

  return undefined
}

function normalizeSoloDecisionType(rawType: string | undefined): SoloCoordinatorDecision['type'] | null {
  const normalized = rawType?.trim().toLowerCase()
  if (!normalized) {
    return null
  }

  if ([
    'dispatch_step',
    'dispatchstep',
    'dispatch',
    'next_step',
    'nextstep',
    'step'
  ].includes(normalized)) {
    return 'dispatch_step'
  }

  if ([
    'complete_run',
    'completerun',
    'complete',
    'completed',
    'finish',
    'finished'
  ].includes(normalized)) {
    return 'complete_run'
  }

  if ([
    'block_run',
    'blockrun',
    'block',
    'blocked',
    'need_input',
    'needinfo',
    'need_info',
    'ask_user'
  ].includes(normalized)) {
    return 'block_run'
  }

  return null
}

function normalizeSoloDecisionRecord(value: Record<string, unknown>): Record<string, unknown> {
  const stepPayload = readRecord(value, 'step', 'payload', 'data')
  const stepCandidate = stepPayload ?? value
  const formSchema = readRecord(value, 'formSchema', 'form_schema')
    ?? (Array.isArray(value.forms) && isRecord(value.forms[0]) ? value.forms[0] as Record<string, unknown> : undefined)

  return {
    ...value,
    ...(normalizeSoloDecisionType(readString(value, 'type', 'action', 'decision', 'phase'))
      ? { type: normalizeSoloDecisionType(readString(value, 'type', 'action', 'decision', 'phase')) }
      : {}),
    stepRef: readString(stepCandidate, 'stepRef', 'step_ref', 'ref'),
    parentStepRef: readString(stepCandidate, 'parentStepRef', 'parent_step_ref', 'parentRef', 'parent_ref'),
    depth: typeof stepCandidate.depth === 'number'
      ? stepCandidate.depth
      : typeof stepCandidate.depth === 'string' && stepCandidate.depth.trim()
        ? Number(stepCandidate.depth)
        : undefined,
    title: readString(stepCandidate, 'title', 'name'),
    description: readString(stepCandidate, 'description', 'desc'),
    selectedExpertId: readString(
      stepCandidate,
      'selectedExpertId',
      'selected_expert_id',
      'expertId',
      'expert_id',
      'assigneeExpertId',
      'assignee_expert_id'
    ),
    executionPrompt: readString(stepCandidate, 'executionPrompt', 'execution_prompt', 'prompt', 'instruction', 'instructions'),
    doneWhen: readStringArray(
      stepCandidate,
      'doneWhen',
      'done_when',
      'acceptanceCriteria',
      'acceptance_criteria',
      'successCriteria',
      'success_criteria'
    ),
    summary: readString(value, 'summary', 'resultSummary', 'result_summary', 'finalSummary', 'final_summary'),
    deliveredArtifacts: readStringArray(value, 'deliveredArtifacts', 'delivered_artifacts', 'artifacts'),
    reason: readString(value, 'reason', 'blockReason', 'block_reason', 'message', 'error'),
    question: readString(value, 'question', 'userQuestion', 'user_question', 'ask'),
    formSchema
  }
}

function extractJsonCandidate(raw: string): string | null {
  const trimmed = raw.trim()
  if (!trimmed) return null
  if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
    return trimmed
  }

  const firstBrace = trimmed.indexOf('{')
  const lastBrace = trimmed.lastIndexOf('}')
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    return trimmed.slice(firstBrace, lastBrace + 1)
  }
  return null
}

function unwrapSoloDecisionCandidate(value: unknown): Record<string, unknown> | null {
  if (!value) {
    return null
  }

  if (typeof value === 'string') {
    const candidate = extractJsonCandidate(value)
    if (!candidate) {
      return null
    }

    return unwrapSoloDecisionCandidate(safeJsonParse<unknown>(candidate))
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      const nested = unwrapSoloDecisionCandidate(item)
      if (nested) {
        return nested
      }
    }
    return null
  }

  if (!isRecord(value)) {
    return null
  }

  const normalizedRecord = normalizeSoloDecisionRecord(value)

  if (typeof normalizedRecord.type === 'string' && ['dispatch_step', 'complete_run', 'block_run'].includes(normalizedRecord.type)) {
    return normalizedRecord
  }

  const inferredType = inferSoloDecisionType(normalizedRecord)
  if (inferredType) {
    return {
      ...normalizedRecord,
      type: inferredType
    }
  }

  const directKeys = ['structured_output', 'structuredOutput', 'result', 'data', 'value', 'output', 'payload', 'response', 'message']
  for (const key of directKeys) {
    const nested = unwrapSoloDecisionCandidate(value[key])
    if (nested) {
      return nested
    }
  }

  for (const nestedValue of Object.values(value)) {
    const nested = unwrapSoloDecisionCandidate(nestedValue)
    if (nested) {
      return nested
    }
  }

  return null
}

function inferSoloDecisionType(value: Record<string, unknown>): SoloCoordinatorDecision['type'] | null {
  const stepCandidate = normalizeSoloDecisionRecord(value)
  const doneWhen = Array.isArray(stepCandidate.doneWhen) ? stepCandidate.doneWhen : null
  if (
    typeof stepCandidate.stepRef === 'string'
    && typeof stepCandidate.title === 'string'
    && typeof stepCandidate.description === 'string'
    && typeof stepCandidate.executionPrompt === 'string'
    && doneWhen
    && doneWhen.length > 0
  ) {
    return 'dispatch_step'
  }

  if (
    typeof value.summary === 'string'
    || Array.isArray(value.deliveredArtifacts)
  ) {
    return 'complete_run'
  }

  if (
    typeof value.reason === 'string'
    || value.formSchema
    || typeof value.question === 'string'
  ) {
    return 'block_run'
  }

  return null
}

export function buildSoloControlJsonSchema(): string {
  return JSON.stringify({
    type: 'object',
    additionalProperties: false,
    properties: {
      type: {
        type: 'string',
        enum: ['dispatch_step', 'complete_run', 'block_run']
      },
      stepRef: { type: 'string' },
      parentStepRef: { type: 'string' },
      depth: { type: 'integer', minimum: 1 },
      title: { type: 'string' },
      description: { type: 'string' },
      selectedExpertId: { type: 'string' },
      executionPrompt: { type: 'string' },
      doneWhen: {
        type: 'array',
        items: { type: 'string' }
      },
      summary: { type: 'string' },
      deliveredArtifacts: {
        type: 'array',
        items: { type: 'string' }
      },
      reason: { type: 'string' },
      question: { type: 'string' },
      formSchema: {
        type: 'object',
        additionalProperties: false,
        properties: {
          formId: { type: 'string' },
          title: { type: 'string' },
          description: { type: 'string' },
          submitText: { type: 'string' },
          fields: {
            type: 'array',
            items: {
              type: 'object',
              additionalProperties: false,
              required: ['name', 'label', 'type'],
              properties: {
                name: { type: 'string' },
                label: { type: 'string' },
                type: {
                  type: 'string',
                  enum: ['text', 'textarea', 'select', 'multiselect', 'number', 'checkbox', 'radio', 'date', 'file', 'code', 'slider']
                },
                placeholder: { type: 'string' },
                required: { type: 'boolean' },
                options: {
                  type: 'array',
                  items: {
                    type: 'object',
                    additionalProperties: false,
                    required: ['label', 'value'],
                    properties: {
                      label: { type: 'string' },
                      value: {
                        anyOf: [
                          { type: 'string' },
                          { type: 'number' },
                          { type: 'boolean' }
                        ]
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    },
    required: ['type']
  }, null, 2)
}

export function buildSoloCoordinatorSystemPrompt(
  coordinatorPrompt: string | null | undefined,
  expertCatalogPrompt: string
): string {
  return buildExpertSystemPrompt(coordinatorPrompt || SOLO_BUILTIN_COORDINATOR_PROMPT, [
    [
      '你是 SOLO 模式里的统一协调 AI。',
      '你不是传统聊天助手，而是一个持续推进任务的内部总控。',
      '你必须围绕用户定义的需求和实现目标，不断判断下一步最有价值的动作。',
      '每个参与专家都在各自独立的内部执行会话里工作，专家之间不能共享原始对话上下文。',
      '你只能看到每个专家回传的结构化交接摘要，不能假设自己看过对方的完整执行日志。',
      '你要把 SOLO 当成一个 harness：靠明确步骤边界、验收条件、历史摘要和日志回写来稳定推进，而不是靠自由发挥。',
      '这是纯决策回合，不是代码执行回合。你不能调用 StructuredOutput、Skill、Task、TodoWrite、Read、Edit、Write、Bash、WebFetch、WebSearch 或任何其他工具。',
      '你也不能先去探索代码库；你只能基于当前提供的上下文决定下一步。',
      '每一轮只能做一件事：派发下一步、宣布完成、或阻塞等待用户输入。',
      '派发步骤时必须保持粒度足够小，确保该步骤可以被一次连续执行完成并验证。',
      '派发步骤时，默认要求执行专家在同一步内完成“修改/验证/回写证据”，不要把明显可以一起完成的验证动作拆到下一轮。',
      '如果当前问题超出最大调度层数，禁止继续拆细，必须 block_run 说明原因。',
      '如果最近一步专家已经明确了下一步改动范围、目标文件或验证动作，优先直接派发该动作，不要再次安排盘点现状。',
      '如果信息已经足够，就继续执行；如果信息不足，只收集继续推进所必需的最小信息。',
      '直接输出一个 JSON 对象，不要输出解释、Markdown、代码块或任何额外文本。',
      '禁止空回复；如果拿不准，也必须返回合法 JSON，并优先使用 block_run 请求最小必要信息。',
      '输出前自行检查：整段内容必须可被 JSON.parse 直接解析，且顶层必须包含 type。'
    ].join('\n'),
    expertCatalogPrompt
  ])
}

export function buildSoloBuiltinCoordinatorPrompt(): string {
  return SOLO_BUILTIN_COORDINATOR_PROMPT
}

export function buildSoloControlPrompt(input: {
  run: SoloRun
  steps: SoloStep[]
  inputResponse?: Record<string, unknown> | null
}): string {
  const { run, steps, inputResponse } = input
  const recentSteps = steps.slice(-MAX_CONTEXT_STEPS)
  const lines: string[] = [
    '你正在进行一个 SOLO 运行，请决定下一步。',
    '',
    `运行名称: ${run.name}`,
    '需求说明:',
    escapeMultiline(run.requirement),
    '',
    '实现目标:',
    escapeMultiline(run.goal),
    '',
    `执行路径: ${run.executionPath}`,
    '',
    `最大调度层数: ${run.maxDispatchDepth}`,
    `当前已达到层数: ${run.currentDepth}`,
    ''
  ]

  if (recentSteps.length > 0) {
    lines.push('最近步骤进展:')
    recentSteps.forEach((step, index) => {
      lines.push(
        `${index + 1}. [${step.status}] depth=${step.depth} ref=${step.stepRef} title=${step.title}`
      )
      if (step.summary) {
        lines.push(`   handoff_overview: ${step.summary}`)
      }
      if (step.resultSummary) {
        lines.push(`   result_summary: ${step.resultSummary}`)
      }
      if (step.resultFiles.length > 0) {
        lines.push(`   changed_files: ${step.resultFiles.join(', ')}`)
      }
      if (step.failReason) {
        lines.push(`   fail: ${step.failReason}`)
      }
    })
    lines.push('')
  } else {
    lines.push('当前还没有步骤，请从最关键的起始动作开始。', '')
  }

  if (inputResponse && Object.keys(inputResponse).length > 0) {
    lines.push('用户最近补充的信息:')
    Object.entries(inputResponse).forEach(([key, value]) => {
      lines.push(`- ${key}: ${typeof value === 'object' ? JSON.stringify(value) : String(value)}`)
    })
    lines.push('')
  }

  lines.push('决策规则:')
  lines.push('- 如果目标已经完成，输出 complete_run。')
  lines.push('- 如果必须等待用户补充信息，输出 block_run。')
  lines.push('- 否则输出 dispatch_step，且该步骤必须是当前最有价值、最小可执行的一步。')
  lines.push('- 你必须根据最近一步专家返回的结构化交接摘要（任务概述 / 任务状态 / 文件变更 / 结果总结），判断下一步最适合交给哪个 expertId。')
  lines.push('- 如果最近一步结果已经明确推荐了下一位专家或下一项实现/验证动作，默认承接该建议继续推进。')
  lines.push('- dispatch_step 必须明确本步作用范围、关键约束和验证方式；不要给出空泛任务标题。')
  lines.push('- 如果本步涉及代码修改，优先把必要验证一并纳入同一步 doneWhen。')
  lines.push('- dispatch_step.depth 不能超过最大调度层数。')
  lines.push('- doneWhen 必须是可观察、可验证的完成条件。')
  lines.push('- selectedExpertId 应从专家目录中选择最贴近该步骤的视角；如果没有合适专家，可以留空。')
  lines.push('- 返回 dispatch_step 时，请直接在顶层返回 stepRef、depth、title、description、executionPrompt、doneWhen，不要嵌套 step 对象。')
  lines.push('- 顶层键名必须严格使用 type、stepRef、parentStepRef、selectedExpertId、executionPrompt、doneWhen、summary、deliveredArtifacts、reason、question、formSchema；不要使用 action、payload、step_ref、selected_expert_id、execution_prompt、done_when、delivered_artifacts、form_schema 等别名。')
  lines.push('- 严禁调用 StructuredOutput、Skill、Read、Bash 或任何工具。')
  lines.push('- 你不需要阅读仓库；此回合只做调度决策。')
  lines.push('- 禁止空回复；如果无法继续推进，也必须返回合法的 block_run JSON，不要留空。')
  lines.push('- 输出前再次自检：只能返回 JSON，对象顶层必须有 type，且不能带 Markdown 代码块或额外解释。')
  lines.push('')
  lines.push('合法示例:')
  lines.push('{"type":"dispatch_step","stepRef":"inspect-current-solo-ui","depth":1,"title":"检查当前 SOLO 页面链路","description":"确认现有 SOLO 页面、状态与日志结构，识别下一步实现边界。","selectedExpertId":"expert-id","executionPrompt":"检查当前 SOLO 页面和相关状态管理实现，归纳现状与缺口。","doneWhen":["明确页面入口与状态流转","确认日志与步骤展示方式","产出下一步实现建议"]}')

  return lines.join('\n')
}

export function buildSoloExecutionPrompt(input: {
  run: SoloRun
  step: SoloStep
  doneWhen: string[]
  inputResponse?: Record<string, unknown> | null
  resumeContext?: string
}): string {
  const { run, step, doneWhen, inputResponse, resumeContext } = input
  const lines: string[] = [
    '你正在执行 SOLO 当前步骤。',
    '',
    `运行名称: ${run.name}`,
    `步骤标题: ${step.title}`,
    `步骤层数: ${step.depth}`,
    '',
    '整体目标:',
    escapeMultiline(run.goal),
    '',
    '当前步骤说明:',
    escapeMultiline(step.description || step.executionPrompt || step.title),
    '',
    '执行要求:',
    escapeMultiline(step.executionPrompt || step.description || step.title),
    '',
    '本步完成条件:'
  ]

  doneWhen.forEach((item, index) => {
    lines.push(`${index + 1}. ${item}`)
  })
  lines.push('')

  if (inputResponse && Object.keys(inputResponse).length > 0) {
    lines.push('用户补充信息:')
    Object.entries(inputResponse).forEach(([key, value]) => {
      lines.push(`- ${key}: ${typeof value === 'object' ? JSON.stringify(value) : String(value)}`)
    })
    lines.push('')
  }

  if (resumeContext) {
    lines.push('恢复上下文:')
    lines.push(resumeContext)
    lines.push('')
  }

  lines.push('输出要求:')
  lines.push('- 直接推进执行，不要重新规划整个项目。')
  lines.push('- 先用最少量的读取动作确认必要上下文；一旦足够，就立刻开始改代码、执行命令或做验证。')
  lines.push('- 把自己当成 harness 里的执行工位：你的职责是把这一步做完并留下证据，而不是重新设计整套流程。')
  lines.push('- 不要输出“我将要做什么”“我正在做什么”这类元叙述进度播报，除非是在最终结构化结果中总结。')
  lines.push('- 不要为了可选基础设施、后续优化或与当前步骤无关的环境问题偏航；例如未被本步明确要求时，不要自行扩展到 MCP bridge、CI、发布脚本或额外重构。')
  lines.push('- 若必须补充信息，输出 form_request JSON。')
  lines.push('- 你只能在自己的专家会话里完成当前步骤；不要模拟协调师，不要替其他专家做后续调度。')
  lines.push('- 如果可以运行命令、浏览器或 Tauri MCP 验证，就优先给出真实验证证据，不要只做静态猜测。')
  lines.push('- 如果完成本步，请在最后输出一个结构化 JSON，把当前专家结果明确交回给规划协调师。')
  lines.push('```json')
  lines.push('{"task_overview":"一句话说明本步任务与结论","task_status":"completed","completed_points":["完成点1","完成点2"],"result_summary":"1-3句总结本步结果","generated_files":[],"modified_files":[],"changed_files":[],"deleted_files":[]}')
  lines.push('```')
  lines.push('- task_status 只能填写 completed、blocked、failed 之一；正常完成时填 completed。')
  lines.push('- completed_points 要列出本步真正完成的事项。')
  lines.push('- modified_files / generated_files / deleted_files 要写出实际涉及的文件路径。')
  lines.push('- result_summary 要让规划协调师读完后就知道下一步应该交给哪个专家继续推进，并包含必要的验证结论。')
  lines.push('- 如果你执行了浏览器、Tauri MCP、端到端或命令验证，要把验证路径和结论压缩写进 completed_points 或 result_summary。')

  return lines.join('\n')
}

export function buildSoloControlRepairPrompt(input: {
  errorMessage: string
  rawOutput: string
  maxDispatchDepth: number
}): string {
  const { errorMessage, rawOutput, maxDispatchDepth } = input
  return [
    '你上一条 SOLO 协调回复不符合格式要求。',
    '不要继续规划，不要解释原因，不要输出 Markdown 或代码块；只基于你刚才已经做出的判断，重新整理并输出一个合法 JSON。',
    '',
    `错误原因: ${errorMessage}`,
    `最大调度层数: ${maxDispatchDepth}`,
    '',
    '你上一条原始输出:',
    rawOutput.trim() || '（空）',
    '',
    '重新输出规则:',
    '- 只能返回一个 JSON 对象。',
    '- type 只能是 dispatch_step、complete_run、block_run 之一。',
    '- 如果是 dispatch_step，必须包含 stepRef、depth、title、description、executionPrompt、doneWhen。',
    '- 键名必须使用 stepRef / parentStepRef / selectedExpertId / executionPrompt / doneWhen / deliveredArtifacts / formSchema，不要使用 snake_case 或 action/payload 包装。',
    '- dispatch_step.depth 不能超过最大调度层数。',
    '- 禁止附加解释、前后缀文本、Markdown 代码块或第二个对象。'
  ].join('\n')
}

export function buildSoloExecutionRepairPrompt(input: {
  errorMessage: string
  rawOutput: string
}): string {
  const { errorMessage, rawOutput } = input
  return [
    '你上一条 SOLO 执行回复不符合结果输出要求。',
    '不要再次读取、修改、执行或调用工具；只把你刚才已经完成的结果重新整理成合法 JSON。',
    '',
    `错误原因: ${errorMessage}`,
    '',
    '你上一条原始输出:',
    rawOutput.trim() || '（空）',
    '',
    '重新输出规则:',
    '- 如果确实还需要用户补充信息，输出一个合法的 form_request JSON。',
    '- 否则输出一个合法的结果 JSON，至少包含 result_summary，并尽量补齐 generated_files / modified_files / changed_files / deleted_files。',
    '- 可以补充 task_overview、task_status、completed_points，但最终只能返回一个 JSON 对象。',
    '- 禁止附加解释、前后缀文本、Markdown 代码块或第二个对象。'
  ].join('\n')
}

export function buildSoloExecutionSystemPrompt(expertPrompt: string | null | undefined): string {
  return buildExpertSystemPrompt(expertPrompt, [
    [
      '你当前在 SOLO 模式下执行一个具体步骤。',
      '保持执行导向，优先交付可以验证的结果。',
      '只处理当前步骤，不要自己切换为宏观项目规划。',
      '遇到缺失信息时，优先最小化提问；如果确实无法继续，可输出 form_request。',
      '若步骤是实现类任务，最多做少量必要读取后就开始落代码，不要长时间停留在分析或自述状态。',
      '不要把回复写成工作日志；中间过程以工具调用和实际改动为主，最终再用结构化 JSON 总结结果。',
      '如果你被分配的是测试或验证类步骤，优先实际使用浏览器自动化、Tauri MCP、命令行验证、控制台检查或端到端路径验证，而不是只停留在代码静态判断。'
    ].join('\n')
  ])
}

export function buildSoloResumeContext(logs: SoloLogEntry[]): string {
  const transcript = logs
    .slice(-MAX_RESUME_LOG_LINES)
    .map((log) => {
      const normalized = log.content.replace(/\s+/g, ' ').trim()
      if (!normalized) {
        return ''
      }
      if (log.type === 'tool_use') {
        return `[tool:${log.metadata?.toolName || 'unknown'}] ${normalized}`
      }
      if (log.type === 'error') {
        return `[error] ${normalized}`
      }
      if (log.type === 'thinking' || log.type === 'thinking_start') {
        return `[thinking] ${normalized}`
      }
      return `[${log.scope}] ${normalized}`
    })
    .filter(Boolean)
    .join('\n')

  return transcript.trim()
}

export function compactSoloSummary(content: string | null | undefined, fallback = '暂无摘要'): string {
  const normalized = (content || '')
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()

  if (!normalized) {
    return fallback
  }

  if (normalized.length <= MAX_SUMMARY_LENGTH) {
    return normalized
  }

  return `${normalized.slice(0, MAX_SUMMARY_LENGTH)}...`
}

export function buildSoloInputRequest(
  formSchema: SoloInputRequest['formSchema'],
  question: string | undefined,
  source: SoloInputRequest['source'],
  stepId?: string
): SoloInputRequest {
  return {
    formSchema,
    question,
    source,
    stepId,
    requestedAt: new Date().toISOString()
  }
}

export function parseSoloCoordinatorDecision(content: string): SoloCoordinatorDecision {
  const candidate = extractJsonCandidate(content) || '{}'
  const rawParsed = safeJsonParse<unknown>(candidate)
  const parsed = unwrapSoloDecisionCandidate(rawParsed)

  if (!parsed) {
    throw new Error('协调 AI 返回结果缺少 type 字段')
  }

  if (typeof parsed.type !== 'string') {
    throw new Error('协调 AI 返回结果缺少 type 字段')
  }

  if (parsed.type === 'complete_run') {
    return {
      type: 'complete_run',
      summary: typeof parsed.summary === 'string' ? parsed.summary.trim() : '',
      deliveredArtifacts: Array.isArray(parsed.deliveredArtifacts)
        ? parsed.deliveredArtifacts.map((item) => String(item))
        : []
    }
  }

  if (parsed.type === 'block_run') {
    const blockResult: SoloCoordinatorBlockResult = {
      type: 'block_run',
      reason: typeof parsed.reason === 'string' ? parsed.reason.trim() : '需要更多输入',
      question: typeof parsed.question === 'string' ? parsed.question : undefined
    }

    if (parsed.formSchema && typeof parsed.formSchema === 'object') {
      blockResult.formSchema = parsed.formSchema as SoloCoordinatorBlockResult['formSchema']
    } else if (Array.isArray(parsed.forms) && parsed.forms[0] && typeof parsed.forms[0] === 'object') {
      blockResult.formSchema = parsed.forms[0] as SoloCoordinatorBlockResult['formSchema']
    }

    return blockResult
  }

  if (parsed.type === 'dispatch_step') {
    const step = parsed.step && typeof parsed.step === 'object'
      ? parsed.step as Record<string, unknown>
      : parsed
    const doneWhen = Array.isArray(step.doneWhen)
      ? step.doneWhen.map((item) => String(item).trim()).filter(Boolean)
      : []
    if (!step.stepRef || !step.title || !step.description || !step.executionPrompt || doneWhen.length === 0) {
      throw new Error('dispatch_step 缺少必填字段')
    }
    return {
      type: 'dispatch_step',
      step: {
        stepRef: String(step.stepRef),
        parentStepRef: typeof step.parentStepRef === 'string' ? step.parentStepRef : undefined,
        depth: Number(step.depth || 1),
        title: String(step.title),
        description: String(step.description),
        selectedExpertId: typeof step.selectedExpertId === 'string' ? step.selectedExpertId : undefined,
        executionPrompt: String(step.executionPrompt),
        doneWhen
      }
    }
  }

  throw new Error('协调 AI 返回了不支持的 SOLO 决策类型')
}
