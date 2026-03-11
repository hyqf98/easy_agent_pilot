export interface InstallOption {
  method: string
  command: string
  recommended: boolean
  available: boolean
  display_name: string
}

export interface CliInstallerInfo {
  cli_name: string
  installed: boolean
  current_version: string | null
  install_options: InstallOption[]
}

export interface VersionInfo {
  current: string | null
  latest: string | null
  has_update: boolean
  release_notes: string | null
}

export interface InstallLogEvent {
  cli_name: string
  message: string
  timestamp: string
}

export interface CliInstallerCard {
  key: string
  label: string
  info: CliInstallerInfo | null
  versionInfo: VersionInfo | null
}
