import type { TaskPriority, TaskStatus } from '@/types/plan'

export type UnattendedIntentType =
  | 'chat'
  | 'list_projects'
  | 'switch_project'
  | 'switch_agent'
  | 'switch_model'
  | 'create_plan'
  | 'query_plan_progress'
  | 'query_task_status'
  | 'query_execution'
  | 'create_task'
  | 'update_task'
  | 'stop_task'
  | 'start_task'
  | 'start_plan'
  | 'pause_plan'
  | 'resume_plan'
  | 'start_split'
  | 'continue_split'
  | 'form_response'

export interface UnattendedIntent {
  type: UnattendedIntentType
  targetName?: string
  projectHint?: string
  agentHint?: string
  modelHint?: string
  planName?: string
  taskHint?: string
  executeAfterCreate?: boolean
  rawText: string
}

export interface UnattendedTaskCreateDraft {
  title?: string
  planHint?: string
  priority?: TaskPriority
}

export interface UnattendedTaskUpdateDraft {
  targetHint?: string
  title?: string
  description?: string
  priority?: TaskPriority
  status?: TaskStatus
}

export interface UnattendedTaskExecutionDraft {
  targetHint?: string
  planHint?: string
}

function normalize(text: string): string {
  return text.trim().replace(/\s+/g, ' ')
}

function extractPlanHint(text: string): string | undefined {
  const matched = text.match(/(?:在|到|往)(.+?)(?:计划|plan)(?:里|中|下面|下)?/iu)
  return matched?.[1]?.trim()
}

function extractTaskPriority(text: string): TaskPriority | undefined {
  if (/(高优先级|优先级高|highest|high priority)/iu.test(text)) {
    return 'high'
  }
  if (/(低优先级|优先级低|lowest|low priority)/iu.test(text)) {
    return 'low'
  }
  if (/(中优先级|优先级中|medium priority)/iu.test(text)) {
    return 'medium'
  }
  return undefined
}

function extractTaskStatus(text: string): TaskStatus | undefined {
  if (/(已完成|完成了|标记完成|设为完成|completed?)/iu.test(text)) {
    return 'completed'
  }
  if (/(进行中|执行中|开发中|开始做|started?|in progress)/iu.test(text)) {
    return 'in_progress'
  }
  if (/(待办|未开始|pending|todo)/iu.test(text)) {
    return 'pending'
  }
  if (/(阻塞|卡住|blocked)/iu.test(text)) {
    return 'blocked'
  }
  if (/(失败|报错|failed?)/iu.test(text)) {
    return 'failed'
  }
  if (/(取消|停止|cancelled?|canceled)/iu.test(text)) {
    return 'cancelled'
  }
  return undefined
}

