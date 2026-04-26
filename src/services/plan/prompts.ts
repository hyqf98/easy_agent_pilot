import i18n from '@/i18n'
import type { AITaskItem, TaskCountMode } from '@/types/plan'

export interface PlanSplitPromptContext {
  planName: string
  planDescription?: string
  minTaskCount: number
}

export interface TaskResplitPromptContext {
  planName: string
  planDescription?: string
  taskTitle: string
  taskDescription?: string
  implementationSteps: string[]
  testSteps: string[]
  acceptanceCriteria?: string[]
  userPrompt?: string
  minTaskCount: number
}

export interface TaskListOptimizePromptContext {
  planName: string
  planDescription?: string
  tasks: AITaskItem[]
  userPrompt?: string
  targetIndex?: number
  taskCountMode?: TaskCountMode
  minTaskCount?: number
}

function t(key: string, params?: Record<string, unknown>): string {
  return params ? i18n.global.t(key, params) as string : i18n.global.t(key) as string
}

export function buildPlanSplitSystemPrompt(): string {
  return t('prompts.plan.splitSystem').trim()
}

export function buildPlanSplitKickoffPrompt(context: PlanSplitPromptContext): string {
  return [
    `${t('prompts.plan.kickoffPlanName')}: ${context.planName}`,
    `${t('prompts.plan.kickoffPlanDescription')}: ${context.planDescription?.trim() || t('prompts.plan.none')}`,
    `${t('prompts.plan.kickoffMinTaskCount')}: ${context.minTaskCount}`,
    '',
    t('prompts.plan.kickoffStart')
  ].join('\n').trim()
}

export function buildTaskResplitKickoffPrompt(context: TaskResplitPromptContext): string {
  const stepsList = context.implementationSteps.length > 0
    ? context.implementationSteps.map((s, i) => `   ${i + 1}. ${s}`).join('\n')
    : `   ${t('prompts.plan.none')}`

  const testStepsList = context.testSteps.length > 0
    ? context.testSteps.map((s, i) => `   ${i + 1}. ${s}`).join('\n')
    : `   ${t('prompts.plan.none')}`

  const criteriaList = context.acceptanceCriteria && context.acceptanceCriteria.length > 0
    ? context.acceptanceCriteria.map((c, i) => `   ${i + 1}. ${c}`).join('\n')
    : `   ${t('prompts.plan.none')}`

  const userPromptSection = context.userPrompt
    ? `\n\n${t('prompts.plan.extraRequirements')}:\n${context.userPrompt}`
    : ''

  return `${t('prompts.plan.resplitIntro', { minTaskCount: context.minTaskCount })}

${t('prompts.plan.plan')}: ${context.planName}
${t('prompts.plan.task')}: ${context.taskTitle}
${t('prompts.plan.description')}: ${context.taskDescription?.trim() || t('prompts.plan.none')}

${t('prompts.plan.implementationSteps')}:
${stepsList}

${t('prompts.plan.testSteps')}:
${testStepsList}

${t('prompts.plan.acceptanceCriteria')}:
${criteriaList}${userPromptSection}

${t('prompts.plan.directTaskSplitDone')}`.trim()
}

function formatTaskList(tasks: AITaskItem[]): string {
  if (tasks.length === 0) {
    return t('prompts.plan.none')
  }

  return tasks.map((task, index) => [
    `${index + 1}. ${task.title}`,
    `   ${t('prompts.plan.description')}: ${task.description?.trim() || t('prompts.plan.none')}`,
    `   ${t('prompts.plan.implementationSteps')}:`,
    ...(task.implementationSteps.length > 0
      ? task.implementationSteps.map((step, stepIndex) => `     ${stepIndex + 1}. ${step}`)
      : [`     ${t('prompts.plan.none')}`]),
    `   ${t('prompts.plan.testSteps')}:`,
    ...(task.testSteps.length > 0
      ? task.testSteps.map((step, stepIndex) => `     ${stepIndex + 1}. ${step}`)
      : [`     ${t('prompts.plan.none')}`]),
    `   ${t('prompts.plan.acceptanceCriteria')}:`,
    ...(task.acceptanceCriteria.length > 0
      ? task.acceptanceCriteria.map((criteria, criteriaIndex) => `     ${criteriaIndex + 1}. ${criteria}`)
      : [`     ${t('prompts.plan.none')}`]),
    `   ${t('prompts.plan.dependsOnDescription')}: ${task.dependsOn?.join('、') || t('prompts.plan.none')}`
  ].join('\n')).join('\n\n')
}

