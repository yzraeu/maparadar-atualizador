export interface SessionInfo {
  username: string
  expiresAtUnix: number
}

export interface DeviceInfo {
  kind: 'igo8' | 'ndrive'
  display: string
  drive: string
}

export interface AlertType {
  code: number
  label: string
  icon: string
  default: boolean
}

export interface UpdateSummary {
  filesWritten: string[]
  filesDeleted: string[]
}

export interface AppInfo {
  name: string
  version: string
  platform: string
  arch: string
  tauriVersion: string
}

export interface LogEntry {
  timestampUnixMs: number
  level: 'info' | 'warn' | 'error' | string
  message: string
}

export interface AppErrorPayload {
  kind: string
  message: string
}
