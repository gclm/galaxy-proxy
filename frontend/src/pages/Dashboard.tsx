import { useEffect, useState, useMemo, useRef } from 'react'
import { apiClient, statsApi, type StatsParams } from '@/api'
import type { StatsOverview, SystemInfo, DailyStats, ModelStats, ChannelStats } from '@/api/types'
import {
  Activity, MessageSquare, Coins,
  Cpu, Clock, Radio, Layers, Key,
} from 'lucide-react'
import {
  BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer,
  LineChart, Line, CartesianGrid, PieChart, Pie, Cell, Legend,
} from 'recharts'

const CHART_COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff7300', '#0088fe', '#00C49F']

const RANGE_TABS = [
  { label: '今天', days: 1 },
  { label: '7天', days: 7 },
  { label: '14天', days: 14 },
  { label: '30天', days: 30 },
  { label: '90天', days: 90 },
] as const

function formatUptime(secs: number): string {
  const d = Math.floor(secs / 86400)
  const h = Math.floor((secs % 86400) / 3600)
  const m = Math.floor((secs % 3600) / 60)
  if (d > 0) return `${d}天 ${h}小时`
  if (h > 0) return `${h}小时 ${m}分钟`
  return `${m}分钟`
}

const fmt = (n: number) => n.toLocaleString()
const fmtCost = (n: number) => n.toFixed(4)
const fmtTokens = (n: number) => {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toLocaleString()
}