function extractQuotedSegment(text: string): string | undefined {
  const matched = text.match(/[“"'《](.+?)[”"'》]/u)
  return matched?.[1]?.trim()
}

export function detectUnattendedIntent(text: string): UnattendedIntent {
  const rawText = normalize(text)
  const lowered = rawText.toLowerCase()
  const rawWithoutPlanPrefix = rawText.replace(/(?:在|到|往).+?(?:计划|plan)(?:里|中|下面|下)?/iu, '').trim()

  if (/[:：]/.test(rawText) && /[\n\r]/.test(rawText)) {
    return { type: 'form_response', rawText }
  }

  if (/当前工作区.*项目|导入了哪些项目|有哪些项目|项目列表|workspace project/iu.test(rawText)) {
    return {
      type: 'list_projects',
      rawText
    }
  }

  if (/切换.*项目|切到.*项目|使用.*项目|换成.*项目|当前项目切换到|项目切换到/.test(rawText) || lowered.includes('switch project')) {
    return {
      type: 'switch_project',
      projectHint: rawText,
      rawText
    }
  }

  if (/切换.*agent|切到.*agent|改用|使用.*agent|换成/.test(rawText) || lowered.includes('use agent')) {
    return {
      type: 'switch_agent',
      agentHint: rawText,
      rawText
    }
  }

  if (/切换.*模型|切到.*模型|使用.*模型|换成.*模型|改用.*模型/.test(rawText) || lowered.includes('switch model')) {
    return {
      type: 'switch_model',
      modelHint: rawText,
      rawText
    }
  }

  if (/创建.*计划|新建.*计划|生成.*计划/.test(rawText) || lowered.includes('create plan')) {
    const planName = rawText
      .replace(/^(请)?(帮我)?(创建|新建|生成)(一个)?/u, '')
      .replace(/(并)?(开始|启动|执行|跑起来|执行起来).*$/u, '')
      .replace(/^计划/u, '')
      .trim()

    return {
      type: 'create_plan',
      rawText,
      planName: planName || rawText,
      executeAfterCreate: /并.*(开始|启动|执行|跑起来)|创建.*计划.*(开始|启动|执行)/.test(rawText)
    }
  }

  if (/创建.*任务|新建.*任务|新增.*任务|加一个任务|加个任务/.test(rawText) || lowered.includes('create task')) {
    return {
      type: 'create_task',
      taskHint: rawText,
      planName: extractPlanHint(rawText),
      rawText
    }
  }

  if (/停止.*任务|停掉.*任务|中止.*任务|取消.*任务/.test(rawText) || lowered.includes('stop task')) {
    return {
      type: 'stop_task',
      taskHint: rawText,
      rawText
    }
  }

  if (/修改.*任务|更新.*任务|编辑.*任务|把任务.*改|任务.*改成/.test(rawText) || lowered.includes('update task')) {
    return {
      type: 'update_task',
      taskHint: rawText,
      rawText
    }
  }

  if (
    /^(请)?(帮我)?(开始|执行|启动|跑(?:一下)?|继续执行).*(任务)/u.test(rawText)
    || /^(请)?(帮我)?(开始|执行|启动|跑(?:一下)?|继续执行).*(任务)/u.test(rawWithoutPlanPrefix)
    || lowered.includes('start task')
  ) {
    return {
      type: 'start_task',
      taskHint: rawText,
      planName: extractPlanHint(rawText),
      rawText
    }
  }

  if (/继续拆分|继续计划拆分|继续上次拆分/.test(rawText)) {
    return { type: 'continue_split', rawText, targetName: rawText }
  }

  if (/开始拆分|创建拆分|拆分计划/.test(rawText)) {
    return { type: 'start_split', rawText, targetName: rawText }
  }

  if (/暂停.*计划|暂停执行/.test(rawText)) {
    return { type: 'pause_plan', rawText, targetName: rawText }
  }

  if (/恢复.*计划|继续执行计划|恢复执行/.test(rawText)) {
    return { type: 'resume_plan', rawText, targetName: rawText }
  }

  if (/开始.*计划|执行.*计划|跑一下.*计划|启动.*计划/.test(rawText)) {
    return { type: 'start_plan', rawText, targetName: rawText }
  }

  if (/任务状态|任务进度|哪个任务|任务执行/.test(rawText)) {
    return { type: 'query_task_status', rawText, targetName: rawText }
  }

  if (/当前执行|执行进度|进度怎么样|正在执行/.test(rawText)) {
    return { type: 'query_execution', rawText, targetName: rawText }
  }

  if (/计划进度|计划状态|有哪些计划|计划怎么样/.test(rawText)) {
    return { type: 'query_plan_progress', rawText, targetName: rawText }
  }

  return { type: 'chat', rawText }
}

export function extractTaskCreateDraft(text: string): UnattendedTaskCreateDraft {
  const rawText = normalize(text)
  const withoutPlanPrefix = rawText.replace(/(?:在|到|往).+?(?:计划|plan)(?:里|中|下面|下)?/iu, '').trim()
  const title = withoutPlanPrefix
    .replace(/^(请)?(帮我)?(创建|新建|新增|加一个|加个)/u, '')
    .replace(/^任务/u, '')
    .replace(/[，。,；;！!].*$/u, '')
    .trim()

  return {
    title: title || extractQuotedSegment(rawText),
    planHint: extractPlanHint(rawText),
    priority: extractTaskPriority(rawText)
  }
}

export function extractTaskUpdateDraft(text: string): UnattendedTaskUpdateDraft {
  const rawText = normalize(text)
  const quotedTarget = extractQuotedSegment(rawText)
  const titleMatched = rawText.match(/(?:把)?任务(.+?)(?:标题|名称)改成(.+)$/u)
  const descriptionMatched = rawText.match(/任务(.+?)(?:说明|描述)改成(.+)$/u)

  return {
    targetHint: quotedTarget
      || descriptionMatched?.[1]?.trim()
      || titleMatched?.[1]?.trim()
      || rawText.match(/任务(.+?)(?:状态|优先级|标题|名称|说明|描述|改成|改为|设为)/u)?.[1]?.trim(),
    title: titleMatched?.[2]?.trim(),
    description: descriptionMatched?.[2]?.trim(),
    priority: extractTaskPriority(rawText),
    status: extractTaskStatus(rawText)
  }
}

export function extractTaskExecutionDraft(text: string): UnattendedTaskExecutionDraft {
  const rawText = normalize(text)
  const withoutPlanPrefix = rawText.replace(/(?:在|到|往).+?(?:计划|plan)(?:里|中|下面|下)?/iu, '').trim()
  const targetHint = withoutPlanPrefix
    .replace(/^(请)?(帮我)?(开始|执行|启动|继续执行|跑(?:一下)?)(?:一下)?/u, '')
    .replace(/^任务/u, '')
    .replace(/[，。,；;！!].*$/u, '')
    .trim()

  return {
    targetHint: targetHint || extractQuotedSegment(rawText),
    planHint: extractPlanHint(rawText)
  }
}

export function parseStructuredFormResponse(text: string): Record<string, string> {
  return text
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter(Boolean)
    .reduce<Record<string, string>>((acc, line) => {
      const separatorIndex = line.indexOf(':') >= 0 ? line.indexOf(':') : line.indexOf('：')
      if (separatorIndex <= 0) {
        return acc
      }
      const key = line.slice(0, separatorIndex).trim()
      const value = line.slice(separatorIndex + 1).trim()
      if (key && value) {
        acc[key] = value
      }
      return acc
    }, {})
}