export function buildTaskListOptimizeKickoffPrompt(context: TaskListOptimizePromptContext): string {
  const userPromptSection = context.userPrompt
    ? `\n\n${t('prompts.plan.extraRequirements')}:\n${context.userPrompt}`
    : ''
  const taskCountMode = context.taskCountMode ?? 'exact'
  const minTaskCount = Math.max(1, context.minTaskCount ?? 1)

  if (
    taskCountMode === 'exact'
    && context.targetIndex !== undefined
    && context.targetIndex >= 0
    && context.targetIndex < context.tasks.length
  ) {
    const targetIndex = context.targetIndex
    const targetTask = context.tasks[targetIndex]
    const otherTasksJson = context.tasks.map((task, i) => {
      if (i === targetIndex) return null
      return {
        title: task.title,
        description: task.description || '',
        priority: task.priority,
        expertId: task.expertId || '',
        implementationSteps: task.implementationSteps || [],
        testSteps: task.testSteps || [],
        acceptanceCriteria: task.acceptanceCriteria || [],
        dependsOn: task.dependsOn || []
      }
    })
    const targetTaskJson = {
      title: targetTask.title,
      description: targetTask.description || '',
      priority: targetTask.priority,
      expertId: targetTask.expertId || '',
      implementationSteps: targetTask.implementationSteps || [],
      testSteps: targetTask.testSteps || [],
      acceptanceCriteria: targetTask.acceptanceCriteria || [],
      dependsOn: targetTask.dependsOn || []
    }
    return [
      `你只需要优化任务 ${targetIndex + 1}《${targetTask.title}》，其余任务必须逐字原样输出，不可做任何修改。`,
      '如果用户没有明确要求整体优化或全量优化，你必须把这次请求理解为单任务局部修改，而不是整表优化。',
      '',
      '规则：',
      `1. 任务总数必须严格等于 ${context.tasks.length}，不可增删。`,
      `2. 仅允许修改任务 ${targetIndex + 1} 的字段内容。`,
      `3. 其余任务（索引 0-${context.tasks.length - 1} 中排除 ${targetIndex} 的部分）的所有字段必须与下方「其余任务原始数据」完全一致，禁止改写、润色或重排。`,
      '4. 对于目标任务，你可以重写标题、描述、实现步骤、测试步骤、验收标准，调整优先级。',
      '',
      `${t('prompts.plan.plan')}: ${context.planName}`,
      `${t('prompts.plan.description')}: ${context.planDescription?.trim() || t('prompts.plan.none')}`,
      '',
      `--- 目标任务（任务 ${targetIndex + 1}，可修改） ---`,
      JSON.stringify(targetTaskJson, null, 2),
      '',
      '--- 其余任务原始数据（必须原样输出，禁止修改） ---',
      JSON.stringify(otherTasksJson),
      userPromptSection,
      '',
      `输出 task_split（status=DONE），tasks 数组长度必须严格等于 ${context.tasks.length}，其中任务 ${targetIndex + 1} 是优化后的版本，其余任务与上方「其余任务原始数据」逐字段一致。`
    ].join('\n').trim()
  }

  if (taskCountMode === 'min') {
    return [
      '你将根据用户的最新要求重新整理当前任务列表。',
      `要求：可以新增、删除、合并、拆分或重排任务，但返回的 tasks 数量至少为 ${minTaskCount}。`,
      '允许：重写任务标题、描述、实现步骤、测试步骤、验收标准、优先级与 dependsOn 依赖关系。',
      '目标：输出一份更合理、可执行、可验证的最新任务列表，而不是机械保留旧列表。',
      '',
      `${t('prompts.plan.plan')}: ${context.planName}`,
      `${t('prompts.plan.description')}: ${context.planDescription?.trim() || t('prompts.plan.none')}`,
      '',
      '当前任务列表：',
      formatTaskList(context.tasks),
      userPromptSection,
      '',
      `输出 task_split（status=DONE），并保证 tasks 数量不少于 ${minTaskCount}。`
    ].join('\n').trim()
  }

  return [
    '你将对当前整份任务列表进行整体优化。',
    `要求：必须保留任务数量不变，当前任务数为 ${context.tasks.length}。`,
    '允许：重写任务描述、实现步骤、测试步骤、验收标准；调整任务顺序；修正 dependsOn 依赖关系。',
    '禁止：新增任务、删除任务，或返回与当前任务数不一致的结果。',
    '',
    `${t('prompts.plan.plan')}: ${context.planName}`,
    `${t('prompts.plan.description')}: ${context.planDescription?.trim() || t('prompts.plan.none')}`,
    '',
    '当前任务列表：',
    formatTaskList(context.tasks),
    userPromptSection,
    '',
    `输出 task_split（status=DONE），并保证 tasks 数量必须严格等于 ${context.tasks.length}。`
  ].join('\n').trim()
}

