import { useEffect, useState, useMemo } from 'react'
import { statsApi, type StatsParams } from '@/api/stats'
import type { DailyStats, ModelStats, ChannelStats } from '@/api/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Activity, MessageSquare, Coins, Zap, Gauge } from 'lucide-react'
import {
  BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer,
  LineChart, Line, CartesianGrid, PieChart, Pie, Cell, Legend,
} from 'recharts'

const COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff7300', '#0088fe', '#00C49F']

const QUICK_RANGES = [
  { label: '今天', days: 1 },
  { label: '7天', days: 7 },
  { label: '14天', days: 14 },
  { label: '30天', days: 30 },
  { label: '90天', days: 90 },
]

export function Stats() {
  const [daily, setDaily] = useState<DailyStats[]>([])
  const [models, setModels] = useState<ModelStats[]>([])
  const [channels, setChannels] = useState<ChannelStats[]>([])
  const [loading, setLoading] = useState(true)

  const [activeRange, setActiveRange] = useState(30)
  const [startDate, setStartDate] = useState('')
  const [endDate, setEndDate] = useState('')
  const [customMode, setCustomMode] = useState(false)

  const params = useMemo<StatsParams>(() => {
    if (customMode && startDate && endDate) {
      return { start_date: startDate, end_date: endDate }
    }
    return { days: activeRange }
  }, [customMode, startDate, endDate, activeRange])

  useEffect(() => {
    fetchStats()
  }, [params])

  const fetchStats = async () => {
    setLoading(true)
    try {
      const [dailyData, modelsData, channelsData] = await Promise.all([
        statsApi.daily(params),
        statsApi.models(params),
        statsApi.channels(params),
      ])
      setDaily(dailyData)
      setModels(modelsData)
      setChannels(channelsData)
    } catch (error) {
      console.error('Failed to fetch stats:', error)
    } finally {
      setLoading(false)
    }
  }

  const formatNumber = (n: number | undefined) => (n ?? 0).toLocaleString()
  const formatCost = (n: number | undefined) => (n ?? 0).toFixed(4)

  // 按日期聚合 daily 数据（后端返回的可能是按 model 分组的）
  const aggregatedDaily = useMemo(() => {
    const map = new Map<string, { request_count: number; input_tokens: number; output_tokens: number; total_cost: number; success_count: number; failure_count: number }>()
    for (const d of daily) {
      const existing = map.get(d.date) ?? { request_count: 0, input_tokens: 0, output_tokens: 0, total_cost: 0, success_count: 0, failure_count: 0 }
      existing.request_count += d.request_count ?? 0
      existing.input_tokens += d.input_tokens ?? 0
      existing.output_tokens += d.output_tokens ?? 0
      existing.total_cost += d.total_cost ?? 0
      existing.success_count += d.success_count ?? 0
      existing.failure_count += d.failure_count ?? 0
      map.set(d.date, existing)
    }
    return Array.from(map.entries())
      .map(([date, v]) => ({ date, ...v }))
      .sort((a, b) => a.date.localeCompare(b.date))
  }, [daily])

  const totalRequests = aggregatedDaily.reduce((s, d) => s + d.request_count, 0)
  const totalTokens = aggregatedDaily.reduce((s, d) => s + d.input_tokens + d.output_tokens, 0)
  const totalCost = aggregatedDaily.reduce((s, d) => s + d.total_cost, 0)
  const dayCount = aggregatedDaily.length

  // RPM / TPM: 平均每分钟的请求和 Token
  const totalMinutes = dayCount * 24 * 60
  const rpm = totalMinutes > 0 ? totalRequests / totalMinutes : 0
  const tpm = totalMinutes > 0 ? totalTokens / totalMinutes : 0

  const handleQuickRange = (days: number) => {
    setCustomMode(false)
    setActiveRange(days)
  }

  const handleCustomApply = () => {
    if (startDate && endDate) {
      setCustomMode(true)
    }
  }

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">查看请求量、Token 用量与成本趋势</p>
        <div className="flex items-center gap-2">
          {QUICK_RANGES.map((r) => (
            <button
              key={r.days}
              onClick={() => handleQuickRange(r.days)}
              className={`rounded-lg px-3 py-1.5 text-xs font-medium transition-colors ${
                !customMode && activeRange === r.days
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted hover:bg-muted/80'
              }`}
            >
              {r.label}
            </button>
          ))}
          <span className="text-muted-foreground mx-1">|</span>
          <input
            type="date"
            value={startDate}
            onChange={(e) => setStartDate(e.target.value)}
            className="input w-36 text-xs"
          />
          <span className="text-muted-foreground text-xs">至</span>
          <input
            type="date"
            value={endDate}
            onChange={(e) => setEndDate(e.target.value)}
            className="input w-36 text-xs"
          />
          <button
            onClick={handleCustomApply}
            disabled={!startDate || !endDate}
            className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground disabled:opacity-50"
          >
            查询
          </button>
        </div>
      </div>

      {/* 总览卡片 */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
        {[
          { label: '总请求数', value: formatNumber(totalRequests), color: 'from-blue-500 to-blue-600', icon: Activity },
          { label: '总 Token 数', value: formatNumber(totalTokens), color: 'from-violet-500 to-violet-600', icon: MessageSquare },
          { label: '总成本', value: `$${formatCost(totalCost)}`, color: 'from-amber-500 to-amber-600', icon: Coins },
          { label: '平均 RPM', value: rpm.toFixed(1), color: 'from-rose-500 to-rose-600', icon: Zap },
          { label: '平均 TPM', value: formatNumber(Math.round(tpm)), color: 'from-cyan-500 to-cyan-600', icon: Gauge },
        ].map((item) => (
          <div key={item.label} className="card-hover rounded-2xl border bg-card p-5 flex items-start gap-4">
            <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br ${item.color} text-white shadow-sm`}>
              <item.icon className="h-5 w-5" />
            </div>
            <div className="min-w-0">
              <p className="text-sm text-muted-foreground">{item.label}</p>
              <p className="text-2xl font-bold tracking-tight mt-0.5">{item.value}</p>
            </div>
          </div>
        ))}
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        {/* 每日请求趋势 */}
        <Card className="rounded-2xl">
          <CardHeader>
            <CardTitle>每日请求趋势</CardTitle>
          </CardHeader>
          <CardContent>
            {aggregatedDaily.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={aggregatedDaily}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                  <YAxis />
                  <Tooltip formatter={(value) => formatNumber(value as number)} />
                  <Line type="monotone" dataKey="request_count" stroke="#8884d8" name="请求数" />
                </LineChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[300px] items-center justify-center text-muted-foreground">暂无数据</div>
            )}
          </CardContent>
        </Card>

        {/* Token 用量趋势 */}
        <Card className="rounded-2xl">
          <CardHeader>
            <CardTitle>Token 用量趋势</CardTitle>
          </CardHeader>
          <CardContent>
            {aggregatedDaily.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={aggregatedDaily}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                  <YAxis />
                  <Tooltip formatter={(value) => formatNumber(value as number)} />
                  <Legend />
                  <Line type="monotone" dataKey="input_tokens" stroke="#8884d8" name="输入 Token" />
                  <Line type="monotone" dataKey="output_tokens" stroke="#82ca9d" name="输出 Token" />
                </LineChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[300px] items-center justify-center text-muted-foreground">暂无数据</div>
            )}
          </CardContent>
        </Card>

        {/* 模型分布 + 排行 */}
        <Card className="rounded-2xl">
          <CardHeader>
            <CardTitle>模型统计</CardTitle>
          </CardHeader>
          <CardContent>
            {models.length > 0 ? (
              <div className="space-y-4">
                <ResponsiveContainer width="100%" height={240}>
                  <PieChart>
                    <Pie
                      data={models.slice(0, 6)}
                      cx="50%"
                      cy="50%"
                      labelLine={false}
                      outerRadius={90}
                      fill="#8884d8"
                      dataKey="request_count"
                      nameKey="model"
                    >
                      {models.slice(0, 6).map((_, index) => (
                        <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                      ))}
                    </Pie>
                    <Tooltip formatter={(value) => formatNumber(value as number)} />
                    <Legend />
                  </PieChart>
                </ResponsiveContainer>
                <div className="overflow-auto max-h-48">
                  <table className="w-full text-xs">
                    <thead>
                      <tr className="border-b text-muted-foreground">
                        <th className="text-left py-1.5 font-medium">#</th>
                        <th className="text-left py-1.5 font-medium">模型</th>
                        <th className="text-right py-1.5 font-medium">请求数</th>
                        <th className="text-right py-1.5 font-medium">Token</th>
                        <th className="text-right py-1.5 font-medium">成本</th>
                      </tr>
                    </thead>
                    <tbody>
                      {models.map((m, i) => (
                        <tr key={m.model} className="border-b last:border-0">
                          <td className="py-1.5 text-muted-foreground">{i + 1}</td>
                          <td className="py-1.5 font-medium max-w-[200px] truncate" title={m.model}>{m.model}</td>
                          <td className="py-1.5 text-right">{formatNumber(m.request_count)}</td>
                          <td className="py-1.5 text-right">{formatNumber(m.input_tokens + m.output_tokens)}</td>
                          <td className="py-1.5 text-right">${formatCost(m.total_cost)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            ) : (
              <div className="flex h-[300px] items-center justify-center text-muted-foreground">暂无数据</div>
            )}
          </CardContent>
        </Card>

        {/* 渠道请求量 */}
        <Card className="rounded-2xl">
          <CardHeader>
            <CardTitle>渠道请求量</CardTitle>
          </CardHeader>
          <CardContent>
            {channels.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={channels.slice(0, 10)}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="channel_name" tick={{ fontSize: 10 }} />
                  <YAxis />
                  <Tooltip formatter={(value) => formatNumber(value as number)} />
                  <Bar dataKey="request_count" fill="#82ca9d" name="请求数" />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[300px] items-center justify-center text-muted-foreground">暂无数据</div>
            )}
          </CardContent>
        </Card>

        {/* 每日成本 */}
        <Card className="rounded-2xl md:col-span-2">
          <CardHeader>
            <CardTitle>每日成本</CardTitle>
          </CardHeader>
          <CardContent>
            {aggregatedDaily.length > 0 ? (
              <ResponsiveContainer width="100%" height={250}>
                <BarChart data={aggregatedDaily}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                  <YAxis />
                  <Tooltip formatter={(value) => `$${formatCost(value as number)}`} />
                  <Bar dataKey="total_cost" fill="#ffc658" name="成本" />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[250px] items-center justify-center text-muted-foreground">暂无数据</div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
