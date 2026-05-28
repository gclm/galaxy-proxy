import { useCallback, useEffect, useState } from 'react'
import { formatDate } from '@/lib/utils'
import { statsApi } from '@/api/stats'
import type { RequestLog, RequestLogDetail } from '@/api/types'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Search,
  RefreshCw,
  ChevronLeft,
  ChevronRight,
  CheckCircle2,
  XCircle,
  Copy,
  Check,
  Loader2,
  Clock,
  Zap,
  ArrowDownToLine,
  ArrowUpFromLine,
  DollarSign,
  Send,
  MessageSquare,
  AlertCircle,
} from 'lucide-react'

function formatNumber(n: number | undefined) {
  return (n ?? 0).toLocaleString()
}

function formatCost(n: number | null) {
  return n != null ? `$${n.toFixed(6)}` : '-'
}

function formatLatency(ms: number | null) {
  if (ms == null) return '-'
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(2)}s`
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)
  const handleCopy = () => {
    navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }
  return (
    <button onClick={handleCopy} className="shrink-0 p-1 rounded hover:bg-muted/80 text-muted-foreground hover:text-foreground transition-colors" title="复制">
      {copied ? <Check className="h-3.5 w-3.5 text-green-500" /> : <Copy className="h-3.5 w-3.5" />}
    </button>
  )
}

function tryParseJson(text: string): { isJson: boolean; data: unknown } {
  try {
    const parsed = JSON.parse(text)
    if (typeof parsed === 'object' && parsed !== null) {
      return { isJson: true, data: parsed }
    }
    return { isJson: false, data: text }
  } catch {
    return { isJson: false, data: text }
  }
}

function JsonBlock({ content, fallback }: { content: string | null; fallback: string }) {
  if (!content) {
    return <pre className="p-4 text-xs text-muted-foreground whitespace-pre-wrap">{fallback}</pre>
  }

  const { isJson, data } = tryParseJson(content)
  const displayText = isJson ? JSON.stringify(data, null, 2) : String(data)

  return (
    <div className="relative">
      <div className="sticky top-2 float-right mr-2 z-10">
        <CopyButton text={displayText} />
      </div>
      <pre className="p-4 text-xs font-mono whitespace-pre-wrap break-all leading-relaxed text-foreground/90">
        {displayText}
      </pre>
    </div>
  )
}

export function Logs() {
  const [logs, setLogs] = useState<RequestLog[]>([])
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)

  const [page, setPage] = useState(1)
  const pageSize = 20

  const [searchModel, setSearchModel] = useState('')
  const [searchModelInput, setSearchModelInput] = useState('')
  const [status, setStatus] = useState('')

  const [detailLog, setDetailLog] = useState<RequestLog | null>(null)
  const [logDetail, setLogDetail] = useState<RequestLogDetail | null>(null)
  const [detailLoading, setDetailLoading] = useState(false)

  useEffect(() => {
    const timer = setTimeout(() => {
      setSearchModel(searchModelInput)
      setPage(1)
    }, 300)
    return () => clearTimeout(timer)
  }, [searchModelInput])

  const fetchLogs = useCallback(async () => {
    setLoading(true)
    try {
      const data = await statsApi.logs({
        page,
        page_size: pageSize,
        model: searchModel || undefined,
        status: status || undefined,
      })
      setLogs(data.items)
      setTotal(data.total)
    } catch (error) {
      console.error('Failed to fetch logs:', error)
    } finally {
      setLoading(false)
    }
  }, [page, searchModel, status])

  useEffect(() => { fetchLogs() }, [fetchLogs])

  const openDetail = async (log: RequestLog) => {
    setDetailLog(log)
    setLogDetail(null)
    setDetailLoading(true)
    try {
      const detail = await statsApi.logDetail(log.id)
      setLogDetail(detail)
    } catch {
      console.error('Failed to fetch log detail')
    } finally {
      setDetailLoading(false)
    }
  }

  const closeDetail = () => {
    setDetailLog(null)
    setLogDetail(null)
  }

  const totalPages = Math.max(1, Math.ceil(total / pageSize))

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">查看每次 API 请求的详细记录</p>
      </div>

      {/* 筛选栏 */}
      <div className="flex items-center gap-3 flex-wrap">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <input
            type="text"
            value={searchModelInput}
            onChange={(e) => setSearchModelInput(e.target.value)}
            placeholder="搜索模型名称..."
            className="input pl-9"
          />
        </div>
        <select
          value={status}
          onChange={(e) => { setStatus(e.target.value); setPage(1) }}
          className="input w-28"
        >
          <option value="">全部状态</option>
          <option value="success">成功</option>
          <option value="failure">失败</option>
        </select>
        <Button variant="outline" size="icon" onClick={fetchLogs} title="刷新">
          <RefreshCw className="h-4 w-4" />
        </Button>
      </div>

      {/* 表格 */}
      <div className="rounded-2xl border bg-card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b bg-muted/50">
                <th className="text-left px-4 py-3 font-medium whitespace-nowrap">时间</th>
                <th className="text-left px-4 py-3 font-medium">模型</th>
                <th className="text-left px-4 py-3 font-medium">渠道</th>
                <th className="text-left px-4 py-3 font-medium">Key</th>
                <th className="text-center px-4 py-3 font-medium">协议</th>
                <th className="text-center px-4 py-3 font-medium">类型</th>
                <th className="text-center px-4 py-3 font-medium">状态</th>
                <th className="text-right px-4 py-3 font-medium">输入</th>
                <th className="text-right px-4 py-3 font-medium">输出</th>
                <th className="text-right px-4 py-3 font-medium">耗时</th>
                <th className="text-right px-4 py-3 font-medium">成本</th>
              </tr>
            </thead>
            <tbody>
              {loading ? (
                <tr>
                  <td colSpan={11} className="text-center py-12 text-muted-foreground">加载中...</td>
                </tr>
              ) : logs.length === 0 ? (
                <tr>
                  <td colSpan={11} className="text-center py-12 text-muted-foreground">暂无请求日志</td>
                </tr>
              ) : (
                logs.map((log) => (
                  <tr
                    key={log.id}
                    className="border-b last:border-0 hover:bg-muted/30 transition-colors cursor-pointer"
                    onClick={() => openDetail(log)}
                  >
                    <td className="px-4 py-3 text-xs text-muted-foreground whitespace-nowrap">{formatDate(log.created_at)}</td>
                    <td className="px-4 py-3">
                      <div>
                        <p className="font-medium text-sm">{log.requested_model}</p>
                        {log.actual_model && log.actual_model !== log.requested_model && (
                          <p className="text-xs text-muted-foreground">→ {log.actual_model}</p>
                        )}
                      </div>
                    </td>
                    <td className="px-4 py-3 text-muted-foreground text-xs">{log.channel_name ?? '-'}</td>
                    <td className="px-4 py-3 text-muted-foreground text-xs">{log.api_key_name ?? '-'}</td>
                    <td className="px-4 py-3 text-center">
                      <span className="inline-flex items-center rounded-md bg-primary/10 px-1.5 py-0.5 text-xs font-medium text-primary">
                        {log.endpoint_type ?? '-'}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-center">
                      <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                        log.request_type === 'passthrough'
                          ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400'
                          : 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400'
                      }`}>
                        {log.request_type === 'passthrough' ? '直通' : '转换'}
                      </span>
                      {log.is_stream && (
                        <span className="ml-1 inline-flex items-center rounded-full px-1.5 py-0.5 text-xs font-medium bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
                          流式
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-center">
                      {log.error_message ? (
                        <span className="inline-flex items-center gap-1 text-destructive text-xs">
                          <XCircle className="h-3.5 w-3.5" />
                          {log.status_code ?? 'ERR'}
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1 text-green-600 text-xs">
                          <CheckCircle2 className="h-3.5 w-3.5" />
                          {log.status_code ?? 200}
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-right text-xs">{formatNumber(log.input_tokens)}</td>
                    <td className="px-4 py-3 text-right text-xs">{formatNumber(log.output_tokens)}</td>
                    <td className="px-4 py-3 text-right text-xs text-muted-foreground">
                      {formatLatency(log.latency_ms)}
                    </td>
                    <td className="px-4 py-3 text-right text-xs">{formatCost(log.cost)}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        {/* 分页 */}
        <div className="flex items-center justify-between px-4 py-3 border-t bg-muted/30">
          <span className="text-sm text-muted-foreground">共 {total} 条</span>
          <div className="flex items-center gap-1">
            <Button variant="outline" size="icon" className="h-8 w-8" disabled={page <= 1} onClick={() => setPage(page - 1)}>
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <span className="px-3 text-sm">{page} / {totalPages}</span>
            <Button variant="outline" size="icon" className="h-8 w-8" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* 详情弹窗 */}
      <Dialog open={!!detailLog} onOpenChange={(open) => { if (!open) closeDetail() }}>
        <DialogContent className="max-w-4xl h-[85vh] overflow-hidden flex flex-col p-0 gap-0">
          {detailLog && (
            <>
              <DialogHeader className="px-6 pt-6 pb-0">
                <DialogTitle className="flex items-center gap-3 text-base">
                  <span className="font-semibold">{detailLog.requested_model}</span>
                  {detailLog.actual_model && detailLog.actual_model !== detailLog.requested_model && (
                    <>
                      <span className="text-muted-foreground">→</span>
                      <span className="text-muted-foreground">{detailLog.actual_model}</span>
                    </>
                  )}
                  {detailLog.error_message ? (
                    <span className="ml-2 inline-flex items-center gap-1 text-xs text-destructive font-normal">
                      <XCircle className="h-3.5 w-3.5" />
                      {detailLog.status_code ?? 'ERR'}
                    </span>
                  ) : (
                    <span className="ml-2 inline-flex items-center gap-1 text-xs text-green-600 font-normal">
                      <CheckCircle2 className="h-3.5 w-3.5" />
                      {detailLog.status_code ?? 200}
                    </span>
                  )}
                </DialogTitle>
              </DialogHeader>

              {/* 指标条 */}
              <div className="flex flex-wrap items-center gap-x-5 gap-y-2 px-6 py-3 text-xs text-muted-foreground border-b">
                <div className="flex items-center gap-1.5">
                  <Clock className="h-3.5 w-3.5" />
                  <span className="tabular-nums">{formatDate(detailLog.created_at)}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span>渠道: {detailLog.channel_name ?? '-'}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span>Key: {detailLog.api_key_name ?? '-'}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <Zap className="h-3.5 w-3.5 text-amber-500" />
                  <span>耗时 {formatLatency(detailLog.latency_ms)}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <ArrowDownToLine className="h-3.5 w-3.5 text-green-500" />
                  <span>输入 {formatNumber(detailLog.input_tokens)}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <ArrowUpFromLine className="h-3.5 w-3.5 text-purple-500" />
                  <span>输出 {formatNumber(detailLog.output_tokens)}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <DollarSign className="h-3.5 w-3.5 text-emerald-500" />
                  <span className="font-medium text-emerald-600 dark:text-emerald-400">{formatCost(detailLog.cost)}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="inline-flex items-center rounded-md bg-primary/10 px-1.5 py-0.5 text-primary">
                    {detailLog.endpoint_type ?? '-'}
                  </span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                    detailLog.request_type === 'passthrough'
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400'
                      : 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400'
                  }`}>
                    {detailLog.request_type === 'passthrough' ? '直通' : '转换'}
                  </span>
                  <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                    detailLog.is_stream
                      ? 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400'
                      : 'bg-slate-100 text-slate-600 dark:bg-slate-800/30 dark:text-slate-400'
                  }`}>
                    {detailLog.is_stream ? '流式' : '非流式'}
                  </span>
                </div>
              </div>

              {/* 错误信息 */}
              {detailLog.error_message && (
                <div className="mx-6 mt-4 p-3 rounded-xl bg-destructive/10 border border-destructive/20">
                  <div className="flex items-start gap-2">
                    <AlertCircle className="h-4 w-4 shrink-0 mt-0.5 text-destructive" />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-sm font-medium text-destructive">错误信息</span>
                        <CopyButton text={detailLog.error_message} />
                      </div>
                      <pre className="text-xs text-destructive whitespace-pre-wrap break-all leading-relaxed">{detailLog.error_message}</pre>
                    </div>
                  </div>
                </div>
              )}

              {/* 请求/响应内容 */}
              <div className="flex-1 min-h-0 px-6 py-4">
                {detailLoading ? (
                  <div className="flex items-center justify-center h-48">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                    <span className="ml-2 text-sm text-muted-foreground">加载内容...</span>
                  </div>
                ) : logDetail ? (
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4 h-full min-h-0">
                    <div className="flex flex-col rounded-xl border bg-muted/30 min-h-0 overflow-hidden">
                      <div className="flex items-center gap-2 px-3 py-2.5 border-b bg-muted/50 shrink-0">
                        <Send className="h-4 w-4 text-green-500" />
                        <span className="text-sm font-medium">请求内容</span>
                        <span className="ml-auto text-xs text-muted-foreground">{formatNumber(detailLog.input_tokens)} tokens</span>
                      </div>
                      <div className="flex-1 min-h-0 overflow-auto">
                        <JsonBlock content={logDetail.request_content} fallback="无请求内容" />
                      </div>
                    </div>
                    <div className="flex flex-col rounded-xl border bg-muted/30 min-h-0 overflow-hidden">
                      <div className="flex items-center gap-2 px-3 py-2.5 border-b bg-muted/50 shrink-0">
                        <MessageSquare className="h-4 w-4 text-purple-500" />
                        <span className="text-sm font-medium">响应内容</span>
                        <span className="ml-auto text-xs text-muted-foreground">{formatNumber(detailLog.output_tokens)} tokens</span>
                      </div>
                      <div className="flex-1 min-h-0 overflow-auto">
                        <JsonBlock content={logDetail.response_content} fallback="无响应内容" />
                      </div>
                    </div>
                  </div>
                ) : null}
              </div>
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}