export function buildFormResponsePrompt(formId: string, values: Record<string, unknown>): string {
  const valueStr = Object.entries(values)
    .map(([key, val]) => `${key}: ${typeof val === 'object' ? JSON.stringify(val) : val}`)
    .join(', ')

  return [
    t('prompts.plan.formResponse', { formId, valueStr }),
    '',
    t('prompts.plan.formResponseContinue')
  ].join('\n').trim()
}

export function buildOutputCorrectionPrompt(minTaskCount: number): string {
  return t('prompts.plan.outputCorrection', { minTaskCount })
}

type PlanSplitSchemaProvider = 'claude' | 'codex' | 'generic'

function buildPlanSplitFieldSchema() {
  return {
    type: 'object',
    required: ['name', 'label', 'type'],
    properties: {
      name: { type: 'string', minLength: 1 },
      label: { type: 'string', minLength: 1 },
      type: {
        type: 'string',
        enum: ['text', 'textarea', 'select', 'multiselect', 'number', 'checkbox', 'radio', 'date', 'slider']
      },
      required: { type: 'boolean' },
      placeholder: { type: 'string' },
      suggestion: {
        anyOf: [
          { type: 'string' },
          { type: 'number' },
          { type: 'boolean' },
          {
            type: 'array',
            items: {
              anyOf: [
                { type: 'string' },
                { type: 'number' },
                { type: 'boolean' }
              ]
            }
          }
        ]
      },
      suggestionReason: { type: 'string' },
      optionReasons: {
        type: 'object',
        additionalProperties: { type: 'string' }
      },
      options: {
        type: 'array',
        items: {
          type: 'object',
          required: ['label', 'value'],
          properties: {
            label: { type: 'string' },
            value: { type: 'string', minLength: 1 }
          },
          additionalProperties: false
        }
      },
      validation: {
        type: 'object',
        properties: {
          min: { type: 'number' },
          max: { type: 'number' },
          pattern: { type: 'string' },
          message: { type: 'string' }
        },
        additionalProperties: false
      },
      allowOther: { type: 'boolean' },
      otherLabel: { type: 'string' }
    },
    additionalProperties: false
  }
}

function buildPlanSplitFormSchema() {
  return {
    type: 'object',
    required: ['formId', 'title', 'fields'],
    properties: {
      formId: { type: 'string', minLength: 1 },
      title: { type: 'string', minLength: 1 },
      description: { type: 'string' },
      submitText: { type: 'string' },
      fields: {
        type: 'array',
        minItems: 1,
        items: buildPlanSplitFieldSchema()
      }
    },
    additionalProperties: false
  }
}

