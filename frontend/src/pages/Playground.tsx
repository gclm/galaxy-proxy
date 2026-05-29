import { useEffect, useRef, useState } from 'react'
import { apiKeysApi } from '@/api/api-keys'
import { statsApi } from '@/api/stats'
import type { ApiKey, RequestLog, EndpointType } from '@/api/types'
import { ENDPOINT_LABELS } from '@/api/types'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Play, Square, RefreshCw } from 'lucide-react'

const PLAYGROUND_PROTOCOLS: EndpointType[] = [
  'openai_chat',
  'openai_response',
  'anthropic',
  'openai_embedding',
  'openai_images',
]

const PROXY_PATHS: Record<string, string> = {
  openai_chat: '/v1/chat/completions',
  openai_response: '/v1/responses',
  anthropic: '/v1/messages',
  openai_embedding: '/v1/embeddings',
  openai_images: '/v1/images/generations',
}

const STREAMABLE_PROTOCOLS = new Set(['openai_chat', 'openai_response', 'anthropic'])

function buildRequestConfig(
  protocol: string,
  apiKey: string,
  model: string,
  prompt: string,
  stream: boolean,
): { path: string; headers: Record<string, string>; body: Record<string, unknown> } {
  const defaultPrompt = prompt || 'Hello! Please introduce yourself briefly.'

  switch (protocol) {
    case 'openai_chat':
      return {
        path: PROXY_PATHS.openai_chat,
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`,
        },
        body: { model, stream, messages: [{ role: 'user', content: defaultPrompt }] },
      }
    case 'openai_response':
      return {
        path: PROXY_PATHS.openai_response,
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`,
        },
        body: { model, stream, input: defaultPrompt },
      }
    case 'anthropic':
      return {
        path: PROXY_PATHS.anthropic,
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': apiKey,
          'anthropic-version': '2023-06-01',
        },
        body: {
          model,
          stream,
          max_tokens: 1024,
          messages: [{ role: 'user', content: defaultPrompt }],
        },
      }
    case 'openai_embedding':
      return {
        path: PROXY_PATHS.openai_embedding,
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`,
        },
        body: { model, input: defaultPrompt },
      }
    case 'openai_images':
      return {
        path: PROXY_PATHS.openai_images,
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${apiKey}`,
        },
        body: { model, prompt: defaultPrompt, n: 1, size: '1024x1024' },
      }
    default:
      throw new Error(`Unknown protocol: ${protocol}`)
  }
}

function extractErrorMessage(body: Record<string, unknown>, protocol: string): string {
  if (protocol === 'anthropic') {
    const error = body.error as Record<string, unknown> | undefined
    return (error?.message as string) ?? 'Unknown error'
  }
  const error = body.error as Record<string, unknown> | undefined
  return (error?.message as string) ?? 'Unknown error'
}

type TabType = 'rendered' | 'raw'

