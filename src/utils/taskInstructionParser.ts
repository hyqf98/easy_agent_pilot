import type { AITaskItem } from '@/types/plan'

/** 意图类型 */
export type TaskInstructionType =
  | 'delete'
  | 'update'
  | 'add'
  | 'resplit'
  | 'optimize'
  | 'reorder'
  | 'move_up'
  | 'move_down'
  | 'unknown'

/** 解析结果 */
export interface TaskInstructionResult {
  type: TaskInstructionType
  /** 0-based 任务索引 */
  targetIndex?: number
  /** 更新字段（update 类型使用） */
  updates?: Partial<AITaskItem>
  /** 自定义提示语（resplit / optimize 使用） */
  customPrompt?: string
}

const BRACKET_MENTION_PATTERN = /@\[(\d+)(?::[^\]]*)?\]/g

/** 将任务引用标准化为可解析的文本。 */
export function normalizeTaskInstructionInput(input: string): string {
  return input.replace(BRACKET_MENTION_PATTERN, (_matched, numeric) => `任务${numeric}`)
}

/** 将任务引用替换成明确标题，便于拼到 AI 提示词中。 */
export function materializeTaskMentions(input: string, tasks: AITaskItem[]): string {
  return input.replace(BRACKET_MENTION_PATTERN, (_matched, numeric) => {
    const index = Number.parseInt(numeric, 10) - 1
    const task = tasks[index]
    if (!task) {
      return `任务${numeric}`
    }
    return `任务${numeric}《${task.title || '未命名任务'}》`
  })
}

function extractFirstNumber(text: string, taskCount: number): number | null {
  const matched = text.match(/(?:任务\s*)?(\d+)/)
  return resolveByNumber(matched?.[1], taskCount)
}

function extractPromptSuffix(text: string): string | undefined {
  for (const separator of ['，', ',', '：', ':']) {
    const index = text.indexOf(separator)
    if (index >= 0 && index < text.length - 1) {
      const suffix = text.slice(index + 1).trim()
      return suffix || undefined
    }
  }
  return undefined
}

/**
 * 按数字序号解析目标索引（1-based → 0-based）
 */
function resolveByNumber(matched: string | undefined, taskCount: number): number | null {
  if (!matched) return null
  const num = parseInt(matched, 10)
  return typeof num === 'number' && isFinite(num) && num >= 1 && num <= taskCount ? num - 1 : null
}

/**
 * 按标题模糊匹配，返回相似度最高的索引
 */
function resolveByTitle(keyword: string, tasks: AITaskItem[]): number | null {
  let bestIdx: number | null = null
  let bestLen = 0
  for (let i = 0; i < tasks.length; i++) {
    const pos = tasks[i].title.indexOf(keyword)
    if (pos !== -1) {
      // 优先匹配长度更长的（更精确）
      if (keyword.length > bestLen) {
        bestIdx = i
        bestLen = keyword.length
      }
    }
  }
  return bestIdx
}

/**
 * 解析优先级字符串为 TaskPriority
 */
function parsePriority(raw: string): AITaskItem['priority'] {
  const map: Record<string, AITaskItem['priority']> = {
    '高': 'high', '低': 'low', '中': 'medium',
    'high': 'high', 'low': 'low', 'medium': 'medium'
  }
  return map[raw.toLowerCase()] || 'medium'
}

/**
 * 解析用户自然语言指令，返回结构化的操作意图
 *
 * @param input  用户输入文本
 * @param tasks  当前任务列表
 * @returns      解析结果
 */