function buildPlanSplitTaskSchema() {
  return {
    type: 'object',
    required: ['title', 'description', 'priority', 'expertId', 'implementationSteps', 'testSteps', 'acceptanceCriteria'],
    properties: {
      title: { type: 'string', minLength: 1 },
      description: { type: 'string', minLength: 1 },
      priority: { type: 'string', enum: ['high', 'medium', 'low'] },
      expertId: { type: 'string', minLength: 1 },
      implementationSteps: {
        type: 'array',
        minItems: 1,
        items: { type: 'string', minLength: 1 }
      },
      testSteps: {
        type: 'array',
        minItems: 1,
        items: { type: 'string', minLength: 1 }
      },
      acceptanceCriteria: {
        type: 'array',
        minItems: 1,
        items: { type: 'string', minLength: 1 }
      },
      dependsOn: {
        type: 'array',
        description: t('prompts.plan.dependsOnDescription'),
        items: { type: 'string' }
      }
    },
    additionalProperties: false
  }
}

function buildCodexPlanSplitFieldSchema() {
  return {
    type: 'object',
    required: [
      'name',
      'label',
      'type',
      'required',
      'placeholder',
      'suggestion',
      'suggestionReason',
      'optionReasons',
      'options',
      'allowOther',
      'otherLabel'
    ],
    properties: {
      name: { type: 'string', minLength: 1 },
      label: { type: 'string', minLength: 1 },
      type: {
        type: 'string',
        enum: ['text', 'textarea', 'select', 'multiselect', 'number', 'checkbox', 'radio', 'date', 'slider']
      },
      required: { type: 'boolean' },
      placeholder: { type: ['string', 'null'] },
      suggestion: {
        anyOf: [
          { type: 'string' },
          { type: 'number' },
          { type: 'boolean' },
          {
            type: 'array',
            items: {
              anyOf: [
                { type: 'string' },
                { type: 'number' },
                { type: 'boolean' }
              ]
            }
          },
          { type: 'null' }
        ]
      },
      suggestionReason: { type: ['string', 'null'] },
      optionReasons: {
        anyOf: [
          {
            type: 'object',
            additionalProperties: { type: 'string' }
          },
          { type: 'null' }
        ]
      },
      options: {
        type: 'array',
        items: {
          type: 'object',
          required: ['label', 'value'],
          properties: {
            label: { type: 'string', minLength: 1 },
            value: { type: 'string', minLength: 1 }
          },
          additionalProperties: false
        }
      },
      allowOther: { type: 'boolean' },
      otherLabel: { type: ['string', 'null'] }
    },
    additionalProperties: false
  }
}

function buildCodexPlanSplitFormSchema() {
  return {
    type: 'object',
    required: ['formId', 'title', 'description', 'submitText', 'fields'],
    properties: {
      formId: { type: 'string', minLength: 1 },
      title: { type: 'string', minLength: 1 },
      description: { type: ['string', 'null'] },
      submitText: { type: ['string', 'null'] },
      fields: {
        type: 'array',
        minItems: 1,
        items: buildCodexPlanSplitFieldSchema()
      }
    },
    additionalProperties: false
  }
}

function buildCodexPlanSplitTaskSchema() {
  return {
    type: 'object',
    required: ['title', 'description', 'priority', 'expertId', 'implementationSteps', 'testSteps', 'acceptanceCriteria', 'dependsOn'],
    properties: {
      title: { type: 'string', minLength: 1 },
      description: { type: 'string', minLength: 1 },
      priority: { type: 'string', enum: ['high', 'medium', 'low'] },
      expertId: { type: 'string', minLength: 1 },
      implementationSteps: {
        type: 'array',
        minItems: 1,
        items: { type: 'string', minLength: 1 }
      },
      testSteps: {
        type: 'array',
        minItems: 1,
        items: { type: 'string', minLength: 1 }
      },
      acceptanceCriteria: {
        type: 'array',
        minItems: 1,
        items: { type: 'string', minLength: 1 }
      },
      dependsOn: {
        type: 'array',
        items: { type: 'string' }
      }
    },
    additionalProperties: false
  }
}

