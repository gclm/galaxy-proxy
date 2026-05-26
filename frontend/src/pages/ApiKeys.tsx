import { useEffect, useState } from 'react'
import { apiKeysApi } from '@/api'
import type { ApiKey } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Plus, Trash2, Copy, Key } from 'lucide-react'

export function ApiKeys() {
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([])
  const [loading, setLoading] = useState(true)
  const [newKey, setNewKey] = useState<ApiKey | null>(null)

  useEffect(() => {
    fetchApiKeys()
  }, [])

  const fetchApiKeys = async () => {
    try {
      const data = await apiKeysApi.list()
      setApiKeys(data)
    } catch (error) {
      console.error('Failed to fetch API keys:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleCreate = async () => {
    const name = prompt('请输入 API Key 名称')
    if (!name) return

    try {
      const key = await apiKeysApi.create({ name })
      setApiKeys([key, ...apiKeys])
      setNewKey(key)
    } catch (error) {
      console.error('Failed to create API key:', error)
    }
  }

  const handleDelete = async (id: string) => {
    if (!confirm('确定删除此 API Key？')) return

    try {
      await apiKeysApi.delete(id)
      setApiKeys(apiKeys.filter((k) => k.id !== id))
    } catch (error) {
      console.error('Failed to delete API key:', error)
    }
  }

  const handleToggle = async (id: string, enabled: boolean) => {
    try {
      const key = await apiKeysApi.update(id, { enabled: !enabled })
      setApiKeys(apiKeys.map((k) => (k.id === id ? key : k)))
    } catch (error) {
      console.error('Failed to update API key:', error)
    }
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
    alert('已复制到剪贴板')
  }

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">API Keys</h1>
        <Button onClick={handleCreate}>
          <Plus className="mr-2 h-4 w-4" />
          创建 API Key
        </Button>
      </div>

      {newKey && (
        <Card className="border-green-500 bg-green-50 dark:bg-green-950">
          <CardHeader>
            <CardTitle className="text-green-700 dark:text-green-300">
              API Key 创建成功
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="mb-2 text-sm text-muted-foreground">
              请立即复制保存，此密钥只会显示一次：
            </p>
            <div className="flex items-center gap-2">
              <code className="flex-1 rounded bg-background p-2 text-sm">
                {newKey.api_key}
              </code>
              <Button
                variant="outline"
                size="sm"
                onClick={() => copyToClipboard(newKey.api_key)}
              >
                <Copy className="mr-2 h-4 w-4" />
                复制
              </Button>
            </div>
            <Button
              variant="ghost"
              className="mt-2"
              onClick={() => setNewKey(null)}
            >
              我已保存，关闭提示
            </Button>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-4">
        {apiKeys.map((apiKey) => (
          <Card key={apiKey.id}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0">
              <div className="flex items-center gap-2">
                <Key className="h-5 w-5 text-muted-foreground" />
                <CardTitle className="text-lg">{apiKey.name}</CardTitle>
              </div>
              <div className="flex items-center gap-2">
                <Button
                  variant={apiKey.enabled ? 'default' : 'secondary'}
                  size="sm"
                  onClick={() => handleToggle(apiKey.id, apiKey.enabled)}
                >
                  {apiKey.enabled ? '启用' : '禁用'}
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => handleDelete(apiKey.id)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <code className="rounded bg-muted px-2 py-1">
                  {apiKey.api_key.substring(0, 20)}...
                </code>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => copyToClipboard(apiKey.api_key)}
                >
                  <Copy className="h-4 w-4" />
                </Button>
              </div>
              <div className="mt-2 text-xs text-muted-foreground">
                创建时间: {new Date(apiKey.created_at).toLocaleString()}
              </div>
            </CardContent>
          </Card>
        ))}

        {apiKeys.length === 0 && (
          <Card>
            <CardContent className="flex items-center justify-center py-8 text-muted-foreground">
              暂无 API Key，点击上方按钮创建
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  )
}
