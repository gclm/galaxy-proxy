import { useEffect, useState } from 'react'
import { authApi, settingsApi } from '@/api'
import type { SettingItem, InfraConfig } from '@/api/types'
import { Button } from '@/components/ui/button'
import { useAuthStore } from '@/stores/auth'
import { User, Shield, Settings2, TrendingUp, Activity, Sliders, Server } from 'lucide-react'

const tabs = [
  { id: 'account', label: '账户安全', icon: Shield },
  { id: 'scheduler', label: '调度策略', icon: Sliders },
  { id: 'sticky-session', label: '粘性会话', icon: TrendingUp },
  { id: 'stats', label: '统计日志', icon: Activity },
  { id: 'pricing', label: '成本定价', icon: Settings2 },
  { id: 'infra', label: '基础配置', icon: Server },
] as const

type TabId = typeof tabs[number]['id']

type FieldDef = {
  key: string
  label: string
  description?: string
  type: 'switch' | 'number' | 'text' | 'select'
  options?: { value: string; label: string }[]
  unit?: string
  min?: number
  max?: number
}

const fieldDefs: Record<string, FieldDef[]> = {
  scheduler: [
    { key: 'scheduler.top_k', label: 'Top-K 候选数量', description: '每次选择时保留的最佳候选数量', type: 'number', min: 1, max: 50 },
    { key: 'scheduler.priority', label: '优先级权重', type: 'number', min: 0, max: 5, unit: '' },
    { key: 'scheduler.load', label: '负载权重', type: 'number', min: 0, max: 5, unit: '' },
    { key: 'scheduler.queue', label: '队列权重', type: 'number', min: 0, max: 5, unit: '' },
    { key: 'scheduler.error_rate', label: '错误率权重', type: 'number', min: 0, max: 5, unit: '' },
    { key: 'scheduler.ttft', label: '首 Token 时间权重', type: 'number', min: 0, max: 5, unit: '' },
  ],
  sticky_session: [
    { key: 'sticky_session.enabled', label: '启用粘性会话', description: '同一 session_hash 路由到同一上游', type: 'switch' },
    { key: 'sticky_session.ttl_seconds', label: '会话保持时间', type: 'number', min: 60, max: 86400, unit: '秒' },
  ],
  stats: [
    { key: 'stats.log_detail_mode', label: '日志模式', type: 'select', options: [
      { value: 'all', label: '记录全部' },
      { value: 'failures_only', label: '仅失败' },
      { value: 'none', label: '关闭' },
    ] },
    { key: 'stats.cost.source', label: '定价数据源', type: 'select', options: [
      { value: 'models.dev', label: 'models.dev' },
      { value: 'local', label: '本地数据库' },
    ] },
    { key: 'stats.cost.refresh_interval_hours', label: '刷新间隔', type: 'number', min: 1, max: 168, unit: '小时' },
  ],
  pricing: [],
}

