import { useEffect, useState } from 'react'
import { statsApi } from '@/api'
import type { DailyStats, ModelStats, ChannelStats } from '@/api/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

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

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

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

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>每日趋势</CardTitle>
          </CardHeader>
          <CardContent>
            {daily.length > 0 ? (
              <div className="space-y-2">
                {daily.slice(-7).map((item) => (
                  <div
                    key={item.date}
                    className="flex items-center justify-between text-sm"
                  >
                    <span className="text-muted-foreground">{item.date}</span>
                    <span className="font-medium">
                      {item.requests.toLocaleString()} 请求
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-muted-foreground">暂无数据</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>模型统计</CardTitle>
          </CardHeader>
          <CardContent>
            {models.length > 0 ? (
              <div className="space-y-2">
                {models.slice(0, 5).map((item) => (
                  <div
                    key={item.model}
                    className="flex items-center justify-between text-sm"
                  >
                    <span className="text-muted-foreground truncate max-w-[200px]">
                      {item.model}
                    </span>
                    <span className="font-medium">
                      {item.requests.toLocaleString()} 请求
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-muted-foreground">暂无数据</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>渠道统计</CardTitle>
          </CardHeader>
          <CardContent>
            {channels.length > 0 ? (
              <div className="space-y-2">
                {channels.slice(0, 5).map((item) => (
                  <div
                    key={item.channel_id}
                    className="flex items-center justify-between text-sm"
                  >
                    <span className="text-muted-foreground">
                      {item.channel_name}
                    </span>
                    <span className="font-medium">
                      {item.requests.toLocaleString()} 请求
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-muted-foreground">暂无数据</p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
