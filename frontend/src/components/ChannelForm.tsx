import { useState } from 'react'
import type { Channel, CreateChannelRequest, EndpointConfig, EndpointType, ModelsConfig } from '@/api/types'
import { channelsApi } from '@/api/channels'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Plus, Trash2, RefreshCw } from 'lucide-react'

interface ChannelFormProps {
  channel?: Channel
  onSubmit: (data: CreateChannelRequest) => Promise<void>
  onCancel: () => void
}

const ENDPOINT_TYPES: EndpointType[] = [
  'openai_chat',
  'openai_response',
  'anthropic',
  'gemini',
  'openai_embedding',
  'openai_images',
]

const ENDPOINT_LABELS: Record<EndpointType, string> = {
  openai_chat: 'OpenAI Chat',
  openai_response: 'OpenAI Responses',
  anthropic: 'Anthropic',
  gemini: 'Gemini',
  openai_embedding: 'OpenAI Embedding',
  openai_images: 'OpenAI Images',
}

export function ChannelForm({ channel, onSubmit, onCancel }: ChannelFormProps) {
  const [name, setName] = useState(channel?.name ?? '')
  const [apiKeys, setApiKeys] = useState<string[]>(channel?.api_keys ?? [''])
  const [endpoints, setEndpoints] = useState<EndpointConfig[]>(
    channel?.endpoints ?? [{ type: 'openai_chat', base_url: '' }]
  )
  const [modelsConfig, setModelsConfig] = useState<ModelsConfig>(
    channel?.models ?? { available_models: [], model_maps: {} }
  )
  const [modelMapsText, setModelMapsText] = useState(
    channel?.models?.model_maps ? JSON.stringify(channel.models.model_maps, null, 2) : ''
  )
  const [rateLimitRpm, setRateLimitRpm] = useState(channel?.rate_limit_rpm?.toString() ?? '')
  const [rateLimitTpm, setRateLimitTpm] = useState(channel?.rate_limit_tpm?.toString() ?? '')
  const [failureThreshold, setFailureThreshold] = useState(channel?.failure_threshold?.toString() ?? '3')
  const [blacklistMinutes, setBlacklistMinutes] = useState(channel?.blacklist_minutes?.toString() ?? '5')
  const [concurrency, setConcurrency] = useState(channel?.concurrency?.toString() ?? '10')
  const [enabled, setEnabled] = useState(channel?.enabled ?? true)
  const [submitting, setSubmitting] = useState(false)
  const [fetchingModels, setFetchingModels] = useState(false)

  const handleFetchModels = async () => {
    const endpoint = endpoints[0]
    const apiKey = apiKeys[0]

    if (!endpoint?.base_url || !apiKey) {
      alert('请先填写端点地址和 API Key')
      return
    }

    setFetchingModels(true)
    try {
      const models = await channelsApi.fetchModels({
        endpoint_type: endpoint.type,
        base_url: endpoint.base_url,
        api_key: apiKey,
      })
      setModelsConfig(prev => ({
        ...prev,
        available_models: models,
      }))
    } catch (error: any) {
      alert(`获取模型失败: ${error.response?.data?.message || error.message}`)
    } finally {
      setFetchingModels(false)
    }
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitting(true)

    try {
      const data: CreateChannelRequest = {
        name,
        api_keys: apiKeys.filter((k) => k.trim()),
        endpoints: endpoints.filter((ep) => ep.base_url.trim()),
        enabled,
        failure_threshold: parseInt(failureThreshold) || 3,
        blacklist_minutes: parseInt(blacklistMinutes) || 5,
        concurrency: parseInt(concurrency) || 10,
      }

      if (rateLimitRpm) data.rate_limit_rpm = parseInt(rateLimitRpm)
      if (rateLimitTpm) data.rate_limit_tpm = parseInt(rateLimitTpm)

      // 解析 model_maps
      let modelMaps: Record<string, string> = {}
      if (modelMapsText.trim()) {
        try {
          modelMaps = JSON.parse(modelMapsText)
        } catch {
          alert('模型映射 JSON 格式错误')
          return
        }
      }

      // 构建 models 配置
      data.models = {
        available_models: modelsConfig.available_models,
        model_maps: modelMaps,
      }

      await onSubmit(data)
    } finally {
      setSubmitting(false)
    }
  }

  const addApiKey = () => setApiKeys([...apiKeys, ''])
  const removeApiKey = (index: number) => setApiKeys(apiKeys.filter((_, i) => i !== index))
  const updateApiKey = (index: number, value: string) => {
    const newKeys = [...apiKeys]
    newKeys[index] = value
    setApiKeys(newKeys)
  }

  const addEndpoint = () => setEndpoints([...endpoints, { type: 'openai_chat', base_url: '' }])
  const removeEndpoint = (index: number) => setEndpoints(endpoints.filter((_, i) => i !== index))
  const updateEndpoint = (index: number, field: keyof EndpointConfig, value: string) => {
    const newEndpoints = [...endpoints]
    newEndpoints[index] = { ...newEndpoints[index], [field]: value }
    setEndpoints(newEndpoints)
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{channel ? '编辑渠道' : '创建渠道'}</CardTitle>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-6">
          <div className="space-y-4">
            <h3 className="text-sm font-medium">基本信息</h3>
            <div>
              <label className="block text-sm font-medium mb-1">渠道名称 *</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                placeholder="例如：OpenAI 主力渠道"
                required
              />
            </div>

            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="enabled"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
                className="rounded"
              />
              <label htmlFor="enabled" className="text-sm">启用渠道</label>
            </div>
          </div>

          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-medium">上游 API Keys *</h3>
              <Button type="button" variant="outline" size="sm" onClick={addApiKey}>
                <Plus className="h-4 w-4 mr-1" /> 添加
              </Button>
            </div>
            {apiKeys.map((key, index) => (
              <div key={index} className="flex gap-2">
                <input
                  type="text"
                  value={key}
                  onChange={(e) => updateApiKey(index, e.target.value)}
                  className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm font-mono"
                  placeholder="sk-..."
                />
                {apiKeys.length > 1 && (
                  <Button type="button" variant="ghost" size="icon" onClick={() => removeApiKey(index)}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
              </div>
            ))}
          </div>

          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-medium">端点配置 *</h3>
              <Button type="button" variant="outline" size="sm" onClick={addEndpoint}>
                <Plus className="h-4 w-4 mr-1" /> 添加
              </Button>
            </div>
            {endpoints.map((ep, index) => (
              <div key={index} className="flex gap-2">
                <select
                  value={ep.type}
                  onChange={(e) => updateEndpoint(index, 'type', e.target.value)}
                  className="rounded-md border border-input bg-background px-3 py-2 text-sm"
                >
                  {ENDPOINT_TYPES.map((t) => (
                    <option key={t} value={t}>{ENDPOINT_LABELS[t]}</option>
                  ))}
                </select>
                <input
                  type="text"
                  value={ep.base_url}
                  onChange={(e) => updateEndpoint(index, 'base_url', e.target.value)}
                  className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm"
                  placeholder="https://api.openai.com/v1"
                />
                {endpoints.length > 1 && (
                  <Button type="button" variant="ghost" size="icon" onClick={() => removeEndpoint(index)}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
              </div>
            ))}
          </div>

          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-medium">模型配置</h3>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleFetchModels}
                disabled={fetchingModels}
              >
                <RefreshCw className={`h-4 w-4 mr-1 ${fetchingModels ? 'animate-spin' : ''}`} />
                {fetchingModels ? '获取中...' : '获取模型'}
              </Button>
            </div>

            {modelsConfig.available_models.length > 0 && (
              <div>
                <label className="block text-sm font-medium mb-1">
                  可用模型 ({modelsConfig.available_models.length})
                </label>
                <div className="max-h-40 overflow-y-auto rounded-md border border-input bg-background p-2 text-sm">
                  <div className="flex flex-wrap gap-1">
                    {modelsConfig.available_models.map((model) => (
                      <span
                        key={model}
                        className="inline-flex items-center px-2 py-1 rounded-md bg-secondary text-secondary-foreground text-xs"
                      >
                        {model}
                      </span>
                    ))}
                  </div>
                </div>
              </div>
            )}

            <div>
              <label className="block text-sm font-medium mb-1">模型映射 (JSON)</label>
              <textarea
                value={modelMapsText}
                onChange={(e) => setModelMapsText(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono"
                rows={4}
                placeholder={'{\n  "gpt-4": "gpt-4-turbo"\n}'}
              />
              <p className="text-xs text-muted-foreground mt-1">
                将请求的模型名映射到实际上游模型名
              </p>
            </div>
          </div>

          <div className="space-y-4">
            <h3 className="text-sm font-medium">高级配置</h3>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium mb-1">RPM 限制</label>
                <input
                  type="number"
                  value={rateLimitRpm}
                  onChange={(e) => setRateLimitRpm(e.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                  placeholder="不限"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">TPM 限制</label>
                <input
                  type="number"
                  value={rateLimitTpm}
                  onChange={(e) => setRateLimitTpm(e.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                  placeholder="不限"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">失败阈值</label>
                <input
                  type="number"
                  value={failureThreshold}
                  onChange={(e) => setFailureThreshold(e.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">黑名单分钟</label>
                <input
                  type="number"
                  value={blacklistMinutes}
                  onChange={(e) => setBlacklistMinutes(e.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">并发数</label>
                <input
                  type="number"
                  value={concurrency}
                  onChange={(e) => setConcurrency(e.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                />
              </div>
            </div>
          </div>

          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" onClick={onCancel}>
              取消
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? '保存中...' : channel ? '更新' : '创建'}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