function buildCodexPlanSplitJsonSchema(minTaskCount: number, taskCountMode: TaskCountMode) {
  const normalizedMinTaskCount = Math.max(1, Math.floor(minTaskCount || 1))
  const exactTaskCount = taskCountMode === 'exact' ? normalizedMinTaskCount : undefined

  // Codex 的 response_format schema 不支持 allOf/if/then，使用扁平结构，
  // 同时把可选字段显式列出为 null，减少模型漏字段导致前端表单渲染不稳定。
  return {
    type: 'object',
    required: ['type', 'question', 'forms', 'formSchema', 'status', 'summary', 'tasks'],
    properties: {
      type: { type: 'string', enum: ['form_request', 'task_split'] },
      question: { type: ['string', 'null'] },
      forms: {
        type: ['array', 'null'],
        items: buildCodexPlanSplitFormSchema()
      },
      formSchema: {
        anyOf: [
          buildCodexPlanSplitFormSchema(),
          { type: 'null' }
        ]
      },
      status: {
        anyOf: [
          { type: 'string', enum: ['DONE'] },
          { type: 'null' }
        ]
      },
      summary: { type: ['string', 'null'] },
      tasks: {
        type: ['array', 'null'],
        minItems: normalizedMinTaskCount,
        ...(exactTaskCount ? { maxItems: exactTaskCount } : {}),
        items: buildCodexPlanSplitTaskSchema()
      }
    },
    additionalProperties: false
  }
}

function buildClaudePlanSplitJsonSchema(minTaskCount: number, taskCountMode: TaskCountMode) {
  const normalizedMinTaskCount = Math.max(1, Math.floor(minTaskCount || 1))
  const exactTaskCount = taskCountMode === 'exact' ? normalizedMinTaskCount : undefined

  return {
    type: 'object',
    required: ['type'],
    properties: {
      type: { type: 'string', enum: ['form_request', 'task_split'] },
      question: { type: 'string' },
      formSchema: { type: 'object' },
      status: { type: 'string', enum: ['DONE'] },
      summary: { type: 'string' },
      tasks: {
        type: 'array',
        items: { type: 'object' }
      }
    },
    additionalProperties: false,
    allOf: [
      {
        if: {
          type: 'object',
          properties: {
            type: { const: 'form_request' }
          },
          required: ['type']
        },
        then: {
          required: ['type'],
          properties: {
            type: { const: 'form_request' },
            question: { type: 'string' },
            forms: {
              type: 'array',
              minItems: 1,
              items: buildPlanSplitFormSchema()
            },
            formSchema: buildPlanSplitFormSchema()
          }
        }
      },
      {
        if: {
          type: 'object',
          properties: {
            type: { const: 'task_split' }
          },
          required: ['type']
        },
        then: {
          required: ['type', 'status', 'summary', 'tasks'],
          properties: {
            type: { const: 'task_split' },
            status: { const: 'DONE' },
            summary: { type: 'string', minLength: 1 },
            tasks: {
              type: 'array',
              minItems: normalizedMinTaskCount,
              ...(exactTaskCount ? { maxItems: exactTaskCount } : {}),
              items: buildPlanSplitTaskSchema()
            }
          }
        }
      }
    ]
  }
}

export function buildPlanSplitJsonSchema(
  minTaskCount: number,
  provider: PlanSplitSchemaProvider = 'generic',
  taskCountMode: TaskCountMode = 'min'
): string {
  const normalizedProvider = provider.toLowerCase() as PlanSplitSchemaProvider
  const schema = normalizedProvider === 'codex'
    ? buildCodexPlanSplitJsonSchema(minTaskCount, taskCountMode)
    : buildClaudePlanSplitJsonSchema(minTaskCount, taskCountMode)

  return JSON.stringify(schema)
}
