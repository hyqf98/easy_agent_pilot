import { invoke } from '@tauri-apps/api/core'

export interface LspServerInfo {
  id: string
  name: string
  description: string
  monacoLanguageId: string
  fileExtensions: string[]
  installed: boolean
  installPath: string
  installedAt: string | null
}

export interface LspActivationResult {
  filePath: string
  serverId: string | null
  monacoLanguageId: string
  activated: boolean
  message: string
}

export async function getLspStorageDir(): Promise<string> {
  return invoke<string>('get_lsp_storage_dir')
}

export async function listLspServers(): Promise<LspServerInfo[]> {
  return invoke<LspServerInfo[]>('list_lsp_servers')
}

export async function downloadLspServer(serverId: string): Promise<LspServerInfo> {
  return invoke<LspServerInfo>('download_lsp_server', { serverId })
}

export async function removeLspServer(serverId: string): Promise<LspServerInfo> {
  return invoke<LspServerInfo>('remove_lsp_server', { serverId })
}

export async function activateLspForFile(filePath: string): Promise<LspActivationResult> {
  return invoke<LspActivationResult>('activate_lsp_for_file', { filePath })
}
