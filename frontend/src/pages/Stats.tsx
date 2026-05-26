import { useEffect, useState } from 'react'
import { statsApi } from '@/api'
import type { DailyStats, ModelStats, ChannelStats } from '@/api/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  LineChart,
  Line,
  CartesianGrid,
  PieChart,
  Pie,
  Cell,
  Legend,
} from 'recharts'

const COLORS = ['#8884d8', '#82ca9d', '#ffc658', '#ff7300', '#0088fe', '#00C49F']

export function Stats() {
  const [daily, setDaily] = useState<DailyStats[]>([])
  const [models, setModels] = useState<ModelStats[]>([])
  const [channels, setChannels] = useState<ChannelStats[]>([])
  const [days, setDays] = useState(30)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetchStats()
  }, [days])

  const fetchStats = async () => {
    setLoading(true)
    try {
      const [dailyData, modelsData, channelsData] = await Promise.all([
        statsApi.daily(days),
        statsApi.models(days),
        statsApi.channels(days),
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

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

  const totalRequests = daily.reduce((sum, d) => sum + d.requests, 0)
  const totalTokens = daily.reduce((sum, d) => sum + d.tokens, 0)
  const totalCost = daily.reduce((sum, d) => sum + d.cost, 0)

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">统计分析</h1>
        <div className="flex items-center gap-2">
          {[7, 14, 30, 90].map((d) => (
            <Button
              key={d}
              variant={days === d ? 'default' : 'outline'}
              size="sm"
              onClick={() => setDays(d)}
            >
              {d} 天
            </Button>
          ))}
        </div>
      </div>

      {/* 总览卡片 */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold">{formatNumber(totalRequests)}</div>
            <div className="text-sm text-muted-foreground">总请求数</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold">{formatNumber(totalTokens)}</div>
            <div className="text-sm text-muted-foreground">总 Token 数</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold">${formatCost(totalCost)}</div>
            <div className="text-sm text-muted-foreground">总成本</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold">{daily.length}</div>
            <div className="text-sm text-muted-foreground">有数据的天数</div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        {/* 每日趋势 */}
        <Card>
          <CardHeader>
            <CardTitle>每日请求趋势</CardTitle>
          </CardHeader>
          <CardContent>
            {daily.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={daily}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                  <YAxis />
                  <Tooltip formatter={(value) => formatNumber(value as number)} />
                  <Line type="monotone" dataKey="requests" stroke="#8884d8" name="请求数" />
                </LineChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[300px] items-center justify-center text-muted-foreground">
                暂无数据
              </div>
            )}
          </CardContent>
        </Card>

        {/* 模型统计 */}
        <Card>
          <CardHeader>
            <CardTitle>模型统计</CardTitle>
          </CardHeader>
          <CardContent>
            {models.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <PieChart>
                  <Pie
                    data={models.slice(0, 6)}
                    cx="50%"
                    cy="50%"
                    labelLine={false}
                    outerRadius={100}
                    fill="#8884d8"
                    dataKey="requests"
                  >
                    {models.slice(0, 6).map((_, index) => (
                      <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value) => formatNumber(value as number)} />
                  <Legend />
                </PieChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[300px] items-center justify-center text-muted-foreground">
                暂无数据
              </div>
            )}
          </CardContent>
        </Card>

        {/* 渠道统计 */}
        <Card>
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
                  <Bar dataKey="requests" fill="#82ca9d" name="请求数" />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[300px] items-center justify-center text-muted-foreground">
                暂无数据
              </div>
            )}
          </CardContent>
        </Card>

        {/* 每日成本 */}
        <Card>
          <CardHeader>
            <CardTitle>每日成本</CardTitle>
          </CardHeader>
          <CardContent>
            {daily.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={daily}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="date" tick={{ fontSize: 12 }} />
                  <YAxis />
                  <Tooltip formatter={(value) => `$${formatCost(value as number)}`} />
                  <Bar dataKey="cost" fill="#ffc658" name="成本" />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[300px] items-center justify-center text-muted-foreground">
                暂无数据
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
