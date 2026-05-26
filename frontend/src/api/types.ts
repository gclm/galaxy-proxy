// Auth types
export interface InitRequest {
  username: string
  password: string
  site_title?: string
}

export interface LoginRequest {
  username: string
  password: string
}

export interface AuthResponse {
  token: string
  expires_in: number
}

export interface UserInfoResponse {
  id: string
  username: string
}

export interface ChangePasswordRequest {
  old_password: string
  new_password: string
}

// Channel types
export type EndpointType =
  | 'openai_chat'
  | 'openai_response'
  | 'anthropic'
  | 'gemini'
  | 'openai_embedding'
  | 'openai_images'

export interface EndpointConfig {
  type: EndpointType
  base_url: string
}

export interface ModelsConfig {
  available_models: string[]
  model_maps: Record<string, string>
}

export interface Channel {
  id: string
  name: string
  api_keys: string[]
  endpoints: EndpointConfig[]
  models: ModelsConfig | null
  rate_limit_rpm: number | null
  rate_limit_tpm: number | null
  failure_threshold: number
  blacklist_minutes: number
  concurrency: number
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface CreateChannelRequest {
  name: string
  api_keys: string[]
  endpoints: EndpointConfig[]
  models?: ModelsConfig
  rate_limit_rpm?: number
  rate_limit_tpm?: number
  failure_threshold?: number
  blacklist_minutes?: number
  concurrency?: number
  enabled?: boolean
}

export interface UpdateChannelRequest {
  name?: string
  api_keys?: string[]
  endpoints?: EndpointConfig[]
  models?: ModelsConfig
  rate_limit_rpm?: number
  rate_limit_tpm?: number
  failure_threshold?: number
  blacklist_minutes?: number
  concurrency?: number
  enabled?: boolean
}

export interface FetchModelsRequest {
  endpoints: EndpointConfig[]
  api_key: string
}

export interface TestModelRequest {
  endpoint: EndpointConfig
  api_key: string
  model: string
}

export interface TestModelResponse {
  success: boolean
  message: string
  latency_ms: number
}

// Group types
export interface GroupItem {
  id: string
  channel_id: string
  model_name: string
  priority: number
  weight: number
}

export interface Group {
  id: string
  name: string
  match_regex: string | null
  retry_enabled: boolean
  max_retries: number
  first_token_timeout_secs: number
  enabled: boolean
  items: GroupItem[]
  created_at: string
  updated_at: string
}

export interface CreateGroupRequest {
  name: string
  match_regex?: string
  retry_enabled?: boolean
  max_retries?: number
  first_token_timeout_secs?: number
  enabled?: boolean
  items: CreateGroupItemRequest[]
}

export interface CreateGroupItemRequest {
  channel_id: string
  model_name: string
  priority?: number
  weight?: number
}

export interface UpdateGroupRequest {
  name?: string
  match_regex?: string
  retry_enabled?: boolean
  max_retries?: number
  first_token_timeout_secs?: number
  enabled?: boolean
}

export interface AddGroupItemRequest {
  channel_id: string
  model_name: string
  priority?: number
  weight?: number
}

// API Key types
export interface ApiKey {
  id: string
  name: string
  api_key: string
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface CreateApiKeyRequest {
  name: string
}

export interface UpdateApiKeyRequest {
  name?: string
  enabled?: boolean
}

// Stats types
export interface StatsOverview {
  total_requests: number
  total_tokens: number
  total_cost: number
}

export interface DailyStats {
  date: string
  requests: number
  tokens: number
  cost: number
}

export interface ModelStats {
  model: string
  requests: number
  tokens: number
  cost: number
}

export interface ChannelStats {
  channel_id: string
  channel_name: string
  requests: number
  tokens: number
  cost: number
}
