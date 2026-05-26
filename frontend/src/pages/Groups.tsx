import { useEffect, useState } from 'react'
import { groupsApi } from '@/api'
import type { Group, CreateGroupRequest } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { GroupForm } from '@/components/GroupForm'
import { Plus, Pencil, Trash2, Layers } from 'lucide-react'

export function Groups() {
  const [groups, setGroups] = useState<Group[]>([])
  const [loading, setLoading] = useState(true)
  const [editingGroup, setEditingGroup] = useState<Group | null>(null)
  const [showForm, setShowForm] = useState(false)

  useEffect(() => {
    fetchGroups()
  }, [])

  const fetchGroups = async () => {
    try {
      const data = await groupsApi.list()
      setGroups(data)
    } catch (error) {
      console.error('Failed to fetch groups:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleCreate = async (data: CreateGroupRequest) => {
    await groupsApi.create(data)
    setShowForm(false)
    fetchGroups()
  }

  const handleUpdate = async (data: CreateGroupRequest) => {
    if (!editingGroup) return
    await groupsApi.update(editingGroup.id, data)
    setEditingGroup(null)
    fetchGroups()
  }

  const handleDelete = async (id: string) => {
    if (!confirm('确定删除此分组？')) return
    try {
      await groupsApi.delete(id)
      setGroups(groups.filter((g) => g.id !== id))
    } catch (error) {
      console.error('Failed to delete group:', error)
    }
  }

  if (loading) {
    return <div className="flex items-center justify-center h-full">加载中...</div>
  }

  if (showForm || editingGroup) {
    return (
      <div className="space-y-6">
        <h1 className="text-3xl font-bold">
          {editingGroup ? '编辑分组' : '创建分组'}
        </h1>
        <GroupForm
          group={editingGroup ?? undefined}
          onSubmit={editingGroup ? handleUpdate : handleCreate}
          onCancel={() => {
            setShowForm(false)
            setEditingGroup(null)
          }}
        />
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold">分组管理</h1>
        <Button onClick={() => setShowForm(true)}>
          <Plus className="mr-2 h-4 w-4" />
          添加分组
        </Button>
      </div>

      <div className="grid gap-4">
        {groups.map((group) => (
          <Card key={group.id}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0">
              <div className="flex items-center gap-2">
                <Layers className="h-5 w-5 text-muted-foreground" />
                <CardTitle className="text-lg">{group.name}</CardTitle>
              </div>
              <div className="flex items-center gap-2">
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => setEditingGroup(group)}
                >
                  <Pencil className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => handleDelete(group.id)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid gap-2 text-sm text-muted-foreground">
                {group.match_regex && (
                  <div>
                    <span className="font-medium text-foreground">匹配规则:</span>{' '}
                    <code className="rounded bg-muted px-1 py-0.5">
                      {group.match_regex}
                    </code>
                  </div>
                )}
                <div>
                  <span className="font-medium text-foreground">分组项:</span>{' '}
                  {group.items.length} 个渠道
                </div>
                <div>
                  <span className="font-medium text-foreground">重试:</span>{' '}
                  {group.retry_enabled
                    ? `启用 (最多 ${group.max_retries} 次)`
                    : '禁用'}
                </div>
                <div>
                  <span className="font-medium text-foreground">状态:</span>{' '}
                  <span className={group.enabled ? 'text-green-600' : 'text-red-600'}>
                    {group.enabled ? '启用' : '禁用'}
                  </span>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}

        {groups.length === 0 && (
          <Card>
            <CardContent className="flex items-center justify-center py-8 text-muted-foreground">
              暂无分组，点击上方按钮添加
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  )
}
