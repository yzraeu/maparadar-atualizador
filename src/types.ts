export interface SessionInfo {
  username: string
  expiresAtUnix: number
}

export interface DeviceInfo {
  kind: string
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

export interface AppErrorPayload {
  kind: string
  message: string
}
