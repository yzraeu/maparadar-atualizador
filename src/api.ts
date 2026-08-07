import { invoke } from '@tauri-apps/api/core'
import type { AlertType, AppErrorPayload, AppInfo, DeviceInfo, LogEntry, SessionInfo, UpdateSummary } from './types'

export const getAlertTypes = () => invoke<AlertType[]>('get_alert_types')

export const getAppInfo = () => invoke<AppInfo>('get_app_info')

export const getLogs = () => invoke<LogEntry[]>('get_logs')

export const login = (username: string, password: string) =>
  invoke<SessionInfo>('login', { username, password })

export const logout = () => invoke<void>('logout')

export const sessionStatus = () => invoke<SessionInfo | null>('session_status')

export const detectDevice = () => invoke<DeviceInfo[]>('detect_device')

export const previewCount = (radarTypes: string) =>
  invoke<number>('preview_count', { radarTypes })

export const updateDevice = (kind: 'igo8' | 'ndrive', radarTypes: string) =>
  invoke<UpdateSummary>('update_device', { kind, radarTypes })

export const radarTypesString = (selected: number[]): string | null => {
  if (selected.length === 0) return null
  return [...selected].sort((a, b) => a - b).join(',')
}

export const toAppError = (e: unknown): AppErrorPayload => {
  if (typeof e === 'object' && e !== null) {
    const obj = e as Record<string, unknown>
    if (typeof obj.kind === 'string' && typeof obj.message === 'string') {
      return { kind: obj.kind, message: obj.message }
    }
    if (typeof obj.message === 'string') {
      try {
        const parsed = JSON.parse(obj.message) as Record<string, unknown>
        if (typeof parsed.kind === 'string' && typeof parsed.message === 'string') {
          return { kind: parsed.kind, message: parsed.message }
        }
      } catch {
        /* not JSON, fall through */
      }
      return { kind: 'unknown', message: obj.message }
    }
    if (obj.message !== undefined) {
      return { kind: 'unknown', message: String(obj.message) }
    }
    if (typeof obj.kind === 'string') {
      return { kind: obj.kind, message: 'Ocorreu um erro inesperado.' }
    }
  }
  return { kind: 'unknown', message: String(e) }
}
