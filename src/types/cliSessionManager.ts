export interface ScannedCliSession {
  session_id: string
  session_path: string
  project_path: string | null
  first_message: string | null
  message_count: number
  created_at: string
  updated_at: string
}

export interface AgentCliSessionsResult {
  agent_id: string
  cli_name: string
  session_root: string
  sessions: ScannedCliSession[]
  project_paths: string[]
}

export interface CliSessionMessage {
  line_no: number
  message_type: string
  role: string | null
  timestamp: string | null
  content: string | null
  raw_json: string
}

export interface CliSessionDetail {
  session_id: string
  session_path: string
  project_path: string | null
  first_message: string | null
  message_count: number
  created_at: string
  updated_at: string
  messages: CliSessionMessage[]
}

export interface DeleteCliSessionsResult {
  deleted_count: number
  failed_paths: string[]
}
