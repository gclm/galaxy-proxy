import { apiClient } from './client'
import type { ApiKey, CreateApiKeyRequest, UpdateApiKeyRequest } from './types'

export const apiKeysApi = {
  list: () => apiClient.get<ApiKey[]>('/api-keys'),

  get: (id: string) => apiClient.get<ApiKey>(`/api-keys/${id}`),

  create: (data: CreateApiKeyRequest) =>
    apiClient.post<ApiKey>('/api-keys', data),

  update: (id: string, data: UpdateApiKeyRequest) =>
    apiClient.put<ApiKey>(`/api-keys/${id}`, data),

  delete: (id: string) => apiClient.delete<void>(`/api-keys/${id}`),
}
