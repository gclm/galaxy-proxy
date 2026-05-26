import { useEffect, useState } from 'react'
import { statsApi } from '@/api'
import type { StatsOverview } from '@/api/types'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Activity, Coins, MessageSquare } from 'lucide-react'

export function Dashboard() {
  const [overview, setOverview] = useState<StatsOverview | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const fetchOverview = async () => {
      try {
        const data = await statsApi.overview()
        setOverview(data)
      } catch (error) {
        console.error('Failed to fetch overview:', error)
      } finally {
        setLoading(false)
      }
    }

    fetchOverview()
  }, [])

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">仪表盘</h1>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">总请求数</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {overview?.total_requests?.toLocaleString() ?? 0}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">总 Token 数</CardTitle>
            <MessageSquare className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {overview?.total_tokens?.toLocaleString() ?? 0}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">总成本</CardTitle>
            <Coins className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              ${overview?.total_cost?.toFixed(4) ?? '0.0000'}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
