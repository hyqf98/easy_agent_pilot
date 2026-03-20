/**
 * 计划模式相关类型定义
 */

// 计划状态
export type PlanStatus = 'draft' | 'planning' | 'ready' | 'executing' | 'completed' | 'paused'

// 计划执行状态
export type PlanExecutionStatus = 'idle' | 'running' | 'paused' | 'completed' | 'error'

// 计划调度状态
export type ScheduleStatus = 'none' | 'scheduled' | 'triggered' | 'cancelled'

// 计划拆分模式
export type PlanSplitMode = 'ai' | 'manual'
export type PlanSplitExecutionMode = 'single' | 'coordinator_subagents'

// 任务状态
export type TaskStatus = 'pending' | 'in_progress' | 'completed' | 'blocked' | 'failed' | 'cancelled'

// 任务阻塞原因
export type BlockReason = 'waiting_input'  // 预留扩展

// 任务优先级
export type TaskPriority = 'low' | 'medium' | 'high'

// 任务输入请求（等待用户输入时的表单请求）
export interface TaskInputRequest {
  formSchema: DynamicFormSchema
  question?: string
  requestedAt: string
}

// 智能体角色 - 只保留规划者，用于需求分析和任务拆分
export type AgentRole = 'planner'

// 计划接口
export interface Plan {
  id: string
  projectId: string
  name: string
  description?: string
  splitMode: PlanSplitMode    // 拆分模式: ai | manual
  splitExecutionMode?: PlanSplitExecutionMode
  splitTeamId?: string
  splitAgentId?: string
  splitModelId?: string
  status: PlanStatus
  executionStatus?: PlanExecutionStatus
  currentTaskId?: string
  agentTeam?: AgentRole[]
  granularity: number        // 任务拆分颗粒度(最小任务数)
  maxRetryCount: number      // 最大重试次数
  scheduledAt?: string       // 计划执行时间 (ISO 8601)
  scheduleStatus?: ScheduleStatus  // 调度状态
  createdAt: string
  updatedAt: string
}

// 任务接口
export interface Task {
  id: string
  planId: string
  parentId?: string
  title: string
  description?: string
  status: TaskStatus
  priority: TaskPriority
  assignee?: AgentRole
  /** 执行智能体 ID */
  agentId?: string
  /** 执行模型 ID */
  modelId?: string
  /** 推荐执行智能体 ID */
  recommendedAgentId?: string
  /** 推荐执行模型 ID */
  recommendedModelId?: string
  /** 推荐原因 */
  recommendationReason?: string
  sessionId?: string
  progressFile?: string
  dependencies?: string[]
  order: number
  retryCount: number
  maxRetries: number
  errorMessage?: string
  implementationSteps?: string[]
  testSteps?: string[]
  acceptanceCriteria?: string[]
  blockReason?: BlockReason         // 阻塞原因
  inputRequest?: TaskInputRequest   // 等待输入的表单请求
  inputResponse?: Record<string, any> // 用户提交的表单数据
  createdAt: string
  updatedAt: string
}

// 创建计划输入
export interface CreatePlanInput {
  projectId: string
  name: string
  description?: string
  splitMode?: PlanSplitMode    // 拆分模式: ai | manual
  splitExecutionMode?: PlanSplitExecutionMode
  splitTeamId?: string
  splitAgentId?: string
  splitModelId?: string
  agentTeam?: AgentRole[]
  granularity?: number
  maxRetryCount?: number
  scheduledAt?: string
}

// 更新计划输入
export interface UpdatePlanInput {
  name?: string
  description?: string
  splitMode?: PlanSplitMode    // 拆分模式: ai | manual
  splitExecutionMode?: PlanSplitExecutionMode
  splitTeamId?: string
  splitAgentId?: string
  splitModelId?: string
  status?: PlanStatus
  executionStatus?: PlanExecutionStatus
  currentTaskId?: string
  agentTeam?: AgentRole[]
  granularity?: number
  maxRetryCount?: number
  scheduledAt?: string
  scheduleStatus?: ScheduleStatus
}

