import { create } from 'zustand'
import { apiClient, authApi, initSystem } from '@/api'
import type { UserInfoResponse } from '@/api/types'

interface AuthState {
  user: UserInfoResponse | null
  isAuthenticated: boolean
  isLoading: boolean
  login: (username: string, password: string) => Promise<void>
  setup: (username: string, password: string, siteTitle?: string) => Promise<void>
  logout: () => void
  checkAuth: () => Promise<void>
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  isAuthenticated: false,
  isLoading: true,

  login: async (username, password) => {
    const { token } = await authApi.login({ username, password })
    apiClient.setToken(token)
    const user = await authApi.me()
    set({ user, isAuthenticated: true })
  },

  setup: async (username, password, siteTitle) => {
    const { token } = await initSystem({ username, password, site_title: siteTitle })
    apiClient.setToken(token)
    const user = await authApi.me()
    set({ user, isAuthenticated: true })
  },

  logout: () => {
    apiClient.setToken(null)
    set({ user: null, isAuthenticated: false })
  },

  checkAuth: async () => {
    const token = apiClient.getToken()
    if (!token) {
      set({ isLoading: false })
      return
    }

    try {
      const user = await authApi.me()
      set({ user, isAuthenticated: true, isLoading: false })
    } catch {
      apiClient.setToken(null)
      set({ user: null, isAuthenticated: false, isLoading: false })
    }
  },
}))
