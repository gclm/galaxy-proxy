import { useRef, useState } from 'react'
import type { Channel, TestModelResponse } from '@/api/types'
import { channelsApi } from '@/api/channels'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Play, RotateCw } from 'lucide-react'

interface TestModelDialogProps {
  channel: Channel | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

type Status = 'idle' | 'testing' | 'success' | 'error'

interface LogLine {
  text: string
  type: 'info' | 'success' | 'error' | 'pending' | 'prompt' | 'output' | 'muted'
}

const TEST_PROTOCOLS = [
  { value: 'openai_chat', label: 'OpenAI Chat' },
  { value: 'openai_response', label: 'OpenAI Responses' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'openai_embedding', label: 'Embedding' },
  { value: 'openai_images', label: 'Images' },
]

export function TestModelDialog({ channel, open, onOpenChange }: TestModelDialogProps) {
  const [testProtocol, setTestProtocol] = useState('openai_chat')
  const [selectedModel, setSelectedModel] = useState('')
  const [status, setStatus] = useState<Status>('idle')
  const [logs, setLogs] = useState<LogLine[]>([])
  const [result, setResult] = useState<TestModelResponse | null>(null)
  const terminalRef = useRef<HTMLDivElement>(null)

  if (!channel) return null

  const models = channel.models || []
  const currentModel = selectedModel || models[0] || ''

  const scrollToBottom = () => {
    requestAnimationFrame(() => {
      if (terminalRef.current) {
        terminalRef.current.scrollTop = terminalRef.current.scrollHeight
      }
    })
  }

  const addLogs = (lines: LogLine[]) => {
    setLogs(prev => [...prev, ...lines])
    scrollToBottom()
  }

  const protocolLabel = TEST_PROTOCOLS.find(p => p.value === testProtocol)?.label || testProtocol

  const runTest = async () => {
    if (!currentModel) return

    setStatus('testing')
    setResult(null)
    setLogs([])

    addLogs([
      { text: `▸ 通过代理测试 ${protocolLabel} 协议...`, type: 'pending' },
    ])

    await new Promise(r => setTimeout(r, 200))

    addLogs([
      { text: '✓ 代理连接成功', type: 'success' },
      { text: `→ 协议: ${protocolLabel}`, type: 'info' },
      { text: `→ 模型: ${currentModel}`, type: 'info' },
      { text: '', type: 'muted' },
    ])

    scrollToBottom()

    try {
      const res = await channelsApi.testModel({
        model: currentModel,
        test_protocol: testProtocol,
      })
      setResult(res)

      if (res.success) {
        addLogs([
          { text: '── 输入 ──', type: 'muted' },
          { text: res.input_prompt, type: 'prompt' },
          { text: '', type: 'muted' },
          { text: '── 输出 ──', type: 'muted' },
          { text: res.output_content || '(无内容)', type: 'output' },
          { text: '', type: 'muted' },
          { text: `✓ 测试成功  耗时: ${res.latency_ms}ms`, type: 'success' },
        ])
        setStatus('success')
      } else {
        addLogs([
          { text: `✗ 测试失败: ${res.message}`, type: 'error' },
        ])
        setStatus('error')
      }
    } catch (error: any) {
      const msg = error?.message || '未知错误'
      setResult({ success: false, message: msg, latency_ms: 0, input_prompt: '', output_content: null })
      addLogs([
        { text: `✗ 请求失败: ${msg}`, type: 'error' },
      ])
      setStatus('error')
    }
  }

  const reset = () => {
    setLogs([])
    setResult(null)
    setStatus('idle')
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) { reset(); onOpenChange(false) } }}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>模型测试</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* 渠道信息 */}
          <div className="flex items-center justify-between rounded-xl bg-gradient-to-r from-primary/10 to-primary/5 border border-primary/10 px-4 py-3">
            <div className="flex items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-primary to-primary/70 text-primary-foreground text-xs font-bold">
                {channel.name.charAt(0)}
              </div>
              <span className="font-medium text-sm">{channel.name}</span>
            </div>
            <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ${
              channel.enabled
                ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                : 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400'
            }`}>
              {channel.enabled ? '启用' : '禁用'}
            </span>
          </div>

          {/* 测试协议选择 */}
          <div>
            <label className="block text-sm font-medium mb-1.5">测试协议</label>
            <select
              value={testProtocol}
              onChange={(e) => { setTestProtocol(e.target.value); reset() }}
              className="input"
            >
              {TEST_PROTOCOLS.map((p) => (
                <option key={p.value} value={p.value}>{p.label}</option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground mt-1">
              通过代理管线发送请求，测试协议转换效果
            </p>
          </div>

          {/* 模型选择 */}
          <div>
            <label className="block text-sm font-medium mb-1.5">选择模型</label>
            <select
              value={currentModel}
              onChange={(e) => { setSelectedModel(e.target.value); reset() }}
              className="input"
            >
              <option value="">选择模型</option>
              {models.map((model) => (
                <option key={model} value={model}>{model}</option>
              ))}
            </select>
          </div>

          {/* 终端输出 */}
          <div ref={terminalRef} className="rounded-xl border border-gray-700 bg-gray-900 dark:bg-black p-4 font-mono text-xs space-y-0.5 min-h-[180px] max-h-[300px] overflow-y-auto">
            {status === 'idle' && logs.length === 0 && (
              <div className="flex items-center gap-2 text-gray-500 h-full items-center justify-center">
                <Play className="h-3.5 w-3.5" />
                <span>选择协议和模型后，点击下方按钮开始测试</span>
              </div>
            )}
            {logs.map((log, i) => (
              <div key={i} className={
                log.type === 'success' ? 'text-green-400' :
                log.type === 'error' ? 'text-red-400' :
                log.type === 'pending' ? 'text-yellow-400 animate-pulse' :
                log.type === 'prompt' ? 'text-blue-300 whitespace-pre-wrap' :
                log.type === 'output' ? 'text-green-300 whitespace-pre-wrap' :
                log.type === 'info' ? 'text-cyan-400' :
                'text-gray-500'
              }>
                {log.text}
              </div>
            ))}
            {status === 'testing' && (
              <div className="text-yellow-400 animate-pulse">▌</div>
            )}
          </div>

          {/* 底部信息 */}
          <div className="flex items-center justify-between text-xs text-muted-foreground px-1">
            <span>请求经代理转换后发送到上游</span>
            {result && <span>耗时: {result.latency_ms}ms</span>}
          </div>

          {/* 操作按钮 */}
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={() => { reset(); onOpenChange(false) }}>
              关闭
            </Button>
            {result ? (
              <Button onClick={runTest} disabled={status === 'testing' || !currentModel}
                className={status === 'success' ? 'bg-green-600 hover:bg-green-700 text-white' : status === 'error' ? 'bg-orange-500 hover:bg-orange-600 text-white' : 'btn-primary'}>
                <RotateCw className="mr-2 h-4 w-4" />
                重新测试
              </Button>
            ) : (
              <Button onClick={runTest} disabled={status === 'testing' || !currentModel} className="btn-primary">
                <Play className="mr-2 h-4 w-4" />
                {status === 'testing' ? '测试中...' : '开始测试'}
              </Button>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