export function Playground() {
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([])
  const [selectedApiKeyId, setSelectedApiKeyId] = useState<string>('')
  const [models, setModels] = useState<string[]>([])
  const [selectedModel, setSelectedModel] = useState<string>('')
  const [protocol, setProtocol] = useState<EndpointType>('openai_chat')
  const [prompt, setPrompt] = useState('')
  const [stream, setStream] = useState(true)
  const [tab, setTab] = useState<TabType>('rendered')

  const [loading, setLoading] = useState(false)
  const [renderedContent, setRenderedContent] = useState('')
  const [rawContent, setRawContent] = useState('')
  const [error, setError] = useState('')
  const [statusCode, setStatusCode] = useState(0)
  const [latency, setLatency] = useState(0)
  const [routeLog, setRouteLog] = useState<RequestLog | null>(null)

  const abortRef = useRef<AbortController | null>(null)
  const startTimeRef = useRef(0)

  const selectedApiKey = apiKeys.find((k) => k.id === selectedApiKeyId)

  // 获取 API Key 列表
  useEffect(() => {
    apiKeysApi.list().then((keys) => {
      setApiKeys(keys.filter((k) => k.enabled))
      if (keys.length > 0 && !selectedApiKeyId) {
        const first = keys.find((k) => k.enabled)
        if (first) setSelectedApiKeyId(first.id)
      }
    })
  }, [])

  // 切换 API Key 时刷新模型列表
  useEffect(() => {
    if (!selectedApiKey) return
    fetchModels(selectedApiKey.api_key)
  }, [selectedApiKeyId])

  const fetchModels = async (apiKey: string) => {
    try {
      const res = await fetch('/v1/models', {
        headers: { Authorization: `Bearer ${apiKey}` },
      })
      const data = await res.json()
      if (data.data) {
        const names = data.data.map((m: { id: string }) => m.id).sort()
        setModels(names)
        setSelectedModel((prev) => (prev && names.includes(prev) ? prev : names[0]))
      }
    } catch {
      setModels([])
    }
  }

  // 流式 SSE 解析
  const parseStreamResponse = async (
    reader: ReadableStreamDefaultReader<Uint8Array>,
    resProtocol: string,
  ) => {
    const decoder = new TextDecoder()
    let buffer = ''
    let fullRaw = ''
    let rendered = ''

    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      const chunk = decoder.decode(value, { stream: true })
      fullRaw += chunk
      buffer += chunk

      const lines = buffer.split('\n')
      buffer = lines.pop() ?? ''

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6).trim()
          if (data === '[DONE]') continue
          try {
            const json = JSON.parse(data)
            if (resProtocol === 'openai_chat') {
              const delta = json.choices?.[0]?.delta?.content
              if (delta) {
                rendered += delta
                setRenderedContent(rendered)
              }
            } else if (resProtocol === 'openai_response') {
              const delta = json.delta
              if (typeof delta === 'string') {
                rendered += delta
                setRenderedContent(rendered)
              }
            } else if (resProtocol === 'anthropic') {
              const delta = json.delta?.text
              if (delta) {
                rendered += delta
                setRenderedContent(rendered)
              }
            }
          } catch {
            // 非 JSON 行跳过
          }
        }
      }
      setRawContent(fullRaw)
    }
  }

  // 非流式响应解析
  const parseJsonResponse = (json: Record<string, unknown>, resProtocol: string) => {
    switch (resProtocol) {
      case 'openai_chat': {
        const choices = json.choices as Array<{ message?: { content?: string } }> | undefined
        return choices?.[0]?.message?.content ?? ''
      }
      case 'openai_response': {
        const output = json.output as Array<{ type: string; content?: Array<{ type: string; text?: string }> }> | undefined
        const msg = output?.find((o) => o.type === 'message')
        return msg?.content?.[0]?.text ?? ''
      }
      case 'anthropic': {
        const content = json.content as Array<{ type: string; text?: string }> | undefined
        return content?.[0]?.text ?? ''
      }
      case 'openai_embedding': {
        const data = json.data as Array<{ embedding: number[] }> | undefined
        const emb = data?.[0]?.embedding
        if (!emb) return ''
        return `维度: ${emb.length} | 前 10 值: [${emb.slice(0, 10).map((v) => v.toFixed(6)).join(', ')}...]`
      }
      case 'openai_images': {
        const data = json.data as Array<{ url?: string; b64_json?: string }> | undefined
        const img = data?.[0]
        if (img?.url) return `![image](${img.url})`
        if (img?.b64_json) return `data:image/png;base64,${img.b64_json}`
        return ''
      }
      default:
        return JSON.stringify(json, null, 2)
    }
  }

  // 获取路由信息
  const fetchRouteInfo = async (apiKeyId: string, requestedModel: string) => {
    try {
      const logs = await statsApi.logs({ page: 1, page_size: 5 })
      const match = logs.items.find(
        (l) => l.api_key_id === apiKeyId && l.requested_model === requestedModel,
      )
      if (match) setRouteLog(match)
    } catch {
      // 忽略路由信息获取失败
    }
  }

  const handleSend = async () => {
    if (!selectedApiKey || !selectedModel) return

    setLoading(true)
    setError('')
    setRenderedContent('')
    setRawContent('')
    setStatusCode(0)
    setLatency(0)
    setRouteLog(null)

    const config = buildRequestConfig(
      protocol,
      selectedApiKey.api_key,
      selectedModel,
      prompt,
      stream && STREAMABLE_PROTOCOLS.has(protocol),
    )

    const controller = new AbortController()
    abortRef.current = controller
    startTimeRef.current = Date.now()

    try {
      const res = await fetch(config.path, {
        method: 'POST',
        headers: config.headers,
        body: JSON.stringify(config.body),
        signal: controller.signal,
      })

      setStatusCode(res.status)
      const elapsed = Date.now() - startTimeRef.current
      setLatency(elapsed)

      if (!res.ok) {
        const body = await res.json()
        setError(extractErrorMessage(body, protocol))
        setRawContent(JSON.stringify(body, null, 2))
        setTab('raw')
        setLoading(false)
        fetchRouteInfo(selectedApiKey.id, selectedModel)
        return
      }

      if (stream && STREAMABLE_PROTOCOLS.has(protocol) && res.body) {
        const reader = res.body.getReader()
        await parseStreamResponse(reader, protocol)
        setLoading(false)
        fetchRouteInfo(selectedApiKey.id, selectedModel)
      } else {
        const text = await res.text()
        setRawContent(text)
        try {
          const json = JSON.parse(text)
          const content = parseJsonResponse(json, protocol)
          setRenderedContent(content)
          // Images 特殊处理：base64 直接展示
          if (protocol === 'openai_images') {
            const data = json.data as Array<{ url?: string; b64_json?: string }> | undefined
            const img = data?.[0]
            if (img?.url) {
              setRenderedContent(`![image](${img.url})`)
            } else if (img?.b64_json) {
              setRenderedContent(`data:image/png;base64,${img.b64_json}`)
            }
          }
        } catch {
          setRenderedContent(text)
        }
        setLoading(false)
        fetchRouteInfo(selectedApiKey.id, selectedModel)
      }
    } catch (err) {
      if ((err as Error).name === 'AbortError') {
        setRawContent((prev) => prev + '\n\n--- 请求已取消 ---')
      } else {
        setError((err as Error).message)
      }
      setLoading(false)
    } finally {
      abortRef.current = null
    }
  }

  const handleStop = () => {
    abortRef.current?.abort()
  }

  const canStream = STREAMABLE_PROTOCOLS.has(protocol)

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">操练场</h1>
        <p className="text-sm text-muted-foreground mt-1">
          用真实客户端请求测试代理管线：认证 → 路由 → 转换 → 上游
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[360px_1fr] gap-6">
        {/* 配置面板 */}
        <Card>
          <CardContent className="p-5 space-y-4">
            <div className="space-y-1.5">
              <label className="text-sm font-medium">客户端协议</label>
              <select
                value={protocol}
                onChange={(e) => {
                  setProtocol(e.target.value as EndpointType)
                  if (!STREAMABLE_PROTOCOLS.has(e.target.value)) setStream(false)
                }}
                className="input"
              >
                {PLAYGROUND_PROTOCOLS.map((p) => (
                  <option key={p} value={p}>
                    {ENDPOINT_LABELS[p]}
                  </option>
                ))}
              </select>
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium">API Key</label>
              <select
                value={selectedApiKeyId}
                onChange={(e) => setSelectedApiKeyId(e.target.value)}
                className="input"
              >
                <option value="">选择 API Key</option>
                {apiKeys.map((k) => (
                  <option key={k.id} value={k.id}>
                    {k.name} (...{k.api_key.slice(-4)})
                  </option>
                ))}
              </select>
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium">模型</label>
              <select
                value={selectedModel}
                onChange={(e) => setSelectedModel(e.target.value)}
                className="input"
                disabled={!selectedApiKeyId}
              >
                <option value="">选择模型</option>
                {models.map((m) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
              </select>
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium">Prompt</label>
              <textarea
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder="Hello! Please introduce yourself briefly."
                rows={3}
                className="input resize-none"
              />
            </div>

            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={stream && canStream}
                onChange={(e) => setStream(e.target.checked)}
                disabled={!canStream}
                className="rounded"
              />
              流式输出
            </label>

            {loading ? (
              <Button onClick={handleStop} variant="destructive" className="w-full">
                <Square className="h-4 w-4" />
                停止
              </Button>
            ) : (
              <Button
                onClick={handleSend}
                className="w-full"
                disabled={!selectedApiKey || !selectedModel}
              >
                <Play className="h-4 w-4" />
                发送请求
              </Button>
            )}
          </CardContent>
        </Card>

        {/* 结果面板 */}
        <div className="space-y-4">
          {/* 路由信息 */}
          {routeLog && (
            <Card>
              <CardContent className="p-4">
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
                  <div>
                    <span className="text-muted-foreground">渠道</span>
                    <p className="font-medium truncate">{routeLog.channel_name ?? '-'}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">模型</span>
                    <p className="font-medium truncate">{routeLog.actual_model ?? routeLog.requested_model}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">端点</span>
                    <p className="font-medium truncate">{routeLog.endpoint_type ?? '-'}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">耗时</span>
                    <p className="font-medium">{routeLog.latency_ms ?? latency}ms</p>
                  </div>
                </div>
              </CardContent>
            </Card>
          )}

          {/* 状态栏 */}
          {(statusCode > 0 || error) && (
            <div className="flex items-center gap-3 text-sm">
              {error ? (
                <span className="text-destructive font-medium">✗ {error}</span>
              ) : (
                <span className="text-emerald-600 font-medium">
                  ✓ 成功 · {statusCode} · {(latency / 1000).toFixed(1)}s
                </span>
              )}
            </div>
          )}

          {/* 响应内容 */}
          {(renderedContent || rawContent || loading) && (
            <Card>
              <CardContent className="p-4">
                {/* Tab 切换 */}
                <div className="flex items-center gap-1 mb-3 border-b pb-2">
                  <button
                    onClick={() => setTab('rendered')}
                    className={`px-3 py-1 text-sm rounded-md transition-colors ${
                      tab === 'rendered'
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-accent'
                    }`}
                  >
                    渲染
                  </button>
                  <button
                    onClick={() => setTab('raw')}
                    className={`px-3 py-1 text-sm rounded-md transition-colors ${
                      tab === 'raw'
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-accent'
                    }`}
                  >
                    原始
                  </button>
                </div>

                {tab === 'rendered' && (
                  <div className="min-h-[200px] max-h-[500px] overflow-y-auto">
                    {loading && !renderedContent ? (
                      <div className="flex items-center gap-2 text-muted-foreground text-sm">
                        <RefreshCw className="h-4 w-4 animate-spin" />
                        等待响应...
                      </div>
                    ) : protocol === 'openai_images' && renderedContent.startsWith('data:image') ? (
                      <img src={renderedContent} alt="generated" className="max-w-full rounded-lg" />
                    ) : protocol === 'openai_images' && renderedContent.startsWith('![image]') ? (
                      <img src={renderedContent.match(/\(([^)]+)\)/)?.[1]} alt="generated" className="max-w-full rounded-lg" />
                    ) : (
                      <pre className="whitespace-pre-wrap text-sm font-mono leading-relaxed">
                        {renderedContent}
                      </pre>
                    )}
                    {loading && renderedContent && (
                      <span className="inline-block w-2 h-4 bg-primary animate-pulse ml-0.5" />
                    )}
                  </div>
                )}

                {tab === 'raw' && (
                  <div className="min-h-[200px] max-h-[500px] overflow-y-auto">
                    <pre className="whitespace-pre-wrap text-xs font-mono text-muted-foreground leading-relaxed">
                      {rawContent || (loading ? '等待数据...' : '')}
                    </pre>
                  </div>
                )}
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  )
}
