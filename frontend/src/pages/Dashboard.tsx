import { useEffect, useState } from 'react'
import { apiClient, statsApi } from '@/api'
import type { StatsOverview, SystemInfo } from '@/api/types'
import { Activity, Coins, Cpu, MessageSquare, Radio, Layers, Key, Clock, Zap } from 'lucide-react'

function formatUptime(secs: number): string {
  const d = Math.floor(secs / 86400)
  const h = Math.floor((secs % 86400) / 3600)
  const m = Math.floor((secs % 3600) / 60)
  if (d > 0) return `${d}天 ${h}小时 ${m}分钟`
  if (h > 0) return `${h}小时 ${m}分钟`
  return `${m}分钟`
}

const statCards = [
  { key: 'requests' as const, label: '总请求数', icon: Activity, color: 'from-blue-500 to-blue-600' },
  { key: 'tokens' as const, label: '总 Token 数', icon: MessageSquare, color: 'from-violet-500 to-violet-600' },
  { key: 'cost' as const, label: '总成本', icon: Coins, color: 'from-amber-500 to-amber-600' },
] as const

const systemItems = [
  { key: 'version' as const, label: '版本', icon: Cpu },
  { key: 'uptime' as const, label: '运行时间', icon: Clock },
  { key: 'channels' as const, label: '渠道', icon: Radio },
  { key: 'groups' as const, label: '分组', icon: Layers },
  { key: 'keys' as const, label: 'API Key', icon: Key },
] as const

export function Dashboard() {
  const [overview, setOverview] = useState<StatsOverview | null>(null)
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    Promise.all([
      statsApi.overview().catch(() => null),
      apiClient.get<SystemInfo>('/system-info').catch(() => null),
    ]).then(([stats, sys]) => {
      if (stats) setOverview(stats)
      if (sys) setSystemInfo(sys)
      setLoading(false)
    })
  }, [])

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

  const formatStat = (key: typeof statCards[number]['key']) => {
    if (!overview) return '0'
    if (key === 'requests') return (overview.total_requests ?? 0).toLocaleString()
    if (key === 'tokens') return ((overview.total_input_tokens + overview.total_output_tokens) ?? 0).toLocaleString()
    return `$${(overview.total_cost ?? 0).toFixed(4)}`
  }

  const formatSysValue = (key: typeof systemItems[number]['key']) => {
    if (!systemInfo) return '-'
    if (key === 'version') return `v${systemInfo.version}`
    if (key === 'uptime') return formatUptime(systemInfo.uptime_secs)
    if (key === 'channels') return `${systemInfo.channel_count} 个`
    if (key === 'groups') return `${systemInfo.group_count} 个`
    return `${systemInfo.api_key_count} 个`
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">Galaxy Router 运行概览</p>
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Zap className="h-3.5 w-3.5 text-green-500" />
          {systemInfo ? '运行中' : '状态未知'}
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-3">
        {statCards.map((card) => (
          <div
            key={card.key}
            className="card-hover rounded-2xl border bg-card p-5 flex items-start gap-4"
          >
            <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br ${card.color} text-white shadow-sm`}>
              <card.icon className="h-5 w-5" />
            </div>
            <div className="min-w-0">
              <p className="text-sm text-muted-foreground">{card.label}</p>
              <p className="text-2xl font-bold tracking-tight mt-0.5">
                {formatStat(card.key)}
              </p>
            </div>
          </div>
        ))}
      </div>

      <div className="rounded-2xl border bg-card p-5">
        <h2 className="text-sm font-medium text-muted-foreground mb-4">系统信息</h2>
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-4">
          {systemItems.map((item) => (
            <div
              key={item.key}
              className="rounded-xl bg-muted/50 px-4 py-3 flex items-center gap-3"
            >
              <item.icon className="h-4 w-4 text-muted-foreground shrink-0" />
              <div className="min-w-0">
                <p className="text-xs text-muted-foreground">{item.label}</p>
                <p className="text-sm font-medium truncate">{formatSysValue(item.key)}</p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