export function Dashboard() {
  const [overview, setOverview] = useState<StatsOverview | null>(null)
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null)
  const [daily, setDaily] = useState<DailyStats[]>([])
  const [models, setModels] = useState<ModelStats[]>([])
  const [channels, setChannels] = useState<ChannelStats[]>([])
  const [loading, setLoading] = useState(true)

  const [activeRange, setActiveRange] = useState(1)
  const [customStart, setCustomStart] = useState('')
  const [customEnd, setCustomEnd] = useState('')
  const [customMode, setCustomMode] = useState(false)

  const chartParams = useMemo<StatsParams>(() => {
    if (customMode && customStart && customEnd) {
      return { start_date: customStart, end_date: customEnd }
    }
    return { days: activeRange }
  }, [customMode, customStart, customEnd, activeRange])

  const initialFetchDone = useRef(false)

  useEffect(() => {
    const fetch = async () => {
      const params = chartParams
      if (!initialFetchDone.current) {
        initialFetchDone.current = true
        const [stats, sys, dailyData, modelsData, channelsData] = await Promise.all([
          statsApi.overview().catch(() => null),
          apiClient.get<SystemInfo>('/system-info').catch(() => null),
          statsApi.daily(params).catch(() => []),
          statsApi.models(params).catch(() => []),
          statsApi.channels(params).catch(() => []),
        ])
        if (stats) setOverview(stats)
        if (sys) setSystemInfo(sys as SystemInfo)
        setDaily(dailyData as DailyStats[])
        setModels(modelsData as ModelStats[])
        setChannels(channelsData as ChannelStats[])
        setLoading(false)
      } else {
        const [d, m, c] = await Promise.all([
          statsApi.daily(params).catch(() => null),
          statsApi.models(params).catch(() => null),
          statsApi.channels(params).catch(() => null),
        ])
        if (d) setDaily(d)
        if (m) setModels(m)
        if (c) setChannels(c)
      }
    }
    fetch()
  }, [chartParams])

  const handleRangeTab = (days: number) => {
    setCustomMode(false)
    setActiveRange(days)
  }

  const handleCustomApply = () => {
    if (customStart && customEnd) setCustomMode(true)
  }

  const sortedDaily = useMemo(() =>
    [...daily].sort((a, b) => a.date.localeCompare(b.date))
  , [daily])

  if (loading) {
    return <div className="flex items-center justify-center h-full text-muted-foreground">加载中...</div>
  }

  const today = overview
    ? { requests: overview.today_requests, tokens: overview.today_input_tokens + overview.today_output_tokens, cost: overview.today_cost }
    : { requests: 0, tokens: 0, cost: 0 }
  const total = overview
    ? { requests: overview.total_requests, tokens: overview.total_input_tokens + overview.total_output_tokens, cost: overview.total_cost }
    : { requests: 0, tokens: 0, cost: 0 }

  return (
    <div className="space-y-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold">仪表盘</h1>
          <p className="text-sm text-muted-foreground mt-0.5">Galaxy Router 运行概览</p>
        </div>
        {systemInfo && (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-75" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500" />
            </span>
            运行中
          </div>
        )}
      </div>

      {/* KPI Cards */}
      <div className="grid gap-4 sm:grid-cols-3">
        {[
          { label: '今日请求', value: fmt(today.requests), sub: `累计 ${fmt(total.requests)}`, icon: Activity, gradient: 'from-blue-500 to-blue-600' },
          { label: '今日 Token', value: fmtTokens(today.tokens), sub: `累计 ${fmtTokens(total.tokens)}`, icon: MessageSquare, gradient: 'from-violet-500 to-violet-600' },
          { label: '今日成本', value: `$${fmtCost(today.cost)}`, sub: `累计 $${fmtCost(total.cost)}`, icon: Coins, gradient: 'from-amber-500 to-amber-600' },
        ].map((item) => (
          <div key={item.label} className="card-hover rounded-2xl border bg-card p-4">
            <div className="flex items-start gap-3">
              <div className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br ${item.gradient} text-white shadow-sm`}>
                <item.icon className="h-4 w-4" />
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-xs text-muted-foreground">{item.label}</p>
                <p className="text-2xl font-bold tracking-tight leading-7">{item.value}</p>
                <p className="text-[11px] text-muted-foreground/70">{item.sub}</p>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* System Info Strip */}
      {systemInfo && (
        <div className="rounded-2xl border bg-card p-4">
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3">
            {[
              { label: '版本', value: `v${systemInfo.version}`, icon: Cpu },
              { label: '运行时间', value: formatUptime(systemInfo.uptime_secs), icon: Clock },
              { label: '渠道', value: `${systemInfo.channel_count} 个`, icon: Radio },
              { label: '分组', value: `${systemInfo.group_count} 个`, icon: Layers },
              { label: 'API Key', value: `${systemInfo.api_key_count} 个`, icon: Key },
            ].map((item) => (
              <div key={item.label} className="flex items-center gap-2.5 rounded-xl bg-muted/50 px-3 py-2.5">
                <item.icon className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <div className="min-w-0">
                  <p className="text-[11px] text-muted-foreground leading-tight">{item.label}</p>
                  <p className="text-sm font-medium truncate leading-tight mt-0.5">{item.value}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Charts Section */}
      <div className="rounded-2xl border bg-card">
        {/* Chart Header with Range Tabs */}
        <div className="flex flex-wrap items-center justify-between gap-3 border-b px-5 py-3.5">
          <h2 className="text-sm font-semibold">趋势分析</h2>
          <div className="flex items-center gap-2">
            {RANGE_TABS.map((tab) => (
              <button
                key={tab.days}
                onClick={() => handleRangeTab(tab.days)}
                className={`rounded-lg px-3 py-1 text-xs font-medium transition-colors ${
                  !customMode && activeRange === tab.days
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:bg-muted'
                }`}
              >
                {tab.label}
              </button>
            ))}
            <span className="text-border mx-1">|</span>
            <input type="date" value={customStart} onChange={(e) => setCustomStart(e.target.value)} className="input h-7 w-32 text-xs py-0 px-2" />
            <span className="text-xs text-muted-foreground">至</span>
            <input type="date" value={customEnd} onChange={(e) => setCustomEnd(e.target.value)} className="input h-7 w-32 text-xs py-0 px-2" />
            <button
              onClick={handleCustomApply}
              disabled={!customStart || !customEnd}
              className="rounded-lg bg-primary px-3 py-1 text-xs font-medium text-primary-foreground disabled:opacity-50 h-7"
            >
              查询
            </button>
          </div>
        </div>

        <div className="p-5 space-y-5">
          {/* Row 1: Request trend + Token trend */}
          <div className="grid gap-5 md:grid-cols-2">
            <ChartCard title="请求趋势" data={sortedDaily} dataKey="request_count" color="#8884d8" emptyText="暂无请求数据" />
            <TokenTrendChart data={sortedDaily} />
          </div>

          {/* Row 2: Model distribution + Channel requests */}
          <div className="grid gap-5 md:grid-cols-2">
            <ModelDistributionChart models={models} />
            <ChannelBarChart channels={channels} />
          </div>

          {/* Row 3: Cost trend (full width) */}
          <CostTrendChart data={sortedDaily} />
        </div>
      </div>
    </div>
  )
}

/* ---- Sub-components ---- */

function ChartCard({ title, data, dataKey, color, emptyText }: {
  title: string
  data: DailyStats[]
  dataKey: string
  color: string
  emptyText: string
}) {
  return (
    <div>
      <h3 className="text-xs font-medium text-muted-foreground mb-3">{title}</h3>
      {data.length > 0 ? (
        <ResponsiveContainer width="100%" height={220}>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} className="text-muted-foreground" />
            <YAxis tick={{ fontSize: 11 }} className="text-muted-foreground" />
            <Tooltip formatter={(v) => fmt(Number(v))} />
            <Line type="monotone" dataKey={dataKey} stroke={color} strokeWidth={2} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      ) : (
        <div className="flex h-[220px] items-center justify-center text-sm text-muted-foreground">{emptyText}</div>
      )}
    </div>
  )
}

function TokenTrendChart({ data }: { data: DailyStats[] }) {
  return (
    <div>
      <h3 className="text-xs font-medium text-muted-foreground mb-3">Token 用量趋势</h3>
      {data.length > 0 ? (
        <ResponsiveContainer width="100%" height={220}>
          <LineChart data={data}>
            <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} />
            <YAxis tick={{ fontSize: 11 }} />
            <Tooltip formatter={(v) => fmt(Number(v))} />
            <Legend wrapperStyle={{ fontSize: 11 }} />
            <Line type="monotone" dataKey="input_tokens" stroke="#8884d8" strokeWidth={2} dot={false} name="输入" />
            <Line type="monotone" dataKey="output_tokens" stroke="#82ca9d" strokeWidth={2} dot={false} name="输出" />
          </LineChart>
        </ResponsiveContainer>
      ) : (
        <div className="flex h-[220px] items-center justify-center text-sm text-muted-foreground">暂无 Token 数据</div>
      )}
    </div>
  )
}

function ModelDistributionChart({ models }: { models: ModelStats[] }) {
  return (
    <div>
      <h3 className="text-xs font-medium text-muted-foreground mb-3">模型分布</h3>
      {models.length > 0 ? (
        <div className="flex gap-4">
          <div className="w-1/2">
            <ResponsiveContainer width="100%" height={200}>
              <PieChart>
                <Pie data={models.slice(0, 6)} dataKey="request_count" nameKey="model" outerRadius={80} labelLine={false}>
                  {models.slice(0, 6).map((_, i) => (
                    <Cell key={i} fill={CHART_COLORS[i % CHART_COLORS.length]} />
                  ))}
                </Pie>
                <Tooltip formatter={(v) => fmt(Number(v))} />
              </PieChart>
            </ResponsiveContainer>
          </div>
          <div className="w-1/2 overflow-auto max-h-[200px]">
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b text-muted-foreground">
                  <th className="text-left py-1 font-medium">#</th>
                  <th className="text-left py-1 font-medium">模型</th>
                  <th className="text-right py-1 font-medium">请求</th>
                  <th className="text-right py-1 font-medium">成本</th>
                </tr>
              </thead>
              <tbody>
                {models.map((m, i) => (
                  <tr key={m.model} className="border-b last:border-0">
                    <td className="py-1 text-muted-foreground">{i + 1}</td>
                    <td className="py-1 font-medium max-w-[140px] truncate" title={m.model}>{m.model}</td>
                    <td className="py-1 text-right">{fmt(m.request_count)}</td>
                    <td className="py-1 text-right">${fmtCost(m.total_cost)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : (
        <div className="flex h-[200px] items-center justify-center text-sm text-muted-foreground">暂无模型数据</div>
      )}
    </div>
  )
}

function ChannelBarChart({ channels }: { channels: ChannelStats[] }) {
  return (
    <div>
      <h3 className="text-xs font-medium text-muted-foreground mb-3">渠道请求量</h3>
      {channels.length > 0 ? (
        <ResponsiveContainer width="100%" height={200}>
          <BarChart data={channels.slice(0, 8)}>
            <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
            <XAxis dataKey="channel_name" tick={{ fontSize: 10 }} />
            <YAxis tick={{ fontSize: 11 }} />
            <Tooltip formatter={(v) => fmt(Number(v))} />
            <Bar dataKey="request_count" fill="#82ca9d" radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      ) : (
        <div className="flex h-[200px] items-center justify-center text-sm text-muted-foreground">暂无渠道数据</div>
      )}
    </div>
  )
}

function CostTrendChart({ data }: { data: DailyStats[] }) {
  return (
    <div>
      <h3 className="text-xs font-medium text-muted-foreground mb-3">每日成本</h3>
      {data.length > 0 ? (
        <ResponsiveContainer width="100%" height={200}>
          <BarChart data={data}>
            <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
            <XAxis dataKey="date" tick={{ fontSize: 11 }} />
            <YAxis tick={{ fontSize: 11 }} />
            <Tooltip formatter={(v) => `$${fmtCost(Number(v))}`} />
            <Bar dataKey="total_cost" fill="#ffc658" radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      ) : (
        <div className="flex h-[200px] items-center justify-center text-sm text-muted-foreground">暂无成本数据</div>
      )}
    </div>
  )
}
