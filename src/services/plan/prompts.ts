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

export function buildPlanSplitSystemPrompt(): string {
  return `你是项目规划助手，目标是把需求拆成可执行任务。

规则：
1. 只输出单个 JSON 对象，不要 markdown 或解释。
2. 信息不足输出 form_request；信息足够输出 task_split。
3. form_request 优先输出 forms 数组；字段 type 只能是 text、textarea、select、multiselect、number、checkbox、radio、date、slider。
4. select / radio / multiselect 的 options 必须是 [{ "label": "...", "value": "..." }]，并保留 allowOther；可补充 suggestion、suggestionReason、optionReasons。
5. 条件显示仅使用 condition: { field, value }。
6. task_split 必须包含 status:"DONE"、tasks、dependsOn；每个任务都要有 title、description、priority、implementationSteps、testSteps、acceptanceCriteria。
7. 任务要边界清晰，可直接执行。`.trim()
}

export function buildPlanSplitKickoffPrompt(context: PlanSplitPromptContext): string {
  return `计划名称: ${context.planName}
计划描述: ${context.planDescription?.trim() || '（无）'}
最少任务数: ${context.minTaskCount}

开始拆分：信息不足就输出 form_request，信息足够就直接输出 task_split。`.trim()
}

export function buildTaskResplitKickoffPrompt(context: TaskResplitPromptContext): string {
  const stepsList = context.implementationSteps.length > 0
    ? context.implementationSteps.map((s, i) => `   ${i + 1}. ${s}`).join('\n')
    : '   （无）'

  const testStepsList = context.testSteps.length > 0
    ? context.testSteps.map((s, i) => `   ${i + 1}. ${s}`).join('\n')
    : '   （无）'

  const criteriaList = context.acceptanceCriteria && context.acceptanceCriteria.length > 0
    ? context.acceptanceCriteria.map((c, i) => `   ${i + 1}. ${c}`).join('\n')
    : '   （无）'

  const userPromptSection = context.userPrompt
    ? `\n\n用户额外要求:\n${context.userPrompt}`
    : ''

  return `将以下任务继续拆分为至少 ${context.minTaskCount} 个子任务：

计划: ${context.planName}
任务: ${context.taskTitle}
描述: ${context.taskDescription?.trim() || '（无）'}

实现步骤:
${stepsList}

测试步骤:
${testStepsList}

验收标准:
${criteriaList}${userPromptSection}

直接输出 task_split（status=DONE）。`.trim()
}

export function buildFormResponsePrompt(formId: string, values: Record<string, unknown>): string {
  const valueStr = Object.entries(values)
    .map(([key, val]) => `${key}: ${typeof val === 'object' ? JSON.stringify(val) : val}`)
    .join(', ')

  return `表单 ${formId} 回答: ${valueStr}

继续：需要更多信息就输出 form_request；足够则输出 task_split（status=DONE）。`.trim()
}

export function buildOutputCorrectionPrompt(minTaskCount: number): string {
  return `输出格式错误，请重新输出：
- form_request：必须输出 forms 数组（兼容单个 formSchema 但优先 forms）
- task_split：必须含 status:DONE，tasks >= ${minTaskCount}
- 禁止 markdown 代码块和额外文字`
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
    required: ['title', 'description', 'priority', 'implementationSteps', 'testSteps', 'acceptanceCriteria'],
    properties: {
      title: { type: 'string', minLength: 1 },
      description: { type: 'string', minLength: 1 },
      priority: { type: 'string', enum: ['high', 'medium', 'low'] },
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
        description: '依赖的任务标题列表（必须先完成的任务）',
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
    required: ['title', 'description', 'priority', 'implementationSteps', 'testSteps', 'acceptanceCriteria', 'dependsOn'],
    properties: {
      title: { type: 'string', minLength: 1 },
      description: { type: 'string', minLength: 1 },
      priority: { type: 'string', enum: ['high', 'medium', 'low'] },
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

function buildCodexPlanSplitJsonSchema(minTaskCount: number) {
  const normalizedMinTaskCount = Math.max(1, Math.floor(minTaskCount || 1))

  // Codex 的 response_format schema 不支持 allOf/if/then，使用扁平结构，
  // 同时把可选字段显式列出为 null，减少模型漏字段导致前端表单渲染不稳定。
  return {
    type: 'object',
    required: ['type', 'question', 'forms', 'formSchema', 'status', 'tasks'],
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
      tasks: {
        type: ['array', 'null'],
        minItems: normalizedMinTaskCount,
        items: buildCodexPlanSplitTaskSchema()
      }
    },
    additionalProperties: false
  }
}

function buildClaudePlanSplitJsonSchema(minTaskCount: number) {
  const normalizedMinTaskCount = Math.max(1, Math.floor(minTaskCount || 1))

  return {
    type: 'object',
    required: ['type'],
    properties: {
      type: { type: 'string', enum: ['form_request', 'task_split'] },
      question: { type: 'string' },
      formSchema: { type: 'object' },
      status: { type: 'string', enum: ['DONE'] },
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
          required: ['type', 'status', 'tasks'],
          properties: {
            type: { const: 'task_split' },
            status: { const: 'DONE' },
            tasks: {
              type: 'array',
              minItems: normalizedMinTaskCount,
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
  provider: PlanSplitSchemaProvider = 'generic'
): string {
  const normalizedProvider = provider.toLowerCase() as PlanSplitSchemaProvider
  const schema = normalizedProvider === 'codex'
    ? buildCodexPlanSplitJsonSchema(minTaskCount)
    : buildClaudePlanSplitJsonSchema(minTaskCount)

  return JSON.stringify(schema)
}
