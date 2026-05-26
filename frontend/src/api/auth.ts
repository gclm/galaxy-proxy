import { apiClient } from './client'
import type {
  InitRequest,
  LoginRequest,
  AuthResponse,
  UserInfoResponse,
  ChangePasswordRequest,
} from './types'

// health API 不需要认证
export const getHealth = async (): Promise<{ needs_setup: boolean }> => {
  const response = await fetch('/api/v1/health')
  const data = await response.json()
  return { needs_setup: data.needs_setup }
}

// init API 不需要认证
export const initSystem = async (data: InitRequest): Promise<AuthResponse> => {
  const response = await fetch('/api/v1/init', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  })
  const result = await response.json()
  if (result.code !== 0) {
    throw new Error(result.message || '初始化失败')
  }
  return result.data
}

export const authApi = {
  login: (data: LoginRequest) =>
    apiClient.post<AuthResponse>('/auth/login', data),

  me: () => apiClient.get<UserInfoResponse>('/auth/me'),

  changePassword: (data: ChangePasswordRequest) =>
    apiClient.put<void>('/auth/password', data),
}