// 创建任务输入
export interface CreateTaskInput {
  planId: string
  parentId?: string
  title: string
  description?: string
  priority?: TaskPriority
  assignee?: AgentRole
  /** 执行智能体 ID */
  agentId?: string
  /** 执行模型 ID */
  modelId?: string
  recommendedAgentId?: string
  recommendedModelId?: string
  recommendationReason?: string
  dependencies?: string[]
  order?: number
  maxRetries?: number
  implementationSteps?: string[]
  testSteps?: string[]
  acceptanceCriteria?: string[]
}

// 更新任务输入
export interface UpdateTaskInput {
  title?: string
  description?: string
  status?: TaskStatus
  priority?: TaskPriority
  assignee?: AgentRole
  /** 执行智能体 ID */
  agentId?: string
  /** 执行模型 ID */
  modelId?: string
  recommendedAgentId?: string
  recommendedModelId?: string
  recommendationReason?: string
  sessionId?: string
  progressFile?: string
  dependencies?: string[]
  order?: number
  retryCount?: number
  maxRetries?: number
  errorMessage?: string
  implementationSteps?: string[]
  testSteps?: string[]
  acceptanceCriteria?: string[]
  blockReason?: BlockReason
  inputRequest?: TaskInputRequest
  inputResponse?: Record<string, any>
}

// 任务顺序项
export interface TaskOrderItem {
  id: string
  order: number
}

// 批量更新任务顺序输入
export interface ReorderTasksInput {
  taskOrders: TaskOrderItem[]
}

// ==================== 动态表单相关类型 ====================

// 表单字段类型
export type FieldType =
  | 'text'
  | 'textarea'
  | 'select'
  | 'multiselect'
  | 'number'
  | 'checkbox'
  | 'radio'
  | 'date'
  | 'file'
  | 'code'
  | 'slider'

// 表单字段选项
export interface FormFieldOption {
  label: string
  value: any
}

// 字段验证规则
export interface FieldValidation {
  min?: number
  max?: number
  pattern?: string
  message?: string
}

// 条件显示配置
export interface FieldCondition {
  field: string
  value: any
}

// 表单字段定义
export interface FormField {
  name: string
  label: string
  type: FieldType
  placeholder?: string
  required?: boolean
  default?: any
  suggestion?: any
  suggestionReason?: string
  optionReasons?: Record<string, string>
  options?: FormFieldOption[]
  validation?: FieldValidation
  condition?: FieldCondition
  allowOther?: boolean  // 是否允许用户输入"其他"选项
  otherLabel?: string   // "其他"选项的标签，默认为"其他"
}

// 动态表单 Schema
export interface DynamicFormSchema {
  formId: string
  title: string
  description?: string
  fields: FormField[]
  submitText?: string
}

// 表单模板
export interface FormTemplate {
  id: string
  name: string
  description: string
  category: 'requirement' | 'config' | 'review' | 'deploy'
  schema: DynamicFormSchema
}

// AI 输出的表单请求
export interface FormRequest {
  type: 'form_request'
  mode: 'schema' | 'template'
  schema?: DynamicFormSchema
  templateId?: string
  defaultValues?: Record<string, any>
}

// 用户提交的表单响应
export interface FormResponse {
  type: 'form_response'
  formId: string
  values: Record<string, any>
}

// ==================== 进度文件相关类型 ====================

// 进度文件内容
export interface ProgressFile {
  planId: string
  taskId: string
  status: TaskStatus
  summary: string
  lastUpdated: string
  artifacts: string[]
  notes: string
}

// ==================== 智能体角色配置 ====================

// 智能体角色配置
export interface AgentRoleConfig {
  role: AgentRole
  name: string
  description: string
  systemPrompt: string
  capabilities: string[]
  triggers: string[]
}

