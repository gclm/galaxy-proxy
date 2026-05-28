import { apiClient } from './client'
import type {
  Group,
  GroupItem,
  CreateGroupRequest,
  UpdateGroupRequest,
  AddGroupItemRequest,
  PaginatedResponse,
} from './types'

export interface GroupListParams {
  search?: string
  status?: string
  sort_by?: string
  sort_order?: string
  page?: number
  page_size?: number
}

export const groupsApi = {
  list: (params?: GroupListParams) =>
    apiClient.get<PaginatedResponse<Group>>('/groups', params as Record<string, string | number | undefined>),

  get: (id: string) => apiClient.get<Group>(`/groups/${id}`),

  create: (data: CreateGroupRequest) =>
    apiClient.post<Group>('/groups', data),

  update: (id: string, data: UpdateGroupRequest) =>
    apiClient.put<Group>(`/groups/${id}`, data),

  delete: (id: string) => apiClient.delete<void>(`/groups/${id}`),

  addItem: (groupId: string, data: AddGroupItemRequest) =>
    apiClient.post<GroupItem>(`/groups/${groupId}/items`, data),

  deleteItem: (groupId: string, itemId: string) =>
    apiClient.delete<void>(`/groups/${groupId}/items/${itemId}`),
}
