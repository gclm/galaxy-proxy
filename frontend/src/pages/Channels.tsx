import { useEffect, useState } from 'react'
import { channelsApi } from '@/api'
import type { Channel, CreateChannelRequest, EndpointConfig, TestModelResponse } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ChannelForm } from '@/components/ChannelForm'
import { Plus, Pencil, Trash2, Play, X, ChevronDown, ChevronUp } from 'lucide-react'

const ENDPOINT_LABELS: Record<string, string> = {
  openai_chat: 'OpenAI Chat',
  openai_response: 'OpenAI Responses',
  anthropic: 'Anthropic',
  gemini: 'Gemini',
  openai_embedding: 'OpenAI Embedding',
  openai_images: 'OpenAI Images',
}

export function Channels() {
  const [channels, setChannels] = useState<Channel[]>([])
  const [loading, setLoading] = useState(true)
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [testingChannelId, setTestingChannelId] = useState<string | null>(null)
  const [testEndpoint, setTestEndpoint] = useState<EndpointConfig | null>(null)
  const [testModel, setTestModel] = useState('')
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<TestModelResponse | null>(null)
  const [expandedChannelId, setExpandedChannelId] = useState<string | null>(null)

  useEffect(() => {
    fetchChannels()
  }, [])

  const fetchChannels = async () => {
    try {
      const data = await channelsApi.list()
      setChannels(data)
    } catch (error) {
      console.error('Failed to fetch channels:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleCreate = async (data: CreateChannelRequest) => {
    await channelsApi.create(data)
    setShowForm(false)
    fetchChannels()
  }

  const handleUpdate = async (data: CreateChannelRequest) => {
    if (!editingChannel) return
    await channelsApi.update(editingChannel.id, data)
    setEditingChannel(null)
    fetchChannels()
  }

  const handleDelete = async (id: string) => {
    if (!confirm('确定删除此渠道？')) return
    try {
      await channelsApi.delete(id)
      setChannels(channels.filter((c) => c.id !== id))
    } catch (error) {
      console.error('Failed to delete channel:', error)
    }
  }

  const openTestPanel = (channel: Channel) => {
    setTestingChannelId(channel.id)
    setTestEndpoint(channel.endpoints[0] || null)
    setTestModel(channel.models?.available_models?.[0] || '')
    setTestResult(null)
  }

  const closeTestPanel = () => {
    setTestingChannelId(null)
    setTestEndpoint(null)
    setTestModel('')
    setTestResult(null)
  }

  const handleTestModel = async () => {
    if (!testEndpoint || !testModel || !testingChannelId) return

    const channel = channels.find(c => c.id === testingChannelId)
    if (!channel) return

    setTesting(true)
    setTestResult(null)
    try {
      const result = await channelsApi.testModel({
        endpoint: testEndpoint,
        api_key: channel.api_keys[0],
        model: testModel,
      })
      setTestResult(result)
    } catch (error: any) {
      setTestResult({
        success: false,
        message: error.response?.data?.message || error.message,
        latency_ms: 0,
      })
    } finally {
      setTesting(false)
    }
  }

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

  if (showForm || editingChannel) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold">
          {editingChannel ? '编辑渠道' : '创建渠道'}
        </h1>
        <ChannelForm
          channel={editingChannel ?? undefined}
          onSubmit={editingChannel ? handleUpdate : handleCreate}
          onCancel={() => {
            setShowForm(false)
            setEditingChannel(null)
          }}
        />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">渠道管理</h1>
        <Button onClick={() => setShowForm(true)} className="btn-primary">
          <Plus className="mr-2 h-4 w-4" />
          添加渠道
        </Button>
      </div>

      <div className="grid gap-4">
        {channels.map((channel) => (
          <Card key={channel.id} className="card-hover">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
              <div className="flex items-center gap-3">
                <CardTitle className="text-lg">{channel.name}</CardTitle>
                <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${
                  channel.enabled
                    ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400'
                    : 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-400'
                }`}>
                  {channel.enabled ? '启用' : '禁用'}
                </span>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => openTestPanel(channel)}
                  className="text-blue-600 hover:text-blue-700 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20"
                >
                  <Play className="h-4 w-4 mr-1" />
                  测试
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => setEditingChannel(channel)}
                >
                  <Pencil className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => handleDelete(channel.id)}
                  className="text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid gap-2 text-sm">
                <div className="flex items-center gap-2">
                  <span className="text-muted-foreground">端点:</span>
                  <div className="flex flex-wrap gap-1">
                    {channel.endpoints.map((ep, i) => (
                      <span key={i} className="inline-flex items-center rounded bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-800 dark:bg-blue-900/30 dark:text-blue-400">
                        {ENDPOINT_LABELS[ep.type] || ep.type}
                      </span>
                    ))}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-muted-foreground">API Keys:</span>
                  <span className="font-medium">{channel.api_keys.length} 个</span>
                </div>
                {channel.models?.available_models && channel.models.available_models.length > 0 && (
                  <div className="flex items-center gap-2">
                    <span className="text-muted-foreground">模型:</span>
                    <span className="font-medium">{channel.models.available_models.length} 个</span>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 px-2 text-xs"
                      onClick={() => setExpandedChannelId(expandedChannelId === channel.id ? null : channel.id)}
                    >
                      {expandedChannelId === channel.id ? (
                        <ChevronUp className="h-3 w-3" />
                      ) : (
                        <ChevronDown className="h-3 w-3" />
                      )}
                    </Button>
                  </div>
                )}
                {expandedChannelId === channel.id && channel.models?.available_models && (
                  <div className="flex flex-wrap gap-1 mt-1 ml-16">
                    {channel.models.available_models.map((model) => (
                      <span key={model} className="inline-flex items-center rounded bg-gray-100 px-2 py-0.5 text-xs text-gray-700 dark:bg-gray-800 dark:text-gray-300">
                        {model}
                      </span>
                    ))}
                  </div>
                )}
              </div>
            </CardContent>

            {/* 测试面板 */}
            {testingChannelId === channel.id && (
              <div className="border-t border-gray-100 dark:border-gray-800 p-4 bg-gray-50 dark:bg-gray-900/50">
                <div className="flex items-center justify-between mb-3">
                  <h4 className="text-sm font-medium">模型测试</h4>
                  <Button variant="ghost" size="icon" className="h-6 w-6" onClick={closeTestPanel}>
                    <X className="h-4 w-4" />
                  </Button>
                </div>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                  <div>
                    <label className="block text-xs font-medium text-muted-foreground mb-1">端点</label>
                    <select
                      className="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm"
                      value={channel.endpoints.indexOf(testEndpoint!)}
                      onChange={(e) => setTestEndpoint(channel.endpoints[parseInt(e.target.value)])}
                    >
                      {channel.endpoints.map((ep, i) => (
                        <option key={i} value={i}>{ENDPOINT_LABELS[ep.type] || ep.type}</option>
                      ))}
                    </select>
                  </div>
                  <div>
                    <label className="block text-xs font-medium text-muted-foreground mb-1">模型</label>
                    <select
                      className="w-full rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-3 py-2 text-sm"
                      value={testModel}
                      onChange={(e) => setTestModel(e.target.value)}
                    >
                      <option value="">选择模型</option>
                      {channel.models?.available_models?.map((model) => (
                        <option key={model} value={model}>{model}</option>
                      ))}
                    </select>
                  </div>
                  <div className="flex items-end">
                    <Button
                      onClick={handleTestModel}
                      disabled={testing || !testModel}
                      className="w-full"
                    >
                      {testing ? '测试中...' : '开始测试'}
                    </Button>
                  </div>
                </div>
                {testResult && (
                  <div className={`mt-3 rounded-lg p-3 text-sm ${
                    testResult.success
                      ? 'bg-green-50 border border-green-200 text-green-800 dark:bg-green-900/20 dark:border-green-800 dark:text-green-400'
                      : 'bg-red-50 border border-red-200 text-red-800 dark:bg-red-900/20 dark:border-red-800 dark:text-red-400'
                  }`}>
                    <div className="flex items-center justify-between">
                      <span className="font-medium">{testResult.success ? '测试成功' : '测试失败'}</span>
                      {testResult.latency_ms > 0 && <span className="text-xs">延迟: {testResult.latency_ms}ms</span>}
                    </div>
                    <div className="mt-1">{testResult.message}</div>
                  </div>
                )}
              </div>
            )}
          </Card>
        ))}

        {channels.length === 0 && (
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12 text-center">
              <div className="rounded-full bg-gray-100 dark:bg-gray-800 p-3 mb-4">
                <Plus className="h-6 w-6 text-muted-foreground" />
              </div>
              <h3 className="font-medium mb-1">暂无渠道</h3>
              <p className="text-sm text-muted-foreground mb-4">创建您的第一个渠道来开始使用</p>
              <Button onClick={() => setShowForm(true)} className="btn-primary">
                <Plus className="mr-2 h-4 w-4" />
                添加渠道
              </Button>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  )
}
