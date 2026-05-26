import { apiClient } from './client'
import type {
  Channel,
  CreateChannelRequest,
  FetchModelsRequest,
  UpdateChannelRequest,
} from './types'

export const channelsApi = {
  list: () => apiClient.get<Channel[]>('/channels'),

  get: (id: string) => apiClient.get<Channel>(`/channels/${id}`),

  create: (data: CreateChannelRequest) =>
    apiClient.post<Channel>('/channels', data),

  update: (id: string, data: UpdateChannelRequest) =>
    apiClient.put<Channel>(`/channels/${id}`, data),

  delete: (id: string) => apiClient.delete<void>(`/channels/${id}`),

  fetchModels: (data: FetchModelsRequest) =>
    apiClient.post<string[]>('/fetch-models', data),
}
