import { useState } from 'react'
import { useAuthStore } from '@/stores/auth'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Settings } from 'lucide-react'

export function Setup() {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [siteTitle, setSiteTitle] = useState('Galaxy Router')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const { setup } = useAuthStore()

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')

    if (password !== confirmPassword) {
      setError('两次输入的密码不一致')
      return
    }

    if (password.length < 8) {
      setError('密码至少 8 个字符')
      return
    }

    if (username.length < 3) {
      setError('用户名至少 3 个字符')
      return
    }

    setLoading(true)
    try {
      await setup(username, password, siteTitle)
      window.location.href = '/'
    } catch (err) {
      setError(err instanceof Error ? err.message : '初始化失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
            <Settings className="h-6 w-6 text-primary" />
          </div>
          <CardTitle className="text-2xl">系统初始化</CardTitle>
          <p className="text-sm text-muted-foreground">
            首次使用，请创建管理员账户并配置站点
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="siteTitle" className="block text-sm font-medium mb-1">
                站点标题
              </label>
              <input
                id="siteTitle"
                type="text"
                value={siteTitle}
                onChange={(e) => setSiteTitle(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                placeholder="Galaxy Router"
              />
            </div>

            <div>
              <label htmlFor="username" className="block text-sm font-medium mb-1">
                管理员用户名
              </label>
              <input
                id="username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                placeholder="至少 3 个字符"
                required
              />
            </div>

            <div>
              <label htmlFor="password" className="block text-sm font-medium mb-1">
                密码
              </label>
              <input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                placeholder="至少 8 个字符"
                required
              />
            </div>

            <div>
              <label htmlFor="confirmPassword" className="block text-sm font-medium mb-1">
                确认密码
              </label>
              <input
                id="confirmPassword"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                placeholder="再次输入密码"
                required
              />
            </div>

            {error && (
              <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
                {error}
              </div>
            )}

            <Button type="submit" className="w-full" disabled={loading}>
              {loading ? '初始化中...' : '开始使用'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
