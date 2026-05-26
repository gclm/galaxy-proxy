import { apiClient } from './client'
import type { StatsOverview, DailyStats, ModelStats, ChannelStats } from './types'

export const statsApi = {
  overview: () => apiClient.get<StatsOverview>('/stats/overview'),

  daily: (days?: number) =>
    apiClient.get<DailyStats[]>(`/stats/daily${days ? `?days=${days}` : ''}`),

  models: (days?: number) =>
    apiClient.get<ModelStats[]>(`/stats/models${days ? `?days=${days}` : ''}`),

  channels: (days?: number) =>
    apiClient.get<ChannelStats[]>(`/stats/channels${days ? `?days=${days}` : ''}`),
}
