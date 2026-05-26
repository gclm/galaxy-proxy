import { useEffect, useState } from 'react'
import { channelsApi } from '@/api'
import type { Channel, CreateChannelRequest } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ChannelForm } from '@/components/ChannelForm'
import { Plus, Pencil, Trash2 } from 'lucide-react'

export function Channels() {
  const [channels, setChannels] = useState<Channel[]>([])
  const [loading, setLoading] = useState(true)
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null)
  const [showForm, setShowForm] = useState(false)

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

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

  if (showForm || editingChannel) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">
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
        <h1 className="text-3xl font-bold">渠道管理</h1>
        <Button onClick={() => setShowForm(true)}>
          <Plus className="mr-2 h-4 w-4" />
          添加渠道
        </Button>
      </div>

      <div className="grid gap-4">
        {channels.map((channel) => (
          <Card key={channel.id}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0">
              <CardTitle className="text-lg">{channel.name}</CardTitle>
              <div className="flex items-center gap-2">
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
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid gap-2 text-sm text-muted-foreground">
                <div>
                  <span className="font-medium text-foreground">端点:</span>{' '}
                  {channel.endpoints.map((ep) => ep.type).join(', ')}
                </div>
                <div>
                  <span className="font-medium text-foreground">状态:</span>{' '}
                  <span className={channel.enabled ? 'text-green-600' : 'text-red-600'}>
                    {channel.enabled ? '启用' : '禁用'}
                  </span>
                </div>
                <div>
                  <span className="font-medium text-foreground">API Keys:</span>{' '}
                  {channel.api_keys.length} 个
                </div>
                {channel.rate_limit_rpm && (
                  <div>
                    <span className="font-medium text-foreground">RPM 限制:</span>{' '}
                    {channel.rate_limit_rpm}
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        ))}

        {channels.length === 0 && (
          <Card>
            <CardContent className="flex items-center justify-center py-8 text-muted-foreground">
              暂无渠道，点击上方按钮添加
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  )
}
