import { apiClient } from './client'
import type { SettingItem, InfraConfig } from './types'

export const settingsApi = {
  list: () => apiClient.get<SettingItem[]>('/settings'),

  update: (key: string, value: string) =>
    apiClient.put<void>(`/settings/${encodeURIComponent(key)}`, { value }),

  infra: () => apiClient.get<InfraConfig>('/settings/infra'),
}
