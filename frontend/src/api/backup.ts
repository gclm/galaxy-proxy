import { apiClient } from './client'
import type { Channel } from './types'

export interface GroupExport {
  name: string
  match_regex: string | null
  retry_enabled: boolean
  max_retries: number
  first_token_timeout_secs: number
  enabled: boolean
  items: {
    channel_name: string
    model_name: string
    priority: number
    weight: number
  }[]
}

export interface ApiKeyExport {
  name: string
  api_key: string
  enabled: boolean
}

export interface SettingExport {
  key: string
  value: string
}

export interface BackupFile {
  format: string
  version: number
  exported_at: string
  app_version: string
  data: {
    channels: Channel[]
    groups: GroupExport[]
    api_keys: ApiKeyExport[]
    settings: SettingExport[]
  }
}

export interface ImportResult {
  channels_imported: number
  groups_imported: number
  api_keys_imported: number
  settings_imported: number
  errors: string[]
}

export const backupApi = {
  export: () => apiClient.get<BackupFile>('/backup/export'),

  import: (data: BackupFile) => apiClient.post<ImportResult>('/backup/import', data),
}