export function Settings() {
  const { user } = useAuthStore()
  const [activeTab, setActiveTab] = useState<TabId>('account')
  const [settings, setSettings] = useState<SettingItem[]>([])
  const [infra, setInfra] = useState<InfraConfig | null>(null)

  useEffect(() => {
    settingsApi.list().then(setSettings).catch(() => {})
    settingsApi.infra().then(setInfra).catch(() => {})
  }, [])

  const settingMap = Object.fromEntries(settings.map((s) => [s.key, s]))

  // score_weights 是 JSON，拆成独立字段
  const getWeightValue = (weightKey: string): string => {
    const weightsStr = settingMap['scheduler.score_weights']?.value
    if (!weightsStr) return '1.0'
    try {
      const obj = JSON.parse(weightsStr)
      const field = weightKey.replace('scheduler.', '')
      return String(obj[field] ?? 1.0)
    } catch {
      return '1.0'
    }
  }

  const handleUpdate = async (key: string, value: string) => {
    await settingsApi.update(key, value)
    setSettings((prev) => prev.map((s) => (s.key === key ? { ...s, value } : s)))
  }

  const handleWeightUpdate = async (weightName: string, rawValue: string) => {
    const val = parseFloat(rawValue)
    if (isNaN(val)) return
    const weightsStr = settingMap['scheduler.score_weights']?.value
    let obj: Record<string, number> = { priority: 1.0, load: 1.0, queue: 0.7, error_rate: 0.8, ttft: 0.5 }
    if (weightsStr) {
      try { obj = JSON.parse(weightsStr) } catch {}
    }
    obj[weightName] = val
    await settingsApi.update('scheduler.score_weights', JSON.stringify(obj))
    setSettings((prev) =>
      prev.map((s) => (s.key === 'scheduler.score_weights' ? { ...s, value: JSON.stringify(obj) } : s))
    )
  }

  return (
    <div className="max-w-4xl space-y-6">
      <div className="border-b">
        <nav className="-mb-px flex gap-6 overflow-x-auto">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 whitespace-nowrap border-b-2 px-1 py-3 text-sm font-medium transition-colors ${
                activeTab === tab.id
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              }`}
            >
              <tab.icon className="h-4 w-4" />
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      <div className="mt-6">
        {activeTab === 'account' && <AccountTab user={user} />}
        {activeTab === 'scheduler' && (
          <SchedulerTab
            settingMap={settingMap}
            getWeightValue={getWeightValue}
            onUpdate={handleUpdate}
            onWeightUpdate={handleWeightUpdate}
          />
        )}
        {activeTab === 'sticky-session' && (
          <FieldSetTab category="sticky_session" settingMap={settingMap} onUpdate={handleUpdate} />
        )}
        {activeTab === 'stats' && (
          <FieldSetTab category="stats" settingMap={settingMap} onUpdate={handleUpdate} />
        )}
        {activeTab === 'pricing' && (
          <section className="rounded-2xl border bg-card p-8 text-center">
            <p className="text-sm text-muted-foreground">定价配置暂无可用数据</p>
          </section>
        )}
        {activeTab === 'infra' && infra && <InfraTab config={infra} />}
      </div>
    </div>
  )
}

/* ── 账户安全 ── */

function AccountTab({ user }: { user: { username: string; id: string } | null }) {
  const [oldPassword, setOldPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')
  const [loading, setLoading] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    setSuccess('')
    if (newPassword !== confirmPassword) {
      setError('两次输入的新密码不一致')
      return
    }
    if (newPassword.length < 8) {
      setError('新密码至少 8 个字符')
      return
    }
    setLoading(true)
    try {
      await authApi.changePassword({ old_password: oldPassword, new_password: newPassword })
      setSuccess('密码修改成功')
      setOldPassword('')
      setNewPassword('')
      setConfirmPassword('')
    } catch (err) {
      setError(err instanceof Error ? err.message : '修改失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border bg-card p-5 space-y-3">
        <h2 className="text-sm font-medium text-muted-foreground flex items-center gap-2">
          <User className="h-4 w-4" />
          账户信息
        </h2>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-muted-foreground">用户名</span>
            <p className="font-medium">{user?.username}</p>
          </div>
          <div>
            <span className="text-muted-foreground">用户 ID</span>
            <p><code className="rounded bg-muted px-1.5 py-0.5 text-xs">{user?.id}</code></p>
          </div>
        </div>
      </section>

      <section className="rounded-2xl border bg-card p-5 space-y-4">
        <h2 className="text-sm font-medium text-muted-foreground">修改密码</h2>
        <form onSubmit={handleSubmit} className="space-y-3">
          <div>
            <label className="block text-sm font-medium mb-1">当前密码</label>
            <input type="password" value={oldPassword} onChange={(e) => setOldPassword(e.target.value)} className="input" required />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">新密码</label>
            <input type="password" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} className="input" placeholder="至少 8 个字符" required />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">确认新密码</label>
            <input type="password" value={confirmPassword} onChange={(e) => setConfirmPassword(e.target.value)} className="input" required />
          </div>
          {error && <div className="rounded-lg bg-destructive/10 p-3 text-sm text-destructive">{error}</div>}
          {success && <div className="rounded-lg bg-green-50 dark:bg-green-900/20 p-3 text-sm text-green-700 dark:text-green-400">{success}</div>}
          <Button type="submit" disabled={loading} className="btn-primary">
            {loading ? '修改中...' : '修改密码'}
          </Button>
        </form>
      </section>
    </div>
  )
}

/* ── 调度策略（权重字段拆分） ── */

function SchedulerTab({
  settingMap,
  getWeightValue,
  onUpdate,
  onWeightUpdate,
}: {
  settingMap: Record<string, SettingItem>
  getWeightValue: (key: string) => string
  onUpdate: (key: string, value: string) => Promise<void>
  onWeightUpdate: (weightName: string, value: string) => Promise<void>
}) {
  const topKField = fieldDefs.scheduler.find((f) => f.key === 'scheduler.top_k')!
  const weightFields = fieldDefs.scheduler.filter((f) => f.key !== 'scheduler.top_k')

  return (
    <section className="rounded-2xl border bg-card divide-y">
      {/* Top-K */}
      <SettingRow label={topKField.label} description={topKField.description}>
        <InlineNumberEdit
          value={settingMap[topKField.key]?.value ?? '7'}
          onSave={(v) => onUpdate(topKField.key, v)}
          min={topKField.min}
          max={topKField.max}
        />
      </SettingRow>

      {/* 权重 */}
      {weightFields.map((field) => (
        <SettingRow key={field.key} label={field.label}>
          <InlineNumberEdit
            value={getWeightValue(field.key)}
            onSave={(v) => onWeightUpdate(field.key.replace('scheduler.', ''), v)}
            min={field.min}
            max={field.max}
            step={0.1}
          />
        </SettingRow>
      ))}
    </section>
  )
}

/* ── 通用字段标签页（粘性会话、统计日志） ── */

function FieldSetTab({
  category,
  settingMap,
  onUpdate,
}: {
  category: string
  settingMap: Record<string, SettingItem>
  onUpdate: (key: string, value: string) => Promise<void>
}) {
  const fields = fieldDefs[category] ?? []

  if (fields.length === 0) {
    return (
      <section className="rounded-2xl border bg-card p-8 text-center">
        <p className="text-sm text-muted-foreground">暂无配置项</p>
      </section>
    )
  }

  return (
    <section className="rounded-2xl border bg-card divide-y">
      {fields.map((field) => (
        <SettingRow key={field.key} label={field.label} description={field.description}>
          {field.type === 'switch' ? (
            <SwitchControl
              value={settingMap[field.key]?.value === 'true'}
              onSave={(v) => onUpdate(field.key, v ? 'true' : 'false')}
            />
          ) : field.type === 'select' && field.options ? (
            <SelectControl
              value={settingMap[field.key]?.value ?? ''}
              options={field.options}
              onSave={(v) => onUpdate(field.key, v)}
            />
          ) : (
            <InlineNumberEdit
              value={settingMap[field.key]?.value ?? ''}
              onSave={(v) => onUpdate(field.key, v)}
              min={field.min}
              max={field.max}
              unit={field.unit}
            />
          )}
        </SettingRow>
      ))}
    </section>
  )
}

/* ── 基础配置（只读） ── */

function InfraTab({ config }: { config: InfraConfig }) {
  const sections = [
    {
      title: '服务器',
      items: [
        { label: '监听地址', value: config.server.host },
        { label: '端口', value: String(config.server.port) },
      ],
    },
    {
      title: '数据库',
      items: [{ label: '路径', value: config.database.path }],
    },
    {
      title: '日志',
      items: [
        { label: '级别', value: config.logging.level },
        { label: '格式', value: config.logging.format },
        { label: '输出到文件', value: config.logging.file ? '是' : '否' },
        { label: '文件路径', value: config.logging.file_path },
      ],
    },
    {
      title: '认证',
      items: [{ label: 'Token 过期时间', value: `${config.auth.token_expiry_hours} 小时` }],
    },
  ]

  return (
    <div className="space-y-4">
      {sections.map((section) => (
        <section key={section.title} className="rounded-2xl border bg-card p-5 space-y-3">
          <h2 className="text-sm font-medium text-muted-foreground">{section.title}</h2>
          <div className="grid grid-cols-2 gap-3 text-sm">
            {section.items.map((item) => (
              <div key={item.label}>
                <span className="text-muted-foreground">{item.label}</span>
                <p className="font-medium"><code className="rounded bg-muted px-1.5 py-0.5 text-xs">{item.value}</code></p>
              </div>
            ))}
          </div>
        </section>
      ))}
      <p className="text-xs text-muted-foreground">以上配置来自 config.toml，修改后需重启生效。</p>
    </div>
  )
}

/* ── 通用行组件 ── */

function SettingRow({ label, description, children }: { label: string; description?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 px-5 py-4">
      <div className="min-w-0">
        <p className="text-sm font-medium">{label}</p>
        {description && <p className="text-xs text-muted-foreground mt-0.5">{description}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

/* ── Switch 控件 ── */

function SwitchControl({ value, onSave }: { value: boolean; onSave: (v: boolean) => Promise<void> }) {
  const [pending, setPending] = useState(false)
  const toggle = async () => {
    setPending(true)
    try { await onSave(!value) } finally { setPending(false) }
  }
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      disabled={pending}
      onClick={toggle}
      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary/30 ${
        value ? 'bg-primary' : 'bg-muted'
      }`}
    >
      <span className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out ${value ? 'translate-x-5' : 'translate-x-0'}`} />
    </button>
  )
}

/* ── Select 控件 ── */

function SelectControl({ value, options, onSave }: { value: string; options: { value: string; label: string }[]; onSave: (v: string) => Promise<void> }) {
  const [pending, setPending] = useState(false)
  const handleChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    setPending(true)
    try { await onSave(e.target.value) } finally { setPending(false) }
  }
  return (
    <select
      value={value}
      onChange={handleChange}
      disabled={pending}
      className="input w-auto min-w-[140px]"
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>{opt.label}</option>
      ))}
    </select>
  )
}

/* ── 内联数字编辑控件 ── */

function InlineNumberEdit({ value, onSave, min, max, step, unit }: {
  value: string
  onSave: (v: string) => Promise<void>
  min?: number
  max?: number
  step?: number
  unit?: string
}) {
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(value)
  const [pending, setPending] = useState(false)

  useEffect(() => { if (!editing) setDraft(value) }, [value, editing])

  const save = async () => {
    const num = parseFloat(draft)
    if (isNaN(num)) return
    if (min !== undefined && num < min) return
    if (max !== undefined && num > max) return
    setPending(true)
    try {
      await onSave(String(num))
      setEditing(false)
    } finally {
      setPending(false)
    }
  }

  if (!editing) {
    return (
      <button
        type="button"
        onClick={() => { setDraft(value); setEditing(true) }}
        className="flex items-center gap-1 rounded-lg bg-muted px-3 py-1.5 text-sm font-medium hover:bg-muted/80 transition-colors"
      >
        {value}
        {unit && <span className="text-muted-foreground">{unit}</span>}
      </button>
    )
  }

  return (
    <div className="flex items-center gap-2">
      <input
        type="number"
        className="input w-24"
        value={draft}
        min={min}
        max={max}
        step={step ?? 1}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => { if (e.key === 'Enter') save(); if (e.key === 'Escape') setEditing(false) }}
        autoFocus
        disabled={pending}
      />
      {unit && <span className="text-xs text-muted-foreground">{unit}</span>}
      <Button size="sm" onClick={save} disabled={pending}>保存</Button>
      <Button size="sm" variant="ghost" onClick={() => setEditing(false)}>取消</Button>
    </div>
  )
}
