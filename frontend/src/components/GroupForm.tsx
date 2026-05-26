import { useEffect, useState } from 'react'
import type { Channel, Group, CreateGroupRequest, CreateGroupItemRequest } from '@/api/types'
import { channelsApi } from '@/api'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Plus, Trash2 } from 'lucide-react'

interface GroupFormProps {
  group?: Group
  onSubmit: (data: CreateGroupRequest) => Promise<void>
  onCancel: () => void
}

export function GroupForm({ group, onSubmit, onCancel }: GroupFormProps) {
  const [name, setName] = useState(group?.name ?? '')
  const [matchRegex, setMatchRegex] = useState(group?.match_regex ?? '')
  const [retryEnabled, setRetryEnabled] = useState(group?.retry_enabled ?? true)
  const [maxRetries, setMaxRetries] = useState(group?.max_retries?.toString() ?? '3')
  const [firstTokenTimeout, setFirstTokenTimeout] = useState(group?.first_token_timeout_secs?.toString() ?? '30')
  const [enabled, setEnabled] = useState(group?.enabled ?? true)
  const [items, setItems] = useState<CreateGroupItemRequest[]>(
    group?.items.map((item) => ({
      channel_id: item.channel_id,
      model_name: item.model_name,
      priority: item.priority,
      weight: item.weight,
    })) ?? [{ channel_id: '', model_name: '', priority: 1, weight: 100 }]
  )
  const [channels, setChannels] = useState<Channel[]>([])
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    channelsApi.list().then(setChannels).catch(console.error)
  }, [])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSubmitting(true)

    try {
      const data: CreateGroupRequest = {
        name,
        match_regex: matchRegex || undefined,
        retry_enabled: retryEnabled,
        max_retries: parseInt(maxRetries) || 3,
        first_token_timeout_secs: parseInt(firstTokenTimeout) || 30,
        enabled,
        items: items.filter((item) => item.channel_id && item.model_name),
      }
      await onSubmit(data)
    } finally {
      setSubmitting(false)
    }
  }

  const addItem = () => setItems([...items, { channel_id: '', model_name: '', priority: 1, weight: 100 }])
  const removeItem = (index: number) => setItems(items.filter((_, i) => i !== index))
  const updateItem = (index: number, field: keyof CreateGroupItemRequest, value: string | number) => {
    const newItems = [...items]
    newItems[index] = { ...newItems[index], [field]: value }
    setItems(newItems)
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{group ? '编辑分组' : '创建分组'}</CardTitle>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit} className="space-y-6">
          <div className="space-y-4">
            <h3 className="text-sm font-medium">基本信息</h3>
            <div>
              <label className="block text-sm font-medium mb-1">分组名称 *</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                placeholder="例如：GPT-4 分组"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-1">匹配规则 (正则)</label>
              <input
                type="text"
                value={matchRegex}
                onChange={(e) => setMatchRegex(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono"
                placeholder="例如：^gpt-4"
              />
            </div>

            <div className="grid grid-cols-3 gap-4">
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="retryEnabled"
                  checked={retryEnabled}
                  onChange={(e) => setRetryEnabled(e.target.checked)}
                  className="rounded"
                />
                <label htmlFor="retryEnabled" className="text-sm">启用重试</label>
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">最大重试</label>
                <input
                  type="number"
                  value={maxRetries}
                  onChange={(e) => setMaxRetries(e.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                  disabled={!retryEnabled}
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">首字超时(秒)</label>
                <input
                  type="number"
                  value={firstTokenTimeout}
                  onChange={(e) => setFirstTokenTimeout(e.target.value)}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                />
              </div>
            </div>

            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="enabled"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
                className="rounded"
              />
              <label htmlFor="enabled" className="text-sm">启用分组</label>
            </div>
          </div>

          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-sm font-medium">分组项 (渠道 + 模型)</h3>
              <Button type="button" variant="outline" size="sm" onClick={addItem}>
                <Plus className="h-4 w-4 mr-1" /> 添加
              </Button>
            </div>
            {items.map((item, index) => (
              <div key={index} className="grid grid-cols-12 gap-2">
                <select
                  value={item.channel_id}
                  onChange={(e) => updateItem(index, 'channel_id', e.target.value)}
                  className="col-span-5 rounded-md border border-input bg-background px-3 py-2 text-sm"
                  required
                >
                  <option value="">选择渠道</option>
                  {channels.map((ch) => (
                    <option key={ch.id} value={ch.id}>{ch.name}</option>
                  ))}
                </select>
                <input
                  type="text"
                  value={item.model_name}
                  onChange={(e) => updateItem(index, 'model_name', e.target.value)}
                  className="col-span-4 rounded-md border border-input bg-background px-3 py-2 text-sm"
                  placeholder="模型名称"
                  required
                />
                <input
                  type="number"
                  value={item.priority}
                  onChange={(e) => updateItem(index, 'priority', parseInt(e.target.value) || 1)}
                  className="col-span-1 rounded-md border border-input bg-background px-3 py-2 text-sm"
                  placeholder="优先级"
                  min="1"
                />
                <input
                  type="number"
                  value={item.weight}
                  onChange={(e) => updateItem(index, 'weight', parseInt(e.target.value) || 100)}
                  className="col-span-1 rounded-md border border-input bg-background px-3 py-2 text-sm"
                  placeholder="权重"
                  min="1"
                />
                {items.length > 1 && (
                  <Button type="button" variant="ghost" size="icon" className="col-span-1" onClick={() => removeItem(index)}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
                {items.length === 1 && <div className="col-span-1" />}
              </div>
            ))}
            <div className="flex gap-4 text-xs text-muted-foreground">
              <span>优先级：数字越小越优先</span>
              <span>权重：同优先级下的随机权重</span>
            </div>
          </div>

          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" onClick={onCancel}>
              取消
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? '保存中...' : group ? '更新' : '创建'}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
