import { invoke } from '@tauri-apps/api/core'
import type { AgentConfig } from '@/stores/agent'
import { resolveAgentRuntimeProfile, type AgentRuntimeKey } from './runtimeProfiles'

export interface RuntimeBindingRecord {
  runtimeKey: string
  externalSessionId: string
  createdAt: string
  updatedAt: string
}

interface RawRuntimeBindingRecord {
  runtime_key: string
  external_session_id: string
  created_at: string
  updated_at: string
}

function transformRuntimeBindingRecord(raw: RawRuntimeBindingRecord): RuntimeBindingRecord {
  return {
    runtimeKey: raw.runtime_key,
    externalSessionId: raw.external_session_id,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at
  }
}

export function resolveRuntimeBindingKey(agent: Pick<AgentConfig, 'type' | 'provider' | 'name' | 'cliPath'>): AgentRuntimeKey | null {
  return resolveAgentRuntimeProfile(agent)?.key ?? null
}

export async function getSessionRuntimeBinding(
  sessionId: string,
  runtimeKey: AgentRuntimeKey
): Promise<RuntimeBindingRecord | null> {
  const record = await invoke<RawRuntimeBindingRecord | null>('get_session_runtime_binding', {
    sessionId,
    runtimeKey
  })
  return record ? transformRuntimeBindingRecord(record) : null
}

export async function upsertSessionRuntimeBinding(
  sessionId: string,
  runtimeKey: AgentRuntimeKey,
  externalSessionId: string
): Promise<RuntimeBindingRecord> {
  const record = await invoke<RawRuntimeBindingRecord>('upsert_session_runtime_binding', {
    sessionId,
    runtimeKey,
    externalSessionId
  })
  return transformRuntimeBindingRecord(record)
}

export async function deleteSessionRuntimeBinding(
  sessionId: string,
  runtimeKey: AgentRuntimeKey
): Promise<void> {
  await invoke('delete_session_runtime_binding', { sessionId, runtimeKey })
}

export async function getTaskRuntimeBinding(
  taskId: string,
  runtimeKey: AgentRuntimeKey
): Promise<RuntimeBindingRecord | null> {
  const record = await invoke<RawRuntimeBindingRecord | null>('get_task_runtime_binding', {
    taskId,
    runtimeKey
  })
  return record ? transformRuntimeBindingRecord(record) : null
}

export async function upsertTaskRuntimeBinding(
  taskId: string,
  runtimeKey: AgentRuntimeKey,
  externalSessionId: string
): Promise<RuntimeBindingRecord> {
  const record = await invoke<RawRuntimeBindingRecord>('upsert_task_runtime_binding', {
    taskId,
    runtimeKey,
    externalSessionId
  })
  return transformRuntimeBindingRecord(record)
}

export async function deleteTaskRuntimeBinding(
  taskId: string,
  runtimeKey: AgentRuntimeKey
): Promise<void> {
  await invoke('delete_task_runtime_binding', { taskId, runtimeKey })
}

export async function getSoloRuntimeBinding(
  runId: string,
  runtimeKey: string
): Promise<RuntimeBindingRecord | null> {
  const record = await invoke<RawRuntimeBindingRecord | null>('get_solo_runtime_binding', {
    runId,
    runtimeKey
  })
  return record ? transformRuntimeBindingRecord(record) : null
}

export async function upsertSoloRuntimeBinding(
  runId: string,
  runtimeKey: string,
  externalSessionId: string
): Promise<void> {
  await invoke('upsert_solo_runtime_binding', {
    input: {
      run_id: runId,
      runtime_key: runtimeKey,
      external_session_id: externalSessionId
    }
  })
}

export async function deleteSoloRuntimeBinding(
  runId: string,
  runtimeKey: string
): Promise<void> {
  await invoke('delete_solo_runtime_binding', { runId, runtimeKey })
}

export function isInvalidCliResumeError(
  error: string,
  runtimeKey?: AgentRuntimeKey | null
): boolean {
  const normalized = error.trim().toLowerCase()
  if (!normalized) {
    return false
  }

  const hasResumeKeyword = /(resume|恢复|续接|继续会话)/i.test(error)
  const hasIdKeyword = /(session|conversation|thread|message)[ _-]?id|会话.?id|消息.?id|对话.?id/i.test(error)
  const hasInvalidKeyword = /(not found|invalid|unknown|missing|expired|does not exist|不存在|无效|未找到|找不到)/i.test(error)

  if ((hasResumeKeyword || hasIdKeyword) && hasInvalidKeyword) {
    return true
  }

  if (!runtimeKey) {
    return false
  }

  if (runtimeKey === 'claude-cli') {
    return /(claude|anthropic)/i.test(error) && (hasResumeKeyword || hasIdKeyword) && hasInvalidKeyword
  }

  if (runtimeKey === 'codex-cli') {
    return /codex/i.test(error) && (hasResumeKeyword || hasIdKeyword) && hasInvalidKeyword
  }

  return false
}
