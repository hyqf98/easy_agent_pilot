import type { DynamicFormSchema } from './plan'
import type { ExecutionLogMetadata, TaskTokenUsageWindow } from './taskExecution'
import type { ToolCall } from '@/stores/message'

export type SoloRunStatus =
  | 'draft'
  | 'running'
  | 'blocked'
  | 'completed'
  | 'failed'
  | 'paused'
  | 'stopped'

export type SoloExecutionStatus =
  | 'idle'
  | 'running'
  | 'paused'
  | 'completed'
  | 'error'
  | 'blocked'
  | 'stopped'

export type SoloStepStatus =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'blocked'
  | 'skipped'

export type SoloLogType =
  | 'content'
  | 'thinking'
  | 'thinking_start'
  | 'tool_use'
  | 'tool_input_delta'
  | 'tool_result'
  | 'usage'
  | 'system'
  | 'error'

export type SoloLogScope = 'coordinator' | 'step' | 'system'

export interface SoloInputRequest {
  formSchema: DynamicFormSchema
  question?: string
  requestedAt: string
  source: 'control' | 'execution'
  stepId?: string
}

export interface SoloRun {
  id: string
  projectId: string
  executionPath: string
  name: string
  requirement: string
  goal: string
  participantExpertIds: string[]
  coordinatorExpertId?: string
  coordinatorAgentId?: string
  coordinatorModelId?: string
  maxDispatchDepth: number
  currentDepth: number
  currentStepId?: string
  status: SoloRunStatus
  executionStatus: SoloExecutionStatus
  lastError?: string
  inputRequest?: SoloInputRequest
  inputResponse?: Record<string, unknown>
  createdAt: string
  updatedAt: string
  startedAt?: string
  completedAt?: string
  stoppedAt?: string
}

export interface SoloStep {
  id: string
  runId: string
  stepRef: string
  parentStepRef?: string
  depth: number
  title: string
  description?: string
  executionPrompt?: string
  selectedExpertId?: string
  status: SoloStepStatus
  summary?: string
  resultSummary?: string
  resultFiles: string[]
  failReason?: string
  createdAt: string
  updatedAt: string
  startedAt?: string
  completedAt?: string
}

export interface SoloLogEntry {
  id: string
  runId: string
  stepId?: string
  scope: SoloLogScope
  type: SoloLogType
  content: string
  metadata?: ExecutionLogMetadata
  timestamp: string
}

export interface SoloExecutionState {
  runId: string
  status: SoloExecutionStatus
  sessionId: string | null
  startedAt: string | null
  completedAt: string | null
  currentStepId: string | null
  logs: SoloLogEntry[]
  accumulatedContent: string
  accumulatedThinking: string
  toolCalls: ToolCall[]
  tokenUsage: TaskTokenUsageWindow
}

export interface CreateSoloRunInput {
  projectId: string
  executionPath: string
  name: string
  requirement: string
  goal: string
  participantExpertIds: string[]
  coordinatorExpertId?: string
  coordinatorAgentId?: string
  coordinatorModelId?: string
  maxDispatchDepth: number
}

export interface UpdateSoloRunInput {
  executionPath?: string | null
  name?: string | null
  requirement?: string | null
  goal?: string | null
  participantExpertIds?: string[] | null
  coordinatorExpertId?: string | null
  coordinatorAgentId?: string | null
  coordinatorModelId?: string | null
  maxDispatchDepth?: number | null
  currentDepth?: number | null
  currentStepId?: string | null
  status?: SoloRunStatus
  executionStatus?: SoloExecutionStatus
  lastError?: string | null
  inputRequest?: SoloInputRequest | null
  inputResponse?: Record<string, unknown> | null
  startedAt?: string | null
  completedAt?: string | null
  stoppedAt?: string | null
}

export interface CreateSoloStepInput {
  runId: string
  stepRef: string
  parentStepRef?: string
  depth: number
  title: string
  description?: string
  executionPrompt?: string
  selectedExpertId?: string
  status?: SoloStepStatus
  summary?: string
  startedAt?: string
}

export interface UpdateSoloStepInput {
  parentStepRef?: string | null
  depth?: number | null
  title?: string | null
  description?: string | null
  executionPrompt?: string | null
  selectedExpertId?: string | null
  status?: SoloStepStatus
  summary?: string | null
  resultSummary?: string | null
  resultFiles?: string[]
  failReason?: string | null
  startedAt?: string | null
  completedAt?: string | null
}

export interface CreateSoloLogInput {
  runId: string
  stepId?: string
  scope: SoloLogScope
  type: SoloLogType
  content: string
  metadata?: ExecutionLogMetadata
}

export interface SoloCoordinatorDispatchStep {
  stepRef: string
  parentStepRef?: string
  depth: number
  title: string
  description: string
  selectedExpertId?: string
  executionPrompt: string
  doneWhen: string[]
}

export interface SoloCoordinatorDispatchResult {
  type: 'dispatch_step'
  step: SoloCoordinatorDispatchStep
}

export interface SoloCoordinatorCompleteResult {
  type: 'complete_run'
  summary: string
  deliveredArtifacts: string[]
}

export interface SoloCoordinatorBlockResult {
  type: 'block_run'
  reason: string
  formSchema?: DynamicFormSchema
  question?: string
}

export type SoloCoordinatorDecision =
  | SoloCoordinatorDispatchResult
  | SoloCoordinatorCompleteResult
  | SoloCoordinatorBlockResult
