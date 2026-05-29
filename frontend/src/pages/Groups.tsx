import { useCallback, useEffect, useState } from 'react'
import { groupsApi, type GroupListParams } from '@/api/groups'
import { channelsApi } from '@/api/channels'
import type { Channel, Group, CreateGroupRequest } from '@/api/types'
import { Button } from '@/components/ui/button'
import { StatusBadge } from '@/components/StatusBadge'
import { ConfirmDeleteDialog } from '@/components/ConfirmDeleteDialog'
import { useDebouncedValue } from '@/lib/hooks'
import { GroupForm } from '@/components/GroupForm'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { formatDate } from '@/lib/utils'
import {
  Plus,
  Pencil,
  Trash2,
  Search,
  RefreshCw,
  ChevronLeft,
  ChevronRight,
  ArrowUpDown,
} from 'lucide-react'

export function Groups() {
  const [groups, setGroups] = useState<Group[]>([])
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)

  const [searchInput, setSearchInput] = useState('')
  const search = useDebouncedValue(searchInput)
  const [status, setStatus] = useState<string>('')
  const [sortBy, setSortBy] = useState('created_at')
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc')
  const [page, setPage] = useState(1)
  const pageSize = 20

  const [formOpen, setFormOpen] = useState(false)
  const [editingGroup, setEditingGroup] = useState<Group | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)

  const [channels, setChannels] = useState<Channel[]>([])

  const fetchGroups = useCallback(async () => {
    setLoading(true)
    try {
      const params: GroupListParams = {
        search: search || undefined,
        status: status || undefined,
        sort_by: sortBy,
        sort_order: sortOrder,
        page,
        page_size: pageSize,
      }
      const data = await groupsApi.list(params)
      setGroups(data.items)
      setTotal(data.total)
    } catch (error) {
      console.error('Failed to fetch groups:', error)
    } finally {
      setLoading(false)
    }
  }, [search, status, sortBy, sortOrder, page])

  useEffect(() => {
    fetchGroups()
  }, [fetchGroups])

  // 搜索变化时重置页码
  useEffect(() => { setPage(1) }, [search])

  useEffect(() => {
    channelsApi.list().then(res => setChannels(res.items)).catch(console.error)
  }, [])

  const handleCreate = async (data: CreateGroupRequest) => {
    await groupsApi.create(data)
    setFormOpen(false)
    fetchGroups()
  }

  const handleUpdate = async (data: CreateGroupRequest) => {
    if (!editingGroup) return
    await groupsApi.update(editingGroup.id, data)
    setEditingGroup(null)
    setFormOpen(false)
    fetchGroups()
  }

  const handleToggleEnabled = async (group: Group) => {
    await groupsApi.update(group.id, { enabled: !group.enabled })
    fetchGroups()
  }

  const handleDelete = async () => {
    if (!deleteId) return
    await groupsApi.delete(deleteId)
    setDeleteId(null)
    fetchGroups()
  }

  const handleSort = (field: string) => {
    if (sortBy === field) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')
    } else {
      setSortBy(field)
      setSortOrder('desc')
    }
    setPage(1)
  }

  const openEdit = (group: Group) => {
    setEditingGroup(group)
    setFormOpen(true)
  }

  const openCreate = () => {
    setEditingGroup(null)
    setFormOpen(true)
  }

  const closeForm = () => {
    setFormOpen(false)
    setEditingGroup(null)
  }

  const totalPages = Math.ceil(total / pageSize)

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">配置模型分组与负载均衡策略</p>
        <Button onClick={openCreate} className="btn-primary">
          <Plus className="mr-2 h-4 w-4" />
          添加分组
        </Button>
      </div>

      {/* 筛选栏 */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <input
            type="text"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            placeholder="搜索分组名称..."
            className="input pl-9"
          />
        </div>
        <select
          value={status}
          onChange={(e) => { setStatus(e.target.value); setPage(1) }}
          className="input w-28"
        >
          <option value="">全部状态</option>
          <option value="enabled">启用</option>
          <option value="disabled">禁用</option>
        </select>
        <Button variant="outline" size="icon" onClick={fetchGroups} title="刷新">
          <RefreshCw className="h-4 w-4" />
        </Button>
      </div>

      {/* 表格 */}
      <div className="rounded-2xl border bg-card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b bg-muted/50">
                <th className="text-left px-4 py-3 font-medium">
                  <button className="inline-flex items-center gap-1 hover:text-foreground" onClick={() => handleSort('name')}>
                    名称
                    {sortBy === 'name' && <ArrowUpDown className="h-3 w-3" />}
                  </button>
                </th>
                <th className="text-left px-4 py-3 font-medium">匹配规则</th>
                <th className="text-center px-4 py-3 font-medium">渠道数</th>
                <th className="text-center px-4 py-3 font-medium">重试</th>
                <th className="text-center px-4 py-3 font-medium">状态</th>
                <th className="text-left px-4 py-3 font-medium">
                  <button className="inline-flex items-center gap-1 hover:text-foreground" onClick={() => handleSort('created_at')}>
                    创建时间
                    {sortBy === 'created_at' && <ArrowUpDown className="h-3 w-3" />}
                  </button>
                </th>
                <th className="text-center px-4 py-3 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {loading ? (
                <tr>
                  <td colSpan={7} className="text-center py-12 text-muted-foreground">加载中...</td>
                </tr>
              ) : groups.length === 0 ? (
                <tr>
                  <td colSpan={7} className="text-center py-12 text-muted-foreground">
                    {search || status ? '没有匹配的分组' : '暂无分组，点击上方按钮添加'}
                  </td>
                </tr>
              ) : (
                groups.map((group) => (
                  <tr key={group.id} className="border-b last:border-0 hover:bg-muted/30 transition-colors">
                    <td className="px-4 py-3 font-medium">{group.name}</td>
                    <td className="px-4 py-3">
                      {group.match_regex ? (
                        <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{group.match_regex}</code>
                      ) : (
                        <span className="text-muted-foreground text-xs">精确匹配</span>
                      )}
                    </td>
                    <td className="px-4 py-3 text-center text-muted-foreground">{group.items.length}</td>
                    <td className="px-4 py-3 text-center text-muted-foreground text-xs">
                      {group.retry_enabled ? `${group.max_retries} 次` : '关闭'}
                    </td>
                    <td className="px-4 py-3 text-center">
                      <StatusBadge enabled={group.enabled} onClick={() => handleToggleEnabled(group)} />
                    </td>
                    <td className="px-4 py-3 text-muted-foreground text-xs">{formatDate(group.created_at)}</td>
                    <td className="px-4 py-3">
                      <div className="flex items-center justify-center gap-1">
                        <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => openEdit(group)} title="编辑">
                          <Pencil className="h-3.5 w-3.5" />
                        </Button>
                        <Button variant="ghost" size="icon" className="h-8 w-8 text-destructive hover:text-destructive" onClick={() => setDeleteId(group.id)} title="删除">
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        {/* 分页 */}
        {total > pageSize && (
          <div className="flex items-center justify-between px-4 py-3 border-t bg-muted/30">
            <span className="text-sm text-muted-foreground">共 {total} 条</span>
            <div className="flex items-center gap-1">
              <Button variant="outline" size="icon" className="h-8 w-8" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <span className="px-3 text-sm">{page} / {totalPages}</span>
              <Button variant="outline" size="icon" className="h-8 w-8" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        )}
      </div>

      {/* 创建/编辑 Dialog */}
      <Dialog open={formOpen} onOpenChange={(open) => { if (!open) closeForm() }}>
        <DialogContent className="max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
          <DialogHeader>
            <DialogTitle>{editingGroup ? '编辑分组' : '创建分组'}</DialogTitle>
          </DialogHeader>
          <GroupForm
            group={editingGroup ?? undefined}
            channels={channels}
            onSubmit={editingGroup ? handleUpdate : handleCreate}
            onCancel={closeForm}
          />
        </DialogContent>
      </Dialog>

      {/* 删除确认 Dialog */}
      <ConfirmDeleteDialog
        open={!!deleteId}
        onOpenChange={(open) => { if (!open) setDeleteId(null) }}
        message="确定要删除此分组吗？此操作不可撤销。"
        onConfirm={handleDelete}
      />
    </div>
  )
}