export function parseTaskInstruction(input: string, tasks: AITaskItem[]): TaskInstructionResult {
  const text = normalizeTaskInstructionInput(input).trim()
  if (!text) return { type: 'unknown' }

  // --- 删除任务（按序号或标题） ---
  {
    const m = text.match(/(?:删除|移除|去掉)\s*(?:任务\s*)?(\d+)/)
    if (m) {
      const idx = resolveByNumber(m[1], tasks.length)
      if (idx !== null) return { type: 'delete', targetIndex: idx }
    }
  }
  {
    const m = text.match(/(?:删除|移除)\s*(?:任务\s*)?["'](.+?)["']/)
    if (m) {
      const idx = resolveByTitle(m[1], tasks)
      if (idx !== null) return { type: 'delete', targetIndex: idx }
    }
  }

  // --- 添加任务 ---
  if (/(?:添加|新增|增加)\s*(?:一个\s*)?(?:新\s*)?任务/.test(text)) {
    // 支持在添加时指定标题：添加任务 XXX
    const m = text.match(/(?:添加|新增|增加)\s*(?:一个\s*)?(?:新\s*)?任务[：:\s]*(.+)/)
    if (m && m[1].trim()) {
      return { type: 'add', updates: { title: m[1].trim(), description: '', priority: 'medium', implementationSteps: [], testSteps: [], acceptanceCriteria: [] } }
    }
    return { type: 'add' }
  }

  // --- 再次拆分任务 ---
  {
    if (/(?:再次?拆分|继续拆分|重新拆分|进一步拆分|拆细|细化)/.test(text)) {
      const idx = extractFirstNumber(text, tasks.length)
      if (idx !== null) {
        return {
          type: 'resplit',
          targetIndex: idx,
          customPrompt: extractPromptSuffix(text) || text
        }
      }
    }
  }

  // --- 修改任务标题 ---
  {
    const m = text.match(/(?:修改|更新|更改)\s*(?:任务\s*)?(\d+)\s*的?(?:标题|名称)\s*为\s*(.+)/)
    if (m) {
      const idx = resolveByNumber(m[1], tasks.length)
      if (idx !== null) return { type: 'update', targetIndex: idx, updates: { title: m[2].trim() } }
    }
  }

  // --- 修改任务描述 ---
  {
    const m = text.match(/(?:修改|更新|更改)\s*(?:任务\s*)?(\d+)\s*的?描述\s*为\s*(.+)/)
    if (m) {
      const idx = resolveByNumber(m[1], tasks.length)
      if (idx !== null) return { type: 'update', targetIndex: idx, updates: { description: m[2].trim() } }
    }
  }

  // --- 修改任务优先级 ---
  {
    const m = text.match(/(?:修改|更新|更改|把)\s*(?:任务\s*)?(\d+)\s*的?优先级?\s*为\s*(.+)/)
    if (m) {
      const idx = resolveByNumber(m[1], tasks.length)
      if (idx !== null) {
        return { type: 'update', targetIndex: idx, updates: { priority: parsePriority(m[2].trim()) } }
      }
    }
  }

  // --- 上移任务 ---
  {
    const m = text.match(/(?:上移|往前移)\s*(?:任务\s*)?(\d+)/)
    if (m) {
      const idx = resolveByNumber(m[1], tasks.length)
      if (idx !== null) return { type: 'move_up', targetIndex: idx }
    }
  }

  // --- 下移任务 ---
  {
    const m = text.match(/(?:下移|往后移)\s*(?:任务\s*)?(\d+)/)
    if (m) {
      const idx = resolveByNumber(m[1], tasks.length)
      if (idx !== null) return { type: 'move_down', targetIndex: idx }
    }
  }

  // --- 单任务优化（@某个具体任务 + 优化类关键词） ---
  {
    if (/(?:优化|改进|完善|调整|补充|细化|修改|改写)/.test(text)) {
      const idx = extractFirstNumber(text, tasks.length)
      if (idx !== null) {
        return {
          type: 'optimize',
          targetIndex: idx,
          customPrompt: text
        }
      }
    }
  }

  // --- 整体优化列表 ---
  if (/(?:优化|整理|改进|完善|调整|补充|重排|精简)/.test(text)) {
    const m = text.match(/(?:优化|整理|改进|完善|调整|补充|重排|精简)\s*(?:任务\s*)?(?:列表|清单|全部|整体|右侧任务)?[，,\s]*(.+)?/)
    return { type: 'optimize', customPrompt: m?.[1]?.trim() || text }
  }

  // --- 重新排序 ---
  if (/(?:重新排序|重排|排序)/.test(text)) {
    return { type: 'reorder' }
  }

  return { type: 'unknown' }
}