// 预定义的智能体角色配置 - 只保留规划者
export const AGENT_ROLES: AgentRoleConfig[] = [
  {
    role: 'planner',
    name: '规划者',
    description: '负责需求分析、任务拆分、规划执行顺序',
    systemPrompt: `你是项目规划师，负责分析需求并拆分为可执行任务。

核心职责：
- 分析用户需求
- 拆分复杂需求为子任务
- 确定任务依赖和执行顺序
- 使用动态表单收集信息

禁止执行具体编码任务。`,
    capabilities: ['analyze', 'plan', 'decompose', 'form'],
    triggers: ['规划', '拆分', '计划', '需求']
  }
]

// 获取角色配置
export function getAgentRoleConfig(role: AgentRole): AgentRoleConfig | undefined {
  return AGENT_ROLES.find(r => r.role === role)
}

// ==================== AI 任务拆分相关类型 ====================

// AI 输出类型
export type AIOutputType = 'form_request' | 'task_split'

// AI 表单请求
export interface AIFormRequest {
  type: 'form_request'
  question: string
  forms?: DynamicFormSchema[]
  formSchema?: DynamicFormSchema
}

// AI 任务拆分结果
export interface AITaskSplitResult {
  type: 'task_split'
  tasks: AITaskItem[]
}

// AI 任务项
export interface AITaskItem {
  title: string
  description: string
  priority: TaskPriority
  /** 执行智能体 ID */
  agentId?: string
  /** 执行模型 ID */
  modelId?: string
  recommendedAgentId?: string
  recommendedModelId?: string
  recommendationReason?: string
  implementationSteps: string[]
  testSteps: string[]
  acceptanceCriteria: string[]
  dependsOn?: string[]  // 依赖的任务标题列表
}

// AI 输出联合类型
export type AIOutput = AIFormRequest | AITaskSplitResult

// 拆分消息类型
export interface SplitMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  rawContent?: string
  formSchema?: DynamicFormSchema
  formValues?: Record<string, any>
  cancelled?: boolean // 标记消息是否被取消（关闭弹框时AI正在处理中）
  timestamp: string
}

export type PlanSplitSessionStatus =
  | 'running'
  | 'waiting_input'
  | 'completed'
  | 'failed'
  | 'stopped'

export type PlanSplitLogType =
  | 'content'
  | 'thinking'
  | 'tool_use'
  | 'tool_result'
  | 'usage'
  | 'message_start'
  | 'error'
  | 'system'

export interface PlanSplitSessionRecord {
  id: string
  planId: string
  status: PlanSplitSessionStatus
  executionSessionId?: string | null
  rawContent?: string | null
  resultJson?: string | null
  parseError?: string | null
  errorMessage?: string | null
  granularity: number
  llmMessagesJson?: string | null
  messagesJson?: string | null
  executionRequestJson?: string | null
  formQueueJson?: string | null
  currentFormIndex?: number | null
  createdAt: string
  updatedAt: string
  startedAt?: string | null
  completedAt?: string | null
  stoppedAt?: string | null
}

export interface PlanSplitLogRecord {
  id: string
  planId: string
  sessionId: string
  type: PlanSplitLogType
  content: string
  metadata?: string | null
  createdAt: string
}

export interface PlanSplitStreamPayload {
  type: 'content' | 'thinking' | 'tool_use' | 'tool_result' | 'usage' | 'message_start' | 'error' | 'system' | 'done' | 'session_updated'
  planId: string
  sessionId?: string
  content?: string
  toolName?: string
  toolCallId?: string
  toolInput?: string
  toolResult?: string
  error?: string
  inputTokens?: number
  outputTokens?: number
  model?: string
  metadata?: string | null
  createdAt?: string
  session?: PlanSplitSessionRecord
}

// ==================== 任务继续拆分相关类型 ====================

// 继续拆分配置
export interface TaskResplitConfig {
  taskIndex: number
  customPrompt?: string
  granularity: number
  agentId?: string
  modelId?: string
}

// 继续拆分 prompt 上下文
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
